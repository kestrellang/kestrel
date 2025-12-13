//! Declaration errors.
//!
//! Errors related to binding declarations and signature-level conflicts.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when multiple functions have the same signature.
pub struct DuplicateFunctionSignatureError {
    pub signature: String,
    pub first_span: Span,
    pub duplicate_spans: Vec<Span>,
}

impl IntoDiagnostic for DuplicateFunctionSignatureError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let mut labels = vec![
            Label::secondary(self.first_span.file_id, self.first_span.range())
                .with_message("first defined here"),
        ];

        for span in &self.duplicate_spans {
            labels.push(
                Label::primary(span.file_id, span.range()).with_message("duplicate definition"),
            );
        }

        Diagnostic::error()
            .with_message(format!("duplicate function signature: {}", self.signature))
            .with_labels(labels)
    }
}

/// Error when a type alias requires a type but none was provided.
pub struct TypeAliasRequiresTypeError {
    pub span: Span,
    pub name: String,
    pub context: TypeAliasContext,
}

/// The context where type alias is used.
pub enum TypeAliasContext {
    /// Type alias at module level (requires `= Type`)
    ModuleLevel,
    /// Type alias in struct body without conformance (requires `= Type`)
    StructWithoutConformance,
}

impl IntoDiagnostic for TypeAliasRequiresTypeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let (main_msg, context_msg) = match self.context {
            TypeAliasContext::ModuleLevel => (
                format!("type alias requires a type: '{}'", self.name),
                "must specify a type",
            ),
            TypeAliasContext::StructWithoutConformance => (
                format!("associated type binding requires a type: '{}'", self.name),
                "must specify a type with = Type",
            ),
        };

        Diagnostic::error().with_message(main_msg).with_labels(vec![
            Label::primary(self.span.file_id, self.span.range()).with_message(context_msg),
        ])
    }
}

/// Error when associated type bounds are used at module level.
pub struct AssociatedTypeBoundsInWrongContextError {
    pub span: Span,
    pub name: String,
}

impl IntoDiagnostic for AssociatedTypeBoundsInWrongContextError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("type alias cannot have bounds: '{}'", self.name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("bounds are only allowed on associated types in protocols"),
            ])
    }
}
