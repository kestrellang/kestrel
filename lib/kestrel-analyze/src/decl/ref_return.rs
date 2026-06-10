//! # Reference-Return Analyzer (stage-1 references)
//!
//! Validates ref-returning signatures (`-> &T` / `-> &mutating T`):
//! provenance is inferred single-source, so the declaration must have an
//! unambiguous root the caller can scope the returned reference to.
//!
//! - A METHOD's root is its receiver — always unambiguous (other borrow
//!   params staying alive is the MIR layer's job; `at(index: Int64) -> &T`
//!   is legal).
//! - A FREE FUNCTION's root is its unique non-consuming parameter; two or
//!   more candidates (`func pick(a: T, b: T) -> &T`) is E493. Zero is legal
//!   (a Static- or Pointer-derived body).
//!
//! Body-level root legality (Local escape E494, mutable-root E495,
//! consuming-self E496) is the MIR escape checker's job, not this rule's.
//!
//! ## Diagnostics
//!
//! ### E493 -- `ambiguous_borrow_source` (Error, Correctness)

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Callable, NodeKind};
use kestrel_hir_lower::CallableRefReturn;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E493",
    name: "ambiguous_borrow_source",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct RefReturnAnalyzer;

impl Describe for RefReturnAnalyzer {
    fn id(&self) -> &'static str {
        "ref_return"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for RefReturnAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Function]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Syntactic gate BEFORE the query: CallableRefReturn forces return
        // lowering, which would accumulate name-res diagnostics for
        // signatures nothing else lowers (protocol requirements, unused
        // decls). Only force it when the annotation can possibly be a ref.
        let has_ref_syntax = cx
            .query
            .get::<kestrel_ast_builder::TypeAnnotation>(cx.entity)
            .is_some_and(|ann| crate::util::ast_contains_ref(&ann.0));
        if !has_ref_syntax {
            return Vec::new();
        }
        if cx
            .query
            .query(CallableRefReturn {
                entity: cx.entity,
                root: cx.root,
            })
            .is_none()
        {
            return Vec::new();
        }
        let Some(callable) = cx.query.get::<Callable>(cx.entity) else {
            return Vec::new();
        };
        // Methods: the receiver is THE root; always unambiguous.
        if callable.receiver.is_some() {
            return Vec::new();
        }
        // Free function: count reference-eligible (non-consuming) params.
        let eligible = callable.params.iter().filter(|p| !p.is_consuming).count();
        if eligible < 2 {
            return Vec::new();
        }
        vec![AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: "ambiguous borrow source for the returned reference".into(),
            labels: vec![DiagLabel {
                span: util::entity_span(cx.query, cx.entity),
                message: format!(
                    "{eligible} parameters could root the returned reference; this version \
                     supports a single parameter root"
                ),
                is_primary: true,
            }],
            notes: vec![
                "make it a method (the receiver is the root), or reduce to one borrowed \
                 parameter"
                    .into(),
            ],
        }]
    }
}
