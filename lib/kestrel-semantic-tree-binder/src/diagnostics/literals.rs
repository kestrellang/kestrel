//! Literal value errors.
//!
//! Errors related to string escape sequences and literal parsing.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error for an integer literal that exceeds the u64 range.
pub struct IntegerLiteralOverflowError {
    pub span: Span,
    pub literal: String,
}

impl IntoDiagnostic for IntegerLiteralOverflowError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("integer literal is out of range")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("this value does not fit in `u64`"),
            ])
            .with_notes(vec![
                format!("literal: `{}`", self.literal),
                "maximum supported integer literal is 18446744073709551615".to_string(),
            ])
    }
}

/// Error for an invalid escape sequence in a string literal
pub struct InvalidEscapeSequenceError {
    pub span: Span,
    pub sequence: String,
}

impl IntoDiagnostic for InvalidEscapeSequenceError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("invalid escape sequence `{}`", self.sequence))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("unknown escape sequence"),
            ])
            .with_notes(vec![
                "valid escape sequences are: \\n, \\r, \\t, \\\\, \\\", \\', \\0, \\xNN, \\u{NNNN}"
                    .to_string(),
            ])
    }
}

/// Error for an ASCII escape (\xNN) that is out of the valid range (0x00-0x7F)
pub struct AsciiEscapeOutOfRangeError {
    pub span: Span,
    pub value: u8,
}

impl IntoDiagnostic for AsciiEscapeOutOfRangeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "ASCII escape `\\x{:02X}` is out of range",
                self.value
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("value must be in range 0x00-0x7F"),
            ])
            .with_notes(vec![
                "ASCII escapes (\\xNN) can only represent 7-bit values (0x00-0x7F)".to_string(),
                "use a Unicode escape (\\u{NN}) for values above 0x7F".to_string(),
            ])
    }
}

/// Error for an invalid Unicode escape sequence
pub struct InvalidUnicodeEscapeError {
    pub span: Span,
    pub value: String,
    pub reason: UnicodeEscapeErrorReason,
}

/// Reason for an invalid Unicode escape
pub enum UnicodeEscapeErrorReason {
    /// Missing opening brace after \u
    MissingOpenBrace,
    /// Missing closing brace
    MissingCloseBrace,
    /// Empty braces: \u{}
    EmptyBraces,
    /// Too many hex digits (max 6)
    TooManyDigits,
    /// Invalid hex digit
    InvalidHexDigit,
    /// Value exceeds maximum Unicode code point (0x10FFFF)
    OutOfRange,
}

impl IntoDiagnostic for InvalidUnicodeEscapeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let message = match self.reason {
            UnicodeEscapeErrorReason::MissingOpenBrace => {
                "Unicode escape must be followed by `{`".to_string()
            },
            UnicodeEscapeErrorReason::MissingCloseBrace => {
                "Unicode escape missing closing `}`".to_string()
            },
            UnicodeEscapeErrorReason::EmptyBraces => {
                "Unicode escape cannot have empty braces".to_string()
            },
            UnicodeEscapeErrorReason::TooManyDigits => {
                "Unicode escape can have at most 6 hex digits".to_string()
            },
            UnicodeEscapeErrorReason::InvalidHexDigit => {
                "Unicode escape contains invalid hex digit".to_string()
            },
            UnicodeEscapeErrorReason::OutOfRange => format!(
                "Unicode escape `{}` is out of range (max 0x10FFFF)",
                self.value
            ),
        };

        Diagnostic::error()
            .with_message(format!("invalid Unicode escape `{}`", self.value))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range()).with_message(message),
            ])
            .with_notes(vec![
                "Unicode escapes use the format \\u{NNNN} with 1-6 hex digits".to_string(),
                "valid range is \\u{0} to \\u{10FFFF}".to_string(),
            ])
    }
}

/// Error for an incomplete escape sequence at end of string
pub struct IncompleteEscapeSequenceError {
    pub span: Span,
}

impl IntoDiagnostic for IncompleteEscapeSequenceError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("incomplete escape sequence at end of string")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("escape sequence is incomplete"),
            ])
    }
}

/// Error for an empty character literal ''
pub struct EmptyCharacterLiteralError {
    pub span: Span,
}

impl IntoDiagnostic for EmptyCharacterLiteralError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("empty character literal")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("character literal must contain exactly one codepoint"),
            ])
    }
}

/// Error for a character literal containing multiple codepoints
pub struct MultipleCodepointsInCharLiteralError {
    pub span: Span,
    pub count: usize,
}

impl IntoDiagnostic for MultipleCodepointsInCharLiteralError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("character literal may only contain one codepoint")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message(format!("found {} codepoints", self.count)),
            ])
            .with_notes(vec![
                "use a string literal for multiple characters".to_string(),
            ])
    }
}
