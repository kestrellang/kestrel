//! # Type Alias Validation Analyzer
//!
//! Validates type alias declarations:
//!
//! 1. **Bounds only in protocols** — `type Item: Protocol` bounds are only
//!    valid inside protocol declarations. Struct/module-level type aliases
//!    with bounds are errors.
//! 2. **Requires `= Type`** — Non-protocol type aliases must have a definition
//!    (`type Foo = Int`). Abstract associated types without a definition are only
//!    allowed inside protocols.
//! 3. **Qualified binding validation** — `type Protocol.Item = Concrete` must
//!    reference a protocol the parent type conforms to, and that protocol must
//!    declare the associated type.
//! 4. **Unqualified binding ambiguity** — If multiple conformed protocols declare
//!    the same associated type name, the binding is ambiguous.
//! 5. **Constraint satisfaction** — The bound type must satisfy any protocol
//!    constraints on the associated type.
//!
//! TODO: Checks 3-5 require resolved conformances and protocol entity access.
//! Check 1 and 2 are partially implementable from the ECS. Currently shell
//! for all checks that need resolved types.
//!
//! ## Diagnostics
//!
//! ### E441 -- `associated_type_bounds_in_wrong_context` (Error, Correctness)
//!
//! **Message:** "type alias '{name}' cannot have bounds outside a protocol"
//!
//! **Labels:**
//! - Primary: the type alias declaration
//!   - Span source: `util::entity_span` on the type alias entity
//!   - Message: "bounds not allowed here"
//!
//! **Notes:**
//! - "associated type bounds (`type T: Protocol`) are only valid inside protocol declarations"
//!
//! ### E442 -- `type_alias_requires_type` (Error, Correctness)
//!
//! **Message:** "type alias '{name}' requires a type definition"
//!
//! **Labels:**
//! - Primary: the type alias declaration
//!   - Span source: `util::entity_span` on the type alias entity
//!   - Message: "add '= Type' to provide a definition"
//!
//! **Notes:** (none)
//!
//! ### E443 -- `qualified_binding_not_conforming` (Error, Correctness)
//!
//! **Message:** "'{type_name}' does not conform to '{protocol_name}'"
//!
//! **Labels:**
//! - Primary: the qualified binding
//!   - Span source: `util::entity_span` on the type alias entity
//!   - Message: "qualified binding references non-conformed protocol"
//!
//! **Notes:** (none)
//!
//! ### E444 -- `qualified_binding_wrong_protocol` (Error, Correctness)
//!
//! **Message:** "protocol '{protocol}' has no associated type '{type_name}'"
//!
//! **Labels:**
//! - Primary: the qualified binding
//!   - Span source: `util::entity_span` on the type alias entity
//!   - Message: "no such associated type in protocol"
//!
//! **Notes:** (none)
//!
//! ### E445 -- `ambiguous_associated_type` (Error, Correctness)
//!
//! **Message:** "associated type '{name}' is ambiguous between protocols: {list}"
//!
//! **Labels:**
//! - Primary: the unqualified binding
//!   - Span source: `util::entity_span` on the type alias entity
//!   - Message: "use a qualified binding to disambiguate"
//!
//! **Notes:** (none)
//!
//! ### E446 -- `associated_type_constraint_not_satisfied` (Error, Correctness)
//!
//! **Message:** "type '{bound_type}' does not satisfy constraint '{protocol}' on associated type '{name}'"
//!
//! **Labels:**
//! - Primary: the type alias binding
//!   - Span source: `util::entity_span` on the type alias entity
//!   - Message: "constraint not satisfied"
//!
//! **Notes:** (none)

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Conformances, NodeKind, TypeAnnotation};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E441",
        name: "associated_type_bounds_in_wrong_context",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E442",
        name: "type_alias_requires_type",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E443",
        name: "qualified_binding_not_conforming",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E444",
        name: "qualified_binding_wrong_protocol",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E445",
        name: "ambiguous_associated_type",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E446",
        name: "associated_type_constraint_not_satisfied",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct TypeAliasValidationAnalyzer;

impl Describe for TypeAliasValidationAnalyzer {
    fn id(&self) -> &'static str {
        "type_alias_validation"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for TypeAliasValidationAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::TypeAlias]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        // Determine context: is this inside a protocol, concrete type, extension, or module?
        let parent_kind = cx
            .query
            .parent_of(cx.entity)
            .and_then(|p| cx.query.get::<NodeKind>(p).cloned());

        let is_protocol_context = matches!(parent_kind, Some(NodeKind::Protocol));

        // Check 1: Conformances on a type alias are bounds (type Item: Protocol).
        // Only valid inside protocol declarations.
        if !is_protocol_context {
            if let Some(conformances) = cx.query.get::<Conformances>(cx.entity) {
                if !conformances.0.is_empty() {
                    let name = util::entity_name(cx.query, cx.entity);
                    let span = util::entity_span(cx.query, cx.entity);
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[0].id,
                        severity: DESCRIPTORS[0].default_severity,
                        message: format!(
                            "type alias '{}' cannot have bounds outside a protocol",
                            name
                        ),
                        labels: vec![DiagLabel {
                            span,
                            message: "bounds not allowed here".into(),
                            is_primary: true,
                        }],
                        notes: vec![
                            "associated type bounds (`type T: Protocol`) are only valid inside protocol declarations".into(),
                        ],
                    });
                }
            }
        }

        // Check 2: Non-protocol type aliases require `= Type` definition.
        // Inside protocols, abstract associated types (no definition) are allowed.
        if !is_protocol_context && cx.query.get::<TypeAnnotation>(cx.entity).is_none() {
            let name = util::entity_name(cx.query, cx.entity);
            let span = util::entity_span(cx.query, cx.entity);
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[1].id,
                severity: DESCRIPTORS[1].default_severity,
                message: format!("type alias '{}' requires a type definition", name),
                labels: vec![DiagLabel {
                    span,
                    message: "add '= Type' to provide a definition".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
        }

        // TODO: Checks 3-6 (qualified binding, ambiguity, constraint satisfaction)
        // require resolved conformances and protocol entity access which are not
        // yet available in the ECS at declaration analysis time.

        diags
    }
}
