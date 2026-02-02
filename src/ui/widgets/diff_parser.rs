//! Diff parsing utilities
//!
//! This module contains functions for parsing various content types
//! into displayable DiffLine structures.

use crate::github::PrInfo;

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

/// Create a header line for display
fn make_header_line(text: String, line_type: LineType) -> DiffLine {
    DiffLine {
        left_text: Some(text),
        right_text: None,
        left_num: None,
        right_num: None,
        line_type,
        is_header: true,
    }
}

/// Parse PR details into DiffLines
pub fn parse_pr_details(pr: &PrInfo) -> Vec<DiffLine> {
    let mut lines = vec![];

    // PR header
    lines.push(make_header_line(
        format!("PR #{}", pr.number),
        LineType::Header,
    ));
    lines.push(make_header_line("\u{2500}".repeat(40), LineType::Info));
    lines.push(make_header_line(pr.title.clone(), LineType::Info));
    lines.push(make_header_line(String::new(), LineType::Context));
    lines.push(make_header_line(
        format!("State:  {}", pr.state),
        LineType::Context,
    ));
    lines.push(make_header_line(
        format!("Author: @{}", pr.author),
        LineType::Context,
    ));
    lines.push(make_header_line(
        format!("URL:    {}", pr.url),
        LineType::Context,
    ));

    // Description
    if !pr.body.is_empty() {
        lines.push(make_header_line(String::new(), LineType::Context));
        lines.push(make_header_line("Description".to_string(), LineType::Header));
        lines.push(make_header_line("\u{2500}".repeat(40), LineType::Info));
        for line in pr.body.lines() {
            lines.push(make_header_line(format!("  {}", line), LineType::Context));
        }
    }

    // Reviews
    if !pr.reviews.is_empty() {
        lines.push(make_header_line(String::new(), LineType::Context));
        lines.push(make_header_line("Reviews".to_string(), LineType::Header));
        lines.push(make_header_line("\u{2500}".repeat(40), LineType::Info));
        for review in &pr.reviews {
            let (icon, line_type) = match review.state.as_str() {
                "APPROVED" => ("\u{2713}", LineType::Added),
                "CHANGES_REQUESTED" => ("\u{2717}", LineType::Removed),
                _ => ("\u{25CB}", LineType::Context),
            };
            lines.push(make_header_line(
                format!("  {} {} - {}", icon, review.author, review.state),
                line_type,
            ));
            if !review.body.is_empty() {
                for line in review.body.lines() {
                    lines.push(make_header_line(format!("    {}", line), LineType::Context));
                }
            }
        }
    }

    // General comments
    if !pr.comments.is_empty() {
        lines.push(make_header_line(String::new(), LineType::Context));
        lines.push(make_header_line("Comments".to_string(), LineType::Header));
        lines.push(make_header_line("\u{2500}".repeat(40), LineType::Info));
        for comment in &pr.comments {
            lines.push(make_header_line(
                format!("  \u{1F4AC} {}", comment.author),
                LineType::Comment,
            ));
            for line in comment.body.lines() {
                lines.push(make_header_line(format!("    {}", line), LineType::Context));
            }
            lines.push(make_header_line(String::new(), LineType::Context));
        }
    }

    // File comments (grouped by file)
    if !pr.file_comments.is_empty() {
        lines.push(make_header_line(String::new(), LineType::Context));
        lines.push(make_header_line("File Comments".to_string(), LineType::Header));
        lines.push(make_header_line("\u{2500}".repeat(40), LineType::Info));

        for (path, comments) in &pr.file_comments {
            lines.push(make_header_line(format!("  {}", path), LineType::Info));
            for comment in comments {
                let line_info = comment
                    .line
                    .map(|l| format!(":{}", l))
                    .unwrap_or_default();
                lines.push(make_header_line(
                    format!("    \u{1F4AC} @{}{}", comment.author, line_info),
                    LineType::Comment,
                ));
                for line in comment.body.lines() {
                    lines.push(make_header_line(format!("      {}", line), LineType::Context));
                }
            }
            lines.push(make_header_line(String::new(), LineType::Context));
        }
    }

    lines
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
