use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// A participant in a constraint cycle chain
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleMember {
    pub name: String,
    pub span: Span,
}

/// Error when generic constraints form a cycle
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircularConstraintError {
    pub origin: CycleMember,
    pub cycle: Vec<CycleMember>,
}

impl IntoDiagnostic for CircularConstraintError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let cycle_names: Vec<_> = std::iter::once(&self.origin)
            .chain(self.cycle.iter())
            .map(|p| p.name.as_str())
            .collect();
        let cycle_display = cycle_names.join(" -> ");

        let mut labels = vec![
            Label::primary(self.origin.span.file_id, self.origin.span.range())
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
                "circular generic constraint: {} -> {}",
                cycle_display, self.origin.name
            ))
            .with_labels(labels)
            .with_notes(vec![
                "type parameter constraints cannot reference each other cyclically".to_string(),
            ])
    }
}
