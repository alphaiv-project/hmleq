# hmleq — Design & Implementation Contract

Rust crate that parses the Hancom HWP equation script into an AST and emits LaTeX.
`docs/REFERENCE.md` is the source of truth for **language semantics**; this file is the
source of truth for **implementation behavior** (exact grammar decisions, exact LaTeX
output). Where this file makes a choice REFERENCE.md leaves open, this file wins.

## Module map / ownership

| File | Contents | Status |
|---|---|---|
| `src/error.rs` | `ParseError` | FROZEN — do not modify |
| `src/token.rs` | `Token`, `TokenKind` | FROZEN — do not modify |
| `src/ast.rs` | `Node`, `SpaceKind` | FROZEN — do not modify |
| `src/lib.rs`, `src/bin/hmleq.rs` | public API, CLI | FROZEN — do not modify |
| `src/symbols.rs` | keyword tables + `lookup()` | agent **symbols** (pub types are FROZEN; rewrite the rest) |
| `src/lexer.rs` | `lex()` | agent **lexer** |
| `src/parser.rs` | `parse()` | agent **parser** |
| `src/latex.rs` | `to_latex()` | agent **latex** |
| `tests/examples.rs` | integration tests | agent **tests** |

No external crates. Rust 2021, std only. Each agent owns exactly one file and must not
edit any other.

## 1. Lexer — `lex(src: &str) -> Result<Vec<Token>, ParseError>`

Scan left to right; every token records its byte span (`start`, `end`).

1. **Whitespace** (space, tab, `\n`, `\r`): skipped — separates tokens, never emitted.
2. **Structural single chars** → their `TokenKind`: `{` LBrace, `}` RBrace, `^` Caret,
   `_` Underscore, `#` Hash, `&` Amp, `~` Tilde, `` ` `` Backquote.
3. **`"` … `"`** → `Quoted(inner)` with the quotes stripped. No escape processing (HWP has
   none). EOF before the closing quote → `ParseError("unterminated quoted text")` at the
   opening quote's offset.
4. **Letters** — ASCII `A–Z a–z` and any non-ASCII `char::is_alphabetic` character:
   maximal run → `Word`. Digits never continue a word (`x2` → `Word("x")`, `Number("2")`).
5. **ASCII digits**: maximal run → `Number`. One interior `.` is included only when
   flanked by digits on both sides: `3.14` is one Number; `3.` → `Number("3")` `Op(".")`;
   `.5` → `Op(".")` `Number("5")`.
6. **Anything else** (operators/punctuation): try the 2-char ligatures first, at the
   current position: `+-` `-+` `!=` `<=` `>=` `<<` `>>` `||` → single `Op(ligature)`.
   Otherwise → `Op(single char)`.

## 2. Keyword lookup — `symbols::lookup(word: &str) -> Option<Keyword>`

Whole-word matching only (REFERENCE.md §3): the parser passes a complete `Word` token;
`lookup` never splits or prefix-matches.

1. Try the **exact, case-sensitive** table.
2. Else ASCII-lowercase the word and try the **case-insensitive** table.
3. Else `None` → the parser makes an italic `Ident`.

**Case-sensitive entries** (exact spellings only — these exist because capitalization
changes the meaning): `Lim`, `Exp`, `Pr`, the capital Greek names `Alpha Beta Gamma Delta
Epsilon Zeta Eta Theta Iota Kappa Lambda Mu Nu Xi Omicron Pi Rho Sigma Tau Upsilon Phi
Chi Psi Omega`, the double arrows `LARROW RARROW LRARROW UDARROW` and long forms
`Leftarrow Rightarrow Leftrightarrow Updownarrow`, and the sizes `Big Bigg`.

Everything else lives in the case-insensitive table keyed by lowercase: `sqrt left right
rarrow lim big bigg pi …`. Consequences: `SQRT`→sqrt, `Sqrt`→sqrt, `RARROW`→`\Rightarrow`
(exact hit wins), `Rarrow`→`\rightarrow` (falls through to CI), `LIM`→`\lim`.

## 3. Parser — recursive descent

`parser::parse(src)` lexes internally, parses a *sequence* to EOF, and returns the
collapsed node (`Node::seq`). A **sequence** is a `Vec<Node>` parsed until its stop token
(`}` for groups, `RIGHT` for delimited bodies, EOF at top level); collapse with
`Node::seq` (1 item → itself, else `Row`; empty → `Row(vec![])`).

### Infix forms (handled at sequence level)

`over` / `atop` / `choose`: pop the **last item already in the sequence** as the left
operand (empty sequence → error `"over: missing left operand"` etc.), parse **one**
scripted-term as the right operand (missing → `"over: missing right operand"`), push
`Frac{bar: over→true, atop→false}` or `Binom`. Left-associative: `a over b over c` =
`(a/b)/c`.

### scripted-term

One *prefixed-primary*, then any number of script markers: `Caret`/keyword `sup` →
superscript; `Underscore`/keyword `sub` → subscript. Each marker takes exactly **one**
following prefixed-primary as its argument. A second sup (or sub) on the same base →
error `"duplicate superscript"` / `"duplicate subscript"`. Wrap in
`Script{base, sub, sup}` only if at least one script present.

### prefixed-primary

- **Accent keyword** (`SymKind::Accent`) → base := next prefixed-primary (scripts do NOT
  bind inside) → `Accent`. So `vec A^2` = `Script{base: Accent(vec, A), sup: 2}`.
- **`not`** → next scripted-term → `Not`.
- **`big|Big|bigg|Bigg`** → next prefixed-primary → `Big{size, arg}`.
- **`sqrt`** → next scripted-term → `Sqrt`. (`sqrt x^2` = `Sqrt(x²)`.)
- **`root`** → degree := scripted-term; expect keyword `of` (else error
  `"root: expected 'of'"`); radicand := scripted-term → `Root`.
- **`binom`** → two scripted-terms → `Binom`.
- **`buildrel` / `rel`** → top := scripted-term; expect `over` (else error
  `"buildrel: expected 'over'"`); base := scripted-term → `BuildRel`.
- **Matrix keywords** `matrix pmatrix bmatrix dmatrix cases pile lpile rpile eqalign` →
  expect `LBrace` (else error `"matrix: expected '{'"` with the actual keyword name);
  parse items to the matching `RBrace`; split rows on `Hash` and cells on `Amp` **at this
  depth only** (nested groups/matrices keep theirs); each cell is a collapsed sequence
  (empty cell → `Row(vec![])`, kept, not trimmed) → `Matrix{kind, rows}`.
- **Style keywords** `rm it bold rmbold` → body := the rest of the current sequence
  (parse remaining items with full infix handling until the sequence's stop token), then
  `Style{style, body}`. Thus `rm` scope ends at the enclosing `}` / `RIGHT` / EOF, and a
  later `it` inside simply starts a nested `Style{Italic, …}` covering *its* remainder.
- **`LEFT`** → next token must be an `Op` whose text is a valid delimiter (§5 delimiter
  map) — else error `"LEFT: expected delimiter"`; body := sequence stopping at `RIGHT`
  (EOF first → error `"LEFT without RIGHT"`); after `RIGHT`, read its delimiter the same
  way (error `"RIGHT: expected delimiter"`) → `Delimited{left, right, body}`.
  A `RIGHT` outside any LEFT body → error `"RIGHT without LEFT"`.
- **`LBrace`** → sequence to `RBrace` (EOF → error `"unclosed '{'"` at the opening
  brace) → the collapsed node. Grouping is invisible but binds as ONE term.
  A stray `RBrace` → error `"unmatched '}'"`.
- **`Tilde`** → `Space(Full)`; **`Backquote`** → `Space(Quarter)`; **`Hash`** →
  `Newline`; **`Amp`** → `Align` (when not consumed by a matrix).
- **`Number`** → `Node::Number`; **`Quoted`** → `Node::Text`; **`Op`** → `Node::Op`
  (verbatim — ligature mapping happens in the emitter).
- **`Word`** → `lookup()`:
  - `Cmd` → the constructs above (`Cmd::Of` outside a `root` form → plain
    `Ident("of")`-like treatment: emit as identifier `of`);
  - `Sym` with kind `Accent` → accent rule above; any other `Sym` → `Node::Symbol`;
  - `None` → `Node::Ident` (original spelling preserved).
- A script marker (`^`/`_`/`sup`/`sub`) where a primary is required → error
  `"superscript without a base"` / `"subscript without a base"`. Any other keyword
  appearing where its operands are impossible → error naming the keyword.

All errors carry the byte offset of the offending token.

## 4. AST

`src/ast.rs` is authoritative and frozen. Notes: `Row` doubles as group content — there
is no Group node; a braced group is its collapsed content, and binding decisions were
already made by the parser. `Script` may wrap any base; the emitter decides limit
placement from the base symbol's `ScriptPos`.

## 5. LaTeX emitter — `latex::to_latex(&Node) -> String`

**Golden rules:** children of a `Row` are emitted then **joined with exactly one space**.
Macro arguments are **always braced** — `\frac{…}{…}`, `\sqrt{…}`, `_{…}`, `^{…}`,
`\vec{…}`, `\text{…}`, `\binom{…}{…}`, `\overset{…}{…}`, `\mathrm{…}` — even when the
argument is a single token. The only exception is a **script base**, emitted bare unless
it is a `Row` of ≥ 2 items (then wrapped in `{…}`), and the **root degree**, emitted bare
inside `\sqrt[…]`.

| Node | Output |
|---|---|
| `Number`/`Ident`/`Op` (non-ligature) | verbatim (`sinx` stays `sinx`) |
| `Op` ligature | `+-`→`\pm` `-+`→`\mp` `!=`→`\neq` `<=`→`\leq` `>=`→`\geq` `<<`→`\ll` `>>`→`\gg` `||`→`\Vert` |
| `Text(s)` | `\text{s}` — escape `\ { } $ & # _ ^ % ~` (backslash forms; `\textbackslash{}` for `\`, `\textasciitilde{}` for `~`, `\textasciicircum{}` for `^`) |
| `Symbol(d)` | `d.latex`; **BigOp not directly under a `Script`** with `bin_latex: Some(b)` → `b` |
| `Space(Full)` | `\;` |
| `Space(Quarter)` | `\,` |
| `Newline` / `Align` | `\\` / `&` |
| `Frac{bar:true}` | `\frac{num}{den}` |
| `Frac{bar:false}` | `{num \atop den}` |
| `Binom` | `\binom{top}{bottom}` |
| `Sqrt` | `\sqrt{x}` |
| `Root` | `\sqrt[deg]{x}` (deg emitted bare) |
| `Script` | base′ then `_{sub}` then `^{sup}` (sub first when both). base′: bare unless `Row`≥2 → `{…}`. If base is `Symbol` with `scripts: Beside` → emit `d.latex` + `\nolimits` before the scripts. `Below`/`Normal` → nothing extra. A BigOp base always uses `d.latex` (never `bin_latex`). |
| `Delimited` | `\left` + mapped L + ` ` + body + ` ` + `\right` + mapped R. Delimiter map: `(` `)` `[` `]` `|` `.` as-is; `||`→`\Vert`; `<`→`\langle`; `>`→`\rangle`. (Same map validates delimiters in the parser.) |
| `Matrix` | `\begin{ENV} c & c \\ c & c \end{ENV}` — single spaces around cells, `&`, `\\`. ENV: matrix→`matrix`, pmatrix→`pmatrix`, bmatrix→`bmatrix`, dmatrix→`vmatrix`, cases→`cases`, eqalign→`aligned`. pile/lpile/rpile → `\begin{array}{c|l|r} … \end{array}` (single column; rows joined with ` \\ `). |
| `Accent{a, base}` | `a.latex{base}` e.g. `\vec{A}` |
| `Style` | Roman→`\mathrm{body}`; Bold→`\boldsymbol{body}`; RomanBold→`\mathbf{body}`; Italic→body unwrapped |
| `BuildRel` | `\overset{top}{base}` |
| `Not(x)` | `\not ` + emit(x) (space-joined: `\not =`, `\not \in`) |
| `Big{size, arg}` | size + ` ` + emit(arg) → `\bigg /` |

**Top level only:** if the root node is a `Row` containing a `Newline`, wrap the whole
output in `\begin{aligned} … \end{aligned}` when an `Align` is also present at top level,
else `\begin{gathered} … \end{gathered}` (Newlines emit as `\\` either way). No wrapper
otherwise.

### Symbol table policy (agent: symbols)

Default mapping: the LaTeX command spelled like the keyword (`\alpha`, `\beta`,
`\subseteq`, `\supset`, `\forall`, `\partial`, `\times`, `\cdot`, `\therefore`,
`\because`, `\approx`, `\cong`, `\equiv`, `\asymp`, `\sim`, `\simeq`, `\doteq`, `\leq`,
`\geq`, `\neq`, `\in`, `\notin`, `\subset`, `\emptyset`, `\diamond`, `\aleph`, `\hbar`,
`\imath`, `\jmath`, `\wp`, `\mapsto`, `\nwarrow`, `\nearrow`, `\swarrow`, `\searrow`,
`\uparrow`, `\downarrow`, …). Explicit decisions:

- `inf` → `\infty` (accept alias `infty`); `prime` → `\prime`; `exist` → `\exists`;
  `owns` → `\ni`; `identical` → `\equiv`; `omicron` → `o`.
- `pm`|`plusminus` → `\pm`; `mp`|`minusplus` → `\mp`; `div`|`divide` → `\div`.
- Arrows: `larrow`→`\leftarrow`, `rarrow`→`\rightarrow`, `lrarrow`→`\leftrightarrow`,
  `udarrow`→`\updownarrow`; exact `LARROW`→`\Leftarrow`, `RARROW`→`\Rightarrow`,
  `LRARROW`→`\Leftrightarrow`, `UDARROW`→`\Updownarrow`; `hookleft`→`\hookleftarrow`,
  `hookright`→`\hookrightarrow`; CI long aliases `leftarrow rightarrow leftrightarrow
  updownarrow` and exact `Leftarrow Rightarrow Leftrightarrow Updownarrow`.
- Big operators (kind `BigOp`): `sum`→`\sum`, `prod`→`\prod` (scripts `Below`,
  `bin_latex: None`); `union`→`\bigcup`/bin `\cup`, `inter`→`\bigcap`/bin `\cap`,
  `dsum`→`\bigoplus`/bin `\oplus` (scripts `Below`); `int`→`\int`, `oint`→`\oint`,
  `dint`→`\iint`, `tint`→`\iiint`, `odint`→`\oiint`, `otint`→`\oiiint` (scripts
  `Normal`, `bin_latex: None`); `smallsum smallprod smallint smalloint smallunion
  smallinter` → same latex as the base operator (`\sum`, …, `\bigcup`, `\bigcap`) with
  scripts `Beside`, `bin_latex: None`.
- Functions (kind `Func`, scripts `Normal` unless noted): `sin cos tan cot sec csc sinh
  cosh tanh coth arcsin arccos arctan log ln exp deg arg dim hom ker` → the same-named
  LaTeX builtin (`\sin` …); `cosec`→`\csc`; `lg`→`\operatorname{lg}`; `mod`→`\bmod`;
  `if for and or` → `\operatorname{if}` etc. Scripts `Below`: `lim max min det gcd` and
  exact `Pr` (latex `\lim \max \min \det \gcd \Pr`). Exact `Lim` → latex `\lim`, scripts
  `Beside`. Exact `Exp` → `\operatorname{Exp}`, scripts `Normal`.
- Greek capitals (exact): `Gamma Delta Theta Lambda Xi Pi Sigma Upsilon Phi Psi Omega` →
  `\Gamma` …; the Latin-shaped ones map to plain letters: `Alpha`→`A` `Beta`→`B`
  `Epsilon`→`E` `Zeta`→`Z` `Eta`→`H` `Iota`→`I` `Kappa`→`K` `Mu`→`M` `Nu`→`N`
  `Omicron`→`O` `Rho`→`P` `Tau`→`T` `Chi`→`X`.
- Variants: `vartheta varpi varsigma varphi varepsilon` → same-named; `varupsilon` →
  `\Upsilon`.
- Specials: `ohm`→`\Omega`; `ell`→`\ell`; `liter`→`\ell`; `imag`→`\Im`;
  `angstrom`→`\mathring{A}` (kind `Ord`).
- Accents (kind `Accent`): `acute grave dot ddot bar vec hat check tilde` → same-named
  one-arg macros; `dyad`→`\overleftrightarrow`; `arch`→`\overparen`;
  `under`→`\underline`.

Store defs as `static` arrays of `SymDef` and search linearly (≈250 entries — fine);
`Cmd`s via a `match` on the (exact, then lowercased) word. Keep the FROZEN pub types
exactly as they are.

## 6. Canonical outputs — tests assert these EXACT strings

| # | script | latex |
|---|---|---|
| 1 | `x = {-b +- sqrt{b^2 -4ac}} over {2a}` | `x = \frac{- b \pm \sqrt{b^{2} - 4 ac}}{2 a}` |
| 2 | `lim _{x rarrow 0} {sin x} over x = 1` | `\lim_{x \rightarrow 0} \frac{\sin x}{x} = 1` |
| 3 | `int _1 ^2 {3x^2} dx = LEFT[ x^3 RIGHT] _1 ^2 = 7` | `\int_{1}^{2} 3 x^{2} dx = \left[ x^{3} \right]_{1}^{2} = 7` |
| 4 | `sum _{n=1} ^{inf} {1 over n^2} = {pi^2} over 6` | `\sum_{n = 1}^{\infty} \frac{1}{n^{2}} = \frac{\pi^{2}}{6}` |
| 5 | `pmatrix { a_1 & b_1 # a_2 & b_2 }` | `\begin{pmatrix} a_{1} & b_{1} \\ a_{2} & b_{2} \end{pmatrix}` |
| 6 | `f(x) = cases { x^2 & (x geq 0) # -x & (x < 0) }` | `f ( x ) = \begin{cases} x^{2} & ( x \geq 0 ) \\ - x & ( x < 0 ) \end{cases}` |
| 7 | `A inter B = { x \| x in A ~and~ x in B }` | `A \cap B = x \| x \in A \; \operatorname{and} \; x \in B` |
| 8 | `sinh x` | `\sinh x` |
| 9 | `sinx` | `sinx` |
| 10 | `sin x` | `\sin x` |
| 11 | `pi le` | `\pi le` |
| 12 | `pile {a # b}` | `\begin{array}{c} a \\ b \end{array}` |
| 13 | `not =` | `\not =` |
| 14 | `a^2 2` | `a^{2} 2` |
| 15 | `x sub i sup 2` | `x_{i}^{2}` |
| 16 | `root 3 of {x+1}` | `\sqrt[3]{x + 1}` |
| 17 | `n choose k` | `\binom{n}{k}` |
| 18 | `SQRT 2` | `\sqrt{2}` |
| 19 | `vec A` | `\vec{A}` |
| 20 | `"hello world"` | `\text{hello world}` |
| 21 | `LEFT ( x over y RIGHT )` | `\left( \frac{x}{y} \right)` |
| 22 | `a != b` | `a \neq b` |
| 23 | `Lim _{n} a_n` | `\lim\nolimits_{n} a_{n}` |
| 24 | `H_2 O` | `H_{2} O` |
| 25 | `Alpha + alpha` | `A + \alpha` |
| 26 | `rm ABC` | `\mathrm{ABC}` |
| 27 | `a over b over c` | `\frac{\frac{a}{b}}{c}` |
| 28 | `{a + b}^2` | `{a + b}^{2}` |
| 29 | `x atop y` | `{x \atop y}` |
| 30 | `A union B` | `A \cup B` |
| 31 | `union _{i=1} ^{n} A_i` | `\bigcup_{i = 1}^{n} A_{i}` |
| 32 | `vec A^2` | `\vec{A}^{2}` |
| 33 | `a ~ b `` ` `` c` | `a \; b \, c` |
| 34 | `buildrel def over =` | `\overset{def}{=}` |
| 35 | `smallsum _{k} a_k` | `\sum\nolimits_{k} a_{k}` |

Notes: in #7, `\|` inside the table cells denotes a literal pipe character `|` in both
the input and the output. #33 written out: the input is `a`, tilde, `b`, backquote, `c`
(i.e. ``a ~ b ` c``) and the output is `a \; b \, c`.

**Error cases** (must be `Err`, message containing the quoted phrase):
`{x` → "unclosed '{'" · `x }` → "unmatched '}'" · `1 over` → "missing right operand" ·
`over 2` → "missing left operand" · `root 3 x` → "expected 'of'" · `LEFT ( x` →
"LEFT without RIGHT" · `"abc` → "unterminated quoted text" · `^2` →
"superscript without a base" · `a^1^2` → "duplicate superscript".

## 7. Testing

- `tests/examples.rs`: every row of the §6 table via `hmleq::eq_to_latex`, each error
  case via `.unwrap_err()` checking `message.contains(...)`, plus the REFERENCE.md §3
  keyword-matching invariants already covered by rows 8–12.
- Module owners add small `#[cfg(test)]` unit tests in their own file where cheap
  (lexer spans/ligatures, `lookup` case rules, parser binding shapes).
- The lib.rs doctest also runs; keep it passing.
