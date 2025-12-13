//! Struct containment cycle diagnostics.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// A participant in a cycle chain
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleMember {
    /// Name of the type/symbol
    pub name: String,
    /// Span of the declaration or reference (contains file_id)
    pub span: Span,
}

/// Error when structs form a containment cycle (infinite-size type)
///
/// Example:
/// ```ignore
/// struct A { let b: B }
/// struct B { let a: A }  // Error: A contains B contains A
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircularStructContainmentError {
    /// The struct where the cycle was detected
    pub origin: CycleMember,
    /// The chain of structs that form the cycle
    pub cycle: Vec<CycleMember>,
    /// The field that causes the cycle
    pub field_name: String,
    /// Span of the field declaration
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

        // Add secondary labels for each participant in the cycle
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
///
/// Example:
/// ```ignore
/// struct Node { let next: Node }  // Error: Node contains Node
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfContainingStructError {
    /// The struct that contains itself
    pub struct_name: String,
    /// Span of the struct declaration
    pub struct_span: Span,
    /// The field that contains the struct itself
    pub field_name: String,
    /// Span of the field declaration
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
