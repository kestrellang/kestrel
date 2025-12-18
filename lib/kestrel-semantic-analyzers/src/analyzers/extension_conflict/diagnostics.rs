use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error for duplicate method definitions across extensions of same specificity
#[derive(Debug, Clone)]
pub struct DuplicateExtensionMethodError {
    pub method_name: String,
    pub locations: Vec<Span>,
}

impl IntoDiagnostic for DuplicateExtensionMethodError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let labels: Vec<Label<usize>> = self
            .locations
            .iter()
            .enumerate()
            .map(|(i, span)| {
                let msg = if i == 0 {
                    "first definition here"
                } else {
                    "conflicting definition here"
                };
                if i == 0 {
                    Label::primary(span.file_id, span.range()).with_message(msg)
                } else {
                    Label::secondary(span.file_id, span.range()).with_message(msg)
                }
            })
            .collect();

        Diagnostic::error()
            .with_message(format!(
                "duplicate method '{}' in overlapping extensions",
                self.method_name
            ))
            .with_labels(labels)
            .with_notes(vec![
                "Extensions that overlap must not define methods with the same name unless one is strictly more specific than the other".to_string(),
                "For example, Box[Int] can override Box[T], but Box[Int, T] and Box[T, Int] conflict for Box[Int, Int]".to_string(),
            ])
    }
}

/// Error for extension method conflicting with struct method
#[derive(Debug, Clone)]
pub struct StructExtensionMethodConflictError {
    pub method_name: String,
    pub struct_method_span: Span,
    pub extension_method_span: Span,
}

impl IntoDiagnostic for StructExtensionMethodConflictError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "duplicate method '{}': extension cannot redefine struct method",
                self.method_name
            ))
            .with_labels(vec![
                Label::primary(self.struct_method_span.file_id, self.struct_method_span.range())
                    .with_message("method defined here on struct"),
                Label::secondary(
                    self.extension_method_span.file_id,
                    self.extension_method_span.range(),
                )
                    .with_message("conflicting extension method here"),
            ])
            .with_notes(vec![
                "Extensions cannot define methods that already exist on the struct".to_string(),
                "Consider renaming the extension method or removing it".to_string(),
            ])
    }
}
