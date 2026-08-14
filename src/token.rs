//! Token types produced by the lexer. See `docs/DESIGN.md` §1 for the rules.

/// A token with its byte span in the source.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    /// Byte offset of the first byte of the token.
    pub start: usize,
    /// Byte offset one past the last byte of the token.
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// `{` — begins an invisible group.
    LBrace,
    /// `}`
    RBrace,
    /// `^` — superscript marker.
    Caret,
    /// `_` — subscript marker.
    Underscore,
    /// `#` — line break / matrix row separator.
    Hash,
    /// `&` — alignment / matrix column separator.
    Amp,
    /// `~` — printed full-width space.
    Tilde,
    /// `` ` `` — printed quarter-width space.
    Backquote,
    /// `"..."` — verbatim text (quotes stripped, no escape processing).
    Quoted(String),
    /// Maximal run of alphabetic characters. Keyword lookup happens later,
    /// on the whole word (never on a prefix).
    Word(String),
    /// Maximal run of ASCII digits, allowing one interior `.` flanked by
    /// digits on both sides (e.g. `3.14`).
    Number(String),
    /// A single operator/punctuation character, or one of the multi-char
    /// ligatures listed in DESIGN.md §1 (`+-`, `-+`, `!=`, `<=`, `>=`,
    /// `<<`, `>>`, `||`).
    Op(String),
}
