//! # Indirect Enum Analyzer
//!
//! Rejects `indirect enum` declarations. Indirect enums are not yet supported
//! by the compiler (deinit/codegen crashes on heap-allocated enum variants).
//!
//! ## Diagnostics
//!
//! ### E465 -- `indirect_enum` (Error, Correctness)
//! **Message:** "indirect enums are not yet supported"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{IsIndirect, NodeKind};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E465",
    name: "indirect_enum",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct IndirectEnumAnalyzer;

impl Describe for IndirectEnumAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::IndirectEnum
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for IndirectEnumAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Enum]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        if cx.query.get::<IsIndirect>(cx.entity).is_none() {
            return vec![];
        }

        let span = util::entity_span(cx.query, cx.entity);

        vec![AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: "indirect enums are not yet supported".into(),
            labels: vec![DiagLabel {
                span,
                message: "indirect enums are not yet supported".into(),
                is_primary: true,
            }],
            notes: vec![],
        }]
    }
}
