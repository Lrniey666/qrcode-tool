//! Small text helpers used for display (never for security decisions).

/// Truncates `s` to at most `max_chars` characters (not bytes), appending `…` when cut.
pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let mut out: String = s.chars().take(keep).collect();
    out.push('…');
    out
}

/// Collapses all whitespace runs (including newlines) to single spaces and trims.
pub fn single_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Replaces characters that are invisible or can be used to spoof text direction
/// (bidi controls, zero-width characters) with a visible placeholder.
///
/// Decoded QR text is attacker controlled; we never want it to rearrange itself on screen.
pub fn sanitize_for_display(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            // bidi embedding / override / isolate controls
            '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{200E}' | '\u{200F}' => '\u{FFFD}',
            // zero-width characters
            '\u{200B}'..='\u{200D}' | '\u{2060}' | '\u{FEFF}' => '\u{FFFD}',
            // other control characters except newline and tab
            c if c.is_control() && c != '\n' && c != '\t' => '\u{FFFD}',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_counts_chars_not_bytes() {
        assert_eq!(truncate_chars("繁體中文測試", 4), "繁體中…");
        assert_eq!(truncate_chars("abc", 3), "abc");
        assert_eq!(truncate_chars("abcd", 3), "ab…");
        assert_eq!(truncate_chars("abcd", 0), "");
    }

    #[test]
    fn single_line_collapses_whitespace() {
        assert_eq!(single_line("  a \n b\t\tc "), "a b c");
    }

    #[test]
    fn sanitize_removes_bidi_controls() {
        let evil = "invoice\u{202E}fdp.exe";
        assert!(!sanitize_for_display(evil).contains('\u{202E}'));
        assert_eq!(sanitize_for_display("ok\nline"), "ok\nline");
    }
}
