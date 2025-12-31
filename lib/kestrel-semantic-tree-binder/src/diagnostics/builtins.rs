//! Builtin-related errors.
//!
//! Errors related to the `@builtin(.Feature)` attribute system.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when @builtin is used without an argument.
pub struct BuiltinRequiresArgumentError {
    pub span: Span,
}

impl IntoDiagnostic for BuiltinRequiresArgumentError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("@builtin requires a language feature argument")
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message("expected @builtin(.Feature)")])
            .with_notes(vec!["example: @builtin(.Copyable)".to_string()])
    }
}

/// Error when @builtin argument is not implicit member syntax.
pub struct BuiltinInvalidArgumentError {
    pub span: Span,
}

impl IntoDiagnostic for BuiltinInvalidArgumentError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("@builtin expected implicit member syntax (.Feature)")
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message("expected implicit member syntax")])
            .with_notes(vec!["example: @builtin(.Copyable)".to_string()])
    }
}

/// Error when @builtin references an unknown language feature.
pub struct UnknownLanguageFeatureError {
    pub span: Span,
    pub name: String,
}

impl IntoDiagnostic for UnknownLanguageFeatureError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("unknown language feature '.{}'", self.name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range()).with_message("unknown feature")
            ])
    }
}

/// Error when @builtin is applied to the wrong symbol kind.
pub struct BuiltinWrongKindError {
    pub span: Span,
    pub feature_name: String,
    pub expected_kind: String,
    pub actual_kind: String,
}

impl IntoDiagnostic for BuiltinWrongKindError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "@builtin(.{}) can only be applied to a {}",
                self.feature_name, self.expected_kind
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message(format!("this is a {}", self.actual_kind))])
    }
}

/// Error when a marker-required builtin is applied to a non-marker protocol.
pub struct BuiltinMustBeMarkerError {
    pub span: Span,
    pub feature_name: String,
}

impl IntoDiagnostic for BuiltinMustBeMarkerError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "@builtin(.{}) must be a marker protocol (no required methods or types)",
                self.feature_name
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message("protocol has required members")])
    }
}

/// Error when a language feature is defined more than once.
pub struct DuplicateBuiltinError {
    pub span: Span,
    pub feature_name: String,
}

impl IntoDiagnostic for DuplicateBuiltinError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "language feature '.{}' is already defined by another symbol",
                self.feature_name
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message("duplicate builtin definition")])
    }
}
