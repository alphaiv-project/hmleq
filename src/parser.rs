//! Recursive-descent parser for the HWP equation script.
//! Grammar and binding rules: docs/DESIGN.md §3.

use crate::ast::Node;
use crate::error::ParseError;

/// Parse an equation script into an AST (lexes internally).
pub fn parse(src: &str) -> Result<Node, ParseError> {
    let _ = src;
    todo!("agent(parser): implement per docs/DESIGN.md §3")
}
