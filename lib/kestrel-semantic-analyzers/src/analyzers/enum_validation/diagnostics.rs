//! Enum validation diagnostics.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when an enum has two cases with the same name.
///
/// Example:
/// ```ignore
/// enum Color {
///     case Red
///     case Red  // Error: duplicate case
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateCaseError {
    /// The name of the duplicate case
    pub case_name: String,
    /// Span of the first case definition
    pub first_span: Span,
    /// Span of the duplicate case definition
    pub duplicate_span: Span,
}

impl IntoDiagnostic for DuplicateCaseError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("duplicate enum case '{}'", self.case_name))
            .with_labels(vec![
                Label::primary(self.duplicate_span.file_id, self.duplicate_span.range())
                    .with_message("duplicate case defined here"),
                Label::secondary(self.first_span.file_id, self.first_span.range())
                    .with_message("first defined here"),
            ])
    }
}

/// Error when a case has duplicate parameter labels.
///
/// Example:
/// ```ignore
/// enum Bad {
///     case Foo(x: Int, x: String)  // Error: duplicate label 'x'
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateLabelError {
    /// The name of the duplicate label
    pub label_name: String,
    /// The name of the case containing the duplicate
    pub case_name: String,
    /// Span of the first label definition
    pub first_span: Span,
    /// Span of the duplicate label definition
    pub duplicate_span: Span,
}

impl IntoDiagnostic for DuplicateLabelError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "duplicate label '{}' in case '{}'",
                self.label_name, self.case_name
            ))
            .with_labels(vec![
                Label::primary(self.duplicate_span.file_id, self.duplicate_span.range())
                    .with_message("duplicate label"),
                Label::secondary(self.first_span.file_id, self.first_span.range())
                    .with_message("first defined here"),
            ])
    }
}

/// Error when an enum is recursive but not marked as `indirect`.
///
/// Recursive enums need the `indirect` keyword because they contain themselves,
/// which would have infinite size without boxing.
///
/// Example:
/// ```ignore
/// enum Tree {
///     case Leaf(value: Int)
///     case Node(left: Tree, right: Tree)  // Error: recursive without indirect
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecursiveEnumError {
    /// The name of the enum
    pub enum_name: String,
    /// Span of the enum declaration
    pub enum_span: Span,
    /// The name of the case with the recursive reference
    pub case_name: String,
    /// Span of the parameter type containing the recursive reference
    pub param_span: Span,
}

impl IntoDiagnostic for RecursiveEnumError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("recursive enum requires `indirect`")
            .with_labels(vec![
                Label::primary(self.param_span.file_id, self.param_span.range())
                    .with_message(format!("'{}' contains itself", self.enum_name)),
                Label::secondary(self.enum_span.file_id, self.enum_span.range())
                    .with_message(format!("enum '{}' declared here", self.enum_name)),
            ])
            .with_notes(vec![
                "add `indirect` before `enum` to allow recursive types".to_string(),
            ])
    }
}
