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
    /// Type alias in extension body without conformance (requires `= Type`)
    ExtensionWithoutConformance,
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
            TypeAliasContext::ExtensionWithoutConformance => (
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

/// Error when a generic parameter is declared multiple times in the same parameter list.
pub struct DuplicateTypeParameterError {
    pub name: String,
    pub first_span: Span,
    pub duplicate_span: Span,
}

impl IntoDiagnostic for DuplicateTypeParameterError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("duplicate type parameter: '{}'", self.name))
            .with_labels(vec![
                Label::secondary(self.first_span.file_id, self.first_span.range())
                    .with_message("first defined here"),
                Label::primary(self.duplicate_span.file_id, self.duplicate_span.range())
                    .with_message("duplicate definition"),
            ])
    }
}

/// Error when a type parameter shadows one from an outer scope.
pub struct ShadowedTypeParameterError {
    pub name: String,
    pub outer_span: Span,
    pub inner_span: Span,
}

impl IntoDiagnostic for ShadowedTypeParameterError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "type parameter '{}' shadows one from outer scope",
                self.name
            ))
            .with_labels(vec![
                Label::secondary(self.outer_span.file_id, self.outer_span.range())
                    .with_message("outer type parameter defined here"),
                Label::primary(self.inner_span.file_id, self.inner_span.range())
                    .with_message("shadows outer type parameter"),
            ])
    }
}
