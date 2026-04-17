/// Strips characters that are invisible or hostile to an RSVP reader:
/// zero-width joiners, BOMs, soft hyphens, and C0/C1 control characters
/// (except `\n` and `\t`). Collapses runs of horizontal whitespace to a
/// single space while preserving newlines.
pub fn sanitize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.chars() {
        match ch {
            '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}' | '\u{00AD}' | '\u{2060}'
            | '\u{180E}' | '\u{034F}' => continue,
            '\r' => continue,
            '\n' => {
                prev_space = false;
                out.push('\n');
            }
            '\t' | ' ' => {
                if !prev_space {
                    out.push(' ');
                    prev_space = true;
                }
            }
            c if c.is_control() => continue,
            c => {
                prev_space = false;
                out.push(c);
            }
        }
    }
    out
}

/// Returns a string of at most `max` characters. When truncated, the last
/// character is replaced by `…`.
pub fn truncate_end(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if s.chars().count() <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", head)
}

/// Returns a string of at most `max` characters. When truncated, the first
/// character is replaced by `…` and the tail is preserved.
pub fn truncate_start(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let n = s.chars().count();
    if n <= max {
        return s.to_string();
    }
    let head_len = max.saturating_sub(1);
    let skip = n - head_len;
    let tail: String = s.chars().skip(skip).collect();
    format!("…{}", tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_end_short_unchanged() {
        assert_eq!(truncate_end("hi", 10), "hi");
    }

    #[test]
    fn truncate_end_exact_max_unchanged() {
        assert_eq!(truncate_end("hello", 5), "hello");
    }

    #[test]
    fn truncate_end_longer_gets_ellipsis() {
        assert_eq!(truncate_end("hello world", 6), "hello…");
    }

    #[test]
    fn truncate_end_max_zero_is_empty() {
        assert_eq!(truncate_end("whatever", 0), "");
    }

    #[test]
    fn truncate_end_max_one_is_ellipsis_only() {
        assert_eq!(truncate_end("hello", 1), "…");
    }

    #[test]
    fn truncate_end_counts_unicode_chars() {
        // six chars total, max 4 → first 3 + ellipsis
        assert_eq!(truncate_end("αβγδεζ", 4), "αβγ…");
    }

    #[test]
    fn truncate_start_short_unchanged() {
        assert_eq!(truncate_start("hi", 10), "hi");
    }

    #[test]
    fn truncate_start_exact_max_unchanged() {
        assert_eq!(truncate_start("hello", 5), "hello");
    }

    #[test]
    fn truncate_start_longer_gets_leading_ellipsis() {
        assert_eq!(truncate_start("hello world", 6), "…world");
    }

    #[test]
    fn truncate_start_max_zero_is_empty() {
        assert_eq!(truncate_start("whatever", 0), "");
    }

    #[test]
    fn sanitize_strips_zero_width_and_bom() {
        assert_eq!(
            sanitize("hel\u{200B}lo\u{FEFF} wor\u{00AD}ld"),
            "hello world"
        );
    }

    #[test]
    fn sanitize_drops_control_chars_but_keeps_newline_and_tab() {
        // \x07 (BEL) removed without introducing a separator; \t → space, \n preserved.
        assert_eq!(sanitize("a\x07b\tc\nd"), "ab c\nd");
    }

    #[test]
    fn sanitize_collapses_horizontal_whitespace() {
        assert_eq!(sanitize("a    b  \t  c"), "a b c");
    }

    #[test]
    fn sanitize_preserves_newlines_between_paragraphs() {
        assert_eq!(sanitize("line one   \n   line two"), "line one \n line two");
    }

    #[test]
    fn sanitize_empty_and_plain_pass_through() {
        assert_eq!(sanitize(""), "");
        assert_eq!(sanitize("hello world"), "hello world");
    }
}
