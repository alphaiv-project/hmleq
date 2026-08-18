# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`hmleq` parses the Hancom HWP(한글) equation script language (`x = {-b +- sqrt{b^2 - 4ac}} over {2a}`) into a serde-serializable AST, with an optional LaTeX emitter. Cargo workspace: the root crate is the `hmleq` library; `cli/` holds the `hmleq-cli` package, whose binary is named `hmleq`.

README.md and docs/REFERENCE.md are written in Korean; docs/DESIGN.md and all code comments are in English.

## Commands

```bash
cargo test                    # default features (latex)
cargo test --all-features     # + serde round-trip tests
cargo test --all-features canonical_reference_examples   # single test by name
cargo fmt --all --check
cargo clippy --workspace --all-features -- -D warnings
cargo check -p hmleq --no-default-features   # zero-dep parser/AST-only build must keep compiling
```

CI runs exactly those four check/test commands. The `--no-default-features` build is advertised in the README, so it must not break.

Run the CLI:

```bash
cargo run -q -p hmleq-cli -- 'lim _{x rarrow 0} {sin x} over x = 1'   # LaTeX out
cargo run -q -p hmleq-cli -- --json '1 over pi'                        # AST as JSON
echo 'pmatrix { a & b # c & d }' | cargo run -q -p hmleq-cli           # stdin
```

## Architecture

Single pipeline, one module per stage:

- `src/lexer.rs` -- delimiter-based tokenizer. Whitespace splits and is dropped; maximal alphabetic runs become one `Word` token; 2-char operator ligatures (`+-`, `!=`, ...) are tried before single-char `Op`s. Every token carries its byte span; `ParseError` positions are byte offsets.
- `src/symbols.rs` -- keyword tables (~250 `SymDef` entries, linear search) plus `lookup()`. **Whole-word exact matching only**: the exact case-sensitive table is tried first (keywords where capitalization changes meaning: `Lim`, `RARROW`, capital Greek...), then a case-insensitive table keyed by the ASCII-lowercased word. `None` means ordinary italic identifier -- so `sinh` is one keyword, `sinx` is the identifier *sinx*. Lookup is never called on a prefix.
- `src/parser.rs` -- recursive descent over the token stream. Sequences are parsed to a `Stop` (EOF / `}` / `RIGHT` / matrix cell); `over`/`atop`/`choose` are infix and pop the last already-parsed item as their left operand, which makes them left-associative. Matrix row/cell splitting on `#`/`&` is depth-local.
- `src/ast.rs` -- `Node`. There is no Group node: a `{...}` group collapses to its content (`Node::seq`), binding already decided by the parser. With the `serde` feature, `Symbol`/`Accent` serialize as the canonical keyword name and deserialize by re-resolving through the keyword table, so only real keywords are accepted; `Big` sizes are validated against the four LaTeX size commands.
- `src/latex.rs` -- emitter. Golden rules: `Row` children are joined with exactly one space; macro arguments are always braced, even single tokens (exceptions: script bases, bare unless a `Row` of ≥2 items, and root degrees). Ligature->LaTeX mapping and delimiter mapping live here, not in the parser. The `aligned`/`gathered` wrapper is decided only at top level in `to_latex`.

Features: `latex` (default) gates the emitter; `serde` gates AST serialization. With `default-features = false` the core is dependency-free.

## The docs are the spec

- `docs/REFERENCE.md` -- source of truth for **language semantics** (token rules, keyword matching, full command list).
- `docs/DESIGN.md` -- source of truth for **implementation behavior**: exact grammar decisions, exact LaTeX output, symbol-table mappings, exact error messages. Where DESIGN.md decides something REFERENCE.md leaves open, DESIGN.md wins.
- `tests/examples.rs` asserts every row of DESIGN.md §6's canonical table as an **exact string**, plus the listed error cases. These strings are the contract: an intentional output change means updating the §6 table and the test together; otherwise a failing test means the implementation is wrong, not the expectation.
- The FROZEN / per-file agent-ownership annotations in DESIGN.md's module map are historical (from the initial multi-agent build) -- the behavioral spec still binds, the ownership rules do not.

## Conventions

- Code comments must be self-contained: describe the behavior in place, never cite DESIGN.md/REFERENCE.md section numbers or table rows (deliberate decision, commit 37477bf).
- Comments and string literals in source stay ASCII.
- Conventional-commit messages (`feat:`, `chore(comments):`, `refactor(cli):`, ...).
