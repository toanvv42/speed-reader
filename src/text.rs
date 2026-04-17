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
