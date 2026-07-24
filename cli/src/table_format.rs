use colored::Colorize;

/// Returns the number of visible (non-ANSI-escape) characters in `s`.
/// ANSI color escape sequences (\x1b[...m) are stripped before counting.
fn visible_len(s: &str) -> usize {
    let mut count = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            count += 1;
        }
    }
    count
}

/// Highlights all case-insensitive occurrences of `query` within `text`
/// by wrapping each match in yellow+bold ANSI codes.
/// Non-matching portions are returned verbatim.
pub fn highlight_match(text: &str, query: &str) -> String {
    if query.is_empty() {
        return text.to_string();
    }
    let lower_text = text.to_lowercase();
    let lower_query = query.to_lowercase();

    match lower_text.find(&lower_query) {
        None => text.to_string(),
        Some(start) => {
            let end = (start + lower_query.len()).min(text.len());
            if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
                return text.to_string();
            }
            let before = &text[..start];
            let matched = &text[start..end];
            let rest = &text[end..];
            format!(
                "{}{}{}",
                before,
                matched.yellow().bold(),
                highlight_match(rest, query)
            )
        }
    }
}

/// Pads `s` with trailing spaces so that its visible terminal width equals `width`.
/// Works correctly for strings that contain ANSI escape sequences.
pub fn pad_to(s: &str, width: usize) -> String {
    let vlen = visible_len(s);
    let padding = width.saturating_sub(vlen);
    format!("{}{}", s, " ".repeat(padding))
}

/// Renders a fixed-width table with bold headers, a separator line, and data rows.
///
/// `col_widths` must be the *visible* column widths (not byte lengths).
/// Cells in `rows` may contain ANSI escape sequences; alignment is handled correctly.
pub fn render_table(headers: &[&str], col_widths: &[usize], rows: &[Vec<String>]) -> String {
    let sep = "  ";
    let mut out = String::new();

    let header_parts: Vec<String> = headers
        .iter()
        .zip(col_widths.iter())
        .map(|(h, &w)| pad_to(&h.bold().to_string(), w))
        .collect();
    out.push_str(&header_parts.join(sep));
    out.push('\n');

    let sep_parts: Vec<String> = col_widths.iter().map(|&w| "─".repeat(w)).collect();
    out.push_str(&sep_parts.join(sep).bright_black().to_string());
    out.push('\n');

    for row in rows {
        let row_parts: Vec<String> = row
            .iter()
            .zip(col_widths.iter())
            .map(|(cell, &w)| pad_to(cell, w))
            .collect();
        out.push_str(&row_parts.join(sep));
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_match_no_match_returns_original() {
        assert_eq!(highlight_match("MyToken", "xyz"), "MyToken");
    }

    #[test]
    fn highlight_match_empty_query_returns_original() {
        assert_eq!(highlight_match("hello", ""), "hello");
    }

    #[test]
    fn highlight_match_reconstructs_full_text_on_match() {
        // Even without ANSI codes the original characters must all be present.
        let result = highlight_match("MyToken", "token");
        // "My" prefix and "Token" suffix must both appear
        assert!(result.contains("My"));
        assert!(result.contains("Token"));
    }

    #[test]
    fn highlight_match_is_case_insensitive() {
        // Matching "hello" against "HELLO" (case-insensitive) should find a match
        // and reconstruct the original text (with or without ANSI codes).
        let lower = highlight_match("HELLO", "hello");
        let upper = highlight_match("hello", "HELLO");
        assert!(lower.contains("HELLO"));
        assert!(upper.contains("hello"));
    }

    #[test]
    fn pad_to_adds_trailing_spaces() {
        assert_eq!(pad_to("hi", 5), "hi   ");
    }

    #[test]
    fn pad_to_does_not_truncate_longer_input() {
        assert_eq!(pad_to("hello", 3), "hello");
    }

    #[test]
    fn pad_to_exact_width_no_change() {
        assert_eq!(pad_to("abc", 3), "abc");
    }

    #[test]
    fn render_table_contains_headers_and_row_data() {
        let rows = vec![vec!["alice".to_string(), "testnet".to_string()]];
        let out = render_table(&["Name", "Network"], &[5, 7], &rows);
        assert!(out.contains("Name"));
        assert!(out.contains("Network"));
        assert!(out.contains("alice"));
        assert!(out.contains("testnet"));
    }

    #[test]
    fn render_table_has_separator_line() {
        let rows: Vec<Vec<String>> = vec![];
        let out = render_table(&["Col"], &[3], &rows);
        assert!(out.contains('─'));
    }

    #[test]
    fn render_table_empty_rows_still_renders_header() {
        let out = render_table(&["Name", "Network"], &[4, 7], &[]);
        assert!(out.contains("Name"));
        assert!(out.contains("Network"));
    }
}
