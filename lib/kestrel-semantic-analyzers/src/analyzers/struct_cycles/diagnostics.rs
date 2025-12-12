use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// A participant in a cycle chain
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleMember {
    pub name: String,
    pub span: Span,
}

/// Error when structs form a containment cycle (infinite-size type)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircularStructContainmentError {
    pub origin: CycleMember,
    pub cycle: Vec<CycleMember>,
    pub field_name: String,
    pub field_span: Span,
}

impl IntoDiagnostic for CircularStructContainmentError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let cycle_names: Vec<_> = std::iter::once(&self.origin)
            .chain(self.cycle.iter())
            .map(|p| p.name.as_str())
            .collect();
        let cycle_display = cycle_names.join(" -> ");

        let mut labels = vec![
            Label::primary(self.field_span.file_id, self.field_span.range()).with_message(format!(
                "field '{}' creates infinite-size type",
                self.field_name
            )),
            Label::secondary(self.origin.span.file_id, self.origin.span.range())
                .with_message("cycle starts here"),
        ];

        for participant in &self.cycle {
            labels.push(
                Label::secondary(participant.span.file_id, participant.span.range())
                    .with_message(format!("'{}' is part of the cycle", participant.name)),
            );
        }

        Diagnostic::error()
            .with_message(format!(
                "circular struct containment: {} -> {}",
                cycle_display, self.origin.name
            ))
            .with_labels(labels)
            .with_notes(vec![
                "structs cannot contain themselves directly or indirectly".to_string(),
                "consider using an optional type, array, or reference to break the cycle"
                    .to_string(),
            ])
    }
}

/// Error when a struct directly contains itself
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfContainingStructError {
    pub struct_name: String,
    pub struct_span: Span,
    pub field_name: String,
    pub field_span: Span,
}

impl IntoDiagnostic for SelfContainingStructError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "struct '{}' cannot contain itself",
                self.struct_name
            ))
            .with_labels(vec![
                Label::primary(self.field_span.file_id, self.field_span.range()).with_message(
                    format!(
                        "field '{}' has type '{}', creating infinite-size type",
                        self.field_name, self.struct_name
                    ),
                ),
                Label::secondary(self.struct_span.file_id, self.struct_span.range())
                    .with_message("struct declared here"),
            ])
            .with_notes(vec![
                "a struct cannot directly contain a value of its own type".to_string(),
                "consider using an optional type, array, or reference".to_string(),
            ])
    }
}
