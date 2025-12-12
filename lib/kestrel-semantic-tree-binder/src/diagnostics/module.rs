//! Module declaration errors.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a file has no module declaration.
pub struct NoModuleDeclarationError {
    pub span: Span,
}

impl IntoDiagnostic for NoModuleDeclarationError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("no module declaration found in file")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("module declaration should appear here"),
            ])
            .with_notes(vec![
                "Every Kestrel file must start with a module declaration.".to_string(),
                "Example: module MyModule".to_string(),
            ])
    }
}

/// Error when module declaration is not the first statement.
pub struct ModuleNotFirstError {
    pub module_span: Span,
    pub first_item_span: Span,
    pub first_item_kind: String,
}

impl IntoDiagnostic for ModuleNotFirstError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("module declaration must be the first statement in the file")
            .with_labels(vec![
                Label::secondary(self.first_item_span.file_id, self.first_item_span.range())
                    .with_message(format!(
                        "{} appears before module declaration",
                        self.first_item_kind
                    )),
                Label::primary(self.module_span.file_id, self.module_span.range())
                    .with_message("module declaration should be first"),
            ])
            .with_notes(vec![
                "The module declaration must come before any imports or declarations.".to_string(),
            ])
    }
}

/// Error when a file has multiple module declarations.
pub struct MultipleModuleDeclarationsError {
    pub first_span: Span,
    pub duplicate_spans: Vec<Span>,
    pub count: usize,
}

impl IntoDiagnostic for MultipleModuleDeclarationsError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let mut labels = vec![
            Label::primary(self.first_span.file_id, self.first_span.range())
                .with_message("first module declaration here"),
        ];

        for (i, span) in self.duplicate_spans.iter().enumerate() {
            labels.push(
                Label::secondary(span.file_id, span.range())
                    .with_message(format!("duplicate module declaration #{}", i + 2)),
            );
        }

        Diagnostic::error()
            .with_message(format!(
                "multiple module declarations found ({} total)",
                self.count
            ))
            .with_labels(labels)
            .with_notes(vec![
                "Only one module declaration is allowed per file.".to_string(),
            ])
    }
}
