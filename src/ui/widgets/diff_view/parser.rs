//! Diff parsing utilities
//!
//! This module contains functions for parsing various content types
//! into displayable DiffLine structures.

/// A parsed line ready for display in the diff view
#[derive(Debug, Clone)]
pub struct DiffLine {
    pub left_text: Option<String>,
    pub right_text: Option<String>,
    pub left_num: Option<usize>,
    pub right_num: Option<usize>,
    pub line_type: LineType,
    pub is_header: bool,
}

/// Type of diff line for styling purposes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineType {
    Context,
    Added,
    Removed,
    Header,
    Info,
    Comment,
}

/// Check if content appears to be binary
pub fn is_binary(content: &str) -> bool {
    // Check first ~8KB for null bytes, but ensure we don't slice mid-character
    content.chars().take(8192).any(|c| c == '\0')
}

/// Parse a unified diff into DiffLines
pub fn parse_diff(content: &str) -> Vec<DiffLine> {
    let mut lines = Vec::new();
    let mut left_num = 1usize;
    let mut right_num = 1usize;

    for line in content.lines() {
        if line.starts_with("@@") {
            if let Some((l, r)) = parse_hunk_header(line) {
                left_num = l;
                right_num = r;
            }
            // Skip rendering hunk headers (@@ ... @@)
            continue;
        } else if line.starts_with("diff --git")
            || line.starts_with("index ")
            || line.starts_with("---")
            || line.starts_with("+++")
            || line.starts_with("new file")
            || line.starts_with("deleted file")
        {
            lines.push(DiffLine {
                left_text: Some(line.to_string()),
                right_text: None,
                left_num: None,
                right_num: None,
                line_type: LineType::Header,
                is_header: true,
            });
        } else if line.starts_with('+') {
            lines.push(DiffLine {
                left_text: None,
                right_text: Some(line[1..].to_string()),
                left_num: None,
                right_num: Some(right_num),
                line_type: LineType::Added,
                is_header: false,
            });
            right_num += 1;
        } else if line.starts_with('-') {
            lines.push(DiffLine {
                left_text: Some(line[1..].to_string()),
                right_text: None,
                left_num: Some(left_num),
                right_num: None,
                line_type: LineType::Removed,
                is_header: false,
            });
            left_num += 1;
        } else if line.starts_with(' ') {
            lines.push(DiffLine {
                left_text: Some(line[1..].to_string()),
                right_text: Some(line[1..].to_string()),
                left_num: Some(left_num),
                right_num: Some(right_num),
                line_type: LineType::Context,
                is_header: false,
            });
            left_num += 1;
            right_num += 1;
        } else {
            lines.push(DiffLine {
                left_text: Some(line.to_string()),
                right_text: None,
                left_num: None,
                right_num: None,
                line_type: LineType::Context,
                is_header: true,
            });
        }
    }

    lines
}

/// Extract left (original) and right (new) file content from a diff
pub fn extract_diff_sides(diff: &str) -> (Vec<String>, Vec<String>) {
    let mut left_lines = Vec::new();
    let mut right_lines = Vec::new();

    for line in diff.lines() {
        if line.starts_with("@@")
            || line.starts_with("diff ")
            || line.starts_with("index ")
            || line.starts_with("---")
            || line.starts_with("+++")
            || line.starts_with("new file")
            || line.starts_with("deleted file")
        {
            continue;
        }

        if line.starts_with('-') {
            left_lines.push(line[1..].to_string());
        } else if line.starts_with('+') {
            right_lines.push(line[1..].to_string());
        } else if line.starts_with(' ') {
            left_lines.push(line[1..].to_string());
            right_lines.push(line[1..].to_string());
        }
    }

    (left_lines, right_lines)
}

/// Parse a hunk header to extract starting line numbers
/// Returns (left_start, right_start)
pub fn parse_hunk_header(line: &str) -> Option<(usize, usize)> {
    // Parse @@ -start,count +start,count @@
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }

    let left_start = parts
        .get(1)?
        .trim_start_matches('-')
        .split(',')
        .next()?
        .parse()
        .ok()?;

    let right_start = parts
        .get(2)?
        .trim_start_matches('+')
        .split(',')
        .next()?
        .parse()
        .ok()?;

    Some((left_start, right_start))
}

/// Wrap text at word boundaries to fit within max_width
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            if word.len() > max_width {
                // Word is longer than max_width, split it
                let mut remaining = word;
                while remaining.len() > max_width {
                    let (chunk, rest) = remaining.split_at(max_width);
                    lines.push(chunk.to_string());
                    remaining = rest;
                }
                current_line = remaining.to_string();
            } else {
                current_line = word.to_string();
            }
        } else if current_line.len() + 1 + word.len() <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    }
}

/// Truncate a string to fit within a width, or pad it to that width
pub fn truncate_or_pad(s: &str, width: usize) -> String {
    let char_count = s.chars().count();
    if char_count > width {
        s.chars()
            .take(width.saturating_sub(1))
            .collect::<String>()
            + "\u{2026}"
    } else {
        format!("{:width$}", s, width = width)
    }
}

/// Parse file content (not diff) into DiffLines for display
/// Shows only lines up to max_indent levels of indentation (skeleton view)
pub fn parse_file_content(content: &str, max_indent: usize) -> Vec<DiffLine> {
    let mut lines = Vec::new();

    // Detect indent unit from file (find smallest non-zero indent)
    let indent_unit = detect_indent_unit(content);

    for (line_num, line) in content.lines().enumerate() {
        let indent_level = get_indent_level(line, indent_unit);

        // Only show lines with indent level up to max_indent
        if indent_level <= max_indent {
            lines.push(DiffLine {
                left_text: Some(line.to_string()),
                right_text: None,
                left_num: Some(line_num + 1), // Actual line number in file
                right_num: None,
                line_type: LineType::Context,
                is_header: false,
            });
        }
    }

    lines
}

/// Detect the indentation unit used in a file (e.g., 2 spaces, 4 spaces, 1 tab)
fn detect_indent_unit(content: &str) -> usize {
    let mut min_indent = usize::MAX;

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let leading_spaces = line.len() - line.trim_start().len();
        if leading_spaces > 0 && leading_spaces < min_indent {
            min_indent = leading_spaces;
        }
    }

    // Default to 4 if we couldn't detect
    if min_indent == usize::MAX {
        4
    } else {
        min_indent.max(1) // At least 1
    }
}

/// Calculate indent level for a line based on indent unit
fn get_indent_level(line: &str, indent_unit: usize) -> usize {
    if line.trim().is_empty() {
        return usize::MAX; // Skip empty lines
    }

    let leading_whitespace = line.len() - line.trim_start().len();
    leading_whitespace / indent_unit
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_binary ---

    #[test]
    fn is_binary_detects_null_bytes() {
        assert!(is_binary("hello\0world"));
    }

    #[test]
    fn is_binary_false_for_normal_text() {
        assert!(!is_binary("fn main() {\n    println!(\"hello\");\n}"));
    }

    #[test]
    fn is_binary_false_for_empty() {
        assert!(!is_binary(""));
    }

    // --- parse_hunk_header ---

    #[test]
    fn parse_hunk_header_standard() {
        assert_eq!(parse_hunk_header("@@ -10,5 +15,8 @@"), Some((10, 15)));
    }

    #[test]
    fn parse_hunk_header_single_line() {
        assert_eq!(parse_hunk_header("@@ -1 +1 @@"), Some((1, 1)));
    }

    #[test]
    fn parse_hunk_header_with_context() {
        assert_eq!(
            parse_hunk_header("@@ -100,20 +110,25 @@ fn some_function()"),
            Some((100, 110))
        );
    }

    #[test]
    fn parse_hunk_header_invalid() {
        assert_eq!(parse_hunk_header("not a header"), None);
        assert_eq!(parse_hunk_header("@@"), None);
    }

    // --- parse_diff ---

    #[test]
    fn parse_diff_added_lines() {
        let diff = "@@ -1,2 +1,3 @@\n a\n+b\n c\n";
        let lines = parse_diff(diff);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].line_type, LineType::Context);
        assert_eq!(lines[1].line_type, LineType::Added);
        assert_eq!(lines[1].right_text.as_deref(), Some("b"));
        assert_eq!(lines[1].left_text, None);
        assert_eq!(lines[2].line_type, LineType::Context);
    }

    #[test]
    fn parse_diff_removed_lines() {
        let diff = "@@ -1,3 +1,2 @@\n a\n-b\n c\n";
        let lines = parse_diff(diff);
        assert_eq!(lines[1].line_type, LineType::Removed);
        assert_eq!(lines[1].left_text.as_deref(), Some("b"));
        assert_eq!(lines[1].right_text, None);
    }

    #[test]
    fn parse_diff_line_numbers() {
        let diff = "@@ -5,3 +10,3 @@\n a\n-b\n+c\n";
        let lines = parse_diff(diff);
        // Context line: left=5, right=10
        assert_eq!(lines[0].left_num, Some(5));
        assert_eq!(lines[0].right_num, Some(10));
        // Removed: left=6
        assert_eq!(lines[1].left_num, Some(6));
        assert_eq!(lines[1].right_num, None);
        // Added: right=11
        assert_eq!(lines[2].left_num, None);
        assert_eq!(lines[2].right_num, Some(11));
    }

    #[test]
    fn parse_diff_headers() {
        let diff = "diff --git a/foo.rs b/foo.rs\n--- a/foo.rs\n+++ b/foo.rs\n";
        let lines = parse_diff(diff);
        assert!(lines.iter().all(|l| l.is_header));
        assert!(lines.iter().all(|l| l.line_type == LineType::Header));
    }

    #[test]
    fn parse_diff_empty() {
        assert!(parse_diff("").is_empty());
    }

    // --- extract_diff_sides ---

    #[test]
    fn extract_diff_sides_splits_correctly() {
        let diff = "@@ -1,3 +1,3 @@\n old\n-removed\n+added\n context\n";
        let (left, right) = extract_diff_sides(diff);
        // Left gets: "old" (context without prefix ' '), "removed" (- line)...
        // Wait, "old" starts with space so it's context
        // Actually " old" starts with space
        assert!(left.contains(&"removed".to_string()));
        assert!(right.contains(&"added".to_string()));
        // Context appears in both
        assert!(left.contains(&"old".to_string()));
        assert!(right.contains(&"old".to_string()));
    }

    #[test]
    fn extract_diff_sides_skips_headers() {
        let diff = "diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-old\n+new\n";
        let (left, right) = extract_diff_sides(diff);
        assert_eq!(left, vec!["old"]);
        assert_eq!(right, vec!["new"]);
    }

    // --- wrap_text ---

    #[test]
    fn wrap_text_fits_in_width() {
        assert_eq!(wrap_text("hello world", 20), vec!["hello world"]);
    }

    #[test]
    fn wrap_text_wraps_at_boundary() {
        assert_eq!(wrap_text("hello world", 5), vec!["hello", "world"]);
    }

    #[test]
    fn wrap_text_long_word() {
        let result = wrap_text("abcdefghij", 4);
        assert_eq!(result, vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn wrap_text_empty() {
        assert_eq!(wrap_text("", 10), vec![""]);
    }

    // --- truncate_or_pad ---

    #[test]
    fn truncate_or_pad_short_string() {
        let result = truncate_or_pad("hi", 5);
        assert_eq!(result, "hi   ");
    }

    #[test]
    fn truncate_or_pad_exact_fit() {
        let result = truncate_or_pad("hello", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn truncate_or_pad_long_string() {
        let result = truncate_or_pad("hello world", 5);
        assert_eq!(result.chars().count(), 5);
        assert!(result.ends_with('\u{2026}')); // ends with ellipsis
    }

    // --- parse_file_content ---

    #[test]
    fn parse_file_content_filters_by_indent() {
        let content = "fn main() {\n    let x = 1;\n        nested();\n}\n";
        let lines = parse_file_content(content, 0);
        // Only top-level lines (indent 0)
        assert!(lines.iter().all(|l| {
            let text = l.left_text.as_deref().unwrap_or("");
            let indent = text.len() - text.trim_start().len();
            indent == 0
        }));
    }

    #[test]
    fn parse_file_content_line_numbers() {
        let content = "line1\nline2\nline3\n";
        let lines = parse_file_content(content, 10);
        assert_eq!(lines[0].left_num, Some(1));
        assert_eq!(lines[1].left_num, Some(2));
        assert_eq!(lines[2].left_num, Some(3));
    }

    // --- detect_indent_unit ---

    #[test]
    fn detect_indent_unit_two_spaces() {
        assert_eq!(detect_indent_unit("fn main() {\n  let x = 1;\n}\n"), 2);
    }

    #[test]
    fn detect_indent_unit_four_spaces() {
        assert_eq!(detect_indent_unit("fn main() {\n    let x = 1;\n}\n"), 4);
    }

    #[test]
    fn detect_indent_unit_no_indent() {
        assert_eq!(detect_indent_unit("a\nb\nc\n"), 4); // default
    }

    // --- get_indent_level ---

    #[test]
    fn get_indent_level_zero() {
        assert_eq!(get_indent_level("hello", 4), 0);
    }

    #[test]
    fn get_indent_level_one() {
        assert_eq!(get_indent_level("    hello", 4), 1);
    }

    #[test]
    fn get_indent_level_empty_line() {
        assert_eq!(get_indent_level("   ", 4), usize::MAX);
    }
}
