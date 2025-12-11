//! Visibility consistency errors.
//!
//! Errors when a public item exposes a less-visible type.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a function's return type is less visible than the function.
pub struct ReturnTypeLessVisibleError {
    pub span: Span,
    pub function_name: String,
    pub function_visibility: String,
    pub return_type_visibility: String,
}

impl IntoDiagnostic for ReturnTypeLessVisibleError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "return type of '{}' is less visible than the function",
                self.function_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("return type is less visible than function")
            ])
            .with_notes(vec![
                format!(
                    "function is {} but return type is {}",
                    self.function_visibility, self.return_type_visibility
                )
            ])
    }
}

/// Error when a function's parameter type is less visible than the function.
pub struct ParameterTypeLessVisibleError {
    pub span: Span,
    pub function_name: String,
    pub function_visibility: String,
    pub param_type_visibility: String,
}

impl IntoDiagnostic for ParameterTypeLessVisibleError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "parameter type in '{}' is less visible than the function",
                self.function_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("parameter type is less visible than function")
            ])
            .with_notes(vec![
                format!(
                    "function is {} but parameter type is {}",
                    self.function_visibility, self.param_type_visibility
                )
            ])
    }
}

/// Error when a type alias's underlying type is less visible than the alias.
pub struct AliasedTypeLessVisibleError {
    pub span: Span,
    pub alias_name: String,
    pub alias_visibility: String,
    pub aliased_type_visibility: String,
}

impl IntoDiagnostic for AliasedTypeLessVisibleError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "aliased type in '{}' is less visible than the type alias",
                self.alias_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("aliased type is less visible than alias")
            ])
            .with_notes(vec![
                format!(
                    "type alias is {} but aliased type is {}",
                    self.alias_visibility, self.aliased_type_visibility
                )
            ])
    }
}

/// Error when a field's type is less visible than the field.
pub struct FieldTypeLessVisibleError {
    pub span: Span,
    pub field_name: String,
    pub field_visibility: String,
    pub field_type_visibility: String,
}

impl IntoDiagnostic for FieldTypeLessVisibleError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "field '{}' has type less visible than the field",
                self.field_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("field type is less visible than field")
            ])
            .with_notes(vec![
                format!(
                    "field is {} but field type is {}",
                    self.field_visibility, self.field_type_visibility
                )
            ])
    }
}
