//! Copy semantics diagnostic errors.
//!
//! Errors related to Copyable/Cloneable conformance and field types.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a type conforms to a protocol that refines Copyable but also opts out of Copyable.
///
/// For example, conforming to `Cloneable` (which refines `Copyable`) while also declaring
/// `not Copyable` is a conflict since `Cloneable` requires `Copyable`.
pub struct ConflictingCopyableConformanceError {
    /// Span of the conflicting conformance declaration
    pub span: Span,
    /// Name of the protocol that refines Copyable (e.g., "Cloneable")
    pub refining_protocol: String,
}

impl IntoDiagnostic for ConflictingCopyableConformanceError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "cannot conform to `{}` and opt out of `Copyable`",
                self.refining_protocol
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message(format!(
                    "`{}` refines `Copyable`",
                    self.refining_protocol
                ))])
            .with_notes(vec![
                format!(
                    "`{}` inherits from `Copyable`, so types conforming to `{}` must be `Copyable`",
                    self.refining_protocol, self.refining_protocol
                ),
                "remove either the conformance or the `not Copyable` declaration".to_string(),
            ])
    }
}

/// Error when a struct/enum has a Cloneable field but doesn't conform to Cloneable.
///
/// A type with Cloneable fields must explicitly conform to Cloneable because:
/// - Cloneable types cannot be implicitly copied
/// - The clone() method must be called explicitly
/// - The containing type needs to implement its own clone() logic
pub struct CloneableFieldRequiresCloneableConformance {
    /// Span of the type (struct or enum) declaration
    pub type_span: Span,
    /// Name of the type (struct or enum)
    pub type_name: String,
    /// Name of the field that has a Cloneable type
    pub field_name: String,
    /// Span of the field declaration
    pub field_span: Span,
    /// Whether this is a struct or enum
    pub type_kind: &'static str,
}

impl IntoDiagnostic for CloneableFieldRequiresCloneableConformance {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "{} `{}` has Cloneable field `{}` but does not conform to Cloneable",
                self.type_kind, self.type_name, self.field_name
            ))
            .with_labels(vec![
                Label::primary(self.field_span.file_id, self.field_span.range())
                    .with_message("this field has a Cloneable type"),
                Label::secondary(self.type_span.file_id, self.type_span.range())
                    .with_message(format!("add `: Cloneable` to {}", self.type_name)),
            ])
            .with_notes(vec![
                "types containing Cloneable fields must conform to Cloneable".to_string(),
                "Cloneable types require explicit clone() calls and cannot be implicitly copied"
                    .to_string(),
            ])
    }
}
