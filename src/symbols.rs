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

// ---------------------------------------------------------------------------
// Table constructors
// ---------------------------------------------------------------------------

const fn def(
    name: &'static str,
    kind: SymKind,
    latex: &'static str,
    bin_latex: Option<&'static str>,
    scripts: ScriptPos,
) -> SymDef {
    SymDef {
        name,
        kind,
        latex,
        bin_latex,
        scripts,
    }
}

const fn ord(name: &'static str, latex: &'static str) -> SymDef {
    def(name, SymKind::Ord, latex, None, ScriptPos::Normal)
}
const fn bin(name: &'static str, latex: &'static str) -> SymDef {
    def(name, SymKind::Bin, latex, None, ScriptPos::Normal)
}
const fn rel(name: &'static str, latex: &'static str) -> SymDef {
    def(name, SymKind::Rel, latex, None, ScriptPos::Normal)
}
const fn accent(name: &'static str, latex: &'static str) -> SymDef {
    def(name, SymKind::Accent, latex, None, ScriptPos::Normal)
}
/// Ordinary function name — scripts attach as they do on any other term.
const fn func(name: &'static str, latex: &'static str) -> SymDef {
    def(name, SymKind::Func, latex, None, ScriptPos::Normal)
}
/// Function name that takes limits under/over it (`lim`, `max`, …).
const fn func_below(name: &'static str, latex: &'static str) -> SymDef {
    def(name, SymKind::Func, latex, None, ScriptPos::Below)
}
/// Function name forced to keep its scripts beside it (`Lim`).
const fn func_beside(name: &'static str, latex: &'static str) -> SymDef {
    def(name, SymKind::Func, latex, None, ScriptPos::Beside)
}
/// Large operator with no separate binary form (`\sum`, `\int`, `small*`).
const fn bigop(name: &'static str, latex: &'static str, scripts: ScriptPos) -> SymDef {
    def(name, SymKind::BigOp, latex, None, scripts)
}
/// Large operator that shrinks to `bin_form` when it carries no scripts
/// (`union` → `\bigcup` with limits, `\cup` bare).
const fn bigop_bin(
    name: &'static str,
    latex: &'static str,
    bin_form: &'static str,
    scripts: ScriptPos,
) -> SymDef {
    def(name, SymKind::BigOp, latex, Some(bin_form), scripts)
}

// ---------------------------------------------------------------------------
// Exact (case-sensitive) table — DESIGN.md §2
// ---------------------------------------------------------------------------

/// Words whose capitalization changes their meaning. Every name here contains
/// an uppercase letter, so an all-lowercase word can skip this table entirely.
static EXACT_SYMS: &[SymDef] = &[
    // -- functions distinguished by case (REFERENCE §4.10) ----------------
    func_beside("Lim", r"\lim"),
    func("Exp", r"\operatorname{Exp}"),
    func_below("Pr", r"\Pr"),
    // -- Greek capitals (REFERENCE §4.5) ----------------------------------
    // Those whose shape is a Latin letter map to that letter, not a macro.
    ord("Alpha", "A"),
    ord("Beta", "B"),
    ord("Gamma", r"\Gamma"),
    ord("Delta", r"\Delta"),
    ord("Epsilon", "E"),
    ord("Zeta", "Z"),
    ord("Eta", "H"),
    ord("Theta", r"\Theta"),
    ord("Iota", "I"),
    ord("Kappa", "K"),
    ord("Lambda", r"\Lambda"),
    ord("Mu", "M"),
    ord("Nu", "N"),
    ord("Xi", r"\Xi"),
    ord("Omicron", "O"),
    ord("Pi", r"\Pi"),
    ord("Rho", "P"),
    ord("Sigma", r"\Sigma"),
    ord("Tau", "T"),
    ord("Upsilon", r"\Upsilon"),
    ord("Phi", r"\Phi"),
    ord("Chi", "X"),
    ord("Psi", r"\Psi"),
    ord("Omega", r"\Omega"),
    // -- double arrows (REFERENCE §4.6) -----------------------------------
    rel("LARROW", r"\Leftarrow"),
    rel("RARROW", r"\Rightarrow"),
    rel("LRARROW", r"\Leftrightarrow"),
    rel("UDARROW", r"\Updownarrow"),
    rel("Leftarrow", r"\Leftarrow"),
    rel("Rightarrow", r"\Rightarrow"),
    rel("Leftrightarrow", r"\Leftrightarrow"),
    rel("Updownarrow", r"\Updownarrow"),
];

/// Exact-spelling commands: only the two capitalized delimiter sizes.
fn exact_cmd(word: &str) -> Option<Cmd> {
    match word {
        "Big" => Some(Cmd::Big(r"\Big")),
        "Bigg" => Some(Cmd::Big(r"\Bigg")),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Case-insensitive table — keyed by the ASCII-lowercased word
// ---------------------------------------------------------------------------

/// Every `name` here MUST be ASCII-lowercase (asserted in the unit tests):
/// lookup only ever probes this table with a lowercased word.
static CI_SYMS: &[SymDef] = &[
    // -- Greek lowercase (REFERENCE §4.5) ---------------------------------
    ord("alpha", r"\alpha"),
    ord("beta", r"\beta"),
    ord("gamma", r"\gamma"),
    ord("delta", r"\delta"),
    ord("epsilon", r"\epsilon"),
    ord("zeta", r"\zeta"),
    ord("eta", r"\eta"),
    ord("theta", r"\theta"),
    ord("iota", r"\iota"),
    ord("kappa", r"\kappa"),
    ord("lambda", r"\lambda"),
    ord("mu", r"\mu"),
    ord("nu", r"\nu"),
    ord("xi", r"\xi"),
    // no `\omicron` exists in TeX — it is just a roman `o`.
    ord("omicron", "o"),
    ord("pi", r"\pi"),
    ord("rho", r"\rho"),
    ord("sigma", r"\sigma"),
    ord("tau", r"\tau"),
    ord("upsilon", r"\upsilon"),
    ord("phi", r"\phi"),
    ord("chi", r"\chi"),
    ord("psi", r"\psi"),
    ord("omega", r"\omega"),
    // -- Greek variants ---------------------------------------------------
    ord("vartheta", r"\vartheta"),
    ord("varpi", r"\varpi"),
    ord("varsigma", r"\varsigma"),
    // no lowercase variant upsilon in TeX; DESIGN.md §5 pins it to `\Upsilon`.
    ord("varupsilon", r"\Upsilon"),
    ord("varphi", r"\varphi"),
    ord("varepsilon", r"\varepsilon"),
    // -- specials (REFERENCE §4.5) ----------------------------------------
    ord("aleph", r"\aleph"),
    ord("hbar", r"\hbar"),
    ord("imath", r"\imath"),
    ord("jmath", r"\jmath"),
    ord("ohm", r"\Omega"),
    ord("ell", r"\ell"),
    ord("liter", r"\ell"),
    ord("wp", r"\wp"),
    ord("imag", r"\Im"),
    ord("angstrom", r"\mathring{A}"),
    // -- arrows (REFERENCE §4.6) ------------------------------------------
    rel("larrow", r"\leftarrow"),
    rel("rarrow", r"\rightarrow"),
    rel("uparrow", r"\uparrow"),
    rel("downarrow", r"\downarrow"),
    rel("lrarrow", r"\leftrightarrow"),
    rel("udarrow", r"\updownarrow"),
    // long aliases: the plain (single-stroke) forms
    rel("leftarrow", r"\leftarrow"),
    rel("rightarrow", r"\rightarrow"),
    rel("leftrightarrow", r"\leftrightarrow"),
    rel("updownarrow", r"\updownarrow"),
    rel("nwarrow", r"\nwarrow"),
    rel("nearrow", r"\nearrow"),
    rel("swarrow", r"\swarrow"),
    rel("searrow", r"\searrow"),
    rel("hookleft", r"\hookleftarrow"),
    rel("hookright", r"\hookrightarrow"),
    rel("mapsto", r"\mapsto"),
    // -- relations (REFERENCE §4.7) ---------------------------------------
    rel("leq", r"\leq"),
    rel("geq", r"\geq"),
    rel("neq", r"\neq"),
    rel("doteq", r"\doteq"),
    rel("sim", r"\sim"),
    rel("simeq", r"\simeq"),
    rel("approx", r"\approx"),
    rel("cong", r"\cong"),
    rel("equiv", r"\equiv"),
    rel("asymp", r"\asymp"),
    rel("identical", r"\equiv"),
    // -- sets (REFERENCE §4.8) --------------------------------------------
    rel("in", r"\in"),
    rel("owns", r"\ni"),
    rel("notin", r"\notin"),
    rel("subset", r"\subset"),
    rel("supset", r"\supset"),
    rel("subseteq", r"\subseteq"),
    rel("supseteq", r"\supseteq"),
    ord("emptyset", r"\emptyset"),
    // -- misc symbols (REFERENCE §4.9) ------------------------------------
    ord("inf", r"\infty"),
    ord("infty", r"\infty"),
    ord("partial", r"\partial"),
    rel("therefore", r"\therefore"),
    rel("because", r"\because"),
    bin("pm", r"\pm"),
    bin("plusminus", r"\pm"),
    bin("mp", r"\mp"),
    bin("minusplus", r"\mp"),
    bin("times", r"\times"),
    bin("div", r"\div"),
    bin("divide", r"\div"),
    bin("cdot", r"\cdot"),
    ord("forall", r"\forall"),
    ord("exist", r"\exists"),
    ord("prime", r"\prime"),
    bin("diamond", r"\diamond"),
    // `deg` is listed both as a symbol (§4.9) and as a base function (§4.10);
    // DESIGN.md §5 resolves it to the function `\deg` (see the Func block).
    // -- big operators (REFERENCE §4.2) -----------------------------------
    bigop("sum", r"\sum", ScriptPos::Below),
    bigop("prod", r"\prod", ScriptPos::Below),
    bigop("int", r"\int", ScriptPos::Normal),
    bigop("oint", r"\oint", ScriptPos::Normal),
    bigop("dint", r"\iint", ScriptPos::Normal),
    bigop("tint", r"\iiint", ScriptPos::Normal),
    bigop("odint", r"\oiint", ScriptPos::Normal),
    bigop("otint", r"\oiiint", ScriptPos::Normal),
    bigop_bin("union", r"\bigcup", r"\cup", ScriptPos::Below),
    bigop_bin("inter", r"\bigcap", r"\cap", ScriptPos::Below),
    bigop_bin("dsum", r"\bigoplus", r"\oplus", ScriptPos::Below),
    // `small*`: same glyph, scripts forced beside it (`\nolimits`).
    bigop("smallsum", r"\sum", ScriptPos::Beside),
    bigop("smallprod", r"\prod", ScriptPos::Beside),
    bigop("smallint", r"\int", ScriptPos::Beside),
    bigop("smalloint", r"\oint", ScriptPos::Beside),
    bigop("smallunion", r"\bigcup", ScriptPos::Beside),
    bigop("smallinter", r"\bigcap", ScriptPos::Beside),
    // -- base functions (REFERENCE §4.10) ---------------------------------
    func("sin", r"\sin"),
    func("cos", r"\cos"),
    func("tan", r"\tan"),
    func("cot", r"\cot"),
    func("sec", r"\sec"),
    func("csc", r"\csc"),
    func("cosec", r"\csc"),
    func("sinh", r"\sinh"),
    func("cosh", r"\cosh"),
    func("tanh", r"\tanh"),
    func("coth", r"\coth"),
    func("arcsin", r"\arcsin"),
    func("arccos", r"\arccos"),
    func("arctan", r"\arctan"),
    func("log", r"\log"),
    func("ln", r"\ln"),
    func("lg", r"\operatorname{lg}"),
    func("exp", r"\exp"),
    func("mod", r"\bmod"),
    func("deg", r"\deg"),
    func("arg", r"\arg"),
    func("dim", r"\dim"),
    func("hom", r"\hom"),
    func("ker", r"\ker"),
    func("if", r"\operatorname{if}"),
    func("for", r"\operatorname{for}"),
    func("and", r"\operatorname{and}"),
    func("or", r"\operatorname{or}"),
    func_below("lim", r"\lim"),
    func_below("max", r"\max"),
    func_below("min", r"\min"),
    func_below("det", r"\det"),
    func_below("gcd", r"\gcd"),
    // -- accents (REFERENCE §4.4) — one-argument macros --------------------
    accent("acute", r"\acute"),
    accent("grave", r"\grave"),
    accent("dot", r"\dot"),
    accent("ddot", r"\ddot"),
    accent("bar", r"\bar"),
    accent("vec", r"\vec"),
    accent("dyad", r"\overleftrightarrow"),
    accent("hat", r"\hat"),
    accent("check", r"\check"),
    accent("arch", r"\overparen"),
    accent("tilde", r"\tilde"),
    accent("under", r"\underline"),
];

/// Case-insensitive commands, probed with the lowercased word.
fn ci_cmd(word: &str) -> Option<Cmd> {
    Some(match word {
        // structure (REFERENCE §4.1)
        "over" => Cmd::Over,
        "atop" => Cmd::Atop,
        "sqrt" => Cmd::Sqrt,
        "root" => Cmd::Root,
        "of" => Cmd::Of,
        "left" => Cmd::Left,
        "right" => Cmd::Right,
        "not" => Cmd::Not,
        "sup" => Cmd::Sup,
        "sub" => Cmd::Sub,
        "big" => Cmd::Big(r"\big"),
        "bigg" => Cmd::Big(r"\bigg"),
        // combinations / stacking (REFERENCE §4.3)
        "choose" => Cmd::Choose,
        "binom" => Cmd::Binom,
        "buildrel" | "rel" => Cmd::BuildRel,
        "matrix" => Cmd::Matrix(MatrixKind::Plain),
        "pmatrix" => Cmd::Matrix(MatrixKind::Paren),
        "bmatrix" => Cmd::Matrix(MatrixKind::Bracket),
        "dmatrix" => Cmd::Matrix(MatrixKind::Det),
        "cases" => Cmd::Matrix(MatrixKind::Cases),
        "pile" => Cmd::Matrix(MatrixKind::Pile),
        "lpile" => Cmd::Matrix(MatrixKind::LPile),
        "rpile" => Cmd::Matrix(MatrixKind::RPile),
        "eqalign" => Cmd::Matrix(MatrixKind::EqAlign),
        // fonts (REFERENCE §4.10)
        "rm" => Cmd::Style(StyleKind::Roman),
        "it" => Cmd::Style(StyleKind::Italic),
        "bold" => Cmd::Style(StyleKind::Bold),
        "rmbold" => Cmd::Style(StyleKind::RomanBold),
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// Lookup
// ---------------------------------------------------------------------------

fn find(table: &'static [SymDef], name: &str) -> Option<&'static SymDef> {
    table.iter().find(|d| d.name == name)
}

fn ci_lookup(lower: &str) -> Option<Keyword> {
    if let Some(cmd) = ci_cmd(lower) {
        return Some(Keyword::Cmd(cmd));
    }
    find(CI_SYMS, lower).map(Keyword::Sym)
}

/// Look up a complete word. Exact (case-sensitive) table first, then the
/// case-insensitive table via ASCII lowercasing. `None` → italic identifier.
pub fn lookup(word: &str) -> Option<Keyword> {
    if let Some(cmd) = exact_cmd(word) {
        return Some(Keyword::Cmd(cmd));
    }
    if let Some(d) = find(EXACT_SYMS, word) {
        return Some(Keyword::Sym(d));
    }
    // Only allocate when the word can actually differ from its lowercasing.
    if word.bytes().any(|b| b.is_ascii_uppercase()) {
        ci_lookup(&word.to_ascii_lowercase())
    } else {
        ci_lookup(word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(word: &str) -> &'static SymDef {
        match lookup(word) {
            Some(Keyword::Sym(d)) => d,
            other => panic!("{:?}: expected a symbol, got {:?}", word, other),
        }
    }

    fn cmd(word: &str) -> Cmd {
        match lookup(word) {
            Some(Keyword::Cmd(c)) => c,
            other => panic!("{:?}: expected a command, got {:?}", word, other),
        }
    }

    /// DESIGN §2: commands live in the case-insensitive table.
    #[test]
    fn commands_are_case_insensitive() {
        assert_eq!(cmd("SQRT"), Cmd::Sqrt);
        assert_eq!(cmd("Sqrt"), Cmd::Sqrt);
        assert_eq!(cmd("sqrt"), Cmd::Sqrt);
        assert_eq!(cmd("LEFT"), Cmd::Left);
        assert_eq!(cmd("right"), Cmd::Right);
        assert_eq!(cmd("OVER"), Cmd::Over);
    }

    /// `big`/`bigg` are CI, `Big`/`Bigg` are exact — different sizes.
    #[test]
    fn delimiter_sizes() {
        assert_eq!(cmd("big"), Cmd::Big(r"\big"));
        assert_eq!(cmd("Big"), Cmd::Big(r"\Big"));
        assert_eq!(cmd("bigg"), Cmd::Big(r"\bigg"));
        assert_eq!(cmd("Bigg"), Cmd::Big(r"\Bigg"));
        // BIG is not an exact entry, so it falls through to the CI table.
        assert_eq!(cmd("BIG"), Cmd::Big(r"\big"));
    }

    /// Exact hit wins; anything else falls through to the CI table.
    #[test]
    fn arrow_case_split() {
        assert_eq!(sym("RARROW").latex, r"\Rightarrow");
        assert_eq!(sym("Rarrow").latex, r"\rightarrow");
        assert_eq!(sym("rarrow").latex, r"\rightarrow");
        assert_eq!(sym("LARROW").latex, r"\Leftarrow");
        assert_eq!(sym("larrow").latex, r"\leftarrow");
        assert_eq!(sym("LRARROW").latex, r"\Leftrightarrow");
        assert_eq!(sym("lrarrow").latex, r"\leftrightarrow");
        assert_eq!(sym("UDARROW").latex, r"\Updownarrow");
        assert_eq!(sym("udarrow").latex, r"\updownarrow");
        // long aliases
        assert_eq!(sym("Rightarrow").latex, r"\Rightarrow");
        assert_eq!(sym("rightarrow").latex, r"\rightarrow");
        assert_eq!(sym("Updownarrow").latex, r"\Updownarrow");
    }

    /// `Lim` keeps its scripts beside it; `lim`/`LIM` put them below.
    #[test]
    fn lim_case_split() {
        assert_eq!(
            (sym("Lim").latex, sym("Lim").scripts),
            (r"\lim", ScriptPos::Beside)
        );
        assert_eq!(
            (sym("lim").latex, sym("lim").scripts),
            (r"\lim", ScriptPos::Below)
        );
        // LIM misses the exact table and lands on `lim`.
        assert_eq!(
            (sym("LIM").latex, sym("LIM").scripts),
            (r"\lim", ScriptPos::Below)
        );
        // Exp/exp and Pr split the same way.
        assert_eq!(sym("Exp").latex, r"\operatorname{Exp}");
        assert_eq!(sym("exp").latex, r"\exp");
        assert_eq!(sym("EXP").latex, r"\exp");
        assert_eq!(
            (sym("Pr").latex, sym("Pr").scripts),
            (r"\Pr", ScriptPos::Below)
        );
    }

    /// Capital Greek is exact; the Latin-shaped ones are plain letters.
    #[test]
    fn greek_case_split() {
        assert_eq!(sym("Alpha").latex, "A");
        assert_eq!(sym("alpha").latex, r"\alpha");
        assert_eq!(sym("Omicron").latex, "O");
        assert_eq!(sym("omicron").latex, "o");
        assert_eq!(sym("Pi").latex, r"\Pi");
        assert_eq!(sym("pi").latex, r"\pi");
        assert_eq!(sym("Omega").latex, r"\Omega");
        assert_eq!(sym("omega").latex, r"\omega");
        // ALPHA is not exact → CI → lowercase alpha.
        assert_eq!(sym("ALPHA").latex, r"\alpha");
    }

    /// REFERENCE §3: whole-word match, so prefix pairs never collide.
    #[test]
    fn whole_word_matching() {
        // pi / pile
        assert_eq!(sym("pi").latex, r"\pi");
        assert_eq!(cmd("pile"), Cmd::Matrix(MatrixKind::Pile));
        assert_eq!(cmd("lpile"), Cmd::Matrix(MatrixKind::LPile));
        assert_eq!(cmd("rpile"), Cmd::Matrix(MatrixKind::RPile));
        // sin / sinh / sinx
        let sinh = sym("sinh");
        assert_eq!((sinh.kind, sinh.latex), (SymKind::Func, r"\sinh"));
        assert_eq!(sym("sin").latex, r"\sin");
        assert_eq!(lookup("sinx"), None);
        assert_eq!(lookup("si"), None);
        // in / inf / int / inter
        assert_eq!(sym("in").latex, r"\in");
        assert_eq!(sym("inf").latex, r"\infty");
        assert_eq!(sym("int").latex, r"\int");
        assert_eq!(sym("inter").latex, r"\bigcap");
        // dot / doteq, sim / simeq, sub / subset / subseteq, sup / supset
        assert_eq!(sym("dot").kind, SymKind::Accent);
        assert_eq!(sym("doteq").latex, r"\doteq");
        assert_eq!(sym("sim").latex, r"\sim");
        assert_eq!(sym("simeq").latex, r"\simeq");
        assert_eq!(cmd("sub"), Cmd::Sub);
        assert_eq!(sym("subset").latex, r"\subset");
        assert_eq!(sym("subseteq").latex, r"\subseteq");
        assert_eq!(cmd("sup"), Cmd::Sup);
        assert_eq!(sym("supset").latex, r"\supset");
        assert_eq!(sym("supseteq").latex, r"\supseteq");
    }

    #[test]
    fn big_operators() {
        let sum = sym("sum");
        assert_eq!(
            (sum.kind, sum.latex, sum.bin_latex, sum.scripts),
            (SymKind::BigOp, r"\sum", None, ScriptPos::Below)
        );
        let union = sym("union");
        assert_eq!((union.latex, union.bin_latex), (r"\bigcup", Some(r"\cup")));
        assert_eq!(
            (sym("inter").latex, sym("inter").bin_latex),
            (r"\bigcap", Some(r"\cap"))
        );
        assert_eq!(
            (sym("dsum").latex, sym("dsum").bin_latex),
            (r"\bigoplus", Some(r"\oplus"))
        );
        // small* share the glyph but force \nolimits and never shrink.
        let small = sym("smallsum");
        assert_eq!(
            (small.latex, small.bin_latex, small.scripts),
            (r"\sum", None, ScriptPos::Beside)
        );
        assert_eq!(sym("smallint").scripts, ScriptPos::Beside);
        assert_eq!(sym("smallunion").latex, r"\bigcup");
        // integrals never take limits below.
        assert_eq!(sym("int").scripts, ScriptPos::Normal);
        assert_eq!(sym("otint").latex, r"\oiiint");
    }

    #[test]
    fn accents_and_styles() {
        for (w, l) in [
            ("vec", r"\vec"),
            ("bar", r"\bar"),
            ("dyad", r"\overleftrightarrow"),
            ("arch", r"\overparen"),
            ("under", r"\underline"),
            ("hat", r"\hat"),
        ] {
            let d = sym(w);
            assert_eq!((d.kind, d.latex), (SymKind::Accent, l));
        }
        assert_eq!(cmd("rm"), Cmd::Style(StyleKind::Roman));
        assert_eq!(cmd("it"), Cmd::Style(StyleKind::Italic));
        assert_eq!(cmd("bold"), Cmd::Style(StyleKind::Bold));
        assert_eq!(cmd("rmbold"), Cmd::Style(StyleKind::RomanBold));
        assert_eq!(cmd("rel"), Cmd::BuildRel);
        assert_eq!(cmd("buildrel"), Cmd::BuildRel);
    }

    /// Table hygiene: the linear search must never be ambiguous.
    #[test]
    fn table_invariants() {
        for d in CI_SYMS {
            assert_eq!(
                d.name,
                d.name.to_ascii_lowercase(),
                "CI name must be lowercase"
            );
            assert!(
                ci_cmd(d.name).is_none(),
                "{} is both a CI symbol and a command",
                d.name
            );
            assert_eq!(
                CI_SYMS.iter().filter(|o| o.name == d.name).count(),
                1,
                "duplicate CI entry {}",
                d.name
            );
        }
        for d in EXACT_SYMS {
            assert!(
                d.name.bytes().any(|b| b.is_ascii_uppercase()),
                "{} has no uppercase, it belongs in the CI table",
                d.name
            );
            assert!(
                exact_cmd(d.name).is_none(),
                "{} is both an exact symbol and a command",
                d.name
            );
            assert_eq!(
                EXACT_SYMS.iter().filter(|o| o.name == d.name).count(),
                1,
                "duplicate exact entry {}",
                d.name
            );
        }
        // Only big operators carry a binary fallback (the emitter relies on it).
        for d in CI_SYMS.iter().chain(EXACT_SYMS) {
            assert!(
                d.bin_latex.is_none() || d.kind == SymKind::BigOp,
                "{}",
                d.name
            );
            assert!(!d.latex.is_empty());
        }
    }
}
