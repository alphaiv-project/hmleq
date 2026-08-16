//! LaTeX emitter.

use crate::ast::{Node, SpaceKind};
use crate::symbols::{MatrixKind, ScriptPos, StyleKind, SymDef, SymKind};

/// Render an AST as LaTeX, following the canonical templates in DESIGN.md §5
/// (Row children joined with single spaces, macro arguments always braced, ...).
pub fn to_latex(node: &Node) -> String {
    let body = emit(node);
    // Multi-line wrapping is a property of the whole equation, so it is decided
    // here and never inside `emit` -- DESIGN.md §5 "top level only". Only the
    // root Row's *direct* children count.
    if let Node::Row(items) = node {
        if items.iter().any(|n| matches!(n, Node::Newline)) {
            let env = if items.iter().any(|n| matches!(n, Node::Align)) {
                "aligned"
            } else {
                "gathered"
            };
            return format!(r"\begin{{{0}}} {1} \end{{{0}}}", env, body);
        }
    }
    body
}

/// LaTeX for a `LEFT`/`RIGHT` delimiter as written in the script. `None` marks
/// a delimiter the language does not accept (DESIGN.md §5 -- the parser
/// validates `LEFT`/`RIGHT` arguments against this same map).
pub fn delimiter_latex(raw: &str) -> Option<&'static str> {
    Some(match raw {
        "(" => "(",
        ")" => ")",
        "[" => "[",
        "]" => "]",
        "|" => "|",
        "." => ".",
        "||" => r"\Vert",
        "<" => r"\langle",
        ">" => r"\rangle",
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// Node emission
// ---------------------------------------------------------------------------

fn emit(node: &Node) -> String {
    match node {
        Node::Row(items) => items.iter().map(emit).collect::<Vec<_>>().join(" "),

        Node::Number(s) | Node::Ident(s) => s.clone(),
        Node::Op(s) => match op_ligature(s) {
            Some(tex) => tex.to_string(),
            None => s.clone(),
        },
        Node::Text(s) => format!(r"\text{{{}}}", escape_text(s)),
        Node::Symbol(d) => emit_symbol(d),

        Node::Space(SpaceKind::Full) => r"\;".to_string(),
        Node::Space(SpaceKind::Quarter) => r"\,".to_string(),
        Node::Newline => r"\\".to_string(),
        Node::Align => "&".to_string(),

        Node::Frac { num, den, bar } => {
            if *bar {
                format!(r"\frac{{{}}}{{{}}}", emit(num), emit(den))
            } else {
                format!(r"{{{} \atop {}}}", emit(num), emit(den))
            }
        }
        Node::Binom { top, bottom } => format!(r"\binom{{{}}}{{{}}}", emit(top), emit(bottom)),
        Node::Sqrt(x) => format!(r"\sqrt{{{}}}", emit(x)),
        // The degree is the one macro argument that stays unbraced.
        Node::Root { degree, radicand } => {
            format!(r"\sqrt[{}]{{{}}}", emit(degree), emit(radicand))
        }

        Node::Script { base, sub, sup } => {
            let mut out = emit_script_base(base);
            if let Some(sub) = sub {
                out.push_str(&format!("_{{{}}}", emit(sub)));
            }
            if let Some(sup) = sup {
                out.push_str(&format!("^{{{}}}", emit(sup)));
            }
            out
        }

        Node::Delimited { left, right, body } => {
            format!(
                r"\left{} {} \right{}",
                delim(left),
                emit(body),
                delim(right)
            )
        }

        Node::Matrix { kind, rows } => emit_matrix(*kind, rows),

        Node::Accent { accent, base } => format!("{}{{{}}}", accent.latex, emit(base)),

        Node::Style { style, body } => match style {
            StyleKind::Roman => format!(r"\mathrm{{{}}}", emit(body)),
            StyleKind::Bold => format!(r"\boldsymbol{{{}}}", emit(body)),
            StyleKind::RomanBold => format!(r"\mathbf{{{}}}", emit(body)),
            StyleKind::Italic => emit(body),
        },

        Node::BuildRel { top, base } => format!(r"\overset{{{}}}{{{}}}", emit(top), emit(base)),
        Node::Not(x) => format!(r"\not {}", emit(x)),
        Node::Big { size, arg } => format!("{} {}", size, emit(arg)),
    }
}

/// A delimiter as written -> its LaTeX. Unrecognized text cannot reach here
/// (the parser rejects it against the same map), so it passes through.
fn delim(raw: &str) -> String {
    match delimiter_latex(raw) {
        Some(tex) => tex.to_string(),
        None => raw.to_string(),
    }
}

/// A big operator carrying no scripts shrinks to its binary form
/// (`A union B` -> `A \cup B`); `emit_script_base` bypasses this.
fn emit_symbol(d: &SymDef) -> String {
    match (d.kind, d.bin_latex) {
        (SymKind::BigOp, Some(bin)) => bin.to_string(),
        _ => d.latex.to_string(),
    }
}

/// The base of a `Script`: bare unless it is a `Row` of ≥ 2 items, in which
/// case braces keep the scripts attached to the whole base.
fn emit_script_base(base: &Node) -> String {
    match base {
        // A symbol scripted *directly* keeps its large form and may force the
        // scripts beside it. Anything else (e.g. a symbol inside a Row) is not
        // "directly under a Script" and goes through the normal path.
        Node::Symbol(d) => {
            let mut out = d.latex.to_string();
            if d.scripts == ScriptPos::Beside {
                out.push_str(r"\nolimits");
            }
            out
        }
        Node::Row(items) if items.len() >= 2 => format!("{{{}}}", emit(base)),
        _ => emit(base),
    }
}

fn emit_matrix(kind: MatrixKind, rows: &[Vec<Node>]) -> String {
    let (env, spec) = match kind {
        MatrixKind::Plain => ("matrix", ""),
        MatrixKind::Paren => ("pmatrix", ""),
        MatrixKind::Bracket => ("bmatrix", ""),
        MatrixKind::Det => ("vmatrix", ""),
        MatrixKind::Cases => ("cases", ""),
        MatrixKind::EqAlign => ("aligned", ""),
        MatrixKind::Pile => ("array", "{c}"),
        MatrixKind::LPile => ("array", "{l}"),
        MatrixKind::RPile => ("array", "{r}"),
    };
    // The pile environments declare a single column, so a stray `&` in one of
    // their rows must not turn into an alignment tab.
    let cell_sep = if spec.is_empty() { " & " } else { " " };
    let body = rows
        .iter()
        .map(|cells| cells.iter().map(emit).collect::<Vec<_>>().join(cell_sep))
        .collect::<Vec<_>>()
        .join(r" \\ ");
    format!(r"\begin{{{0}}}{1} {2} \end{{{0}}}", env, spec, body)
}

fn op_ligature(raw: &str) -> Option<&'static str> {
    Some(match raw {
        "+-" => r"\pm",
        "-+" => r"\mp",
        "!=" => r"\neq",
        "<=" => r"\leq",
        ">=" => r"\geq",
        "<<" => r"\ll",
        ">>" => r"\gg",
        "||" => r"\Vert",
        _ => return None,
    })
}

/// Escape the TeX specials inside `\text{...}`. `\`, `~` and `^` have no
/// `\<char>` form, so they use their `\text...{}` macros.
fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str(r"\textbackslash{}"),
            '~' => out.push_str(r"\textasciitilde{}"),
            '^' => out.push_str(r"\textasciicircum{}"),
            '{' | '}' | '$' | '&' | '#' | '_' | '%' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Nodes are built directly here so these tests need neither lexer nor
    // parser; the SymDefs mirror the real entries in `symbols.rs`.
    static LIM_EXACT: SymDef = SymDef {
        name: "Lim",
        kind: SymKind::Func,
        latex: r"\lim",
        bin_latex: None,
        scripts: ScriptPos::Beside,
    };
    static SMALLSUM: SymDef = SymDef {
        name: "smallsum",
        kind: SymKind::BigOp,
        latex: r"\sum",
        bin_latex: None,
        scripts: ScriptPos::Beside,
    };
    static UNION: SymDef = SymDef {
        name: "union",
        kind: SymKind::BigOp,
        latex: r"\bigcup",
        bin_latex: Some(r"\cup"),
        scripts: ScriptPos::Below,
    };
    static VEC: SymDef = SymDef {
        name: "vec",
        kind: SymKind::Accent,
        latex: r"\vec",
        bin_latex: None,
        scripts: ScriptPos::Normal,
    };

    fn ident(s: &str) -> Node {
        Node::Ident(s.to_string())
    }
    fn num(s: &str) -> Node {
        Node::Number(s.to_string())
    }
    fn op(s: &str) -> Node {
        Node::Op(s.to_string())
    }
    fn script(base: Node, sub: Option<Node>, sup: Option<Node>) -> Node {
        Node::Script {
            base: Box::new(base),
            sub: sub.map(Box::new),
            sup: sup.map(Box::new),
        }
    }

    /// A `Beside` symbol base takes `\nolimits` between the glyph and its
    /// scripts; sub is emitted before sup.
    #[test]
    fn beside_symbol_base_gets_nolimits() {
        let lim = script(Node::Symbol(&LIM_EXACT), Some(ident("n")), None);
        assert_eq!(to_latex(&lim), r"\lim\nolimits_{n}");

        let small = script(Node::Symbol(&SMALLSUM), Some(ident("k")), Some(num("2")));
        assert_eq!(to_latex(&small), r"\sum\nolimits_{k}^{2}");

        // Unscripted, the same symbol is just its glyph.
        assert_eq!(to_latex(&Node::Symbol(&LIM_EXACT)), r"\lim");
    }

    /// Script bases are bare -- only a Row of ≥ 2 items needs braces.
    #[test]
    fn row_script_base_is_braced() {
        let row = script(
            Node::Row(vec![ident("a"), op("+"), ident("b")]),
            None,
            Some(num("2")),
        );
        assert_eq!(to_latex(&row), "{a + b}^{2}");

        assert_eq!(to_latex(&script(ident("a"), None, Some(num("2")))), "a^{2}");
        // A single-item Row (a `{a}` group) still binds as one term, no braces.
        assert_eq!(
            to_latex(&script(Node::Row(vec![ident("a")]), None, Some(num("2")))),
            "a^{2}"
        );
        // Compound bases other than Row stay bare as well.
        let delim = Node::Delimited {
            left: "[".to_string(),
            right: "]".to_string(),
            body: Box::new(script(ident("x"), None, Some(num("3")))),
        };
        assert_eq!(
            to_latex(&script(delim, Some(num("1")), Some(num("2")))),
            r"\left[ x^{3} \right]_{1}^{2}"
        );
    }

    /// `bin_latex` applies only when the operator is NOT directly scripted.
    #[test]
    fn bigop_binary_fallback() {
        let bare = Node::Row(vec![ident("A"), Node::Symbol(&UNION), ident("B")]);
        assert_eq!(to_latex(&bare), r"A \cup B");

        let scripted = script(
            Node::Symbol(&UNION),
            Some(Node::Row(vec![ident("i"), op("="), num("1")])),
            Some(ident("n")),
        );
        assert_eq!(to_latex(&scripted), r"\bigcup_{i = 1}^{n}");

        // Wrapped in a Row the operator is no longer *directly* under the
        // Script, so it shrinks again.
        let indirect = script(
            Node::Row(vec![ident("A"), Node::Symbol(&UNION)]),
            None,
            Some(num("2")),
        );
        assert_eq!(to_latex(&indirect), r"{A \cup}^{2}");
    }

    #[test]
    fn text_escaping() {
        assert_eq!(
            to_latex(&Node::Text("hello world".to_string())),
            r"\text{hello world}"
        );
        assert_eq!(
            to_latex(&Node::Text(r"a\b{c}d_e^f~g".to_string())),
            r"\text{a\textbackslash{}b\{c\}d\_e\textasciicircum{}f\textasciitilde{}g}"
        );
        assert_eq!(
            to_latex(&Node::Text("50% $ & #".to_string())),
            r"\text{50\% \$ \& \#}"
        );
    }

    #[test]
    fn pile_is_a_single_column_array() {
        let rows = || vec![vec![ident("a")], vec![ident("b")]];
        for (kind, spec) in [
            (MatrixKind::Pile, "c"),
            (MatrixKind::LPile, "l"),
            (MatrixKind::RPile, "r"),
        ] {
            let n = Node::Matrix { kind, rows: rows() };
            assert_eq!(
                to_latex(&n),
                format!(r"\begin{{array}}{{{}}} a \\ b \end{{array}}", spec)
            );
        }
    }

    #[test]
    fn matrix_environments() {
        let rows = vec![
            vec![
                script(ident("a"), Some(num("1")), None),
                script(ident("b"), Some(num("1")), None),
            ],
            vec![
                script(ident("a"), Some(num("2")), None),
                script(ident("b"), Some(num("2")), None),
            ],
        ];
        let n = Node::Matrix {
            kind: MatrixKind::Paren,
            rows,
        };
        assert_eq!(
            to_latex(&n),
            r"\begin{pmatrix} a_{1} & b_{1} \\ a_{2} & b_{2} \end{pmatrix}"
        );
        // dmatrix -> vmatrix, eqalign -> aligned.
        let one = |k| Node::Matrix {
            kind: k,
            rows: vec![vec![ident("a")]],
        };
        assert_eq!(
            to_latex(&one(MatrixKind::Det)),
            r"\begin{vmatrix} a \end{vmatrix}"
        );
        assert_eq!(
            to_latex(&one(MatrixKind::EqAlign)),
            r"\begin{aligned} a \end{aligned}"
        );
    }

    /// The gathered/aligned wrapper is a top-level-only decision.
    #[test]
    fn top_level_newline_wrapping() {
        let gathered = Node::Row(vec![ident("a"), Node::Newline, ident("b")]);
        assert_eq!(
            to_latex(&gathered),
            r"\begin{gathered} a \\ b \end{gathered}"
        );

        let aligned = Node::Row(vec![
            ident("a"),
            Node::Align,
            ident("b"),
            Node::Newline,
            ident("c"),
            Node::Align,
            ident("d"),
        ]);
        assert_eq!(
            to_latex(&aligned),
            r"\begin{aligned} a & b \\ c & d \end{aligned}"
        );

        // No Newline at top level -> no wrapper, even with an Align.
        let flat = Node::Row(vec![ident("a"), Node::Align, ident("b")]);
        assert_eq!(to_latex(&flat), "a & b");

        // A Newline nested below the root does not trigger the wrapper.
        let nested = Node::Row(vec![Node::Sqrt(Box::new(Node::Row(vec![
            ident("a"),
            Node::Newline,
            ident("b"),
        ])))]);
        assert_eq!(to_latex(&nested), r"\sqrt{a \\ b}");
    }

    #[test]
    fn scalars_and_ligatures() {
        assert_eq!(to_latex(&ident("sinx")), "sinx");
        assert_eq!(to_latex(&num("3.14")), "3.14");
        for (raw, tex) in [
            ("+-", r"\pm"),
            ("-+", r"\mp"),
            ("!=", r"\neq"),
            ("<=", r"\leq"),
            (">=", r"\geq"),
            ("<<", r"\ll"),
            (">>", r"\gg"),
            ("||", r"\Vert"),
        ] {
            assert_eq!(to_latex(&op(raw)), tex);
        }
        // Non-ligature operators are verbatim -- a bare `|` stays a pipe.
        for raw in ["+", "-", "=", "(", "|", "/", "<"] {
            assert_eq!(to_latex(&op(raw)), raw);
        }
        assert_eq!(to_latex(&Node::Space(SpaceKind::Full)), r"\;");
        assert_eq!(to_latex(&Node::Space(SpaceKind::Quarter)), r"\,");
    }

    #[test]
    fn structural_templates() {
        let frac = Node::Frac {
            num: Box::new(ident("x")),
            den: Box::new(ident("y")),
            bar: true,
        };
        assert_eq!(to_latex(&frac), r"\frac{x}{y}");
        let atop = Node::Frac {
            num: Box::new(ident("x")),
            den: Box::new(ident("y")),
            bar: false,
        };
        assert_eq!(to_latex(&atop), r"{x \atop y}");
        let binom = Node::Binom {
            top: Box::new(ident("n")),
            bottom: Box::new(ident("k")),
        };
        assert_eq!(to_latex(&binom), r"\binom{n}{k}");
        // The root degree is the sole unbraced argument.
        let root = Node::Root {
            degree: Box::new(num("3")),
            radicand: Box::new(Node::Row(vec![ident("x"), op("+"), num("1")])),
        };
        assert_eq!(to_latex(&root), r"\sqrt[3]{x + 1}");
        let accent = Node::Accent {
            accent: &VEC,
            base: Box::new(ident("A")),
        };
        assert_eq!(to_latex(&accent), r"\vec{A}");
        // `vec A^2` -- the accent is the script base, and it is not a Row.
        assert_eq!(
            to_latex(&script(accent, None, Some(num("2")))),
            r"\vec{A}^{2}"
        );
        let build = Node::BuildRel {
            top: Box::new(ident("def")),
            base: Box::new(op("=")),
        };
        assert_eq!(to_latex(&build), r"\overset{def}{=}");
        assert_eq!(to_latex(&Node::Not(Box::new(op("=")))), r"\not =");
        assert_eq!(
            to_latex(&Node::Big {
                size: r"\bigg".to_string(),
                arg: Box::new(op("/"))
            }),
            r"\bigg /"
        );
    }

    #[test]
    fn styles_and_delimiters() {
        let body = || Box::new(ident("ABC"));
        assert_eq!(
            to_latex(&Node::Style {
                style: StyleKind::Roman,
                body: body()
            }),
            r"\mathrm{ABC}"
        );
        assert_eq!(
            to_latex(&Node::Style {
                style: StyleKind::Bold,
                body: body()
            }),
            r"\boldsymbol{ABC}"
        );
        assert_eq!(
            to_latex(&Node::Style {
                style: StyleKind::RomanBold,
                body: body()
            }),
            r"\mathbf{ABC}"
        );
        // Italic is the default math face: no wrapper at all.
        assert_eq!(
            to_latex(&Node::Style {
                style: StyleKind::Italic,
                body: body()
            }),
            "ABC"
        );

        let delimited = |l: &str, r: &str| Node::Delimited {
            left: l.to_string(),
            right: r.to_string(),
            body: Box::new(ident("x")),
        };
        assert_eq!(to_latex(&delimited("(", ")")), r"\left( x \right)");
        assert_eq!(
            to_latex(&delimited("||", "||")),
            r"\left\Vert x \right\Vert"
        );
        assert_eq!(
            to_latex(&delimited("<", ">")),
            r"\left\langle x \right\rangle"
        );
        assert_eq!(to_latex(&delimited(".", "|")), r"\left. x \right|");
        assert_eq!(delimiter_latex("["), Some("["));
        assert_eq!(delimiter_latex("!"), None);
    }
}
