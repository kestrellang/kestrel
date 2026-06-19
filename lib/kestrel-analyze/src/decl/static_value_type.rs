//! # Static Value Type Analyzer (references 2a)
//!
//! Globals live for the whole program, so their types must be `Static`
//! (transitively reference-free). Checks module-level value declarations
//! and `static` members — the same selection MIR's `lower_static` uses —
//! against the structural staticness predicate.
//!
//! ## Diagnostics
//!
//! ### E505 -- `static_requires_static_type` (Error, Correctness)
//!
//! **Message:** "static variable '{name}' has non-Static type '{ty}'"
//!
//! **Labels:**
//! - Primary: the declaration — "a global lives for the whole program"
//!
//! **Notes:**
//! - "only Static (reference-free) types can be stored in globals"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Callable, Computed, NodeKind};
use kestrel_hir::Builtin;
use kestrel_hir_lower::LowerTypeAnnotation;
use kestrel_name_res::ResolveBuiltin;
use kestrel_semantics::hir_type_is_static;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E505",
    name: "static_requires_static_type",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct StaticValueTypeAnalyzer;

impl Describe for StaticValueTypeAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::StaticValueType
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for StaticValueTypeAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Field]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Same selection as MIR `lower_static`: a module-level value Field
        // (no Callable — those are function-typed decls), or a member Field
        // carrying the `static` marker. NB `kestrel_ast_builder::Static` is
        // the *modifier* component, unrelated to `Builtin::Static`.
        let is_module_level = cx
            .query
            .parent_of(cx.entity)
            .is_some_and(|p| cx.query.get::<NodeKind>(p) == Some(&NodeKind::Module));
        let is_static_member = cx
            .query
            .get::<kestrel_ast_builder::Static>(cx.entity)
            .is_some();
        if !(is_module_level || is_static_member) {
            return vec![];
        }
        if cx.query.get::<Callable>(cx.entity).is_some()
            || cx.query.get::<Computed>(cx.entity).is_some()
        {
            return vec![];
        }
        // Stdlib-less fixtures without the builtin: the bound system is
        // inert, so is this check.
        if cx
            .query
            .query(ResolveBuiltin {
                builtin: Builtin::Static,
                root: cx.root,
            })
            .is_none()
        {
            return vec![];
        }
        let Some(ty) = cx.query.query(LowerTypeAnnotation {
            entity: cx.entity,
            root: cx.root,
        }) else {
            return vec![];
        };
        if hir_type_is_static(cx.query, &ty, cx.entity, cx.root) {
            return vec![];
        }

        let name = util::entity_name(cx.query, cx.entity);
        let span = util::entity_span(cx.query, cx.entity);
        vec![AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: format!("static variable '{}' has non-Static type", name),
            labels: vec![DiagLabel {
                span,
                message: "a global lives for the whole program".into(),
                is_primary: true,
            }],
            notes: vec!["only Static (reference-free) types can be stored in globals".into()],
        }]
    }
}
