//! Tokeniser. Case of the first letter distinguishes types from objects (§7.1).

use crate::{Diagnostics, Span};

/// A token kind.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Tok {
    /// Identifier starting with an uppercase letter — a type name.
    Upper(String),
    /// Identifier starting with a lowercase letter — an object, predicate, or keyword.
    Lower(String),
    /// `?name` — an action parameter.
    Var(String),
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `,`
    Comma,
    /// `-`
    Dash,
    /// `:`
    Colon,
    /// `*`
    Star,
    /// `&` or `&&`
    Amp,
    /// `|` or `||`
    Bar,
    /// `!` — prefix negation.
    Bang,
    /// `->`
    Arrow,
    /// `<-`
    Gets,
    /// `~` between world names in an explicit state
    Tilde,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `'` — the dual marker in `K'`, `B'`, `S'`
    Prime,
    /// `[]` — ASCII for the safe-belief box
    Box,
    /// `?` used as the ignorant-whether operator
    Question,
    /// `¿` or `??` — suspends judgement
    Undecided,
    /// A lone `_` — the hole in a query pattern. Never legal in a file.
    Hole,
    /// `=` — separates a definition's name from its body.
    Eq,
    /// `^` — the conditional-belief marker in `B^psi`
    Caret,
    /// End of input.
    Eof,
}

/// A token with its source location.
#[derive(Clone, Debug)]
pub struct Token {
    /// What kind.
    pub tok: Tok,
    /// Where.
    pub span: Span,
}

/// Tokenises `src`, recording errors in `diags`. Always ends with [`Tok::Eof`].
pub fn lex(src: &str, diags: &mut Diagnostics) -> Vec<Token> {
    let b = src.as_bytes();
    let mut i = 0usize;
    let mut out = Vec::new();
    while i < b.len() {
        // `src[i..].chars().next()` decodes the full Unicode scalar at `i`,
        // not just its leading byte — required so multi-byte operators like
        // `¿` and `□` compare equal to their `char` literals below. `i` is
        // always kept on a char boundary, so this never panics.
        let c = src[i..].chars().next().expect("i is within bounds");
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < b.len() && b[i + 1] == b'/' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && i + 1 < b.len() && b[i + 1] == b'*' {
            let start = i;
            i += 2;
            loop {
                if i + 1 >= b.len() {
                    diags.push(Span::new(start, b.len()), "unterminated block comment");
                    i = b.len();
                    break;
                }
                if b[i] == b'*' && b[i + 1] == b'/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }
        let start = i;
        let tok = if c.is_ascii_alphabetic() || c == '_' {
            while i < b.len() && ((b[i] as char).is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            let s = src[start..i].to_string();
            // A lone `_` is the query hole; `_x` or `at_park` are ordinary identifiers.
            // Deciding it here, on the whole lexeme, is what keeps the hole from being
            // confused with an underscore *inside* a name — the reason this is a token
            // at all rather than a string substitution.
            if s == "_" {
                Tok::Hole
            } else if c.is_ascii_uppercase() {
                Tok::Upper(s)
            } else {
                Tok::Lower(s)
            }
        } else if c == '?' {
            // `?name` is a variable; a bare `?` is the ignorant-whether operator,
            // and `??` is suspends-judgement.
            if i + 1 < b.len() && b[i + 1] == b'?' {
                i += 2;
                Tok::Undecided
            } else if i + 1 < b.len() && ((b[i + 1] as char).is_ascii_alphabetic() || b[i + 1] == b'_') {
                i += 1;
                let ns = i;
                while i < b.len() && ((b[i] as char).is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                Tok::Var(src[ns..i].to_string())
            } else {
                i += 1;
                Tok::Question
            }
        } else {
            i += c.len_utf8();
            match c {
                '{' => Tok::LBrace,
                '}' => Tok::RBrace,
                '(' => Tok::LParen,
                ')' => Tok::RParen,
                ']' => Tok::RBracket,
                ',' => Tok::Comma,
                ':' => Tok::Colon,
                '*' => Tok::Star,
                '\'' => Tok::Prime,
                '^' => Tok::Caret,
                '=' => Tok::Eq,
                '¿' => Tok::Undecided,
                '□' => Tok::Box,
                '~' => Tok::Tilde,
                '[' => {
                    if i < b.len() && b[i] == b']' {
                        i += 1;
                        Tok::Box
                    } else {
                        Tok::LBracket
                    }
                }
                '&' => {
                    if i < b.len() && b[i] == b'&' { i += 1; }
                    Tok::Amp
                }
                '|' => {
                    if i < b.len() && b[i] == b'|' { i += 1; }
                    Tok::Bar
                }
                '!' => Tok::Bang,
                '-' => {
                    if i < b.len() && b[i] == b'>' { i += 1; Tok::Arrow } else { Tok::Dash }
                }
                '<' => {
                    if i < b.len() && b[i] == b'-' {
                        i += 1;
                        Tok::Gets
                    } else if i < b.len() && b[i] == b'=' {
                        i += 1;
                        Tok::Le
                    } else {
                        Tok::Lt
                    }
                }
                other => {
                    diags.push(Span::new(start, i), format!("unexpected character `{other}`"));
                    continue;
                }
            }
        };
        out.push(Token { tok, span: Span::new(start, i) });
    }
    out.push(Token { tok: Tok::Eof, span: Span::new(b.len(), b.len()) });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Diagnostics;

    fn kinds(src: &str) -> Vec<Tok> {
        let mut d = Diagnostics::default();
        let out: Vec<Tok> = lex(src, &mut d).into_iter().map(|t| t.tok).collect();
        assert!(d.is_empty(), "unexpected lex errors: {}", d.render(src));
        out
    }

    #[test]
    fn distinguishes_upper_lower_and_variables() {
        assert_eq!(
            kinds("Actor alice ?a"),
            vec![
                Tok::Upper("Actor".into()),
                Tok::Lower("alice".into()),
                Tok::Var("a".into()),
                Tok::Eof
            ]
        );
    }

    #[test]
    fn bare_question_mark_is_the_ignorant_operator() {
        // `?[carol] h` — the `?` is an operator, not a variable.
        assert_eq!(
            kinds("?[c]"),
            vec![Tok::Question, Tok::LBracket, Tok::Lower("c".into()), Tok::RBracket, Tok::Eof]
        );
    }

    #[test]
    fn skips_both_comment_forms() {
        assert_eq!(kinds("a // trailing\n/* block\n spanning */ b"),
                   vec![Tok::Lower("a".into()), Tok::Lower("b".into()), Tok::Eof]);
    }

    #[test]
    fn lexes_operators_including_ascii_alternatives() {
        assert_eq!(
            kinds("& | ! -> <- ~ < <= [] ?? *"),
            vec![Tok::Amp, Tok::Bar, Tok::Bang, Tok::Arrow, Tok::Gets, Tok::Tilde,
                 Tok::Lt, Tok::Le, Tok::Box, Tok::Undecided, Tok::Star, Tok::Eof]
        );
    }

    #[test]
    fn unterminated_block_comment_is_an_error() {
        let mut d = Diagnostics::default();
        let _ = lex("a /* never closed", &mut d);
        assert_eq!(d.len(), 1);
        assert!(d.items()[0].message.contains("unterminated"));
    }

    #[test]
    fn digits_start_no_token_of_their_own_but_still_belong_inside_names() {
        // No production in either parser consumes a number, so there is no integer
        // token: a leading digit is simply an unexpected character. Digits after the
        // first letter are part of the identifier, which `p0` and the printer's `w1`
        // both depend on.
        assert_eq!(kinds("p0 w12"), vec![
            Tok::Lower("p0".into()), Tok::Lower("w12".into()), Tok::Eof
        ]);
        let mut d = Diagnostics::default();
        let toks = lex("7", &mut d);
        assert_eq!(toks.iter().map(|t| t.tok.clone()).collect::<Vec<_>>(), vec![Tok::Eof]);
        assert_eq!(d.len(), 1);
        assert!(d.items()[0].message.contains("unexpected character"),
                "got: {}", d.items()[0].message);
    }

    #[test]
    fn lexes_literal_unicode_operators() {
        assert_eq!(kinds("¿ □"), vec![Tok::Undecided, Tok::Box, Tok::Eof]);
    }

    #[test]
    fn multibyte_operator_spans_are_byte_offsets_not_char_offsets() {
        // '¿' is 2 bytes in UTF-8. If the cursor advanced by 1 char instead
        // of `c.len_utf8()` bytes, the second token's span would start at
        // byte 1 — inside the '¿' encoding — rather than at the following
        // 'x'. That would either misplace the span or land it off a char
        // boundary, which panics inside `Diagnostics::render`'s precondition
        // check on other inputs.
        let mut d = Diagnostics::default();
        let src = "¿x";
        let toks = lex(src, &mut d);
        assert!(d.is_empty(), "unexpected lex errors: {}", d.render(src));
        assert_eq!(toks[0].tok, Tok::Undecided);
        assert_eq!(toks[0].span, Span::new(0, '¿'.len_utf8()));
        assert_eq!(toks[1].tok, Tok::Lower("x".into()));
        assert_eq!(toks[1].span, Span::new('¿'.len_utf8(), '¿'.len_utf8() + 1));
    }
}
