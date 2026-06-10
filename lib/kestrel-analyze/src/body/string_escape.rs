//! # String Escape Analyzer
//!
//! Surfaces escape-sequence errors collected during HIR lowering. The decoder
//! in `kestrel-hir-lower::literal` records each malformed escape on the HIR
//! literal node as data; this analyzer translates that data into diagnostics
//! so emission lives in the analyzer framework (memoized per body) rather
//! than as a side-effect of lowering.
//!
//! ## Diagnostics
//!
//! ### E700 — `invalid_escape_sequence` (Error, Correctness)
//!
//! **Message:** "invalid escape sequence `{seq}`"
//!
//! **Labels:**
//! - Primary: the offending backslash escape
//!   - Span source: `EscapeError.span` from the `HirLiteral::String` on the
//!     containing `HirExpr::Literal` or `HirPat::Literal`
//!   - Message: "unknown escape sequence"
//!
//! **Notes:** "valid escape sequences are: \\n, \\r, \\t, \\\\, \\\", \\', \\0, \\xNN, \\u{NNNN}"
//!
//! ### E701 — `ascii_escape_out_of_range` (Error, Correctness)
//!
//! **Message:** "ASCII escape `\\x{NN}` is out of range"
//!
//! **Labels:**
//! - Primary: the `\xNN` escape
//!   - Span source: `EscapeError.span`
//!   - Message: "value must be in range 0x00-0x7F"
//!
//! **Notes:** "ASCII escapes (\\xNN) can only represent 7-bit values (0x00-0x7F)",
//!            "use a Unicode escape (\\u{NN}) for values above 0x7F"
//!
//! ### E702 — `invalid_unicode_escape` (Error, Correctness)
//!
//! **Message:** "invalid Unicode escape `{value}`"
//!
//! **Labels:**
//! - Primary: the `\u{...}` escape
//!   - Span source: `EscapeError.span`
//!   - Message: depends on `UnicodeEscapeErrorReason`
//!
//! **Notes:** "Unicode escapes use the format \\u{NNNN} with 1-6 hex digits",
//!            "valid range is \\u{0} to \\u{10FFFF}"
//!
//! ### E703 — `incomplete_escape_sequence` (Error, Correctness)
//!
//! **Message:** "incomplete escape sequence at end of string"
//!
//! **Labels:**
//! - Primary: the trailing `\`
//!   - Span source: `EscapeError.span`
//!   - Message: "escape sequence is incomplete"
//!
//! **Notes:** (none)
//!
//! ### E704 — `multiline_string_under_indented` (Error, Correctness)
//!
//! Content line of a multi-line string is less indented than the closing
//! `"""` delimiter. The closing delimiter's column defines the strip prefix.
//!
//! ### E705 — `multiline_string_missing_leading_newline` (Error, Correctness)
//!
//! Multi-line string opener `"""` must be followed immediately by a newline.
//!
//! ### E706 — `multiline_string_missing_trailing_newline` (Error, Correctness)
//!
//! Multi-line string closer `"""` must be on its own line, with only
//! whitespace before it.
//!
//! ### E707 — `unterminated_string` (Error, Correctness)
//!
//! String literal has no closing delimiter (single- or multi-line).

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, BodyCheck, Describe};
use kestrel_hir::body::{
    EscapeError, EscapeErrorKind, HirExpr, HirLiteral, HirPat, UnicodeEscapeErrorReason,
};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E700",
        name: "invalid_escape_sequence",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E701",
        name: "ascii_escape_out_of_range",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E702",
        name: "invalid_unicode_escape",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E703",
        name: "incomplete_escape_sequence",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E704",
        name: "multiline_string_under_indented",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E705",
        name: "multiline_string_missing_leading_newline",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E706",
        name: "multiline_string_missing_trailing_newline",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E707",
        name: "unterminated_string",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct StringEscapeAnalyzer;

impl Describe for StringEscapeAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::StringEscape
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for StringEscapeAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut out = Vec::new();

        for (_, expr) in cx.hir.exprs.iter() {
            if let HirExpr::Literal {
                value: HirLiteral::String { escape_errors, .. },
                ..
            } = expr
            {
                for err in escape_errors {
                    out.push(diagnose(err));
                }
            }
        }

        for (_, pat) in cx.hir.pats.iter() {
            if let HirPat::Literal {
                value: HirLiteral::String { escape_errors, .. },
                ..
            } = pat
            {
                for err in escape_errors {
                    out.push(diagnose(err));
                }
            }
        }

        out
    }
}

fn diagnose(err: &EscapeError) -> AnalyzeDiagnostic {
    match &err.kind {
        EscapeErrorKind::InvalidEscape { sequence } => AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: format!("invalid escape sequence `{}`", sequence),
            labels: vec![DiagLabel {
                span: err.span.clone(),
                message: "unknown escape sequence".into(),
                is_primary: true,
            }],
            notes: vec![
                "valid escape sequences are: \\n, \\r, \\t, \\\\, \\\", \\', \\0, \\xNN, \\u{NNNN}"
                    .into(),
            ],
        },
        EscapeErrorKind::AsciiEscapeOutOfRange { value } => AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[1].id,
            severity: DESCRIPTORS[1].default_severity,
            message: format!("ASCII escape `\\x{:02X}` is out of range", value),
            labels: vec![DiagLabel {
                span: err.span.clone(),
                message: "value must be in range 0x00-0x7F".into(),
                is_primary: true,
            }],
            notes: vec![
                "ASCII escapes (\\xNN) can only represent 7-bit values (0x00-0x7F)".into(),
                "use a Unicode escape (\\u{NN}) for values above 0x7F".into(),
            ],
        },
        EscapeErrorKind::InvalidUnicodeEscape { value, reason } => {
            let label_msg = match reason {
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
                UnicodeEscapeErrorReason::OutOfRange => {
                    format!("Unicode escape `{}` is out of range (max 0x10FFFF)", value)
                },
            };
            AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[2].id,
                severity: DESCRIPTORS[2].default_severity,
                message: format!("invalid Unicode escape `{}`", value),
                labels: vec![DiagLabel {
                    span: err.span.clone(),
                    message: label_msg,
                    is_primary: true,
                }],
                notes: vec![
                    "Unicode escapes use the format \\u{NNNN} with 1-6 hex digits".into(),
                    "valid range is \\u{0} to \\u{10FFFF}".into(),
                ],
            }
        },
        EscapeErrorKind::IncompleteEscape => AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[3].id,
            severity: DESCRIPTORS[3].default_severity,
            message: "incomplete escape sequence at end of string".into(),
            labels: vec![DiagLabel {
                span: err.span.clone(),
                message: "escape sequence is incomplete".into(),
                is_primary: true,
            }],
            notes: vec![],
        },
        EscapeErrorKind::MultilineUnderIndented => AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[4].id,
            severity: DESCRIPTORS[4].default_severity,
            message: "multi-line string content less indented than closing delimiter".into(),
            labels: vec![DiagLabel {
                span: err.span.clone(),
                message: "this line must start with at least the closing `\"\"\"`'s indentation"
                    .into(),
                is_primary: true,
            }],
            notes: vec![
                "the indentation of the closing `\"\"\"` defines the strip prefix; every content line must begin with that prefix"
                    .into(),
            ],
        },
        EscapeErrorKind::MultilineMissingLeadingNewline => AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[5].id,
            severity: DESCRIPTORS[5].default_severity,
            message: "multi-line string opener `\"\"\"` must be followed by a newline".into(),
            labels: vec![DiagLabel {
                span: err.span.clone(),
                message: "expected newline immediately after `\"\"\"`".into(),
                is_primary: true,
            }],
            notes: vec![
                "use a single-line string `\"...\"` for short content, or move the body onto a new line"
                    .into(),
            ],
        },
        EscapeErrorKind::MultilineMissingTrailingNewline => AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[6].id,
            severity: DESCRIPTORS[6].default_severity,
            message: "multi-line string closer `\"\"\"` must be on its own line".into(),
            labels: vec![DiagLabel {
                span: err.span.clone(),
                message: "only whitespace allowed before the closing `\"\"\"`".into(),
                is_primary: true,
            }],
            notes: vec![],
        },
        EscapeErrorKind::UnterminatedString => AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[7].id,
            severity: DESCRIPTORS[7].default_severity,
            message: "unterminated string literal".into(),
            labels: vec![DiagLabel {
                span: err.span.clone(),
                message: "no closing delimiter found".into(),
                is_primary: true,
            }],
            notes: vec![],
        },
    }
}
