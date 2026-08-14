//! LaTeX emitter. Exact output templates: docs/DESIGN.md §5.

use crate::ast::Node;

/// Render an AST as LaTeX, following the canonical templates in DESIGN.md §5
/// (Row children joined with single spaces, macro arguments always braced, …).
pub fn to_latex(node: &Node) -> String {
    let _ = node;
    todo!("agent(latex): implement per docs/DESIGN.md §5")
}
