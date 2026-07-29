//! Minimal JSON emission.
//!
//! Hand-rolled rather than reached for from a crate, because `--json` has to work in the
//! `--no-default-features` build, whose entire point is an empty dependency graph. This
//! is emission only — there is no parser here — so the surface is one escape function and
//! three builders, and [`esc`] is the only part that can be wrong.

/// Escapes a string for a JSON double-quoted literal, per RFC 8259 §7.
///
/// Non-ASCII passes through as UTF-8, which JSON permits and which matters here: delhi
/// renders formulas with `□`, `¬` and `→` in them, and `\u`-escaping those would make the
/// output unreadable for no gain. What must be escaped is the quote, the backslash, and
/// every control character below 0x20 — the last of which is the one an author forgets,
/// so a domain with a tab in a proposition name would emit JSON no parser accepts.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// A JSON string literal, quotes included.
pub fn str_(s: &str) -> String {
    format!("\"{}\"", esc(s))
}

/// A JSON boolean.
pub fn bool_(b: bool) -> String {
    if b {
        "true".to_string()
    } else {
        "false".to_string()
    }
}

/// A JSON number from a count.
pub fn num(n: usize) -> String {
    n.to_string()
}

/// A JSON array of already-rendered values.
pub fn arr(items: &[String]) -> String {
    format!("[{}]", items.join(","))
}

/// A JSON array of strings.
pub fn strs<S: AsRef<str>>(items: &[S]) -> String {
    arr(&items.iter().map(|s| str_(s.as_ref())).collect::<Vec<_>>())
}

/// A JSON object from `(key, already-rendered value)` pairs.
pub fn obj(pairs: &[(&str, String)]) -> String {
    let body: Vec<String> = pairs.iter().map(|(k, v)| format!("{}:{}", str_(k), v)).collect();
    format!("{{{}}}", body.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_characters_json_requires_escaping_are_escaped() {
        assert_eq!(esc(r#"say "hi""#), r#"say \"hi\""#);
        assert_eq!(esc(r"back\slash"), r"back\\slash");
        assert_eq!(esc("two\nlines\there"), "two\\nlines\\there");
        // The one an author forgets. A bare control character makes output that no
        // parser will accept, and nothing else in this file would catch it.
        assert_eq!(esc("\u{01}"), "\\u0001");
        assert_eq!(esc("\u{1f}"), "\\u001f");
        assert_eq!(esc("\u{08}\u{0c}"), "\\b\\f");
    }

    #[test]
    fn formula_glyphs_survive_as_utf8_rather_than_being_escaped() {
        // delhi prints these; `\u`-escaping them would be valid JSON and unreadable.
        for s in ["□[a] p", "¬h", "B^ψ[a] q", "w0 → w1", "?[carol] h"] {
            assert_eq!(esc(s), s, "{s} must pass through unchanged");
        }
    }

    #[test]
    fn builders_compose_into_the_shape_the_python_wrapper_expects() {
        let o = obj(&[
            ("ok", bool_(true)),
            ("value", bool_(false)),
            ("matches", strs(&["B[a] h", "B[a] !d"])),
            ("considered", num(4)),
        ]);
        assert_eq!(o, r#"{"ok":true,"value":false,"matches":["B[a] h","B[a] !d"],"considered":4}"#);
    }

    #[test]
    fn an_empty_array_and_an_empty_object_are_still_valid() {
        assert_eq!(strs::<&str>(&[]), "[]");
        assert_eq!(obj(&[]), "{}");
        assert_eq!(str_(""), "\"\"");
    }
}
