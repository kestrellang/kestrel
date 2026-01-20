use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a type alias definition contains an unresolved (inferred) type.
/// Type aliases must have fully specified types - they cannot contain inference placeholders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAliasContainsInferError {
    /// The span of the type alias definition
    pub span: Span,
    /// The name of the type alias
    pub alias_name: String,
}

impl IntoDiagnostic for TypeAliasContainsInferError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "type alias '{}' contains unresolved type argument",
                self.alias_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("type argument required"),
            ])
            .with_notes(vec![
                "type alias definitions must have fully specified types".to_string(),
            ])
    }
}

/// A participant in a circular type alias chain
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleParticipant {
    /// Name of the type alias
    pub name: String,
    /// Span of the type alias declaration's name
    pub name_span: Span,
}

/// Error when type aliases form a circular dependency
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircularTypeAliasError {
    /// The type alias where the cycle was detected (the one being resolved)
    pub origin: CycleParticipant,
    /// The chain of type aliases that form the cycle, in order of resolution.
    /// Does not include the origin (which would be a duplicate at the end).
    pub cycle: Vec<CycleParticipant>,
}

impl IntoDiagnostic for CircularTypeAliasError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let cycle_names: Vec<_> = std::iter::once(&self.origin)
            .chain(self.cycle.iter())
            .map(|p| p.name.as_str())
            .collect();
        let cycle_display = cycle_names.join(" -> ");

        let mut labels = vec![
            Label::primary(self.origin.name_span.file_id, self.origin.name_span.range())
                .with_message("cycle starts here"),
        ];

        // Add secondary labels for each participant in the cycle
        for (i, participant) in self.cycle.iter().enumerate() {
            let message = if i == self.cycle.len() - 1 {
                format!(
                    "'{}' refers back to '{}'",
                    participant.name, self.origin.name
                )
            } else {
                format!(
                    "'{}' refers to '{}'",
                    participant.name,
                    self.cycle
                        .get(i + 1)
                        .map(|p| p.name.as_str())
                        .unwrap_or(&self.origin.name)
                )
            };
            labels.push(
                Label::secondary(participant.name_span.file_id, participant.name_span.range())
                    .with_message(message),
            );
        }

        Diagnostic::error()
            .with_message(format!(
                "circular type alias: {} -> {}",
                cycle_display, self.origin.name
            ))
            .with_labels(labels)
            .with_notes(vec![
                "type aliases cannot reference themselves, directly or indirectly".to_string(),
            ])
    }
}
