//! Integration tests for hmleq — public API only (`eq_to_latex`, `parse`).
//!
//! Every row of the canonical table in `docs/DESIGN.md` §6 is asserted here as an
//! exact string, together with each listed error case and a handful of extra
//! behaviours that `docs/REFERENCE.md` describes and DESIGN.md pins down exactly.
//!
//! These strings are the contract: when one of them fails, the implementation is
//! wrong, not the expectation. Inputs are transcribed verbatim from the table —
//! note that in the table's markdown cells `\|` stands for a literal `|` (row 7).
#![cfg(feature = "latex")]

use hmleq::{eq_to_latex, parse};

/// Assert the exact LaTeX for a script; reports the input on mismatch.
#[track_caller]
fn check(script: &str, expected: &str) {
    match eq_to_latex(script) {
        Ok(got) => assert_eq!(got, expected, "input: {:?}", script),
        Err(e) => panic!("input {:?}: expected Ok, got error: {}", script, e),
    }
}

/// Assert the script fails to parse and the message contains `needle`.
#[track_caller]
fn check_err(script: &str, needle: &str) {
    let err = parse(script).unwrap_err();
    assert!(
        err.message.contains(needle),
        "input {:?}: message {:?} (at byte {}) does not contain {:?}",
        script,
        err.message,
        err.position,
        needle
    );
}

// ---------------------------------------------------------------------------
// DESIGN.md §6 — canonical table, rows 1–35
// ---------------------------------------------------------------------------

/// Rows 1–7: the worked examples from REFERENCE.md §5.
#[test]
fn canonical_reference_examples() {
    // #1 quadratic formula
    check(
        "x = {-b +- sqrt{b^2 -4ac}} over {2a}",
        r"x = \frac{- b \pm \sqrt{b^{2} - 4 ac}}{2 a}",
    );
    // #2 limit
    check(
        "lim _{x rarrow 0} {sin x} over x = 1",
        r"\lim_{x \rightarrow 0} \frac{\sin x}{x} = 1",
    );
    // #3 definite integral
    check(
        "int _1 ^2 {3x^2} dx = LEFT[ x^3 RIGHT] _1 ^2 = 7",
        r"\int_{1}^{2} 3 x^{2} dx = \left[ x^{3} \right]_{1}^{2} = 7",
    );
    // #4 series
    check(
        "sum _{n=1} ^{inf} {1 over n^2} = {pi^2} over 6",
        r"\sum_{n = 1}^{\infty} \frac{1}{n^{2}} = \frac{\pi^{2}}{6}",
    );
    // #5 matrix
    check(
        "pmatrix { a_1 & b_1 # a_2 & b_2 }",
        r"\begin{pmatrix} a_{1} & b_{1} \\ a_{2} & b_{2} \end{pmatrix}",
    );
    // #6 cases
    check(
        "f(x) = cases { x^2 & (x geq 0) # -x & (x < 0) }",
        r"f ( x ) = \begin{cases} x^{2} & ( x \geq 0 ) \\ - x & ( x < 0 ) \end{cases}",
    );
    // #7 set-builder. The `|` is a literal pipe on both sides; a lone `|` is not
    // a ligature (only `||` is), so it survives verbatim into the output.
    check(
        "A inter B = { x | x in A ~and~ x in B }",
        r"A \cap B = x | x \in A \; \operatorname{and} \; x \in B",
    );
}

/// Rows 8–12: whole-word keyword matching (REFERENCE.md §3) — neither a
/// prefix-free keyword set nor maximal munch; the whole word must match.
#[test]
fn canonical_whole_word_keyword_matching() {
    check("sinh x", r"\sinh x"); // #8  not sin + h
    check("sinx", "sinx"); // #9  no keyword, plain italic identifier
    check("sin x", r"\sin x"); // #10 keyword only when it is a whole word
    check("pi le", r"\pi le"); // #11 pi + le, not the `pile` command
    check("pile {a # b}", r"\begin{array}{c} a \\ b \end{array}"); // #12 the command
}

/// Rows 13–22: prefix commands, scripts, roots, delimiters.
#[test]
fn canonical_scripts_roots_and_delimiters() {
    check("not =", r"\not ="); // #13
    check("a^2 2", r"a^{2} 2"); // #14 `^` binds exactly one following term
    check("x sub i sup 2", r"x_{i}^{2}"); // #15 `sub`/`sup` keywords == `_`/`^`
    check("root 3 of {x+1}", r"\sqrt[3]{x + 1}"); // #16 degree emitted bare
    check("n choose k", r"\binom{n}{k}"); // #17
    check("SQRT 2", r"\sqrt{2}"); // #18 commands are case-insensitive
    check("vec A", r"\vec{A}"); // #19 accent takes the following term
    check(r#""hello world""#, r"\text{hello world}"); // #20
    check("LEFT ( x over y RIGHT )", r"\left( \frac{x}{y} \right)"); // #21
    check("a != b", r"a \neq b"); // #22 two-char ligature
}

/// Rows 23–35: symbol/limit placement, styles, stacking, printed spaces.
#[test]
fn canonical_symbols_styles_and_spacing() {
    check("Lim _{n} a_n", r"\lim\nolimits_{n} a_{n}"); // #23 case matters: Lim != lim
    check("H_2 O", r"H_{2} O"); // #24
    check("Alpha + alpha", r"A + \alpha"); // #25 Latin-shaped capital Greek
    check("rm ABC", r"\mathrm{ABC}"); // #26 style runs to the end of the group
    check("a over b over c", r"\frac{\frac{a}{b}}{c}"); // #27 left-associative
    check("{a + b}^2", r"{a + b}^{2}"); // #28 multi-item script base gets braces
    check("x atop y", r"{x \atop y}"); // #29
    check("A union B", r"A \cup B"); // #30 bare big operator uses its binary form
    check("union _{i=1} ^{n} A_i", r"\bigcup_{i = 1}^{n} A_{i}"); // #31 scripted: big form
    check("vec A^2", r"\vec{A}^{2}"); // #32 scripts do not bind inside the accent

    // #33 written out: `a`, tilde, `b`, backquote, `c`.
    check("a ~ b ` c", r"a \; b \, c");
    check("buildrel def over =", r"\overset{def}{=}"); // #34
    check("smallsum _{k} a_k", r"\sum\nolimits_{k} a_{k}"); // #35
}

// ---------------------------------------------------------------------------
// DESIGN.md §6 — error cases
// ---------------------------------------------------------------------------

#[test]
fn error_cases() {
    check_err("{x", "unclosed '{'");
    check_err("x }", "unmatched '}'");
    check_err("1 over", "missing right operand");
    check_err("over 2", "missing left operand");
    check_err("root 3 x", "expected 'of'");
    check_err("LEFT ( x", "LEFT without RIGHT");
    check_err(r#""abc"#, "unterminated quoted text");
    check_err("^2", "superscript without a base");
    check_err("a^1^2", "duplicate superscript");
}

/// Errors carry the byte offset of the offending token (DESIGN.md §1, §3).
#[test]
fn error_positions_point_at_the_offending_token() {
    assert_eq!(parse("{x").unwrap_err().position, 0); // the opening brace
    assert_eq!(parse(r#""abc"#).unwrap_err().position, 0); // the opening quote
    assert_eq!(parse("x }").unwrap_err().position, 2); // the stray brace
}

/// `eq_to_latex` surfaces the same failures as `parse`.
#[test]
fn eq_to_latex_propagates_errors() {
    assert!(eq_to_latex("{x").is_err());
    assert!(parse("1 over 2").is_ok());
}

// ---------------------------------------------------------------------------
// Extra REFERENCE.md behaviours (exact outputs fixed by DESIGN.md)
// ---------------------------------------------------------------------------

/// Basic forms from REFERENCE.md §4.1 / §6 that the canonical table only implies.
#[test]
fn extra_basic_forms() {
    check("E = mc^2", r"E = mc^{2}"); // §4.1 — `mc` is one non-keyword word
    check("sqrt 2", r"\sqrt{2}"); // §4.1 — argument is always braced

    // §6: several terms under a script must be grouped with `{}`.
    check("a^{2b}", r"a^{2 b}");
    // §3.2: breaking the word is how you force a function name to stay italic.
    check("si n", "si n");
    // §1 lexing: a digit never continues a word, so `x2` is two terms.
    check("x2", "x 2");
    // §4.3: `binom` is the two-argument spelling of `choose`.
    check("binom {n}{k}", r"\binom{n}{k}");
    // `of` outside a `root` form degrades to an ordinary identifier.
    check("a of b", "a of b");
    // An empty script is a well-formed (empty) equation.
    assert_eq!(eq_to_latex("").unwrap(), "");
}

/// `over` draws the bar, `atop` does not (REFERENCE.md §4.1).
#[test]
fn extra_over_versus_atop() {
    check("x over y", r"\frac{x}{y}");
    check("x atop y", r"{x \atop y}");
    check("1 over 2", r"\frac{1}{2}");
    // `rel` is the documented alias of `buildrel`.
    check("rel x over y", r"\overset{x}{y}");
}

/// Row/column structures (REFERENCE.md §4.3) and their LaTeX environments.
#[test]
fn extra_matrix_environments() {
    check(
        "eqalign { x & = 1 # y & = 2 }",
        r"\begin{aligned} x & = 1 \\ y & = 2 \end{aligned}",
    );
    check(
        "matrix { a & b # c & d }",
        r"\begin{matrix} a & b \\ c & d \end{matrix}",
    );
    check(
        "bmatrix { a & b # c & d }",
        r"\begin{bmatrix} a & b \\ c & d \end{bmatrix}",
    );
    check(
        "dmatrix { a & b # c & d }",
        r"\begin{vmatrix} a & b \\ c & d \end{vmatrix}",
    );
    check("lpile {a # b}", r"\begin{array}{l} a \\ b \end{array}");
    check("rpile {a # b}", r"\begin{array}{r} a \\ b \end{array}");
}

/// Most keywords are case-insensitive; a documented capitalized spelling wins
/// over it when it exists (REFERENCE.md §1, §6; DESIGN.md §2).
#[test]
fn extra_keyword_case_rules() {
    check("COS x", r"\cos x");
    check("Sin x", r"\sin x");
    check("1 OVER 2", r"\frac{1}{2}");
    check("PI", r"\pi"); // no exact entry `PI` → falls through to `pi`
    check("Pi", r"\Pi"); // exact entry wins
    check("LIM _{n} a", r"\lim_{n} a"); // limits below, unlike `Lim`
    check("RARROW", r"\Rightarrow"); // exact entry
    check("Rarrow", r"\rightarrow"); // no exact entry → CI `rarrow`
}

/// Quoted text is one word verbatim — required for words of 9+ characters
/// (REFERENCE.md §2, §6) — and is escaped for LaTeX on output.
#[test]
fn extra_quoted_text() {
    check(r#""derivative""#, r"\text{derivative}"); // 10 characters
    check(r#"f = "if and only if""#, r"f = \text{if and only if}");
    check(r#""a_b""#, r"\text{a\_b}"); // LaTeX-special characters are escaped
}

/// Printed spaces and line breaks (REFERENCE.md §2). A top-level `#` wraps the
/// whole output; `&` alongside it selects `aligned` over `gathered`.
#[test]
fn extra_spaces_and_line_breaks() {
    check("a ~ b", r"a \; b");
    check("a ` b", r"a \, b");
    check(
        "a = 1 # b = 2",
        r"\begin{gathered} a = 1 \\ b = 2 \end{gathered}",
    );
    check(
        "a & = 1 # b & = 2",
        r"\begin{aligned} a & = 1 \\ b & = 2 \end{aligned}",
    );
}

/// Stretchy delimiters (REFERENCE.md §4.1) and the `not` / `bigg` prefixes.
#[test]
fn extra_delimiters_and_prefixes() {
    check("LEFT < x RIGHT >", r"\left\langle x \right\rangle");
    check("LEFT | x RIGHT |", r"\left| x \right|");
    check("not in", r"\not \in");
    check(
        "{a+b} over {a-b} bigg / {x+y}",
        r"\frac{a + b}{a - b} \bigg / x + y",
    );
}
