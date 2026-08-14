//! Tokenizer for the HWP equation script. Rules: docs/DESIGN.md §1.

use crate::error::ParseError;
use crate::token::{Token, TokenKind};

/// Two-character operator ligatures, tried before single-character `Op`s
/// (DESIGN.md §1.6). All are ASCII and no two overlap at a position, so the
/// first match is the only one. `+ -` (separated) stays two `Op`s because
/// whitespace is consumed before this test runs.
const LIGATURES: [&str; 8] = ["+-", "-+", "!=", "<=", ">=", "<<", ">>", "||"];

/// Word characters: ASCII letters plus any non-ASCII alphabetic scalar.
/// Within ASCII `char::is_alphabetic` is exactly `A–Z a–z`, so this is
/// DESIGN.md §1.4 verbatim. Digits are never word characters, so `x2` lexes
/// as `Word("x") Number("2")`.
fn is_word_char(c: char) -> bool {
    c.is_alphabetic()
}

/// Tokenize `src` into a flat token stream. Every token carries its byte span
/// (`start`..`end`) into `src`; for `Quoted` the span includes both quotes
/// while the payload holds only the inner text.
pub fn lex(src: &str) -> Result<Vec<Token>, ParseError> {
    let bytes = src.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0usize;

    // `i` stays on a char boundary: every branch advances by whole chars.
    while i < src.len() {
        let c = src[i..].chars().next().unwrap();
        let clen = c.len_utf8();

        // 1. Whitespace separates tokens and is never emitted.
        if matches!(c, ' ' | '\t' | '\n' | '\r') {
            i += clen;
            continue;
        }

        // 2. Structural single characters.
        let structural = match c {
            '{' => Some(TokenKind::LBrace),
            '}' => Some(TokenKind::RBrace),
            '^' => Some(TokenKind::Caret),
            '_' => Some(TokenKind::Underscore),
            '#' => Some(TokenKind::Hash),
            '&' => Some(TokenKind::Amp),
            '~' => Some(TokenKind::Tilde),
            '`' => Some(TokenKind::Backquote),
            _ => None,
        };
        if let Some(kind) = structural {
            tokens.push(Token {
                kind,
                start: i,
                end: i + clen,
            });
            i += clen;
            continue;
        }

        // 3. Quoted text — no escape processing (HWP has none), so the first
        // `"` after the opener closes it.
        if c == '"' {
            let body = i + 1;
            let Some(off) = src[body..].find('"') else {
                // Reported at the opening quote, not at EOF.
                return Err(ParseError::new("unterminated quoted text", i));
            };
            let close = body + off;
            tokens.push(Token {
                kind: TokenKind::Quoted(src[body..close].to_string()),
                start: i,
                end: close + 1,
            });
            i = close + 1;
            continue;
        }

        // 4. Maximal alphabetic run → one Word. Keyword lookup happens later,
        // on the whole word: `sinh` and `sinx` are each a single Word.
        if is_word_char(c) {
            let start = i;
            let mut j = i;
            while let Some(ch) = src[j..].chars().next() {
                if !is_word_char(ch) {
                    break;
                }
                j += ch.len_utf8();
            }
            tokens.push(Token {
                kind: TokenKind::Word(src[start..j].to_string()),
                start,
                end: j,
            });
            i = j;
            continue;
        }

        // 5. Digit run, plus at most one interior `.` flanked by digits:
        // `3.14` is one Number, `3.` is `Number("3") Op(".")`, and `.5` never
        // reaches here (a Number cannot start with `.`).
        if c.is_ascii_digit() {
            let start = i;
            let mut j = i;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j + 1 < bytes.len() && bytes[j] == b'.' && bytes[j + 1].is_ascii_digit() {
                j += 1;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
            }
            tokens.push(Token {
                kind: TokenKind::Number(src[start..j].to_string()),
                start,
                end: j,
            });
            i = j;
            continue;
        }

        // 6. Operators: two-character ligature if one matches here, else the
        // single character (which may be multi-byte).
        let text = match LIGATURES
            .iter()
            .find(|lig| src[i..].starts_with(**lig))
            .copied()
        {
            Some(lig) => lig,
            None => &src[i..i + clen],
        };
        tokens.push(Token {
            kind: TokenKind::Op(text.to_string()),
            start: i,
            end: i + text.len(),
        });
        i += text.len();
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex_ok(src: &str) -> Vec<Token> {
        lex(src).unwrap_or_else(|e| panic!("lex({src:?}) failed: {e}"))
    }

    fn kinds(src: &str) -> Vec<TokenKind> {
        lex_ok(src).into_iter().map(|t| t.kind).collect()
    }

    fn spans(src: &str) -> Vec<(usize, usize)> {
        lex_ok(src).iter().map(|t| (t.start, t.end)).collect()
    }

    fn word(s: &str) -> TokenKind {
        TokenKind::Word(s.to_string())
    }
    fn num(s: &str) -> TokenKind {
        TokenKind::Number(s.to_string())
    }
    fn op(s: &str) -> TokenKind {
        TokenKind::Op(s.to_string())
    }
    fn quoted(s: &str) -> TokenKind {
        TokenKind::Quoted(s.to_string())
    }

    #[test]
    fn mixed_input_kinds_and_spans() {
        let src = r#"a_1 + "p q" ~ 3.14"#;
        assert_eq!(src.len(), 18);
        assert_eq!(
            kinds(src),
            vec![
                word("a"),
                TokenKind::Underscore,
                num("1"),
                op("+"),
                quoted("p q"),
                TokenKind::Tilde,
                num("3.14"),
            ]
        );
        assert_eq!(
            spans(src),
            vec![(0, 1), (1, 2), (2, 3), (4, 5), (6, 11), (12, 13), (14, 18)]
        );
    }

    #[test]
    fn structural_chars_are_one_token_each() {
        use TokenKind::*;
        let src = "{}^_#&~`";
        assert_eq!(
            kinds(src),
            vec![LBrace, RBrace, Caret, Underscore, Hash, Amp, Tilde, Backquote]
        );
        assert_eq!(
            spans(src),
            vec![
                (0, 1),
                (1, 2),
                (2, 3),
                (3, 4),
                (4, 5),
                (5, 6),
                (6, 7),
                (7, 8)
            ]
        );
    }

    #[test]
    fn number_dot_rule() {
        assert_eq!(kinds("3.14"), vec![num("3.14")]);
        assert_eq!(kinds("3."), vec![num("3"), op(".")]);
        assert_eq!(kinds(".5"), vec![op("."), num("5")]);
        // Only one interior dot joins; the rest splits off again.
        assert_eq!(kinds("3.14.15"), vec![num("3.14"), op("."), num("15")]);
        assert_eq!(spans("3.14"), vec![(0, 4)]);
        assert_eq!(spans("3."), vec![(0, 1), (1, 2)]);
    }

    #[test]
    fn ligatures_vs_separated_operators() {
        assert_eq!(kinds("+-"), vec![op("+-")]);
        assert_eq!(kinds("+ -"), vec![op("+"), op("-")]);
        assert_eq!(spans("+-"), vec![(0, 2)]);
        assert_eq!(spans("+ -"), vec![(0, 1), (2, 3)]);

        for lig in LIGATURES {
            assert_eq!(kinds(lig), vec![op(lig)], "ligature {lig:?}");
        }
        // Maximal munch left to right: the third `|` is its own Op.
        assert_eq!(kinds("|||"), vec![op("||"), op("|")]);
        assert_eq!(kinds("a != b"), vec![word("a"), op("!="), word("b")]);
        // `^` and `_` are structural, never ligature material.
        assert_eq!(kinds("b^2"), vec![word("b"), TokenKind::Caret, num("2")]);
    }

    #[test]
    fn quoted_keeps_inner_spaces_and_strips_quotes() {
        assert_eq!(kinds(r#""hello world""#), vec![quoted("hello world")]);
        assert_eq!(spans(r#""hello world""#), vec![(0, 13)]);
        assert_eq!(kinds(r#""""#), vec![quoted("")]);
        // Structural characters inside quotes stay literal text.
        assert_eq!(kinds(r#""a ^ {b}""#), vec![quoted("a ^ {b}")]);
    }

    #[test]
    fn unterminated_quote_points_at_the_opening_quote() {
        let err = lex(r#""abc"#).unwrap_err();
        assert!(
            err.message.contains("unterminated quoted text"),
            "{}",
            err.message
        );
        assert_eq!(err.position, 0);

        let err = lex(r#"x "abc"#).unwrap_err();
        assert_eq!(err.position, 2);

        // An odd number of quotes: the third one opens an unterminated run.
        let err = lex(r#""a" "b"#).unwrap_err();
        assert_eq!(err.position, 4);
    }

    #[test]
    fn digits_break_words() {
        assert_eq!(kinds("x2"), vec![word("x"), num("2")]);
        assert_eq!(spans("x2"), vec![(0, 1), (1, 2)]);
        assert_eq!(
            kinds("a_1"),
            vec![word("a"), TokenKind::Underscore, num("1")]
        );
        assert_eq!(kinds("-4ac"), vec![op("-"), num("4"), word("ac")]);
    }

    #[test]
    fn words_are_maximal_runs() {
        // REFERENCE.md §3: keyword resolution is whole-word, so the lexer must
        // not split these — `sinh` is not `sin`+`h`, `sinx` is not `sin`+`x`.
        assert_eq!(kinds("sinh"), vec![word("sinh")]);
        assert_eq!(kinds("sinx"), vec![word("sinx")]);
        assert_eq!(kinds("sinh x"), vec![word("sinh"), word("x")]);
        assert_eq!(kinds("pile"), vec![word("pile")]);
        assert_eq!(kinds("pi le"), vec![word("pi"), word("le")]);
        assert_eq!(spans("sinh x"), vec![(0, 4), (5, 6)]);
    }

    #[test]
    fn non_ascii_letters_form_words_with_byte_spans() {
        let src = "Σ x"; // 'Σ' is two bytes
        assert_eq!(kinds(src), vec![word("Σ"), word("x")]);
        assert_eq!(spans(src), vec![(0, 2), (3, 4)]);

        let src = "한글2";
        assert_eq!(kinds(src), vec![word("한글"), num("2")]);
        assert_eq!(spans(src), vec![(0, 6), (6, 7)]);
    }

    #[test]
    fn whitespace_is_skipped_entirely() {
        assert!(lex_ok("").is_empty());
        assert!(lex_ok("  \t\r\n ").is_empty());
        assert_eq!(kinds("a\n\tb"), vec![word("a"), word("b")]);
    }

    #[test]
    fn spans_are_ordered_and_slice_back_to_the_source() {
        let src = "x = {-b +- sqrt{b^2 -4ac}} over {2a}";
        let mut prev_end = 0;
        for t in lex_ok(src) {
            assert!(t.start >= prev_end && t.end > t.start, "bad span: {t:?}");
            prev_end = t.end;
            let text = &src[t.start..t.end];
            match &t.kind {
                TokenKind::Word(s) | TokenKind::Number(s) | TokenKind::Op(s) => {
                    assert_eq!(text, s.as_str())
                }
                TokenKind::Quoted(inner) => assert_eq!(text, format!("\"{inner}\"")),
                _ => assert_eq!(text.chars().count(), 1),
            }
        }
        assert!(prev_end <= src.len());
    }
}
