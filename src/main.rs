mod compiler;
mod diagnostic;
mod documentation;
mod exporters;
mod handwritten_parser;
mod lexer;
mod parser;
mod preprocessor;
mod source;
mod tree_sitter_parser;
use std::{env, path::Path};

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let (cmd, input) = match args.as_slice() {
        [_, command, input, ..] if command != "adr" => (command.as_str(), input.as_str()),
        [_, command, action, input, ..] if command == "adr" && action == "list" => {
            ("adr-list", input.as_str())
        }
        _ => return usage(),
    };
    let allow_network = args.iter().any(|arg| arg == "--allow-network");
    let strict_safe = args.iter().any(|arg| arg == "--strict-safe");
    if allow_network && strict_safe {
        return Err("--strict-safe conflicts with --allow-network".into());
    }
    let ws = if allow_network || strict_safe {
        compiler::compile_file_with_options(
            input,
            compiler::CompileOptions {
                allow_network,
                strict_safe,
            },
        )?
    } else {
        compiler::compile_file(input)?
    };
    if let Some(warnings) = compiler::warnings(&ws) {
        eprintln!("{warnings}");
    }
    match cmd {
        "validate" => {
            compiler::validate(&ws)?;
            println!(
                "OK: {} elements, {} relationships, {} views",
                ws.elements.len(),
                ws.relationships.len(),
                ws.views.len()
            );
        }
        "inspect" => {
            compiler::validate(&ws)?;
            println!("{}", compiler::inspect(&ws));
        }
        "export" => {
            compiler::validate(&ws)?;
            let format = arg_value(&args, "--format").unwrap_or("mermaid");
            let out = arg_value(&args, "--out").unwrap_or("out");
            exporters::export(
                &ws,
                format,
                Path::new(out),
                exporters::ExportOptions { strict_safe },
            )?;
        }
        "docs" => {
            compiler::validate(&ws)?;
            compiler::export_site(&ws, Path::new(arg_value(&args, "--out").unwrap_or("site")))?;
        }
        "adr-list" => {
            compiler::validate(&ws)?;
            print!("{}", compiler::adr_list(&ws));
        }
        _ => return usage(),
    }
    Ok(())
}

fn arg_value<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.windows(2).find(|w| w[0] == key).map(|w| w[1].as_str())
}

fn usage() -> Result<(), String> {
    Err(
        "usage: c4c <validate|inspect> <workspace.dsl> [--strict-safe] [--allow-network]\n       c4c export <workspace.dsl> [--format json|mermaid|d2|plantuml|c4plantuml|dot|drawio|archimate|archi|archi-native|archimate-native|html|svg|png] [--out out] [--strict-safe]\n       c4c docs <workspace.dsl> [--out site] [--strict-safe]\n       c4c adr list <workspace.dsl> [--strict-safe]"
            .into(),
    )
}
