mod archi_native;
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
use std::{
    env,
    path::{Path, PathBuf},
};

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.get(1).is_some_and(|argument| argument == "archi") {
        return run_archi(&args);
    }
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
            if let Some(sidecar) = arg_value(&args, "--archi-sidecar") {
                exporters::export_with_archi_sidecar(
                    &ws,
                    format,
                    Path::new(out),
                    exporters::ExportOptions { strict_safe },
                    Path::new(sidecar),
                    Path::new(input),
                )?;
            } else {
                exporters::export(
                    &ws,
                    format,
                    Path::new(out),
                    exporters::ExportOptions { strict_safe },
                )?;
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

fn run_archi(args: &[String]) -> Result<(), String> {
    match args {
        [_, _, action, input, ..] if action == "import" => {
            let output =
                arg_value(args, "--out").ok_or("archi import requires --out <workspace.dsl>")?;
            let default_sidecar = Path::new(output).with_extension("archi-sidecar.json");
            let sidecar = arg_value(args, "--sidecar")
                .map(PathBuf::from)
                .unwrap_or(default_sidecar);
            archi_native::import(Path::new(input), Path::new(output), &sidecar)?;
            println!(
                "OK: imported {} to {} with {}",
                input,
                output,
                sidecar.display()
            );
        }
        [_, _, action, a, b] if action == "diff" => {
            archi_native::diff_files(Path::new(a), Path::new(b))?;
            println!("OK: Archi native models are canonically equivalent");
        }
        [_, _, action, a, b, flag] if action == "diff" && flag == "--semantic" => {
            archi_native::semantic_diff_files(Path::new(a), Path::new(b))?;
            println!("OK: Archi native models are semantically equivalent");
        }
        [_, _, action, input, ..] if action == "roundtrip" => {
            let work_dir = PathBuf::from(
                arg_value(args, "--work-dir").ok_or("archi roundtrip requires --work-dir <dir>")?,
            );
            let dsl = work_dir.join("workspace.dsl");
            let sidecar = work_dir.join("workspace.archi-sidecar.json");
            let out = work_dir.join("out");
            archi_native::import(Path::new(input), &dsl, &sidecar)?;
            let workspace = compiler::compile_file(
                dsl.to_str()
                    .ok_or("roundtrip work directory is not valid UTF-8")?,
            )?;
            compiler::validate(&workspace)?;
            exporters::export_with_archi_sidecar(
                &workspace,
                "archi",
                &out,
                exporters::ExportOptions { strict_safe: true },
                &sidecar,
                &dsl,
            )?;
            archi_native::diff_files(Path::new(input), &out.join("workspace.archimate"))?;
            println!("OK: Archi native models are canonically equivalent");
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
        "usage: c4c <validate|inspect> <workspace.dsl> [--strict-safe] [--allow-network]\n       c4c export <workspace.dsl> [--format json|mermaid|d2|plantuml|c4plantuml|dot|drawio|archimate|archi|archi-native|archimate-native|html|svg|png] [--out out] [--archi-sidecar file] [--strict-safe]\n       c4c archi import <input.archimate> --out <workspace.dsl> [--sidecar file]\n       c4c archi roundtrip <input.archimate> --work-dir <dir>\n       c4c archi diff <a.archimate> <b.archimate> [--semantic]\n       c4c docs <workspace.dsl> [--out site] [--strict-safe]\n       c4c adr list <workspace.dsl> [--strict-safe]"
            .into(),
    )
}
