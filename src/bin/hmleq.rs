//! CLI: convert an HWP equation script to LaTeX.
//!
//! Usage:
//!   hmleq '1 over 2'          # arguments joined with spaces
//!   echo '1 over 2' | hmleq   # or read from stdin

use std::io::Read;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
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

    match hmleq::eq_to_latex(src.trim()) {
        Ok(latex) => {
            println!("{latex}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
