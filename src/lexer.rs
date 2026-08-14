//! Tokenizer for the HWP equation script. Rules: docs/DESIGN.md §1.

use crate::error::ParseError;
use crate::token::Token;

/// Tokenize `src` into a flat token stream.
pub fn lex(src: &str) -> Result<Vec<Token>, ParseError> {
    let _ = src;
    todo!("agent(lexer): implement per docs/DESIGN.md §1")
}
