//! CLI: convert an HWP equation script to LaTeX (or, with `--json`, dump the
//! AST as JSON — requires the `json` feature).
//!
//! Usage:
//!   hmleq '1 over 2'            # arguments joined with spaces → LaTeX
//!   echo '1 over 2' | hmleq     # or read from stdin
//!   hmleq --json '1 over 2'     # AST as JSON (feature "json")

use std::io::Read;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let json = matches!(args.first().map(String::as_str), Some("--json"));
    if json {
        args.remove(0);
    }

    let src = if args.is_empty() {
        let mut s = String::new();
        if std::io::stdin().read_to_string(&mut s).is_err() {
            eprintln!("error: could not read stdin");
            return ExitCode::FAILURE;
        }
        s
    } else {
        args.join(" ")
    };

    let ast = match hmleq::parse(src.trim()) {
        Ok(ast) => ast,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    if json {
        return print_json(&ast);
    }
    println!("{}", hmleq::to_latex(&ast));
    ExitCode::SUCCESS
}

#[cfg(feature = "json")]
fn print_json(ast: &hmleq::Node) -> ExitCode {
    match serde_json::to_string(ast) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(feature = "json"))]
fn print_json(_ast: &hmleq::Node) -> ExitCode {
    eprintln!("error: this binary was built without the \"json\" feature");
    ExitCode::FAILURE
}
