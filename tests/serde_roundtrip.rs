//! serde round-trip tests: parse → JSON → deserialize must reproduce the same
//! AST (and therefore the same LaTeX, when that feature is on).
#![cfg(feature = "serde")]

use hmleq::Node;

fn roundtrip(src: &str) -> (Node, Node) {
    let ast = hmleq::parse(src).unwrap();
    let json = serde_json::to_string(&ast).unwrap();
    let back: Node = serde_json::from_str(&json).unwrap();
    (ast, back)
}

#[test]
fn ast_round_trips_through_json() {
    for src in [
        "x = {-b +- sqrt{b^2 -4ac}} over {2a}",
        "lim _{x rarrow 0} {sin x} over x = 1",
        "sum _{n=1} ^{inf} {1 over n^2} = {pi^2} over 6",
        "pmatrix { a_1 & b_1 # a_2 & b_2 }",
        "f(x) = cases { x^2 & (x geq 0) # -x & (x < 0) }",
        "vec A^2 + hat x",
        "Lim _{n} a_n",
        "{a+b} over {a-b} bigg / {x+y}",
        "rm ABC it x # y & z",
        r#"LEFT ( "text with { and \ inside" RIGHT )"#,
        "root 3 of {x+1} ~ n choose k ` buildrel def over =",
    ] {
        let (ast, back) = roundtrip(src);
        assert_eq!(ast, back, "AST changed across serde round-trip for {src:?}");
    }
}

#[test]
fn symbols_serialize_as_their_canonical_name() {
    let json = serde_json::to_string(&hmleq::parse("pi").unwrap()).unwrap();
    assert_eq!(json, r#"{"Symbol":"pi"}"#);

    // Case-sensitive names survive: Lim (≠ lim) keeps its identity.
    let (ast, back) = roundtrip("Lim _{n} a_n");
    assert_eq!(ast, back);
    let json = serde_json::to_string(&hmleq::parse("Lim").unwrap()).unwrap();
    assert_eq!(json, r#"{"Symbol":"Lim"}"#);
}

#[test]
fn unknown_symbol_names_are_rejected() {
    assert!(serde_json::from_str::<Node>(r#"{"Symbol":"nope"}"#).is_err());
    assert!(serde_json::from_str::<Node>(r#"{"Big":{"size":"\\huge","arg":{"Op":"/"}}}"#).is_err());
}

#[test]
fn hand_written_json_deserializes() {
    let node: Node =
        serde_json::from_str(r#"{"Frac":{"num":{"Number":"1"},"den":{"Symbol":"pi"},"bar":true}}"#)
            .unwrap();
    assert_eq!(node, hmleq::parse("1 over pi").unwrap());
}

#[cfg(feature = "latex")]
#[test]
fn latex_is_stable_across_round_trip() {
    for src in [
        "int _1 ^2 {3x^2} dx = LEFT[ x^3 RIGHT] _1 ^2 = 7",
        "A inter B = { x | x in A ~and~ x in B }",
        "smallsum _{k} a_k",
    ] {
        let (ast, back) = roundtrip(src);
        assert_eq!(hmleq::to_latex(&ast), hmleq::to_latex(&back), "{src:?}");
    }
}
