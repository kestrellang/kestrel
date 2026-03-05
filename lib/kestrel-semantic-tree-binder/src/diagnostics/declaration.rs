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
    /// Type alias in struct/enum body without conformance (requires `= Type`)
    ConcreteTypeWithoutConformance,
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
            TypeAliasContext::ConcreteTypeWithoutConformance => (
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

/// Error when a required parameter appears after a parameter with a default value.
pub struct RequiredParameterAfterDefaultError {
    pub required_name: String,
    pub required_span: Span,
    pub default_param_name: String,
    pub default_param_span: Span,
}

impl IntoDiagnostic for RequiredParameterAfterDefaultError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "required parameter '{}' cannot follow parameter '{}' which has a default value",
                self.required_name, self.default_param_name
            ))
            .with_labels(vec![
                Label::secondary(
                    self.default_param_span.file_id,
                    self.default_param_span.range(),
                )
                .with_message("parameter with default value"),
                Label::primary(self.required_span.file_id, self.required_span.range())
                    .with_message("required parameter cannot come after default parameter"),
            ])
    }
}

/// Error when a default value expression has the wrong type.
pub struct DefaultValueTypeMismatchError {
    pub param_name: String,
    pub expected_type: String,
    pub actual_type: String,
    pub default_span: Span,
    pub param_type_span: Span,
}

impl IntoDiagnostic for DefaultValueTypeMismatchError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "cannot use value of type '{}' as default for parameter '{}' of type '{}'",
                self.actual_type, self.param_name, self.expected_type
            ))
            .with_labels(vec![
                Label::primary(self.default_span.file_id, self.default_span.range()).with_message(
                    format!(
                        "expected '{}', found '{}'",
                        self.expected_type, self.actual_type
                    ),
                ),
                Label::secondary(self.param_type_span.file_id, self.param_type_span.range())
                    .with_message("parameter type declared here"),
            ])
    }
}

/// Error when a default value expression references another parameter.
pub struct DefaultValueReferencesParameterError {
    pub param_name: String,
    pub referenced_param: String,
    pub reference_span: Span,
}

impl IntoDiagnostic for DefaultValueReferencesParameterError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "default value for '{}' cannot reference parameter '{}'",
                self.param_name, self.referenced_param
            ))
            .with_labels(vec![
                Label::primary(self.reference_span.file_id, self.reference_span.range())
                    .with_message("cannot reference other parameters in default values"),
            ])
    }
}
