//! Diagnostics for protocol field conformance validation

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

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
