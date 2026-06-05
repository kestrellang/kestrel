//! Shared classification + body-extraction for string-literal tokens.
//!
//! The lexer emits `Token::String` for both single-line and multi-line cooked
//! strings, and `Token::RawString` for any pound-prefixed raw form. This
//! module is the single source of truth for re-deriving the form from the
//! token text and producing the cleaned body that downstream layers operate
//! on (HIR-lower for escape decoding, AST-builder for `\(...)` splitting).

/// Classification of a string-literal token's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringForm {
    /// `#`-prefixed raw form (no escapes, no interpolation).
    pub is_raw: bool,
    /// 3-quote opener (multi-line). Otherwise single-line.
    pub is_multiline: bool,
    /// Number of pounds on each side (0 for cooked).
    pub pound_count: usize,
    /// Byte offset into the raw token where content begins.
    pub body_start: usize,
    /// Byte offset into the raw token where content ends (one past last
    /// content byte). May equal `body_start` for empty content.
    pub body_end: usize,
}

/// Classify a string-literal token from its source text.
///
/// `raw` is the full token slice including delimiters. Returns the form plus
/// the byte range of the content body (between delimiters).
pub fn classify_string_token(raw: &str) -> StringForm {
    let bytes = raw.as_bytes();
    let pound_count = bytes.iter().take_while(|&&b| b == b'#').count();
    let is_raw = pound_count > 0;

    // Count opener quotes after the pounds.
    let after_pounds = pound_count;
    let opener_quotes = bytes[after_pounds..]
        .iter()
        .take_while(|&&b| b == b'"')
        .count();

    let is_multiline = opener_quotes >= 3;
    let opener_quote_count = if is_multiline { 3 } else { 1 };
    let body_start = after_pounds + opener_quote_count;

    // Find the closer at the end: `pound_count` pounds at the very end,
    // preceded by exactly `opener_quote_count` quotes. The lexer guarantees
    // this for terminated tokens; for unterminated (no closer) we set
    // body_end = raw.len().
    let body_end = find_close_offset(bytes, pound_count, opener_quote_count, body_start);

    StringForm {
        is_raw,
        is_multiline,
        pound_count,
        body_start,
        body_end,
    }
}

fn find_close_offset(
    bytes: &[u8],
    pound_count: usize,
    quote_count: usize,
    body_start: usize,
) -> usize {
    let total = bytes.len();
    let closer_len = quote_count + pound_count;
    if total < body_start + closer_len {
        return total;
    }
    // Trailing pounds.
    let trail_pounds = bytes[total - pound_count..total].iter().all(|&b| b == b'#');
    if !trail_pounds {
        return total;
    }
    // Quotes immediately before the trailing pounds.
    let quotes_end = total - pound_count;
    let quotes_start = quotes_end - quote_count;
    if quotes_start < body_start {
        return total;
    }
    let trail_quotes = bytes[quotes_start..quotes_end].iter().all(|&b| b == b'"');
    if !trail_quotes {
        return total;
    }
    quotes_start
}

/// Extract the cleaned body from a multi-line string body. Performs
/// `\r\n` → `\n` normalization and Swift-style indent stripping (the
/// indentation column of the closing `"""` defines the strip prefix).
///
/// Returns `(cleaned_body, errors, body_offset_within_input)`.
///
/// `body` is the raw content between opener and closer. `body_offset` is the
/// byte offset of `body` within the original token (so error spans can be
/// computed). `token_start` is the byte offset of the token within the file.
pub fn process_multiline_body(
    body: &str,
    body_offset_in_token: usize,
    token_start: usize,
) -> ProcessedMultilineBody {
    let mut errors = Vec::new();

    // 1. Normalize `\r\n` → `\n`, `\r` alone → `\n`.
    let normalized = normalize_newlines(body);

    // 2. Body must begin with a newline (the line after the opening `"""`).
    if !normalized.starts_with('\n') {
        errors.push(MultilineError {
            kind: MultilineErrorKind::MissingLeadingNewline,
            span_start: token_start + body_offset_in_token,
            span_end: token_start + body_offset_in_token,
        });
        // Keep going — treat content as-is for best-effort recovery.
        return ProcessedMultilineBody {
            value: normalized,
            errors,
        };
    }

    // 3. Body must end with a newline + (possibly empty) indentation. Find
    // the last newline; everything after is the indent prefix that defines
    // the strip column.
    let Some(last_nl) = normalized.rfind('\n') else {
        errors.push(MultilineError {
            kind: MultilineErrorKind::MissingTrailingNewline,
            span_start: token_start + body_offset_in_token,
            span_end: token_start + body_offset_in_token + normalized.len(),
        });
        return ProcessedMultilineBody {
            value: normalized,
            errors,
        };
    };
    let indent = &normalized[last_nl + 1..];
    if !indent.bytes().all(|b| b == b' ' || b == b'\t') {
        errors.push(MultilineError {
            kind: MultilineErrorKind::MissingTrailingNewline,
            span_start: token_start + body_offset_in_token + last_nl + 1,
            span_end: token_start + body_offset_in_token + normalized.len(),
        });
        return ProcessedMultilineBody {
            value: normalized,
            errors,
        };
    }

    // 4. Strip leading newline, trailing newline+indent, and the indent
    // prefix from each remaining line.
    let inner = &normalized[1..last_nl]; // exclude leading `\n` and trailing `\n` + indent
    let mut out = String::with_capacity(inner.len());
    for (i, line) in inner.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix(indent) {
            out.push_str(rest);
        } else {
            // Under-indented line. Emit error pointing at the line.
            // Compute approximate offset: walk back through the lines.
            let mut line_start_in_inner = 0;
            for (j, prev) in inner.split('\n').enumerate() {
                if j == i {
                    break;
                }
                line_start_in_inner += prev.len() + 1; // +1 for the `\n`
            }
            let abs = token_start + body_offset_in_token + 1 + line_start_in_inner;
            errors.push(MultilineError {
                kind: MultilineErrorKind::ContentUnderIndented,
                span_start: abs,
                span_end: abs + line.len(),
            });
            // Best-effort: trim what whitespace it does have.
            let trimmed = line.trim_start_matches([' ', '\t']);
            out.push_str(trimmed);
        }
    }

    ProcessedMultilineBody { value: out, errors }
}

fn normalize_newlines(s: &str) -> String {
    if !s.contains('\r') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                out.push('\n');
                i += 2;
                continue;
            }
            out.push('\n');
            i += 1;
            continue;
        }
        // Push the next UTF-8 scalar.
        let ch_len = utf8_char_len(bytes[i]);
        out.push_str(&s[i..i + ch_len]);
        i += ch_len;
    }
    out
}

fn utf8_char_len(b: u8) -> usize {
    // < 0xC0 covers ASCII and stray continuation bytes (recovered as length 1).
    if b < 0xC0 {
        1
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    }
}

#[derive(Debug, Clone)]
pub struct ProcessedMultilineBody {
    pub value: String,
    pub errors: Vec<MultilineError>,
}

#[derive(Debug, Clone)]
pub struct MultilineError {
    pub kind: MultilineErrorKind,
    pub span_start: usize,
    pub span_end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultilineErrorKind {
    /// Multi-line string body must start with a newline (after the opening `"""`).
    MissingLeadingNewline,
    /// Multi-line string body must end with a newline before the closing `"""`.
    MissingTrailingNewline,
    /// A content line is less indented than the closing `"""`.
    ContentUnderIndented,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_single_line_cooked() {
        let f = classify_string_token(r#""hello""#);
        assert!(!f.is_raw);
        assert!(!f.is_multiline);
        assert_eq!(f.pound_count, 0);
        assert_eq!(f.body_start, 1);
        assert_eq!(f.body_end, 6);
    }

    #[test]
    fn classify_multiline_cooked() {
        let raw = "\"\"\"\nhello\n\"\"\"";
        let f = classify_string_token(raw);
        assert!(!f.is_raw);
        assert!(f.is_multiline);
        assert_eq!(f.pound_count, 0);
        assert_eq!(f.body_start, 3);
        assert_eq!(f.body_end, raw.len() - 3);
        assert_eq!(&raw[f.body_start..f.body_end], "\nhello\n");
    }

    #[test]
    fn classify_single_line_raw() {
        let f = classify_string_token(r##"#"abc"#"##);
        assert!(f.is_raw);
        assert!(!f.is_multiline);
        assert_eq!(f.pound_count, 1);
        assert_eq!(f.body_start, 2);
        assert_eq!(f.body_end, 5);
    }

    #[test]
    fn classify_multiline_raw_with_double_pound() {
        let raw = "##\"\"\"\nhi\n\"\"\"##";
        let f = classify_string_token(raw);
        assert!(f.is_raw);
        assert!(f.is_multiline);
        assert_eq!(f.pound_count, 2);
        assert_eq!(&raw[f.body_start..f.body_end], "\nhi\n");
    }

    #[test]
    fn classify_empty_single_line_raw() {
        let f = classify_string_token(r##"#""#"##);
        assert!(f.is_raw);
        assert!(!f.is_multiline);
        assert_eq!(f.body_start, 2);
        assert_eq!(f.body_end, 2);
    }

    #[test]
    fn process_multiline_strips_indent() {
        // body = "\n    hello\n      world\n    "
        // closer indent column = 4 spaces → strip 4 spaces from each line.
        let body = "\n    hello\n      world\n    ";
        let p = process_multiline_body(body, 0, 0);
        assert!(p.errors.is_empty(), "errors: {:?}", p.errors);
        assert_eq!(p.value, "hello\n  world");
    }

    #[test]
    fn process_multiline_no_indent() {
        let body = "\nhello\nworld\n";
        let p = process_multiline_body(body, 0, 0);
        assert!(p.errors.is_empty());
        assert_eq!(p.value, "hello\nworld");
    }

    #[test]
    fn process_multiline_under_indented_line_errors() {
        let body = "\n    hello\n  short\n    ";
        let p = process_multiline_body(body, 0, 0);
        assert_eq!(p.errors.len(), 1);
        assert_eq!(p.errors[0].kind, MultilineErrorKind::ContentUnderIndented);
        // Best-effort recovery: trims the line's whitespace and includes it.
        assert_eq!(p.value, "hello\nshort");
    }

    #[test]
    fn process_multiline_normalizes_crlf() {
        let body = "\r\n  hello\r\n  ";
        let p = process_multiline_body(body, 0, 0);
        assert!(p.errors.is_empty());
        assert_eq!(p.value, "hello");
    }
}
