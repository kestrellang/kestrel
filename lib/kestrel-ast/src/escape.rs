//! Backslash-escape decoding — **the** table for the whole compiler.
//!
//! One language rule ("what does `\X` mean?") used to have three independent
//! implementations, and they disagreed in ways that miscompiled (F26):
//!
//! - `kestrel-hir-lower::literal::decode_string` — the full table, errors
//!   returned as data for the E700-E703 analyzer.
//! - `kestrel-hir-lower::pat::unescape_char_content` — char literals. Read
//!   `\u{…}` hex with an unbounded `for c in chars { if c == '}' { break } }`:
//!   no close-brace requirement and no digit limit, so `'\u{00000041}'`
//!   compiled to `'A'` while `"\u{00000041}"` was rejected. Diverged on
//!   unknown escapes and on `\x80` too, and reported *nothing* in pattern
//!   position (`ctx: None`), so `'\u{D800}'` silently became NUL.
//! - `kestrel-ast-builder::lower::unescape_char_simple` — the literal segments
//!   of any string containing `\(`. Had **no `\x` arm, no `\u` arm and no
//!   error path**, so `"\u{41} \(x)"` silently produced the text `u{41} `
//!   with no diagnostic. Nothing re-decoded it downstream.
//!
//! (Body lowering now reads interpolated strings straight from the CST, in
//! `kestrel-hir-lower`; all three callers live there.) This module lives in
//! `kestrel-ast` because `kestrel-hir` needs the error kinds too. Callers
//! differ only in what they do with the outcome: strings and chars keep
//! errors as data, interpolation segments keep whatever the decode produced.

use std::iter::Peekable;
use std::str::CharIndices;

/// Why a `\u{…}` escape is malformed.
#[derive(Clone, Debug, PartialEq)]
pub enum UnicodeEscapeErrorReason {
    MissingOpenBrace,
    MissingCloseBrace,
    EmptyBraces,
    TooManyDigits,
    InvalidHexDigit,
    OutOfRange,
}

/// A malformed escape. Carried as data, never rendered here — the
/// `StringEscapeAnalyzer` owns E700-E703.
#[derive(Clone, Debug, PartialEq)]
pub enum EscapeErrorKind {
    /// Unknown backslash escape (e.g. `\q`) or malformed `\xNN`.
    InvalidEscape { sequence: String },
    /// `\xNN` with value > 0x7F — strings only allow 7-bit ASCII via `\x`.
    AsciiEscapeOutOfRange { value: u8 },
    /// Trailing `\` at end of string.
    IncompleteEscape,
    /// `\u{...}` malformed in some way; `reason` distinguishes.
    InvalidUnicodeEscape {
        value: String,
        reason: UnicodeEscapeErrorReason,
    },
    /// A line in a multi-line string body has less indentation than the
    /// closing `"""` delimiter.
    MultilineUnderIndented,
    /// Multi-line string opener `"""` must be followed immediately by a
    /// newline.
    MultilineMissingLeadingNewline,
    /// Multi-line string closer `"""` must be on its own line (only
    /// whitespace before it on that line).
    MultilineMissingTrailingNewline,
    /// String literal has no closing delimiter.
    UnterminatedString,
}

/// What a well-formed escape denotes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Escaped {
    /// A Unicode scalar value. `\u{…}` is already range- and surrogate-checked,
    /// so this is always a valid `char`.
    Scalar(u32),
    /// `\` immediately before a newline: a line continuation. The decoder has
    /// consumed the newline and the following run of spaces/tabs.
    LineContinuation,
    /// `\(` — string interpolation. Not an escape at all; the caller must
    /// handle it (or report it, if it reached a context that cannot).
    Interpolation,
}

/// The outcome of decoding one escape sequence.
pub struct Decoded {
    pub result: Result<Escaped, EscapeErrorKind>,
    /// The full source text of the sequence, **including** the leading `\`.
    ///
    /// Callers use it two ways: `raw.len() - 1` is the number of bytes
    /// consumed after the backslash (so a span is `start .. start + raw.len()`),
    /// and on error it is what gets pushed back into the decoded output so the
    /// text still round-trips. Byte length, not char count — a malformed
    /// `\u{café}` may hold multi-byte characters.
    pub raw: String,
}

impl Decoded {
    fn ok(escaped: Escaped, raw: String) -> Self {
        Self {
            result: Ok(escaped),
            raw,
        }
    }
    fn err(kind: EscapeErrorKind, raw: String) -> Self {
        Self {
            result: Err(kind),
            raw,
        }
    }
}

/// Maximum hex digits in `\u{…}`. `10FFFF` is six, and more than six is
/// rejected outright rather than silently overflowing.
const MAX_UNICODE_HEX_DIGITS: usize = 6;

/// Decode one escape sequence. The backslash has already been consumed;
/// `chars` is positioned at the character *after* it.
///
/// Consumes exactly the sequence and nothing more, so the caller can keep
/// iterating. A trailing backslash (nothing after it) is
/// `EscapeErrorKind::IncompleteEscape`.
pub fn decode_escape(chars: &mut Peekable<CharIndices<'_>>) -> Decoded {
    let Some((_, next)) = chars.next() else {
        return Decoded::err(EscapeErrorKind::IncompleteEscape, "\\".to_string());
    };

    let simple = |c: char| Decoded::ok(Escaped::Scalar(c as u32), format!("\\{next}"));
    match next {
        'n' => simple('\n'),
        'r' => simple('\r'),
        't' => simple('\t'),
        '\\' => simple('\\'),
        '"' => simple('"'),
        '\'' => simple('\''),
        '0' => simple('\0'),
        // `\` + newline is a line continuation: drop the newline and the
        // indentation that follows it.
        '\n' => {
            let mut raw = String::from("\\\n");
            skip_continuation_whitespace(chars, &mut raw);
            Decoded::ok(Escaped::LineContinuation, raw)
        },
        '\r' => {
            let mut raw = String::from("\\\r");
            if let Some(&(_, '\n')) = chars.peek() {
                chars.next();
                raw.push('\n');
            }
            skip_continuation_whitespace(chars, &mut raw);
            Decoded::ok(Escaped::LineContinuation, raw)
        },
        '(' => Decoded::ok(Escaped::Interpolation, "\\(".to_string()),
        'x' => decode_ascii(chars),
        'u' => decode_unicode(chars),
        other => Decoded::err(
            EscapeErrorKind::InvalidEscape {
                sequence: format!("\\{other}"),
            },
            format!("\\{other}"),
        ),
    }
}

fn skip_continuation_whitespace(chars: &mut Peekable<CharIndices<'_>>, raw: &mut String) {
    while let Some(&(_, ch)) = chars.peek() {
        if ch == ' ' || ch == '\t' {
            raw.push(ch);
            chars.next();
        } else {
            break;
        }
    }
}

/// `\xNN` — exactly two hex digits, value must be 7-bit.
fn decode_ascii(chars: &mut Peekable<CharIndices<'_>>) -> Decoded {
    let mut hex = String::new();
    for _ in 0..2 {
        match chars.peek() {
            Some(&(_, ch)) if ch.is_ascii_hexdigit() => {
                hex.push(ch);
                chars.next();
            },
            _ => break,
        }
    }
    let raw = format!("\\x{hex}");

    if hex.len() != 2 {
        return Decoded::err(
            EscapeErrorKind::InvalidEscape {
                sequence: raw.clone(),
            },
            raw,
        );
    }
    // `hex` is exactly two ASCII hex digits, so this cannot fail and cannot
    // exceed 0xFF.
    let value = u8::from_str_radix(&hex, 16).unwrap_or(0);
    if value > 0x7F {
        return Decoded::err(EscapeErrorKind::AsciiEscapeOutOfRange { value }, raw);
    }
    Decoded::ok(Escaped::Scalar(value as u32), raw)
}

/// `\u{NNNN}` — 1 to 6 hex digits, in range, not a surrogate.
fn decode_unicode(chars: &mut Peekable<CharIndices<'_>>) -> Decoded {
    if chars.peek().map(|&(_, c)| c) != Some('{') {
        return Decoded::err(
            EscapeErrorKind::InvalidUnicodeEscape {
                value: "\\u".to_string(),
                reason: UnicodeEscapeErrorReason::MissingOpenBrace,
            },
            "\\u".to_string(),
        );
    }
    chars.next(); // consume '{'

    let mut hex = String::new();
    let mut found_close = false;
    let mut had_invalid_digit = false;
    while let Some(&(_, ch)) = chars.peek() {
        if ch == '}' {
            chars.next();
            found_close = true;
            break;
        }
        if ch == '"' || ch == '\'' || ch == '\\' {
            // Never swallow the literal's own terminator or the start of the
            // next escape — that is how the char-literal copy ran off the end
            // of the literal and accepted unbounded digit runs.
            break;
        }
        if !ch.is_ascii_hexdigit() {
            had_invalid_digit = true;
        }
        hex.push(ch);
        chars.next();
    }

    let raw = format!("\\u{{{hex}{}", if found_close { "}" } else { "" });
    let value = format!("\\u{{{hex}}}");
    let fail = |reason| {
        Decoded::err(
            EscapeErrorKind::InvalidUnicodeEscape {
                value: value.clone(),
                reason,
            },
            raw.clone(),
        )
    };

    if !found_close {
        return fail(UnicodeEscapeErrorReason::MissingCloseBrace);
    }
    if hex.is_empty() {
        return fail(UnicodeEscapeErrorReason::EmptyBraces);
    }
    if had_invalid_digit {
        return fail(UnicodeEscapeErrorReason::InvalidHexDigit);
    }
    if hex.len() > MAX_UNICODE_HEX_DIGITS {
        return fail(UnicodeEscapeErrorReason::TooManyDigits);
    }
    match u32::from_str_radix(&hex, 16) {
        // `char::from_u32` rejects surrogates and anything above 0x10FFFF, so
        // it is the whole range check.
        Ok(code_point) => match char::from_u32(code_point) {
            Some(ch) => Decoded::ok(Escaped::Scalar(ch as u32), raw),
            None => fail(UnicodeEscapeErrorReason::OutOfRange),
        },
        Err(_) => fail(UnicodeEscapeErrorReason::OutOfRange),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Decode a whole body, returning the scalars and the errors. Mirrors what
    /// every caller does, so the table is tested once for all three of them.
    fn decode_all(s: &str) -> (Vec<u32>, Vec<EscapeErrorKind>) {
        let mut scalars = Vec::new();
        let mut errors = Vec::new();
        let mut chars = s.char_indices().peekable();
        while let Some((_, c)) = chars.next() {
            if c != '\\' {
                scalars.push(c as u32);
                continue;
            }
            let decoded = decode_escape(&mut chars);
            match decoded.result {
                Ok(Escaped::Scalar(v)) => scalars.push(v),
                Ok(_) => {},
                Err(kind) => errors.push(kind),
            }
        }
        (scalars, errors)
    }

    #[test]
    fn simple_escapes() {
        let (v, e) = decode_all(r#"a\nb\tc\\d\0"#);
        assert!(e.is_empty());
        assert_eq!(v, vec![0x61, 0x0A, 0x62, 0x09, 0x63, 0x5C, 0x64, 0x00]);
    }

    #[test]
    fn ascii_escape_range_is_seven_bit() {
        assert_eq!(decode_all(r"\x41").0, vec![0x41]);
        assert!(matches!(
            decode_all(r"\x80").1.as_slice(),
            [EscapeErrorKind::AsciiEscapeOutOfRange { value: 0x80 }]
        ));
        // One digit is incomplete, not a one-digit escape.
        assert!(matches!(
            decode_all(r"\xA").1.as_slice(),
            [EscapeErrorKind::InvalidEscape { .. }]
        ));
    }

    /// The char-literal copy read hex with an unbounded loop and no digit
    /// limit, so this compiled to `A` there while the string decoder rejected
    /// it. Now there is one answer (F26).
    #[test]
    fn unicode_digit_limit_is_six() {
        assert_eq!(decode_all(r"\u{41}").0, vec![0x41]);
        assert_eq!(decode_all(r"\u{10FFFF}").0, vec![0x10FFFF]);
        assert!(matches!(
            decode_all(r"\u{00000041}").1.as_slice(),
            [EscapeErrorKind::InvalidUnicodeEscape {
                reason: UnicodeEscapeErrorReason::TooManyDigits,
                ..
            }]
        ));
    }

    #[test]
    fn surrogates_and_out_of_range_are_rejected() {
        assert!(matches!(
            decode_all(r"\u{D800}").1.as_slice(),
            [EscapeErrorKind::InvalidUnicodeEscape {
                reason: UnicodeEscapeErrorReason::OutOfRange,
                ..
            }]
        ));
        assert!(matches!(
            decode_all(r"\u{110000}").1.as_slice(),
            [EscapeErrorKind::InvalidUnicodeEscape {
                reason: UnicodeEscapeErrorReason::OutOfRange,
                ..
            }]
        ));
    }

    /// An unterminated `\u{` must stop at the literal's own delimiter instead
    /// of consuming it — the char copy's loop had no such guard.
    #[test]
    fn unclosed_unicode_stops_at_the_delimiter() {
        let (_, e) = decode_all("\\u{41\"rest");
        assert!(matches!(
            e.as_slice(),
            [EscapeErrorKind::InvalidUnicodeEscape {
                reason: UnicodeEscapeErrorReason::MissingCloseBrace,
                ..
            }]
        ));
    }

    #[test]
    fn empty_braces_and_bad_digits() {
        assert!(matches!(
            decode_all(r"\u{}").1.as_slice(),
            [EscapeErrorKind::InvalidUnicodeEscape {
                reason: UnicodeEscapeErrorReason::EmptyBraces,
                ..
            }]
        ));
        assert!(matches!(
            decode_all(r"\u{zz}").1.as_slice(),
            [EscapeErrorKind::InvalidUnicodeEscape {
                reason: UnicodeEscapeErrorReason::InvalidHexDigit,
                ..
            }]
        ));
        assert!(matches!(
            decode_all(r"\u41").1.as_slice(),
            [EscapeErrorKind::InvalidUnicodeEscape {
                reason: UnicodeEscapeErrorReason::MissingOpenBrace,
                ..
            }]
        ));
    }

    #[test]
    fn trailing_backslash_is_incomplete() {
        assert!(matches!(
            decode_all("abc\\").1.as_slice(),
            [EscapeErrorKind::IncompleteEscape]
        ));
    }

    #[test]
    fn unknown_escape_reports_and_does_not_consume_more() {
        let (v, e) = decode_all(r"\qz");
        assert!(matches!(e.as_slice(), [EscapeErrorKind::InvalidEscape { .. }]));
        // `z` is still decoded normally.
        assert_eq!(v, vec![0x7A]);
    }

    /// `raw` must be the exact source text, byte-for-byte — callers derive
    /// spans from `raw.len()` and push it back on error.
    #[test]
    fn raw_round_trips_the_source_text() {
        for src in [r"\n", r"\x41", r"\xA", r"\u{41}", r"\u{}", r"\u", r"\q"] {
            let mut chars = src.char_indices().peekable();
            chars.next(); // the backslash
            assert_eq!(decode_escape(&mut chars).raw, src, "raw mismatch for {src}");
        }
    }

    #[test]
    fn line_continuation_eats_following_indentation() {
        let mut chars = "\\\n    x".char_indices().peekable();
        chars.next();
        let d = decode_escape(&mut chars);
        assert_eq!(d.result, Ok(Escaped::LineContinuation));
        assert_eq!(d.raw, "\\\n    ");
        assert_eq!(chars.next().map(|(_, c)| c), Some('x'));
    }
}
