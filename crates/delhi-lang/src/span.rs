//! Source spans and rendered diagnostics.

/// A half-open byte range into the source text.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Span {
    /// First byte, inclusive.
    pub start: usize,
    /// Last byte, exclusive.
    pub end: usize,
}

impl Span {
    /// A span covering `start..end`.
    ///
    /// Precondition: `start <= end`.
    pub fn new(start: usize, end: usize) -> Self {
        debug_assert!(start <= end, "span start must not exceed end");
        Span { start, end }
    }
    /// The smallest span covering both.
    pub fn merge(self, other: Span) -> Span {
        Span::new(self.start.min(other.start), self.end.max(other.end))
    }
}

/// One error, tied to the source text that caused it.
#[derive(Clone, Debug)]
pub struct Diagnostic {
    /// Where in the source.
    pub span: Span,
    /// What went wrong.
    pub message: String,
}

/// A collection of diagnostics, rendered together.
#[derive(Default, Debug)]
pub struct Diagnostics(Vec<Diagnostic>);

impl Diagnostics {
    /// Records an error.
    pub fn push(&mut self, span: Span, message: impl Into<String>) {
        self.0.push(Diagnostic { span, message: message.into() });
    }
    /// Whether nothing has gone wrong.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// How many errors were recorded.
    pub fn len(&self) -> usize {
        self.0.len()
    }
    /// The recorded diagnostics.
    pub fn items(&self) -> &[Diagnostic] {
        &self.0
    }
    /// Renders every diagnostic against `src` as `line:col: message`, echoing the
    /// offending line with a caret run beneath the span.
    ///
    /// Precondition: every diagnostic's span must lie within `src` (`end <=
    /// src.len()`) with `start` and `end` on UTF-8 char boundaries.
    pub fn render(&self, src: &str) -> String {
        let mut out = String::new();
        for d in &self.0 {
            debug_assert!(
                d.span.end <= src.len()
                    && src.is_char_boundary(d.span.start)
                    && src.is_char_boundary(d.span.end),
                "diagnostic span must lie within `src` and on char boundaries"
            );
            let (line_no, line_start) = line_of(src, d.span.start);
            let line_end = src[line_start..].find('\n').map_or(src.len(), |i| line_start + i);
            let col = src[line_start..d.span.start].chars().count() + 1;
            let text = &src[line_start..line_end];
            out.push_str(&format!("{line_no}:{col}: {}\n", d.message));
            out.push_str(&format!("  {text}\n"));
            let pad = " ".repeat(col - 1);
            let width = src[d.span.start..d.span.end.min(line_end)].chars().count().max(1);
            out.push_str(&format!("  {pad}{}\n", "^".repeat(width)));
        }
        out
    }
}

/// One diagnostic with its position resolved against the source.
///
/// [`Diagnostics::render`] bakes position into a string, which is right for a terminal
/// and useless to anything that wants to *act* on the location — a UI that jumps the
/// cursor to the fault needs the numbers, not a rendering of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Located {
    /// 1-based line.
    pub line: usize,
    /// 1-based column, in characters.
    pub col: usize,
    /// Byte offset of the start of the offending text.
    pub start: usize,
    /// Byte offset just past its end.
    pub end: usize,
    /// What went wrong.
    pub message: String,
}

impl Diagnostics {
    /// Every diagnostic with its line, column and byte range resolved.
    pub fn located(&self, src: &str) -> Vec<Located> {
        self.0
            .iter()
            .map(|d| {
                let (line, line_start) = line_of(src, d.span.start);
                let col = src[line_start..d.span.start.min(src.len())].chars().count() + 1;
                Located {
                    line,
                    col,
                    start: d.span.start,
                    end: d.span.end,
                    message: d.message.clone(),
                }
            })
            .collect()
    }
}

/// 1-based line number and the byte offset where that line starts.
fn line_of(src: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut start = 0;
    for (i, c) in src.char_indices() {
        if i >= offset {
            break;
        }
        if c == '\n' {
            line += 1;
            start = i + 1;
        }
    }
    (line, start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_covers_both_spans() {
        let a = Span::new(2, 5);
        let b = Span::new(9, 11);
        assert_eq!(a.merge(b), Span::new(2, 11));
        assert_eq!(b.merge(a), Span::new(2, 11));
    }

    #[test]
    fn render_points_at_the_right_line_and_column() {
        let src = "types {\n  Bad - Nope\n}\n";
        let mut d = Diagnostics::default();
        // "Nope" starts at byte 16
        d.push(Span::new(16, 20), "unknown type `Nope`");
        let out = d.render(src);
        assert!(out.contains("2:9"), "expected line 2 col 9, got:\n{out}");
        assert!(out.contains("unknown type `Nope`"));
        assert!(out.contains("Bad - Nope"), "the offending line should be echoed");
        assert!(out.contains('^'), "a caret should mark the span");
    }

    #[test]
    fn empty_diagnostics_render_to_nothing() {
        assert!(Diagnostics::default().is_empty());
        assert_eq!(Diagnostics::default().render("x"), "");
    }

    #[test]
    fn render_counts_columns_and_width_in_chars_not_bytes() {
        // "¿" is 2 bytes in UTF-8. A byte-based column or caret width would be
        // wrong here even though the span itself lands on valid char
        // boundaries. Task 2's lexer produces exactly this kind of span for
        // multi-byte tokens like `¿` and `□`.
        let src = "line1\n¿¿bad\n";
        let mut d = Diagnostics::default();
        // line 2 starts at byte 6; "¿¿" takes 4 bytes, so "bad" spans 10..13.
        d.push(Span::new(10, 13), "oops");
        let out = d.render(src);
        assert!(out.contains("2:3"), "expected line 2 col 3 (3rd character), got:\n{out}");
        assert!(out.contains("¿¿bad"), "the offending line should be echoed verbatim");
        assert!(out.contains("^^^"), "caret run should be 3 wide (chars in `bad`), got:\n{out}");
    }
}
