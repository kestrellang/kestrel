//! # Infinite Opaque Return Analyzer
//!
//! Reports a `some P` return whose concrete type contains itself through
//! other opaque types — `func g() -> some P { W(inner: h()) }` with
//! `func h() -> some P { W(inner: g()) }`. Unification never sees this
//! infinite type (each body only names the other's opaque type), so it is
//! found by `kestrel_type_infer::OpaqueConcreteTy`, whose cycle recovery
//! answers `Cyclic` for every function on the cycle.
//!
//! A body whose concrete type IS another opaque type (`func f() -> some P
//! { f() }`) is E118 from inference; this analyzer stays quiet there.
//!
//! ## Diagnostics
//!
//! ### E408 -- `infinite_opaque_return` (Error, Correctness)
//! **Message:** "opaque return type of '{name}' contains itself"
//!
//! **Labels:**
//! - Primary: the function declaration
//!   - Span source: `util::entity_span` on the function entity
//!   - Message: "the concrete type behind this `some` return is infinite"
//!
//! **Notes:**
//! - "each function on the cycle wraps another's opaque return; give one of them a concrete return type"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::NodeKind;
use kestrel_type_infer::result::ResolvedTy;
use kestrel_type_infer::{InferBody, OpaqueConcrete, OpaqueConcreteTy};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E408",
    name: "infinite_opaque_return",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct OpaqueCycleAnalyzer;

impl Describe for OpaqueCycleAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::OpaqueCycle
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for OpaqueCycleAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Function]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let body = cx.query.query(InferBody {
            entity: cx.entity,
            root: cx.root,
        });
        let Some(concrete) = body.as_ref().and_then(|b| b.opaque_concrete_type.as_ref()) else {
            return Vec::new();
        };
        // A bare opaque underlier is inference's E118.
        if matches!(concrete, ResolvedTy::Opaque { .. }) {
            return Vec::new();
        }
        let expanded = cx.query.query(OpaqueConcreteTy {
            origin: cx.entity,
            root: cx.root,
        });
        if expanded != OpaqueConcrete::Cyclic {
            return Vec::new();
        }
        let name = util::entity_name(cx.query, cx.entity);
        vec![AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: format!("opaque return type of '{name}' contains itself"),
            labels: vec![DiagLabel {
                span: util::entity_span(cx.query, cx.entity),
                message: "the concrete type behind this `some` return is infinite".into(),
                is_primary: true,
            }],
            notes: vec![
                "each function on the cycle wraps another's opaque return; give one of them a \
                 concrete return type"
                    .into(),
            ],
        }]
    }
}
