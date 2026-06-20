mod compiler;
mod diagnostic;
mod documentation;
mod handwritten_parser;
mod lexer;
mod parser;
mod preprocessor;
mod source;
mod tree_sitter_parser;
use std::{env, fs, path::Path};

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
            fs::create_dir_all(out).map_err(|e| format!("cannot create {out}: {e}"))?;
            match format {
                "mermaid" | "mmd" => compiler::export_mermaid(&ws, Path::new(out))?,
                _ => {
                    return Err(format!(
                        "unsupported format '{format}' in M1; supported: mermaid"
                    ))
                }
            }
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
        "usage: c4c <validate|inspect|export|docs> <workspace.dsl> [--format mermaid] [--out out] [--strict-safe] [--allow-network]\n       c4c adr list <workspace.dsl> [--strict-safe]"
            .into(),
    )
}
