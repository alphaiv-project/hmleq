//! CLI: convert an HWP equation script to LaTeX, or dump the AST as JSON.

use std::io::Read;
use std::process::ExitCode;

use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Print the AST as JSON instead of LaTeX.
    #[arg(long)]
    json: bool,
    /// Equation script (arguments are joined with spaces); reads stdin when omitted.
    #[arg(allow_hyphen_values = true)]
    script: Vec<String>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let src = if cli.script.is_empty() {
        let mut s = String::new();
        if std::io::stdin().read_to_string(&mut s).is_err() {
            eprintln!("error: could not read stdin");
            return ExitCode::FAILURE;
        }
        s
    } else {
        cli.script.join(" ")
    };

    let ast = match hmleq::parse(src.trim()) {
        Ok(ast) => ast,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    if cli.json {
        match serde_json::to_string(&ast) {
            Ok(json) => println!("{json}"),
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        println!("{}", hmleq::to_latex(&ast));
    }
    ExitCode::SUCCESS
}
