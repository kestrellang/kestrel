use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

// Type checking errors (mirrors builder diagnostics for parity)

pub struct TypeMismatchError {
    pub span: Span,
    pub expected: String,
    pub found: String,
    pub context: String,
}

impl IntoDiagnostic for TypeMismatchError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "type mismatch: expected `{}`, found `{}`",
                self.expected, self.found
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message(format!("expected `{}`", self.expected))])
            .with_notes(vec![format!(
                "{}: expected `{}`, found `{}`",
                self.context, self.expected, self.found
            )])
    }
}

pub struct ConditionNotBoolError {
    pub span: Span,
    pub found: String,
    pub condition_kind: &'static str,
}

impl IntoDiagnostic for ConditionNotBoolError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "{} condition must be `Bool`, found `{}`",
                self.condition_kind, self.found
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message(format!("expected `Bool`, found `{}`", self.found))])
    }
}

pub struct BranchTypeMismatchError {
    pub if_span: Span,
    pub then_span: Span,
    pub else_span: Span,
    pub then_type: String,
    pub else_type: String,
}

impl IntoDiagnostic for BranchTypeMismatchError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "if/else branches have incompatible types: `{}` vs `{}`",
                self.then_type, self.else_type
            ))
            .with_labels(vec![
                Label::primary(self.then_span.file_id, self.then_span.range())
                    .with_message(format!("this has type `{}`", self.then_type)),
                Label::secondary(self.else_span.file_id, self.else_span.range())
                    .with_message(format!("this has type `{}`", self.else_type)),
            ])
            .with_notes(vec![
                "if/else branches must have the same type when used as an expression".to_string(),
            ])
    }
}

pub struct ArrayElementTypeMismatchError {
    pub array_span: Span,
    pub first_element_span: Span,
    pub element_span: Span,
    pub element_index: usize,
    pub expected: String,
    pub found: String,
}

impl IntoDiagnostic for ArrayElementTypeMismatchError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "array element type mismatch: expected `{}`, found `{}`",
                self.expected, self.found
            ))
            .with_labels(vec![
                Label::primary(self.element_span.file_id, self.element_span.range()).with_message(
                    format!("expected `{}`, found `{}`", self.expected, self.found),
                ),
                Label::secondary(
                    self.first_element_span.file_id,
                    self.first_element_span.range(),
                )
                .with_message(format!("first element has type `{}`", self.expected)),
            ])
            .with_notes(vec![format!(
                "element at index {} has type `{}`, but array elements must all be `{}`",
                self.element_index, self.found, self.expected
            )])
    }
}
