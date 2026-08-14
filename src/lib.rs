//! hmleq — parser for the Hancom HWP(한글) equation script language,
//! with a LaTeX emitter.
//!
//! The language is the script syntax of the HWP equation editor
//! (`x = {-b +- sqrt{b^2 - 4ac}} over {2a}` and friends). See
//! `docs/REFERENCE.md` for the language and `docs/DESIGN.md` for the
//! architecture and the exact LaTeX mappings.
//!
//! ```
//! assert_eq!(hmleq::eq_to_latex("1 over 2").unwrap(), r"\frac{1}{2}");
//! assert_eq!(hmleq::eq_to_latex("sin x").unwrap(), r"\sin x");
//! assert_eq!(hmleq::eq_to_latex("sinx").unwrap(), "sinx");
//! ```

pub mod ast;
pub mod error;
pub mod latex;
pub mod lexer;
pub mod parser;
pub mod symbols;
pub mod token;

pub use ast::Node;
pub use error::ParseError;

/// Parse an equation script into an AST.
pub fn parse(src: &str) -> Result<Node, ParseError> {
    parser::parse(src)
}

/// Render an AST as LaTeX.
pub fn to_latex(node: &Node) -> String {
    latex::to_latex(node)
}

/// Convenience: parse an equation script and render it as LaTeX.
pub fn eq_to_latex(src: &str) -> Result<String, ParseError> {
    Ok(to_latex(&parse(src)?))
}
