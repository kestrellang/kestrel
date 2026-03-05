use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

// Re-export diagnostics that were moved from the binder
pub use kestrel_semantic_tree_binder::diagnostics::{
    AmbiguousAssociatedTypeError, AssociatedTypeBoundsInWrongContextError,
    AssociatedTypeConstraintNotSatisfiedError, QualifiedBindingNotConformingError,
    QualifiedBindingWrongProtocolError, TypeAliasContext, TypeAliasRequiresTypeError,
};

/// Error when a non-protocol type is used as an associated type bound.
pub struct NotAProtocolBoundError {
    pub span: Span,
    pub name: String,
}

impl IntoDiagnostic for NotAProtocolBoundError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "'{}' is not a protocol; bound must be a protocol",
                self.name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("cannot be used as a type bound"),
            ])
    }
}
