//! AST for the HWP equation script. Produced by `parser`, consumed by `latex`.
//!
//! With the `serde` feature the whole tree derives `Serialize`/`Deserialize`
//! (externally tagged, serde's default): `{"Frac":{"num":…,"den":…,"bar":true}}`.
//! Symbols serialize as their canonical keyword name (`"pi"`, `"Lim"`) and are
//! resolved back through the keyword table on deserialization, so trees
//! round-trip exactly; a `Big` size is validated against the four LaTeX size
//! commands the parser can produce.

use crate::symbols::{MatrixKind, StyleKind, SymDef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SpaceKind {
    /// `~` — one full space.
    Full,
    /// `` ` `` — quarter space.
    Quarter,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// Serialized as the symbol's canonical name.
    Symbol(#[cfg_attr(feature = "serde", serde(with = "symdef_name"))] &'static SymDef),
    /// `A over B` (bar = true) / `A atop B` (bar = false).
    Frac {
        num: Box<Node>,
        den: Box<Node>,
        bar: bool,
    },
    /// `n choose k` / `binom {n} {k}`.
    Binom { top: Box<Node>, bottom: Box<Node> },
    /// `sqrt {x}`.
    Sqrt(Box<Node>),
    /// `root n of {x}`.
    Root {
        degree: Box<Node>,
        radicand: Box<Node>,
    },
    /// Sub/superscripts attached to a base. At least one of `sub`/`sup` is set.
    Script {
        base: Box<Node>,
        sub: Option<Box<Node>>,
        sup: Option<Box<Node>>,
    },
    /// `LEFT ( … RIGHT )`. `left`/`right` hold the raw delimiter as written
    /// (`"("`, `"["`, `"|"`, `"||"`, `"<"`, `">"`, `"."`); the emitter maps them.
    Delimited {
        left: String,
        right: String,
        body: Box<Node>,
    },
    /// All row/column structures: matrix/pmatrix/bmatrix/dmatrix/cases/
    /// pile/lpile/rpile/eqalign.
    Matrix {
        kind: MatrixKind,
        rows: Vec<Vec<Node>>,
    },
    /// Decoration applied to the following term: `vec A`, `hat x`, …
    /// The accent symbol is serialized as its canonical name.
    Accent {
        #[cfg_attr(feature = "serde", serde(with = "symdef_name"))]
        accent: &'static SymDef,
        base: Box<Node>,
    },
    /// Font-style switch applying to the rest of the enclosing group.
    Style { style: StyleKind, body: Box<Node> },
    /// `buildrel T over B` (alias `rel`) — stack `T` above `B`.
    BuildRel { top: Box<Node>, base: Box<Node> },
    /// `not X` — negating slash over the following term (`not =` → ≠).
    Not(Box<Node>),
    /// `bigg X` etc. — enlarge the following delimiter/operator.
    /// `size` is the LaTeX size command; the parser only ever produces
    /// `\big`/`\Big`/`\bigg`/`\Bigg`, and deserialization enforces the same.
    Big {
        #[cfg_attr(feature = "serde", serde(deserialize_with = "big_size::deserialize"))]
        size: String,
        arg: Box<Node>,
    },
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
        if items.len() == 1 {
            items.pop().unwrap()
        } else {
            Node::Row(items)
        }
    }
}

/// Serde representation of a `&'static SymDef`: the canonical keyword name.
/// Deserialization resolves it through `symbols::find_by_name`, so only names
/// that exist in the keyword tables are accepted.
#[cfg(feature = "serde")]
mod symdef_name {
    use crate::symbols::{find_by_name, SymDef};
    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(def: &&'static SymDef, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(def.name)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<&'static SymDef, D::Error> {
        let name = String::deserialize(d)?;
        find_by_name(&name)
            .ok_or_else(|| de::Error::custom(format!("unknown symbol name: {name:?}")))
    }
}

/// Deserialization guard for a `Big` size: only the four LaTeX size commands
/// the parser itself can produce are accepted.
#[cfg(feature = "serde")]
mod big_size {
    use serde::{de, Deserialize, Deserializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
        let raw = String::deserialize(d)?;
        match raw.as_str() {
            r"\big" | r"\Big" | r"\bigg" | r"\Bigg" => Ok(raw),
            _ => Err(de::Error::custom(format!("unknown big size: {raw:?}"))),
        }
    }
}
