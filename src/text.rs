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
}
