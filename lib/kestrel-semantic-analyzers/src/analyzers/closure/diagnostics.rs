//! Closure-specific diagnostic errors.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error: `it` used but closure arity is not 1
pub struct ItUsedWithWrongArityError {
    pub span: Span,
    pub expected_arity: usize,
}

impl IntoDiagnostic for ItUsedWithWrongArityError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "`it` can only be used when closure has exactly 1 parameter, but {} expected",
                self.expected_arity
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range()).with_message("used here")
            ])
    }
}

/// Error: `it` used with explicit parameters
pub struct ItNotInScopeError {
    pub span: Span,
}

impl IntoDiagnostic for ItNotInScopeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("`it` is not in scope; closure has explicit parameters")
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())])
            .with_notes(vec!["use the explicit parameter name instead".to_string()])
    }
}

/// Error: Cannot assign to captured variable
pub struct CannotAssignToCapturedVariableError {
    pub span: Span,
    pub name: String,
}

impl IntoDiagnostic for CannotAssignToCapturedVariableError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "cannot assign to captured variable `{}`",
                self.name
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())])
            .with_notes(vec!["captures are by value and immutable".to_string()])
    }
}

/// Error: Cannot assign to closure parameter
pub struct CannotAssignToClosureParameterError {
    pub span: Span,
    pub name: String,
}

impl IntoDiagnostic for CannotAssignToClosureParameterError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "cannot assign to immutable parameter `{}`",
                self.name
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())])
    }
}

/// Error: Cannot infer closure parameter type
pub struct CannotInferClosureParameterTypeError {
    pub span: Span,
    pub name: String,
}

impl IntoDiagnostic for CannotInferClosureParameterTypeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "cannot infer type for closure parameter `{}`; add a type annotation",
                self.name
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())])
    }
}

/// Error: Closure arity mismatch
pub struct ClosureArityMismatchError {
    pub span: Span,
    pub actual: usize,
    pub expected: usize,
}

impl IntoDiagnostic for ClosureArityMismatchError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "closure has {} parameters but {} expected",
                self.actual, self.expected
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())])
    }
}

/// Error: Closure return type mismatch
pub struct ClosureReturnTypeMismatchError {
    pub span: Span,
    pub actual: String,
    pub expected: String,
}

impl IntoDiagnostic for ClosureReturnTypeMismatchError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "closure returns `{}` but `{}` expected",
                self.actual, self.expected
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())])
    }
}

/// Error: Closure parameter type mismatch
pub struct ClosureParamTypeMismatchError {
    pub span: Span,
    pub index: usize,
    pub actual: String,
    pub expected: String,
}

impl IntoDiagnostic for ClosureParamTypeMismatchError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "closure parameter {} has type `{}` but `{}` expected",
                self.index + 1,
                self.actual,
                self.expected
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())])
    }
}
