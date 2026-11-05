//! # Unsafe Field Analyzer
//!
//! Rejects `unsafe` on a stored property. `unsafe` marks a declaration whose
//! *use* needs an unsafe context; for a property that means its accessors,
//! so only computed properties (and protocol requirements, which a witness
//! satisfies with accessors) may carry it. Unsafe stored fields (Rust's
//! RFC 3458) are not supported.
//!
//! ## Diagnostics
//!
//! ### E645 -- `unsafe_stored_field` (Error, Correctness)
//! **Message:** "only a computed property can be `unsafe`"
//!
//! **Labels:**
//! - Primary: the field declaration
//!   - Span source: `util::entity_span` on the field entity
//!   - Message: "stored property"
//!
//! **Notes:**
//! - "keep the field private and expose it through an `unsafe` computed property"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{FieldBacking, FieldClass, IsUnsafe, NodeKind};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E645",
    name: "unsafe_stored_field",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct UnsafeFieldAnalyzer;

impl Describe for UnsafeFieldAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::UnsafeField
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for UnsafeFieldAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Field]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        if cx.query.get::<IsUnsafe>(cx.entity).is_none() {
            return vec![];
        }
        let Some(class) = cx.query.get::<FieldClass>(cx.entity) else {
            return vec![];
        };
        if class.backing != FieldBacking::Stored || class.is_protocol_requirement() {
            return vec![];
        }
        vec![AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: "only a computed property can be `unsafe`".into(),
            labels: vec![DiagLabel {
                span: util::entity_span(cx.query, cx.entity),
                message: "stored property".into(),
                is_primary: true,
            }],
            notes: vec![
                "keep the field private and expose it through an `unsafe` computed property".into(),
            ],
        }]
    }
}
