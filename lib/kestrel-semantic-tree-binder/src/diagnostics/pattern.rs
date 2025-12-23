//! Pattern matching errors.
//!
//! Errors related to pattern matching constructs.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when or-pattern alternatives have inconsistent bindings
pub struct InconsistentOrPatternBindingsError {
    pub span: Span,
    pub alternative_index: usize,
}

impl IntoDiagnostic for InconsistentOrPatternBindingsError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("inconsistent bindings in or-pattern")
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message(format!(
                    "alternative {} binds different names than alternative 1",
                    self.alternative_index
                ))])
            .with_notes(vec![
                "all alternatives in an or-pattern must bind the same set of names".to_string(),
            ])
    }
}

/// Error when a struct pattern references an unknown field
pub struct UnknownStructPatternFieldError {
    pub span: Span,
    pub field_name: String,
    pub struct_name: String,
}

impl IntoDiagnostic for UnknownStructPatternFieldError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "struct `{}` has no field `{}`",
                self.struct_name, self.field_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("unknown field"),
            ])
    }
}

/// Error when a struct pattern is missing required fields
pub struct MissingStructPatternFieldsError {
    pub span: Span,
    pub struct_name: String,
    pub missing_fields: Vec<String>,
}

impl IntoDiagnostic for MissingStructPatternFieldsError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let missing = self.missing_fields.join(", ");
        Diagnostic::error()
            .with_message(format!(
                "pattern does not mention fields {} of `{}`",
                missing, self.struct_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("missing fields in pattern"),
            ])
            .with_notes(vec![
                "use `..` to ignore the remaining fields".to_string(),
            ])
    }
}

/// Error when a struct pattern is used with a non-struct type
pub struct NotAStructPatternError {
    pub span: Span,
    pub name: String,
}

impl IntoDiagnostic for NotAStructPatternError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("`{}` is not a struct", self.name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("not a struct"),
            ])
    }
}

/// Error when range pattern has invalid bounds (start > end)
pub struct InvalidRangeBoundsError {
    pub span: Span,
    pub inclusive: bool,
}

impl IntoDiagnostic for InvalidRangeBoundsError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let op = if self.inclusive { "..=" } else { "..<" };
        Diagnostic::error()
            .with_message("invalid range bounds: start must be less than or equal to end")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message(format!("range `{op}` has invalid bounds")),
            ])
            .with_notes(vec![
                format!("the lower bound must be less than{} the upper bound", 
                    if self.inclusive { " or equal to" } else { "" }),
            ])
    }
}

/// Error when a binding name appears multiple times in the same pattern
pub struct DuplicateBindingInPatternError {
    pub span: Span,
    pub name: String,
    pub first_span: Span,
}

impl IntoDiagnostic for DuplicateBindingInPatternError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("duplicate binding `{}` in pattern", self.name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("bound again here"),
                Label::secondary(self.first_span.file_id, self.first_span.range())
                    .with_message("first binding"),
            ])
            .with_notes(vec![
                "each binding in a pattern must have a unique name".to_string(),
            ])
    }
}

/// Error when a float literal is used in a pattern
pub struct FloatLiteralInPatternError {
    pub span: Span,
}

impl IntoDiagnostic for FloatLiteralInPatternError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("float literals are not allowed in patterns")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("cannot use float literal in pattern"),
            ])
            .with_notes(vec![
                "patterns can only match exact values, but floating point comparison is imprecise".to_string(),
            ])
    }
}

/// Error when an enum pattern uses an unknown case name
pub struct UnknownEnumCaseError {
    pub span: Span,
    pub case_name: String,
    pub enum_name: String,
}

impl IntoDiagnostic for UnknownEnumCaseError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("enum `{}` has no case `{}`", self.enum_name, self.case_name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("unknown case"),
            ])
    }
}

/// Error when a tuple pattern has the wrong number of elements
pub struct TuplePatternArityMismatchError {
    pub span: Span,
    pub expected: usize,
    pub found: usize,
}

impl IntoDiagnostic for TuplePatternArityMismatchError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "tuple pattern has wrong arity: expected {} elements, found {}",
                self.expected, self.found
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message(format!("expected {} elements", self.expected)),
            ])
    }
}
