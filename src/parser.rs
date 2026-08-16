//! Recursive-descent parser for the HWP equation script.

use crate::ast::{Node, SpaceKind};
use crate::error::ParseError;
use crate::lexer::lex;
use crate::symbols::{lookup, Cmd, Keyword, MatrixKind, SymKind};
use crate::token::{Token, TokenKind};

/// Parse an equation script into an AST (lexes internally).
pub fn parse(src: &str) -> Result<Node, ParseError> {
    let mut parser = Parser {
        tokens: lex(src)?,
        pos: 0,
        src_len: src.len(),
    };
    let items = parser.sequence(Stop::Eof)?;
    debug_assert_eq!(
        parser.pos,
        parser.tokens.len(),
        "the top-level sequence must run to EOF"
    );
    Ok(Node::seq(items))
}

/// What terminates the sequence currently being parsed. Every variant also
/// stops at EOF; whether that is legal is decided by the caller, which knows
/// what it opened (`unclosed '{'`, `LEFT without RIGHT`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stop {
    /// Top level -- only EOF ends it.
    Eof,
    /// `}` -- a braced group.
    Brace,
    /// The `RIGHT` keyword -- a `LEFT ... RIGHT` body.
    Right,
    /// `}`, `#` or `&` -- one matrix cell. Row/column splitting is depth-local,
    /// so nested groups and matrices are parsed with their own `Stop`.
    Cell,
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// Byte length of the source: the position reported for errors at EOF.
    src_len: usize,
}

/// Keyword behind a token, or `None` for anything that is not a `Word`.
fn keyword_of(tok: &Token) -> Option<Keyword> {
    match &tok.kind {
        TokenKind::Word(w) => lookup(w),
        _ => None,
    }
}

fn is_infix(c: &Cmd) -> bool {
    matches!(c, Cmd::Over | Cmd::Atop | Cmd::Choose)
}

fn infix_name(c: Cmd) -> &'static str {
    match c {
        Cmd::Over => "over",
        Cmd::Atop => "atop",
        _ => "choose",
    }
}

/// The delimiters DESIGN.md §5 maps for `\left`/`\right`; the same set
/// validates `LEFT`/`RIGHT` here.
fn is_delimiter(s: &str) -> bool {
    matches!(s, "(" | ")" | "[" | "]" | "|" | "||" | "<" | ">" | ".")
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    /// Byte offset of the token about to be read (EOF -> end of source).
    fn here(&self) -> usize {
        self.peek().map_or(self.src_len, |t| t.start)
    }

    fn peek_cmd(&self) -> Option<Cmd> {
        match keyword_of(self.peek()?)? {
            Keyword::Cmd(c) => Some(c),
            Keyword::Sym(_) => None,
        }
    }

    fn at_stop(&self, stop: Stop) -> bool {
        let Some(tok) = self.peek() else { return true };
        match stop {
            Stop::Eof => false,
            Stop::Brace => matches!(tok.kind, TokenKind::RBrace),
            Stop::Cell => {
                matches!(
                    tok.kind,
                    TokenKind::RBrace | TokenKind::Hash | TokenKind::Amp
                )
            }
            Stop::Right => self.peek_cmd() == Some(Cmd::Right),
        }
    }

    // -- sequence -----------------------------------------------------------

    /// A sequence of terms up to `stop`. The stop token is left unconsumed for
    /// the caller, which is the only one that knows what to do with it.
    fn sequence(&mut self, stop: Stop) -> Result<Vec<Node>, ParseError> {
        let mut items: Vec<Node> = Vec::new();
        while !self.at_stop(stop) {
            // `over`/`atop`/`choose` are infix: they grab the item already
            // parsed, which makes them left-associative for free.
            if let Some(cmd) = self.peek_cmd().filter(is_infix) {
                let name = infix_name(cmd);
                let at = self.here();
                self.pos += 1;
                let left = items
                    .pop()
                    .ok_or_else(|| ParseError::new(format!("{name}: missing left operand"), at))?;
                if self.at_stop(stop) {
                    return Err(ParseError::new(
                        format!("{name}: missing right operand"),
                        at,
                    ));
                }
                let right = self.scripted(stop)?;
                items.push(match cmd {
                    Cmd::Over => Node::Frac {
                        num: Box::new(left),
                        den: Box::new(right),
                        bar: true,
                    },
                    Cmd::Atop => Node::Frac {
                        num: Box::new(left),
                        den: Box::new(right),
                        bar: false,
                    },
                    _ => Node::Binom {
                        top: Box::new(left),
                        bottom: Box::new(right),
                    },
                });
                continue;
            }
            items.push(self.scripted(stop)?);
        }
        Ok(items)
    }

    // -- scripted term ------------------------------------------------------

    /// One prefixed-primary followed by any number of script markers
    /// (`^`, `_`, `sup`, `sub`), each taking exactly one prefixed-primary.
    fn scripted(&mut self, stop: Stop) -> Result<Node, ParseError> {
        let base = self.primary(stop)?;
        let mut sub: Option<Box<Node>> = None;
        let mut sup: Option<Box<Node>> = None;

        while let Some((is_sup, at)) = self.peek_script_marker() {
            self.pos += 1;
            let (slot, what, duplicate) = if is_sup {
                (&mut sup, "superscript", "duplicate superscript")
            } else {
                (&mut sub, "subscript", "duplicate subscript")
            };
            if slot.is_some() {
                return Err(ParseError::new(duplicate, at));
            }
            *slot = Some(Box::new(self.operand_primary(stop, what)?));
        }

        if sub.is_none() && sup.is_none() {
            Ok(base)
        } else {
            Ok(Node::Script {
                base: Box::new(base),
                sub,
                sup,
            })
        }
    }

    /// `Some((is_superscript, offset))` when a script marker is next.
    fn peek_script_marker(&self) -> Option<(bool, usize)> {
        let tok = self.peek()?;
        let is_sup = match &tok.kind {
            TokenKind::Caret => true,
            TokenKind::Underscore => false,
            TokenKind::Word(w) => match lookup(w) {
                Some(Keyword::Cmd(Cmd::Sup)) => true,
                Some(Keyword::Cmd(Cmd::Sub)) => false,
                _ => return None,
            },
            _ => return None,
        };
        Some((is_sup, tok.start))
    }

    // -- operands -----------------------------------------------------------

    /// Reject a construct named `what` whose operand ran into the stop token.
    fn require(&self, stop: Stop, what: &str) -> Result<(), ParseError> {
        if self.at_stop(stop) {
            Err(ParseError::new(
                format!("{what}: missing operand"),
                self.here(),
            ))
        } else {
            Ok(())
        }
    }

    /// A mandatory scripted-term operand (`sqrt`, `not`, `root`, `binom`, ...).
    fn operand(&mut self, stop: Stop, what: &str) -> Result<Node, ParseError> {
        self.require(stop, what)?;
        self.scripted(stop)
    }

    /// A mandatory prefixed-primary operand -- accents, `big`, and script
    /// arguments bind tighter than scripts do.
    fn operand_primary(&mut self, stop: Stop, what: &str) -> Result<Node, ParseError> {
        self.require(stop, what)?;
        self.primary(stop)
    }

    // -- prefixed primary ---------------------------------------------------

    fn primary(&mut self, stop: Stop) -> Result<Node, ParseError> {
        let Some(tok) = self.peek() else {
            return Err(ParseError::new("unexpected end of input", self.src_len));
        };
        let at = tok.start;
        let kind = tok.kind.clone();
        self.pos += 1;

        Ok(match kind {
            // A group is invisible but binds as one term: its collapsed content.
            TokenKind::LBrace => {
                let items = self.sequence(Stop::Brace)?;
                if !matches!(self.peek().map(|t| &t.kind), Some(TokenKind::RBrace)) {
                    return Err(ParseError::new("unclosed '{'", at));
                }
                self.pos += 1;
                Node::seq(items)
            }
            TokenKind::RBrace => return Err(ParseError::new("unmatched '}'", at)),
            TokenKind::Caret => return Err(ParseError::new("superscript without a base", at)),
            TokenKind::Underscore => return Err(ParseError::new("subscript without a base", at)),
            TokenKind::Hash => Node::Newline,
            TokenKind::Amp => Node::Align,
            TokenKind::Tilde => Node::Space(SpaceKind::Full),
            TokenKind::Backquote => Node::Space(SpaceKind::Quarter),
            TokenKind::Number(s) => Node::Number(s),
            TokenKind::Quoted(s) => Node::Text(s),
            // Ligatures stay verbatim; the emitter maps them.
            TokenKind::Op(s) => Node::Op(s),
            TokenKind::Word(w) => self.word(&w, at, stop)?,
        })
    }

    /// Whole-word keyword dispatch (REFERENCE.md §3): the word arrives already
    /// cut at token boundaries, so `sinx` simply misses the table.
    fn word(&mut self, w: &str, at: usize, stop: Stop) -> Result<Node, ParseError> {
        match lookup(w) {
            None => Ok(Node::Ident(w.to_string())),
            // Scripts do not bind inside an accent: `vec A^2` is `(vec A)^2`.
            Some(Keyword::Sym(d)) if d.kind == SymKind::Accent => {
                let base = self.operand_primary(stop, d.name)?;
                Ok(Node::Accent {
                    accent: d,
                    base: Box::new(base),
                })
            }
            Some(Keyword::Sym(d)) => Ok(Node::Symbol(d)),
            // `of` is a keyword only inside `root ... of ...`, which consumes it
            // itself; anywhere else it is an ordinary word.
            Some(Keyword::Cmd(Cmd::Of)) => Ok(Node::Ident(w.to_string())),
            Some(Keyword::Cmd(cmd)) => self.command(cmd, w, at, stop),
        }
    }

    fn command(&mut self, cmd: Cmd, w: &str, at: usize, stop: Stop) -> Result<Node, ParseError> {
        let name = w.to_ascii_lowercase();
        match cmd {
            // Infix keywords are handled by `sequence`; one reaching here has
            // nothing to its left.
            Cmd::Over | Cmd::Atop | Cmd::Choose => Err(ParseError::new(
                format!("{}: missing left operand", infix_name(cmd)),
                at,
            )),
            Cmd::Of => Ok(Node::Ident(w.to_string())),
            Cmd::Sqrt => Ok(Node::Sqrt(Box::new(self.operand(stop, &name)?))),
            Cmd::Root => {
                let degree = self.operand(stop, &name)?;
                if self.peek_cmd() != Some(Cmd::Of) {
                    return Err(ParseError::new("root: expected 'of'", self.here()));
                }
                self.pos += 1;
                let radicand = self.operand(stop, &name)?;
                Ok(Node::Root {
                    degree: Box::new(degree),
                    radicand: Box::new(radicand),
                })
            }
            Cmd::Binom => {
                let top = self.operand(stop, &name)?;
                let bottom = self.operand(stop, &name)?;
                Ok(Node::Binom {
                    top: Box::new(top),
                    bottom: Box::new(bottom),
                })
            }
            Cmd::Not => Ok(Node::Not(Box::new(self.operand(stop, &name)?))),
            Cmd::Big(size) => {
                let arg = self.operand_primary(stop, &name)?;
                Ok(Node::Big {
                    size: size.to_string(),
                    arg: Box::new(arg),
                })
            }
            Cmd::BuildRel => {
                let top = self.operand(stop, &name)?;
                if self.peek_cmd() != Some(Cmd::Over) {
                    return Err(ParseError::new("buildrel: expected 'over'", self.here()));
                }
                self.pos += 1;
                let base = self.operand(stop, &name)?;
                Ok(Node::BuildRel {
                    top: Box::new(top),
                    base: Box::new(base),
                })
            }
            Cmd::Matrix(kind) => self.matrix(kind, &name),
            // A style switch owns the rest of the sequence it appears in, so
            // its scope ends at the enclosing `}` / `RIGHT` / cell end / EOF.
            Cmd::Style(style) => {
                let body = Node::seq(self.sequence(stop)?);
                Ok(Node::Style {
                    style,
                    body: Box::new(body),
                })
            }
            Cmd::Left => {
                let left = self.delimiter("LEFT")?;
                let body = Node::seq(self.sequence(Stop::Right)?);
                if self.peek().is_none() {
                    return Err(ParseError::new("LEFT without RIGHT", at));
                }
                self.pos += 1; // the `RIGHT` keyword
                let right = self.delimiter("RIGHT")?;
                Ok(Node::Delimited {
                    left,
                    right,
                    body: Box::new(body),
                })
            }
            Cmd::Right => Err(ParseError::new("RIGHT without LEFT", at)),
            Cmd::Sup => Err(ParseError::new("superscript without a base", at)),
            Cmd::Sub => Err(ParseError::new("subscript without a base", at)),
        }
    }

    /// `matrix { a & b # c & d }` -- `#` and `&` split at THIS depth only;
    /// nested groups and matrices keep their own separators.
    fn matrix(&mut self, kind: MatrixKind, name: &str) -> Result<Node, ParseError> {
        if !matches!(self.peek().map(|t| &t.kind), Some(TokenKind::LBrace)) {
            return Err(ParseError::new(
                format!("{name}: expected '{{'"),
                self.here(),
            ));
        }
        let open = self.here();
        self.pos += 1;

        let mut rows: Vec<Vec<Node>> = Vec::new();
        let mut row: Vec<Node> = Vec::new();
        loop {
            // Empty cells are kept as empty rows, never trimmed away.
            row.push(Node::seq(self.sequence(Stop::Cell)?));
            match self.peek().map(|t| t.kind.clone()) {
                Some(TokenKind::Amp) => self.pos += 1,
                Some(TokenKind::Hash) => {
                    self.pos += 1;
                    rows.push(std::mem::take(&mut row));
                }
                Some(TokenKind::RBrace) => {
                    self.pos += 1;
                    rows.push(row);
                    break;
                }
                // `sequence(Stop::Cell)` stops only at `&`, `#`, `}` or EOF.
                _ => return Err(ParseError::new("unclosed '{'", open)),
            }
        }
        Ok(Node::Matrix { kind, rows })
    }

    /// One `LEFT`/`RIGHT` delimiter, kept verbatim for the emitter to map.
    fn delimiter(&mut self, what: &str) -> Result<String, ParseError> {
        let text = match self.peek() {
            Some(Token {
                kind: TokenKind::Op(s),
                ..
            }) if is_delimiter(s) => s.clone(),
            _ => {
                return Err(ParseError::new(
                    format!("{what}: expected delimiter"),
                    self.here(),
                ))
            }
        };
        self.pos += 1;
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::{StyleKind, SymDef};

    fn p(src: &str) -> Node {
        parse(src).unwrap_or_else(|e| panic!("parse({src:?}) failed: {e}"))
    }
    fn e(src: &str) -> ParseError {
        match parse(src) {
            Err(err) => err,
            Ok(node) => panic!("parse({src:?}) unexpectedly succeeded: {node:?}"),
        }
    }
    fn ident(s: &str) -> Node {
        Node::Ident(s.to_string())
    }
    fn num(s: &str) -> Node {
        Node::Number(s.to_string())
    }
    fn def(w: &str) -> &'static SymDef {
        match lookup(w) {
            Some(Keyword::Sym(d)) => d,
            other => panic!("{w:?}: expected a symbol, got {other:?}"),
        }
    }
    fn sym(w: &str) -> Node {
        Node::Symbol(def(w))
    }
    fn b(n: Node) -> Box<Node> {
        Box::new(n)
    }

    /// `a over b over c` = `(a/b)/c` -- the infix operator pops the item that is
    /// already in the sequence, so it is left-associative.
    #[test]
    fn over_is_left_associative() {
        assert_eq!(
            p("a over b over c"),
            Node::Frac {
                num: b(Node::Frac {
                    num: b(ident("a")),
                    den: b(ident("b")),
                    bar: true
                }),
                den: b(ident("c")),
                bar: true,
            }
        );
        // `atop` is the same shape without the bar; `choose` builds a Binom.
        assert_eq!(
            p("x atop y"),
            Node::Frac {
                num: b(ident("x")),
                den: b(ident("y")),
                bar: false
            }
        );
        assert_eq!(
            p("n choose k"),
            Node::Binom {
                top: b(ident("n")),
                bottom: b(ident("k"))
            }
        );
    }

    /// Accents take a primary, so a following script wraps the accent.
    #[test]
    fn accent_does_not_swallow_scripts() {
        assert_eq!(
            p("vec A^2"),
            Node::Script {
                base: b(Node::Accent {
                    accent: def("vec"),
                    base: b(ident("A"))
                }),
                sub: None,
                sup: Some(b(num("2"))),
            }
        );
    }

    #[test]
    fn matrix_rows_and_cells() {
        assert_eq!(
            p("pmatrix { a & b # c & d }"),
            Node::Matrix {
                kind: MatrixKind::Paren,
                rows: vec![vec![ident("a"), ident("b")], vec![ident("c"), ident("d")]],
            }
        );
        assert_eq!(
            p("pile {a # b}"),
            Node::Matrix {
                kind: MatrixKind::Pile,
                rows: vec![vec![ident("a")], vec![ident("b")]]
            }
        );
    }

    /// Splitting is depth-local: `#`/`&` inside a nested group are ordinary
    /// Newline/Align nodes belonging to that group.
    #[test]
    fn separators_are_depth_local() {
        assert_eq!(
            p("matrix { {a # b} & c }"),
            Node::Matrix {
                kind: MatrixKind::Plain,
                rows: vec![vec![
                    Node::Row(vec![ident("a"), Node::Newline, ident("b")]),
                    ident("c"),
                ]],
            }
        );
        // Empty cells survive as empty rows.
        assert_eq!(
            p("matrix { a & }"),
            Node::Matrix {
                kind: MatrixKind::Plain,
                rows: vec![vec![ident("a"), Node::Row(vec![])]],
            }
        );
    }

    /// REFERENCE.md §3: whole-token lookup, never a prefix scan.
    #[test]
    fn whole_word_keyword_matching() {
        assert_eq!(p("pi le"), Node::Row(vec![sym("pi"), ident("le")]));
        assert_eq!(p("sinx"), ident("sinx"));
        assert_eq!(p("sin x"), Node::Row(vec![sym("sin"), ident("x")]));
        assert_eq!(p("sinh x"), Node::Row(vec![sym("sinh"), ident("x")]));
        // `pile` is the stacking command, not π + le.
        assert!(matches!(p("pile {a}"), Node::Matrix { .. }));
    }

    #[test]
    fn scripts_bind_one_primary_each() {
        // `a^2 2`: only the first 2 is a superscript.
        assert_eq!(
            p("a^2 2"),
            Node::Row(vec![
                Node::Script {
                    base: b(ident("a")),
                    sub: None,
                    sup: Some(b(num("2")))
                },
                num("2"),
            ])
        );
        // `sub`/`sup` keywords are the same markers as `_`/`^`.
        assert_eq!(
            p("x sub i sup 2"),
            Node::Script {
                base: b(ident("x")),
                sub: Some(b(ident("i"))),
                sup: Some(b(num("2"))),
            }
        );
        // A group is one term, and a multi-item group stays a Row base.
        assert_eq!(
            p("{a + b}^2"),
            Node::Script {
                base: b(Node::Row(vec![
                    ident("a"),
                    Node::Op("+".into()),
                    ident("b")
                ])),
                sub: None,
                sup: Some(b(num("2"))),
            }
        );
    }

    #[test]
    fn root_binom_buildrel_and_delimiters() {
        assert_eq!(
            p("root 3 of {x+1}"),
            Node::Root {
                degree: b(num("3")),
                radicand: b(Node::Row(vec![ident("x"), Node::Op("+".into()), num("1")])),
            }
        );
        assert_eq!(
            p("buildrel def over ="),
            Node::BuildRel {
                top: b(ident("def")),
                base: b(Node::Op("=".into()))
            }
        );
        assert_eq!(
            p("LEFT ( x over y RIGHT )"),
            Node::Delimited {
                left: "(".into(),
                right: ")".into(),
                body: b(Node::Frac {
                    num: b(ident("x")),
                    den: b(ident("y")),
                    bar: true
                }),
            }
        );
        // `of` outside a `root` form is just a word.
        assert_eq!(
            p("a of b"),
            Node::Row(vec![ident("a"), ident("of"), ident("b")])
        );
    }

    /// A style switch owns the remainder of its own sequence, no further.
    #[test]
    fn style_scopes_to_the_rest_of_the_sequence() {
        assert_eq!(
            p("rm a + b"),
            Node::Style {
                style: StyleKind::Roman,
                body: b(Node::Row(vec![
                    ident("a"),
                    Node::Op("+".into()),
                    ident("b")
                ])),
            }
        );
        assert_eq!(
            p("{rm a} b"),
            Node::Row(vec![
                Node::Style {
                    style: StyleKind::Roman,
                    body: b(ident("a"))
                },
                ident("b"),
            ])
        );
    }

    #[test]
    fn errors_carry_the_offending_offset() {
        let cases: &[(&str, &str, usize)] = &[
            ("{x", "unclosed '{'", 0),
            ("x }", "unmatched '}'", 2),
            ("1 over", "over: missing right operand", 2),
            ("over 2", "over: missing left operand", 0),
            ("root 3 x", "root: expected 'of'", 7),
            ("LEFT ( x", "LEFT without RIGHT", 0),
            ("^2", "superscript without a base", 0),
            ("_2", "subscript without a base", 0),
            ("a^1^2", "duplicate superscript", 3),
            ("a_1_2", "duplicate subscript", 3),
            ("RIGHT )", "RIGHT without LEFT", 0),
            ("LEFT x RIGHT x", "LEFT: expected delimiter", 5),
            ("LEFT ( x RIGHT x", "RIGHT: expected delimiter", 15),
            ("pmatrix x", "pmatrix: expected '{'", 8),
            (r#""abc"#, "unterminated quoted text", 0),
        ];
        for &(src, msg, pos) in cases {
            let err = e(src);
            assert!(
                err.message.contains(msg),
                "{src:?}: got {:?}, want {msg:?}",
                err.message
            );
            assert_eq!(err.position, pos, "{src:?}: wrong offset ({err})");
        }
    }
}
