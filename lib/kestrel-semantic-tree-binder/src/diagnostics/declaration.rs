//! Declaration errors.
//!
//! Errors related to duplicate symbols, missing function bodies, and static context.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a symbol is defined multiple times with the same kind.
pub struct DuplicateSymbolError {
    pub name: String,
    pub kind: String,
    pub original_span: Span,
    pub original_file_id: usize,
    pub duplicate_span: Span,
}

impl IntoDiagnostic for DuplicateSymbolError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "duplicate definition of {} '{}'",
                self.kind, self.name
            ))
            .with_labels(vec![
                Label::primary(self.duplicate_span.file_id, self.duplicate_span.range())
                    .with_message(format!("{} defined here", self.kind)),
                Label::secondary(self.original_file_id, self.original_span.range())
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
    pub original_file_id: usize,
    pub duplicate_span: Span,
}

impl IntoDiagnostic for DuplicateSymbolDifferentKindError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "'{}' is already defined as a {}",
                self.name, self.original_kind
            ))
            .with_labels(vec![
                Label::primary(self.duplicate_span.file_id, self.duplicate_span.range())
                    .with_message(format!("{} defined here", self.new_kind)),
                Label::secondary(self.original_file_id, self.original_span.range())
                    .with_message(format!("first defined as {} here", self.original_kind)),
            ])
    }
}

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

/// Error when a function is missing a body (and it's required).
pub struct FunctionMissingBodyError {
    pub span: Span,
    pub function_name: String,
}

impl IntoDiagnostic for FunctionMissingBodyError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("function '{}' requires a body", self.function_name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("function declared without body"),
            ])
    }
}

/// Error when static modifier is used in wrong context.
pub struct StaticInWrongContextError {
    pub span: Span,
    pub name: String,
    pub context: StaticContext,
}

/// The invalid context where static was used.
pub enum StaticContext {
    /// Static used at module level
    ModuleLevel,
}

impl IntoDiagnostic for StaticInWrongContextError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let context_msg = match self.context {
            StaticContext::ModuleLevel => "static is not allowed at module level",
        };

        Diagnostic::error()
            .with_message(format!("'{}' cannot be static in this context", self.name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range()).with_message(context_msg),
            ])
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
