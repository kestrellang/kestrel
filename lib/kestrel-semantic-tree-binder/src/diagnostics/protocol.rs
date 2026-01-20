//! Protocol-related errors.
//!
//! Errors related to protocol binding: protocol-only contexts, inheritance, and associated types.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a type is used where a protocol is expected.
pub struct NotAProtocolError {
    pub span: Span,
    pub name: String,
    /// Context where the protocol was expected (e.g., "bound", "conformance", "inheritance")
    pub context: NotAProtocolContext,
}

/// The context where a protocol was expected.
#[derive(Clone, Copy)]
pub enum NotAProtocolContext {
    /// Used as a generic type bound (e.g., `T: SomeStruct`)
    Bound,
    /// Used as a struct conformance (e.g., `struct Foo: SomeStruct`)
    Conformance,
    /// Used in protocol inheritance (e.g., `protocol Bar: SomeStruct`)
    Inheritance,
}

impl IntoDiagnostic for NotAProtocolError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let (main_msg, label_msg) = match self.context {
            NotAProtocolContext::Bound => (
                format!(
                    "'{}' is not a protocol; bound must be a protocol",
                    self.name
                ),
                "cannot be used as a type bound",
            ),
            NotAProtocolContext::Conformance => (
                format!("'{}' is not a protocol", self.name),
                "cannot be used as a conformance",
            ),
            NotAProtocolContext::Inheritance => (
                format!("'{}' is not a protocol", self.name),
                "cannot be inherited by a protocol",
            ),
        };

        Diagnostic::error().with_message(main_msg).with_labels(vec![
            Label::primary(self.span.file_id, self.span.range()).with_message(label_msg),
        ])
    }
}

/// Error when a protocol has circular inheritance.
pub struct CircularProtocolInheritanceError {
    pub span: Span,
    pub protocol_name: String,
    /// The chain of protocols that form the cycle
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
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("circular inheritance detected"),
            ])
            .with_notes(vec![format!("inheritance cycle: {}", cycle_str)])
    }
}

/// Error when an associated type binding is ambiguous (multiple protocols have same associated type).
pub struct AmbiguousAssociatedTypeError {
    pub span: Span,
    pub type_name: String,
    pub protocols: Vec<String>,
}

impl IntoDiagnostic for AmbiguousAssociatedTypeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let protocols_str = self.protocols.join("', '");
        Diagnostic::error()
            .with_message(format!(
                "ambiguous associated type '{}' - use qualified syntax to disambiguate",
                self.type_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("ambiguous binding"),
            ])
            .with_notes(vec![
                format!(
                    "'{}' is declared in protocols: '{}'",
                    self.type_name, protocols_str
                ),
                format!(
                    "use qualified syntax like 'type {}.{} = ...' to specify which protocol",
                    self.protocols[0], self.type_name
                ),
            ])
    }
}

/// Error when a qualified binding references a protocol the struct doesn't conform to.
pub struct QualifiedBindingNotConformingError {
    pub span: Span,
    pub struct_name: String,
    pub protocol_name: String,
}

impl IntoDiagnostic for QualifiedBindingNotConformingError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "'{}' does not conform to '{}'",
                self.struct_name, self.protocol_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range()).with_message(format!(
                    "struct does not conform to '{}'",
                    self.protocol_name
                )),
            ])
    }
}

/// Error when a qualified binding references an associated type that doesn't exist in the protocol.
pub struct QualifiedBindingWrongProtocolError {
    pub span: Span,
    pub protocol_name: String,
    pub type_name: String,
}

impl IntoDiagnostic for QualifiedBindingWrongProtocolError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "protocol '{}' does not have associated type '{}'",
                self.protocol_name, self.type_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range()).with_message(format!(
                    "'{}' not found in '{}'",
                    self.type_name, self.protocol_name
                )),
            ])
    }
}

/// Error when a where clause references an associated type that doesn't exist.
pub struct WhereClauseAssociatedTypeNotFoundError {
    pub span: Span,
    pub type_param: String,
    pub assoc_type_name: String,
    pub protocol_name: String,
}

impl IntoDiagnostic for WhereClauseAssociatedTypeNotFoundError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "no associated type '{}' in protocol '{}'",
                self.assoc_type_name, self.protocol_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range()).with_message(format!(
                    "'{}.{}' does not exist",
                    self.type_param, self.assoc_type_name
                )),
            ])
    }
}

/// Error when an associated type binding doesn't satisfy the required protocol constraints.
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
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range()).with_message(format!(
                    "type '{}' does not conform to required protocol '{}'",
                    self.bound_type, self.required_protocol
                )),
            ])
            .with_notes(vec![format!(
                "associated type '{}' requires conformance to '{}'",
                self.type_name, self.required_protocol
            )])
    }
}

/// Error for conflicting associated types from multiple inherited protocols
pub struct InheritedAssociatedTypeConflictError {
    pub type_name: String,
    pub span: Span,
    pub protocol1: String,
    pub protocol2: String,
    pub definition_span1: Span,
    pub definition_span2: Span,
}

impl IntoDiagnostic for InheritedAssociatedTypeConflictError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "conflicting associated type '{}' from inherited protocols",
                self.type_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message(format!("conflicting associated type '{}'", self.type_name)),
                Label::secondary(self.definition_span1.file_id, self.definition_span1.range())
                    .with_message(format!("first defined in '{}'", self.protocol1)),
                Label::secondary(self.definition_span2.file_id, self.definition_span2.range())
                    .with_message(format!("also defined in '{}'", self.protocol2)),
            ])
            .with_notes(vec![format!(
                "protocols '{}' and '{}' both define associated type '{}'",
                self.protocol1, self.protocol2, self.type_name
            )])
    }
}

/// Error when a struct conforms to a protocol that inherits from another protocol,
/// but doesn't also declare conformance to the parent protocol.
pub struct MissingParentProtocolConformanceError {
    pub span: Span,
    pub struct_name: String,
    pub child_protocol: String,
    pub parent_protocol: String,
}

impl IntoDiagnostic for MissingParentProtocolConformanceError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "'{}' conforms to '{}' but not its parent protocol '{}'",
                self.struct_name, self.child_protocol, self.parent_protocol
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message(format!("must also conform to '{}'", self.parent_protocol)),
            ])
            .with_notes(vec![
                format!(
                    "protocol '{}' inherits from '{}', so '{}' must explicitly conform to both",
                    self.child_protocol, self.parent_protocol, self.struct_name
                ),
                format!("add ': {}' to the conformance list", self.parent_protocol),
            ])
    }
}

/// Error when trying to use `not` with a protocol that doesn't allow negation.
pub struct NegativeConformanceNotAllowedError {
    pub span: Span,
    pub protocol_name: String,
}

impl IntoDiagnostic for NegativeConformanceNotAllowedError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "cannot use 'not' with protocol '{}': not a language feature protocol",
                self.protocol_name
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message("only builtin protocols with implicit conformance can be negated")])
            .with_notes(vec![
                "only protocols marked with @builtin that have implicit conformance (like Copyable) can be negated".to_string(),
            ])
    }
}
