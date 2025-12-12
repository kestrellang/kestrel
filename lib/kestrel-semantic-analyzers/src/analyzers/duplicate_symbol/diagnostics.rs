use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a symbol is defined multiple times with the same kind.
pub struct DuplicateSymbolError {
    pub name: String,
    pub kind: String,
    pub original_span: Span,
    pub duplicate_span: Span,
}

impl IntoDiagnostic for DuplicateSymbolError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("duplicate definition of {} '{}'", self.kind, self.name))
            .with_labels(vec![
                Label::primary(self.duplicate_span.file_id, self.duplicate_span.range())
                    .with_message(format!("{} defined here", self.kind)),
                Label::secondary(self.original_span.file_id, self.original_span.range())
                    .with_message(format!("first defined as {} here", self.kind)),
            ])
    }
}

/// Error when a symbol is defined multiple times with different kinds.
pub struct DuplicateSymbolDifferentKindError {
    pub name: String,
    pub new_kind: String,
    pub original_kind: String,
    pub original_span: Span,
    pub duplicate_span: Span,
}

impl IntoDiagnostic for DuplicateSymbolDifferentKindError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("'{}' is already defined as a {}", self.name, self.original_kind))
            .with_labels(vec![
                Label::primary(self.duplicate_span.file_id, self.duplicate_span.range())
                    .with_message(format!("{} defined here", self.new_kind)),
                Label::secondary(self.original_span.file_id, self.original_span.range())
                    .with_message(format!("first defined as {} here", self.original_kind)),
            ])
    }
}

