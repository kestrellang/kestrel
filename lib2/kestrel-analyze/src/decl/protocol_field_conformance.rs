//! # Protocol Field Conformance Analyzer
//!
//! Validates that when a struct/enum conforms to a protocol with
//! `requires_fields_conform` (e.g. Hashable, Equatable, FFISafe), all stored
//! fields (or enum case payloads) also conform to that protocol.
//!
//! ## Status: Shell
//!
//! This analyzer requires **resolved field types** and a **conformance oracle**.
//! The available infrastructure provides:
//! - `EntityBuiltin` query: can identify protocols with `requires_fields_conform: true`
//!   via `BuiltinKind::Protocol { requires_fields_conform: true, .. }`
//! - `ResolveTypePath` + `Conformances`: can resolve which protocols a type conforms to
//!   and check if any of those protocols have `requires_fields_conform`
//! - `ResolveBuiltin`: can look up builtin protocol entities by `Builtin` variant
//!
//! What's still missing:
//! - **Resolved field types**: Field entities store `TypeAnnotation(AstType)`, not
//!   resolved type entities. Simple `Named` types can be resolved via `ResolveTypePath`,
//!   but generic types (e.g. `Array[T]`) cannot be checked for conformance.
//! - **Conformance checking oracle**: No query to ask "does type T conform to
//!   protocol P?" at the declaration level. This needs type inference results or
//!   a dedicated conformance resolution pass.
//!
//! Once resolved field types and a conformance oracle are available, the logic is:
//! 1. Get struct/enum's `Conformances`, resolve each to an entity
//! 2. For each conformed protocol, query `EntityBuiltin` to check `requires_fields_conform`
//! 3. For each such protocol, iterate all Field children
//! 4. Resolve each field's type and check if it conforms to the protocol
//! 5. Emit KS420 for each non-conforming field
//!
//! ## Diagnostics
//!
//! ### KS420 -- `fields_not_conforming_to_protocol` (Error, Correctness)
//!
//! **Message:** "{kind} '{type_name}' conforms to '{protocol}' but field '{field_name}' of type '{field_type}' does not"
//!
//! **Labels:**
//! - Primary: the struct/enum declaration
//!   - Span source: `util::entity_span` on the struct/enum entity
//!   - Message: "type declared here"
//! - Secondary: each non-conforming field
//!   - Span source: `util::entity_span` on the field entity
//!   - Message: "'{field_name}' does not conform to '{protocol}'"
//!
//! **Notes:**
//! - "all fields must conform to '{protocol}' for the type to conform"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use kestrel_ast_builder::NodeKind;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "KS420",
    name: "fields_not_conforming_to_protocol",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ProtocolFieldConformanceAnalyzer;

impl Describe for ProtocolFieldConformanceAnalyzer {
    fn id(&self) -> &'static str {
        "protocol_field_conformance"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for ProtocolFieldConformanceAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Struct, NodeKind::Enum]
    }

    fn check(&self, _cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Shell: blocked on resolved field types + conformance oracle.
        // See module doc for what's available and what's still needed.
        vec![]
    }
}
