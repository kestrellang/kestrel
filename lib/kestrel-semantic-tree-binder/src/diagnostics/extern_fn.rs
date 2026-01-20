//! Extern function diagnostics.
//!
//! Errors related to `@extern` attribute validation.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when @extern is missing a calling convention argument.
pub struct ExternRequiresCallingConventionError {
    pub span: Span,
}

impl IntoDiagnostic for ExternRequiresCallingConventionError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("@extern requires a calling convention")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("expected @extern(.C)"),
            ])
    }
}

/// Error when @extern has an invalid calling convention argument.
pub struct ExternInvalidCallingConventionError {
    pub span: Span,
}

impl IntoDiagnostic for ExternInvalidCallingConventionError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("invalid calling convention argument")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("expected implicit member like .C"),
            ])
    }
}

/// Error when @extern has an unknown calling convention.
pub struct ExternUnknownCallingConventionError {
    pub span: Span,
    pub name: String,
}

impl IntoDiagnostic for ExternUnknownCallingConventionError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("unknown calling convention '{}'", self.name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range()).with_message("expected .C"),
            ])
    }
}

/// Error when @extern is applied to a generic function.
pub struct ExternFunctionCannotBeGenericError {
    pub span: Span,
}

impl IntoDiagnostic for ExternFunctionCannotBeGenericError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("@extern functions cannot be generic")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("extern functions must have concrete types"),
            ])
            .with_notes(vec![
                "generic functions cannot have a stable ABI".to_string(),
            ])
    }
}

/// Error when @extern function has a body.
pub struct ExternFunctionCannotHaveBodyError {
    pub span: Span,
}

impl IntoDiagnostic for ExternFunctionCannotHaveBodyError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("@extern functions cannot have a body")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("remove the function body"),
            ])
            .with_notes(vec![
                "extern functions are implemented in external code".to_string(),
            ])
    }
}

/// Error when @extern function parameter is not consuming.
pub struct ExternParameterNotConsumingError {
    pub span: Span,
    pub param_name: String,
}

impl IntoDiagnostic for ExternParameterNotConsumingError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "@extern function parameter '{}' must use consuming access mode",
                self.param_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("parameter must be passed by value"),
            ])
            .with_notes(vec![
                "extern functions receive values, not references".to_string(),
                "use Pointer[T] to pass a pointer explicitly".to_string(),
            ])
    }
}

/// Error when a type used in @extern function does not conform to FFISafe.
pub struct TypeNotFFISafeError {
    pub span: Span,
    pub ty: String,
    pub context: String, // "parameter" or "return type"
}

impl IntoDiagnostic for TypeNotFFISafeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "{} type '{}' does not conform to FFISafe",
                self.context, self.ty
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("type is not FFI-safe"),
            ])
            .with_notes(vec![
                "only types conforming to FFISafe can cross FFI boundaries".to_string(),
                "consider using Pointer[T] or primitive types".to_string(),
            ])
    }
}

/// A field that does not conform to a required protocol.
pub struct NonConformingField {
    pub field_name: String,
    pub field_ty: String,
    pub span: Span,
}

/// Error when a struct/enum conforming to a protocol with requires_fields_conform
/// has fields that don't conform to that protocol.
pub struct FieldsNotConformingToProtocolError {
    pub type_span: Span,
    pub type_name: String,
    pub type_kind: &'static str, // "struct" or "enum"
    pub protocol_name: String,
    pub non_conforming_fields: Vec<NonConformingField>,
}

impl IntoDiagnostic for FieldsNotConformingToProtocolError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let field_list = self
            .non_conforming_fields
            .iter()
            .map(|f| format!("'{}' (type '{}')", f.field_name, f.field_ty))
            .collect::<Vec<_>>()
            .join(", ");

        let mut labels = vec![
            Label::primary(self.type_span.file_id, self.type_span.range()).with_message(format!(
                "{} conforms to {} but has non-conforming fields",
                self.type_kind, self.protocol_name
            )),
        ];

        // Add secondary labels for each non-conforming field
        for field in &self.non_conforming_fields {
            labels.push(
                Label::secondary(field.span.file_id, field.span.range()).with_message(format!(
                    "'{}' does not conform to {}",
                    field.field_ty, self.protocol_name
                )),
            );
        }

        Diagnostic::error()
            .with_message(format!(
                "{} '{}' conforms to {} but fields do not: {}",
                self.type_kind, self.type_name, self.protocol_name, field_list
            ))
            .with_labels(labels)
            .with_notes(vec![format!(
                "all fields of a {} conforming to {} must also conform to {}",
                self.type_kind, self.protocol_name, self.protocol_name
            )])
    }
}

/// Error when an enum tries to conform to a protocol that disallows enum conformance.
pub struct ProtocolDisallowsEnumConformanceError {
    pub span: Span,
    pub enum_name: String,
    pub protocol_name: String,
}

impl IntoDiagnostic for ProtocolDisallowsEnumConformanceError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "enum '{}' cannot conform to protocol '{}'",
                self.enum_name, self.protocol_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("enums cannot conform to this protocol"),
            ])
            .with_notes(vec![format!(
                "'{}' only allows struct conformance",
                self.protocol_name
            )])
    }
}
