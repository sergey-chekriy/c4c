mod compiler;
mod diagnostic;
mod lexer;
mod parser;
mod source;
use std::{env, fs, path::Path};

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        return usage();
    }
    let cmd = &args[1];
    let input = &args[2];
    let ws = compiler::compile_file(input)?;
    match cmd.as_str() {
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
        _ => return usage(),
    }
    Ok(())
}

fn arg_value<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.windows(2).find(|w| w[0] == key).map(|w| w[1].as_str())
}

fn usage() -> Result<(), String> {
    Err(
        "usage: c4c <validate|inspect|export> <workspace.dsl> [--format mermaid] [--out out]"
            .into(),
    )
}
