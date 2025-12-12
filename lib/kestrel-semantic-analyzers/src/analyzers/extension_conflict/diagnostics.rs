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
        let mut labels: Vec<Label<usize>> = self
            .locations
            .iter()
            .enumerate()
            .map(|(i, span)| {
                let msg = if i == 0 {
                    "first definition here"
                } else {
                    "conflicting definition here"
                };
                // Preserve existing behavior that didn't thread file_id (legacy uses 0)
                Label::primary(0, span.range()).with_message(msg)
            })
            .collect();

        // Make only the first label primary, rest secondary
        for label in labels.iter_mut().skip(1) {
            *label = Label::secondary(0, label.range.clone()).with_message(label.message.clone());
        }

        Diagnostic::error()
            .with_message(format!(
                "duplicate method '{}' in extensions with the same specificity",
                self.method_name
            ))
            .with_labels(labels)
            .with_notes(vec![
                "Extensions at the same specificity level cannot define methods with the same name".to_string(),
                "Extensions with different specificity (e.g., Box[T] vs Box[Int]) can have methods with the same name - the more specific one will be preferred".to_string(),
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
                // Preserve legacy behavior using file_id 0
                Label::primary(0, self.struct_method_span.range())
                    .with_message("method defined here on struct"),
                Label::secondary(0, self.extension_method_span.range())
                    .with_message("conflicting extension method here"),
            ])
            .with_notes(vec![
                "Extensions cannot define methods that already exist on the struct".to_string(),
                "Consider renaming the extension method or removing it".to_string(),
            ])
    }
}
