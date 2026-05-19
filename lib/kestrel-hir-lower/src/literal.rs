//! String literal escape decoding.
//!
//! Lowering owns the decode so HIR holds the canonical decoded value.
//! Errors are returned as data alongside the value — `kestrel-analyze`'s
//! `StringEscapeAnalyzer` turns them into diagnostics. This keeps lowering
//! pure (input → data, no diagnostic sink) while ensuring codegen and the
//! analyzer share one source of truth.
//!
//! Ported from lib1's `process_string_escapes`
//! (lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs).

use kestrel_ast_builder::string_token::{self, MultilineErrorKind};
use kestrel_hir::body::{EscapeError, EscapeErrorKind, UnicodeEscapeErrorReason};
use kestrel_span::Span;

/// Decode the unquoted contents of a string literal.
///
/// `content` is the slice between the surrounding `"`s. `file_id` and
/// `content_start` are used to compute absolute file spans for each
/// individual escape error (so diagnostics point at e.g. `\xFF` rather
/// than the whole string).
pub fn decode_string(
    content: &str,
    file_id: usize,
    content_start: usize,
) -> (String, Vec<EscapeError>) {
    let mut result = String::with_capacity(content.len());
    let mut errors = Vec::new();
    let mut chars = content.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        if c != '\\' {
            result.push(c);
            continue;
        }

        let escape_start = content_start + i;
        match chars.next() {
            None => {
                // Trailing backslash at end of string.
                errors.push(EscapeError {
                    span: Span::new(file_id, escape_start..escape_start + 1),
                    kind: EscapeErrorKind::IncompleteEscape,
                });
                result.push('\\');
            },
            Some((j, next_char)) => match next_char {
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                't' => result.push('\t'),
                '\\' => result.push('\\'),
                '"' => result.push('"'),
                '\'' => result.push('\''),
                '0' => result.push('\0'),
                // Line continuation: `\` followed by newline + leading whitespace
                '\n' => skip_continuation_whitespace(&mut chars),
                '\r' => {
                    if let Some(&(_, '\n')) = chars.peek() {
                        chars.next();
                    }
                    skip_continuation_whitespace(&mut chars);
                },
                'x' => decode_ascii_escape(
                    &mut chars,
                    &mut result,
                    &mut errors,
                    file_id,
                    escape_start,
                    content_start + j + 1,
                ),
                'u' => decode_unicode_escape(
                    &mut chars,
                    &mut result,
                    &mut errors,
                    file_id,
                    escape_start,
                    content_start + j,
                ),
                // `\(` is interpolation syntax — the AST builder should have
                // rerouted this string to InterpolatedString before it
                // reached literal decoding. If we get here, treat as invalid.
                '(' => {
                    let paren_len = '('.len_utf8();
                    errors.push(EscapeError {
                        span: Span::new(file_id, escape_start..content_start + j + paren_len),
                        kind: EscapeErrorKind::InvalidEscape {
                            sequence: "\\(".to_string(),
                        },
                    });
                },
                other => {
                    let other_len = other.len_utf8();
                    errors.push(EscapeError {
                        span: Span::new(file_id, escape_start..content_start + j + other_len),
                        kind: EscapeErrorKind::InvalidEscape {
                            sequence: format!("\\{}", other),
                        },
                    });
                    result.push('\\');
                    result.push(other);
                },
            },
        }
    }

    (result, errors)
}

fn skip_continuation_whitespace(chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>) {
    while let Some(&(_, ch)) = chars.peek() {
        if ch == ' ' || ch == '\t' {
            chars.next();
        } else {
            break;
        }
    }
}

fn decode_ascii_escape(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    result: &mut String,
    errors: &mut Vec<EscapeError>,
    file_id: usize,
    escape_start: usize,
    hex_start: usize,
) {
    let mut hex_str = String::new();
    for _ in 0..2 {
        if let Some(&(_, ch)) = chars.peek() {
            if ch.is_ascii_hexdigit() {
                hex_str.push(ch);
                chars.next();
            } else {
                break;
            }
        }
    }

    if hex_str.len() != 2 {
        // Incomplete `\xN` — record the error span over what we read so far.
        errors.push(EscapeError {
            span: Span::new(file_id, escape_start..hex_start + hex_str.len()),
            kind: EscapeErrorKind::InvalidEscape {
                sequence: format!("\\x{}", hex_str),
            },
        });
        result.push_str(&format!("\\x{}", hex_str));
        return;
    }

    let value = u8::from_str_radix(&hex_str, 16).unwrap();
    if value > 0x7F {
        errors.push(EscapeError {
            span: Span::new(file_id, escape_start..hex_start + 2),
            kind: EscapeErrorKind::AsciiEscapeOutOfRange { value },
        });
        result.push_str(&format!("\\x{:02X}", value));
    } else {
        result.push(value as char);
    }
}

fn decode_unicode_escape(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    result: &mut String,
    errors: &mut Vec<EscapeError>,
    file_id: usize,
    escape_start: usize,
    u_pos: usize,
) {
    // Expect opening brace right after `\u`.
    if chars.peek().map(|&(_, c)| c) != Some('{') {
        errors.push(EscapeError {
            span: Span::new(file_id, escape_start..u_pos + 1),
            kind: EscapeErrorKind::InvalidUnicodeEscape {
                value: "\\u".to_string(),
                reason: UnicodeEscapeErrorReason::MissingOpenBrace,
            },
        });
        result.push_str("\\u");
        return;
    }
    chars.next(); // consume '{'

    let mut hex_str = String::new();
    let mut found_close = false;
    let mut had_invalid_digit = false;
    while let Some(&(_, ch)) = chars.peek() {
        if ch == '}' {
            chars.next();
            found_close = true;
            break;
        } else if ch.is_ascii_hexdigit() {
            hex_str.push(ch);
            chars.next();
        } else if ch == '"' || ch == '\\' {
            // Don't consume the terminating quote or another escape.
            break;
        } else {
            had_invalid_digit = true;
            hex_str.push(ch);
            chars.next();
        }
    }

    let escape_end = u_pos + 2 + hex_str.len() + if found_close { 1 } else { 0 };
    let escape_seq = format!("\\u{{{}}}", hex_str);

    if !found_close {
        errors.push(EscapeError {
            span: Span::new(file_id, escape_start..escape_end),
            kind: EscapeErrorKind::InvalidUnicodeEscape {
                value: escape_seq.clone(),
                reason: UnicodeEscapeErrorReason::MissingCloseBrace,
            },
        });
        result.push_str(&escape_seq);
    } else if hex_str.is_empty() {
        errors.push(EscapeError {
            span: Span::new(file_id, escape_start..escape_end),
            kind: EscapeErrorKind::InvalidUnicodeEscape {
                value: escape_seq.clone(),
                reason: UnicodeEscapeErrorReason::EmptyBraces,
            },
        });
        result.push_str(&escape_seq);
    } else if had_invalid_digit {
        errors.push(EscapeError {
            span: Span::new(file_id, escape_start..escape_end),
            kind: EscapeErrorKind::InvalidUnicodeEscape {
                value: escape_seq.clone(),
                reason: UnicodeEscapeErrorReason::InvalidHexDigit,
            },
        });
        result.push_str(&escape_seq);
    } else if hex_str.len() > 6 {
        errors.push(EscapeError {
            span: Span::new(file_id, escape_start..escape_end),
            kind: EscapeErrorKind::InvalidUnicodeEscape {
                value: escape_seq.clone(),
                reason: UnicodeEscapeErrorReason::TooManyDigits,
            },
        });
        result.push_str(&escape_seq);
    } else {
        match u32::from_str_radix(&hex_str, 16) {
            Ok(code_point) if code_point <= 0x10FFFF => {
                if let Some(ch) = char::from_u32(code_point) {
                    result.push(ch);
                } else {
                    // Surrogate or otherwise non-scalar.
                    errors.push(EscapeError {
                        span: Span::new(file_id, escape_start..escape_end),
                        kind: EscapeErrorKind::InvalidUnicodeEscape {
                            value: escape_seq.clone(),
                            reason: UnicodeEscapeErrorReason::OutOfRange,
                        },
                    });
                    result.push_str(&escape_seq);
                }
            },
            _ => {
                errors.push(EscapeError {
                    span: Span::new(file_id, escape_start..escape_end),
                    kind: EscapeErrorKind::InvalidUnicodeEscape {
                        value: escape_seq.clone(),
                        reason: UnicodeEscapeErrorReason::OutOfRange,
                    },
                });
                result.push_str(&escape_seq);
            },
        }
    }
}

/// Strip delimiters from a string-literal token and (for cooked forms)
/// decode escape sequences.
///
/// Handles all four forms:
///   - `"..."`         — single-line cooked; strip 1 quote, decode escapes
///   - `"""\n...\n"""` — multi-line cooked; indent-strip + decode escapes
///   - `#"..."#`       — single-line raw; strip pounds + 1 quote, no decode
///   - `#"""\n...\n"""#` — multi-line raw; indent-strip, no decode
/// Pound count > 1 is supported for both raw forms (`##"..."##`, etc.).
pub fn decode_string_literal_token(
    raw: &str,
    file_id: usize,
    literal_start: usize,
) -> (String, Vec<EscapeError>) {
    let form = string_token::classify_string_token(raw);

    if form.body_end < form.body_start {
        // Defensive — classification should never produce a negative body.
        return (String::new(), Vec::new());
    }

    let body = &raw[form.body_start..form.body_end];
    let body_offset_in_token = form.body_start;

    if form.is_multiline {
        let processed =
            string_token::process_multiline_body(body, body_offset_in_token, literal_start);
        let mut errors: Vec<EscapeError> = processed
            .errors
            .into_iter()
            .map(|e| multiline_error_to_escape(e, file_id))
            .collect();
        if form.is_raw {
            return (processed.value, errors);
        }
        // Cooked multi-line: decode escapes on the indent-stripped body.
        // Note: error spans within the decoded body don't map back to the
        // original source positions byte-for-byte (indent strip changes
        // offsets), but they at least point to the same token region.
        let (value, escape_errs) = decode_string(
            &processed.value,
            file_id,
            literal_start + body_offset_in_token + 1,
        );
        errors.extend(escape_errs);
        return (value, errors);
    }

    if form.is_raw {
        // Single-line raw — no escape processing.
        return (body.to_string(), Vec::new());
    }

    // Single-line cooked.
    decode_string(body, file_id, literal_start + body_offset_in_token)
}

fn multiline_error_to_escape(err: string_token::MultilineError, file_id: usize) -> EscapeError {
    let kind = match err.kind {
        MultilineErrorKind::ContentUnderIndented => EscapeErrorKind::MultilineUnderIndented,
        MultilineErrorKind::MissingLeadingNewline => {
            EscapeErrorKind::MultilineMissingLeadingNewline
        },
        MultilineErrorKind::MissingTrailingNewline => {
            EscapeErrorKind::MultilineMissingTrailingNewline
        },
    };
    EscapeError {
        span: Span::new(file_id, err.span_start..err.span_end),
        kind,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_escapes_decode_clean() {
        let (s, errs) = decode_string("hello\\nworld", 0, 0);
        assert_eq!(s, "hello\nworld");
        assert!(errs.is_empty());
    }

    #[test]
    fn invalid_escape_reported() {
        let (_, errs) = decode_string("a\\qb", 0, 0);
        assert_eq!(errs.len(), 1);
        match &errs[0].kind {
            EscapeErrorKind::InvalidEscape { sequence } => assert_eq!(sequence, "\\q"),
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn ascii_escape_out_of_range() {
        let (_, errs) = decode_string("\\xFF", 0, 0);
        assert_eq!(errs.len(), 1);
        assert!(matches!(
            errs[0].kind,
            EscapeErrorKind::AsciiEscapeOutOfRange { value: 0xFF }
        ));
    }

    #[test]
    fn unicode_empty_braces() {
        let (_, errs) = decode_string("\\u{}", 0, 0);
        assert_eq!(errs.len(), 1);
        assert!(matches!(
            errs[0].kind,
            EscapeErrorKind::InvalidUnicodeEscape {
                reason: UnicodeEscapeErrorReason::EmptyBraces,
                ..
            }
        ));
    }

    #[test]
    fn unicode_too_many_digits() {
        let (_, errs) = decode_string("\\u{1234567}", 0, 0);
        assert_eq!(errs.len(), 1);
        assert!(matches!(
            errs[0].kind,
            EscapeErrorKind::InvalidUnicodeEscape {
                reason: UnicodeEscapeErrorReason::TooManyDigits,
                ..
            }
        ));
    }

    #[test]
    fn unicode_out_of_range() {
        let (_, errs) = decode_string("\\u{110000}", 0, 0);
        assert_eq!(errs.len(), 1);
        assert!(matches!(
            errs[0].kind,
            EscapeErrorKind::InvalidUnicodeEscape {
                reason: UnicodeEscapeErrorReason::OutOfRange,
                ..
            }
        ));
    }

    #[test]
    fn unicode_missing_brace() {
        let (_, errs) = decode_string("\\u1234", 0, 0);
        assert_eq!(errs.len(), 1);
        assert!(matches!(
            errs[0].kind,
            EscapeErrorKind::InvalidUnicodeEscape {
                reason: UnicodeEscapeErrorReason::MissingOpenBrace,
                ..
            }
        ));
    }

    #[test]
    fn incomplete_hex() {
        let (_, errs) = decode_string("\\xZ", 0, 0);
        assert_eq!(errs.len(), 1);
        match &errs[0].kind {
            EscapeErrorKind::InvalidEscape { sequence } => assert_eq!(sequence, "\\x"),
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn pound_raw_string_skips_escapes() {
        // `#"a\nb"#` — single-line raw; backslash is literal.
        let (s, errs) = decode_string_literal_token("#\"a\\nb\"#", 0, 0);
        assert_eq!(s, "a\\nb");
        assert!(errs.is_empty());
    }

    #[test]
    fn multiline_raw_strips_indent_and_keeps_escapes_literal() {
        // `#"""\n    a\\nb\n    """#` → "a\\nb" (no escape decoding).
        let raw = "#\"\"\"\n    a\\nb\n    \"\"\"#";
        let (s, errs) = decode_string_literal_token(raw, 0, 0);
        assert!(errs.is_empty(), "errors: {:?}", errs);
        assert_eq!(s, "a\\nb");
    }

    #[test]
    fn multiline_cooked_strips_indent_and_decodes_escapes() {
        // `"""\n    a\nb\n    """` → "a\nb" (escape `\n` decoded to newline).
        let raw = "\"\"\"\n    a\\nb\n    \"\"\"";
        let (s, errs) = decode_string_literal_token(raw, 0, 0);
        assert!(errs.is_empty(), "errors: {:?}", errs);
        assert_eq!(s, "a\nb");
    }

    #[test]
    fn interpolation_escape_in_decoder_is_invalid() {
        // The AST builder should reroute interpolation-bearing strings to
        // `InterpolatedString` before they reach this decoder. If `\(` does
        // arrive here, that's a bug upstream — record an InvalidEscape so
        // the analyzer surfaces it.
        let (_, errs) = decode_string("a=\\(x)", 0, 0);
        assert_eq!(errs.len(), 1);
        assert!(matches!(
            &errs[0].kind,
            EscapeErrorKind::InvalidEscape { sequence } if sequence == "\\("
        ));
    }

    #[test]
    fn span_offsets_track_content_start() {
        let (_, errs) = decode_string("ab\\xFF", 7, 100);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].span.file_id, 7);
        // \xFF starts at offset 2 within content, content starts at 100, so escape_start=102.
        assert_eq!(errs[0].span.start, 102);
        assert_eq!(errs[0].span.end, 106);
    }
}
