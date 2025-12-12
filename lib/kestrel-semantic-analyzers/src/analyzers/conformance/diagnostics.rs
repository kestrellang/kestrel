use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

// Diagnostics for protocol conformance and inheritance

pub struct CircularProtocolInheritanceError {
    pub span: Span,
    pub protocol_name: String,
    pub cycle: Vec<String>,
}

impl IntoDiagnostic for CircularProtocolInheritanceError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let cycle_str = self.cycle.join(" -> ");
        Diagnostic::error()
            .with_message(format!(
                "protocol '{}' has circular inheritance",
                self.protocol_name
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message("circular inheritance detected")])
            .with_notes(vec![format!("inheritance cycle: {}", cycle_str)])
    }
}

pub struct MissingProtocolMethodError {
    pub span: Span,
    pub struct_name: String,
    pub protocol_name: String,
    pub method_name: String,
}

impl IntoDiagnostic for MissingProtocolMethodError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "type '{}' does not implement method '{}' from protocol '{}'",
                self.struct_name, self.method_name, self.protocol_name
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message(format!("missing method '{}'", self.method_name))])
    }
}

pub struct MissingAssociatedTypeError {
    pub span: Span,
    pub struct_name: String,
    pub protocol_name: String,
    pub type_name: String,
}

impl IntoDiagnostic for MissingAssociatedTypeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "type '{}' does not provide associated type '{}' from protocol '{}'",
                self.struct_name, self.type_name, self.protocol_name
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message(format!(
                    "missing associated type '{}'",
                    self.type_name
                ))])
    }
}

pub struct WrongMethodReturnTypeError {
    pub span: Span,
    pub method_name: String,
    pub protocol_name: String,
    pub expected_type: String,
    pub actual_type: String,
}

impl IntoDiagnostic for WrongMethodReturnTypeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "method '{}' has wrong return type for protocol '{}'",
                self.method_name, self.protocol_name
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message(format!(
                    "expected '{}', found '{}'",
                    self.expected_type, self.actual_type
                ))])
    }
}

pub struct AssociatedTypeConstraintNotSatisfiedError {
    pub span: Span,
    pub type_name: String,
    pub bound_type: String,
    pub required_protocol: String,
}

impl IntoDiagnostic for AssociatedTypeConstraintNotSatisfiedError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("type '{}' does not satisfy bound", self.bound_type))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message(format!(
                    "type '{}' does not conform to required protocol '{}'",
                    self.bound_type, self.required_protocol
                ))])
            .with_notes(vec![format!(
                "associated type '{}' requires conformance to '{}'",
                self.type_name, self.required_protocol
            )])
    }
}

pub struct ProtocolMethodReceiverMismatchError {
    pub span: Span,
    pub method_name: String,
    pub protocol_name: String,
    pub expected_receiver: String,
    pub actual_receiver: String,
}

impl IntoDiagnostic for ProtocolMethodReceiverMismatchError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "method '{}' has incorrect receiver kind for protocol '{}'",
                self.method_name, self.protocol_name
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message(format!(
                    "expected {} method, found {} method",
                    self.expected_receiver, self.actual_receiver
                ))])
    }
}

pub struct AmbiguousProtocolMethodError {
    pub span: Span,
    pub method_name: String,
    pub protocols: Vec<String>,
}

impl IntoDiagnostic for AmbiguousProtocolMethodError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let protocols_str = self.protocols.join("', '");
        Diagnostic::error()
            .with_message(format!(
                "method '{}' ambiguously implements protocol requirements",
                self.method_name
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message("ambiguous implementation")])
            .with_notes(vec![
                format!(
                    "this method would satisfy requirements from protocols: '{}'",
                    protocols_str
                ),
                "consider using a different method name or refactoring the protocol design"
                    .to_string(),
            ])
    }
}
