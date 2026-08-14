//! AST for the HWP equation script. Produced by `parser`, consumed by `latex`.

use crate::symbols::{MatrixKind, StyleKind, SymDef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpaceKind {
    /// `~` — one full space.
    Full,
    /// `` ` `` — quarter space.
    Quarter,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// A horizontal sequence of terms. Also used for the (invisible) content
    /// of a `{ … }` group — grouping affects binding, not rendering.
    Row(Vec<Node>),
    /// A number literal, verbatim (`"3.14"`).
    Number(String),
    /// A non-keyword word — rendered in math italic, verbatim (`"sinx"`).
    Ident(String),
    /// `"quoted"` text — rendered upright, verbatim.
    Text(String),
    /// A literal operator/punctuation token, including unresolved ligatures
    /// (`"+"`, `"="`, `"("`, `"+-"`, `"!="`, …). The emitter maps ligatures.
    Op(String),
    /// A keyword symbol: Greek letter, arrow, relation, big operator,
    /// upright function, … (everything in the tables except accents).
    Symbol(&'static SymDef),
    /// `A over B` (bar = true) / `A atop B` (bar = false).
    Frac { num: Box<Node>, den: Box<Node>, bar: bool },
    /// `n choose k` / `binom {n} {k}`.
    Binom { top: Box<Node>, bottom: Box<Node> },
    /// `sqrt {x}`.
    Sqrt(Box<Node>),
    /// `root n of {x}`.
    Root { degree: Box<Node>, radicand: Box<Node> },
    /// Sub/superscripts attached to a base. At least one of `sub`/`sup` is set.
    Script { base: Box<Node>, sub: Option<Box<Node>>, sup: Option<Box<Node>> },
    /// `LEFT ( … RIGHT )`. `left`/`right` hold the raw delimiter as written
    /// (`"("`, `"["`, `"|"`, `"||"`, `"<"`, `">"`, `"."`); the emitter maps them.
    Delimited { left: String, right: String, body: Box<Node> },
    /// All row/column structures: matrix/pmatrix/bmatrix/dmatrix/cases/
    /// pile/lpile/rpile/eqalign.
    Matrix { kind: MatrixKind, rows: Vec<Vec<Node>> },
    /// Decoration applied to the following term: `vec A`, `hat x`, …
    Accent { accent: &'static SymDef, base: Box<Node> },
    /// Font-style switch applying to the rest of the enclosing group.
    Style { style: StyleKind, body: Box<Node> },
    /// `buildrel T over B` (alias `rel`) — stack `T` above `B`.
    BuildRel { top: Box<Node>, base: Box<Node> },
    /// `not X` — negating slash over the following term (`not =` → ≠).
    Not(Box<Node>),
    /// `bigg X` etc. — enlarge the following delimiter/operator.
    /// `size` is the LaTeX size command (`"\\bigg"`).
    Big { size: &'static str, arg: Box<Node> },
    /// `~` / `` ` `` — printed space.
    Space(SpaceKind),
    /// `#` outside a matrix context — line break.
    Newline,
    /// `&` outside a matrix context — alignment point.
    Align,
}

impl Node {
    /// Collapse a parsed sequence: one item stays itself, otherwise a `Row`.
    pub fn seq(mut items: Vec<Node>) -> Node {
        if items.len() == 1 { items.pop().unwrap() } else { Node::Row(items) }
    }
}
