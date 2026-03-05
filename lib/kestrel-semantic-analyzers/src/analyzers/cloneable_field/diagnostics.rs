//! Diagnostics for cloneable field validation

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a struct/enum has a Cloneable field but doesn't conform to Cloneable.
pub struct CloneableFieldRequiresCloneableConformance {
    pub type_span: Span,
    pub type_name: String,
    pub field_name: String,
    pub field_span: Span,
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
