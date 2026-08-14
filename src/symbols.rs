//! Keyword tables for the HWP equation script.
//!
//! Lookup contract (docs/DESIGN.md §2): whole-word matching only — the parser
//! hands `lookup` a complete word cut at token boundaries; it is NEVER called
//! on a prefix. Resolution order: exact case-sensitive table first, then the
//! case-insensitive table keyed by the ASCII-lowercased word. `None` means the
//! word is an ordinary italic identifier.

/// How sub/superscripts attach to a symbol (docs/DESIGN.md §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptPos {
    /// Like any ordinary term (`\int`, `\sin`, plain letters).
    Normal,
    /// Below/above in display style: emit the plain LaTeX command and let TeX
    /// place the limits (`\sum`, `\lim`).
    Below,
    /// Forced beside: append `\nolimits` before the scripts (`smallsum`, `Lim`).
    Beside,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymKind {
    /// Ordinary symbol (Greek letters, ∞, ∂, ′, …).
    Ord,
    /// Binary operator (×, ·, ±, …).
    Bin,
    /// Relation (≤, ∈, ≐, …).
    Rel,
    /// Large operator (∑, ∫, ⋃, …).
    BigOp,
    /// Upright function name (sin, log, lim, …).
    Func,
    /// Accent / decoration (vec, hat, …) — consumes the following term.
    Accent,
}

/// One keyword symbol.
#[derive(Debug, PartialEq, Eq)]
pub struct SymDef {
    /// Canonical name as documented in docs/REFERENCE.md.
    pub name: &'static str,
    pub kind: SymKind,
    /// Primary LaTeX. For `Accent` this is a one-argument macro (`\vec`).
    pub latex: &'static str,
    /// `BigOp` only: LaTeX emitted when the operator carries no scripts
    /// (`union` → `\cup` vs `\bigcup`). `None` elsewhere.
    pub bin_latex: Option<&'static str>,
    pub scripts: ScriptPos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatrixKind {
    /// `matrix` — no fences.
    Plain,
    /// `pmatrix` — parentheses.
    Paren,
    /// `bmatrix` — square brackets.
    Bracket,
    /// `dmatrix` — vertical bars (determinant).
    Det,
    /// `cases` — left brace only.
    Cases,
    /// `pile` / `lpile` / `rpile` — vertical stack (centre/left/right).
    Pile,
    LPile,
    RPile,
    /// `eqalign` — multi-line alignment on `&`.
    EqAlign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleKind {
    /// `rm`
    Roman,
    /// `it`
    Italic,
    /// `bold`
    Bold,
    /// `rmbold`
    RomanBold,
}

/// Structural keywords the parser interprets itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmd {
    Over,
    Atop,
    Choose,
    Binom,
    Sqrt,
    Root,
    /// `of` — only meaningful inside a `root … of …` form; the parser treats
    /// it as an identifier elsewhere.
    Of,
    Left,
    Right,
    Matrix(MatrixKind),
    Style(StyleKind),
    Not,
    /// `buildrel` (and its alias `rel`).
    BuildRel,
    /// `big`/`Big`/`bigg`/`Bigg` — payload is the LaTeX size command
    /// (`"\\big"`, `"\\Big"`, `"\\bigg"`, `"\\Bigg"`).
    Big(&'static str),
    /// `sup` keyword — same as `^`.
    Sup,
    /// `sub` keyword — same as `_`.
    Sub,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Keyword {
    Sym(&'static SymDef),
    Cmd(Cmd),
}

/// Look up a complete word. Exact (case-sensitive) table first, then the
/// case-insensitive table via ASCII lowercasing. `None` → italic identifier.
pub fn lookup(word: &str) -> Option<Keyword> {
    let _ = word;
    todo!("agent(symbols): implement per docs/DESIGN.md §2 and the symbol tables in §5")
}
