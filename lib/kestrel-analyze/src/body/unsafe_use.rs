//! # Unsafe Use Analyzer
//!
//! Reports every use of an unsafe declaration outside an unsafe context, and
//! every `unsafe` that covers no unsafe use. Whether a declaration is unsafe
//! comes from `UnsafetyOf`, the one query the witness rule and the LSP also
//! read. A use is anything inference resolved to the declaration: a call, a
//! property read, an initializer, a subscript, or naming it as a value.
//! Calling a C function value (`@extern(.C) (…) -> …`) is unsafe too: the
//! address came from C or a cast, and nothing checks what it points at.
//!
//! An unsafe context is the operand of `unsafe expr` or the body of
//! `unsafe { … }`, and nothing else: the body of an `unsafe func` is not one.
//! The rule is lexical, so a closure written inside an unsafe region is
//! inside it too. HIR lowering records each region's source extent in
//! `HirBody::unsafe_regions`; a use is covered when its span lies inside one.
//!
//! ## Diagnostics
//!
//! ### E641 — `unsafe_use` (Error, Correctness)
//!
//! **Message:** "{what} '{name}' is unsafe and requires an `unsafe` expression or block",
//! or for a call through a C function value: "calling a C function value is
//! unsafe and requires an `unsafe` expression or block"
//!
//! **Labels:**
//! - Primary: the use
//!   - Span source: `util::expr_span` on the referencing `HirExprId`
//!   - Message: "unsafe use"
//! - Secondary: the declaration (not for C function value calls)
//!   - Span source: `util::entity_span` on the unsafe entity
//!   - Message: "declared unsafe here" / "declared `@extern` here"
//!
//! **Notes:**
//! - "wrap the use in `unsafe` once you have checked its safety contract"
//!
//! ### E642 — `unused_unsafe` (Warning, Usage)
//!
//! **Message:** "unnecessary `unsafe`: it covers no unsafe operation"
//!
//! **Labels:**
//! - Primary: the region
//!   - Span source: the recorded `HirBody::unsafe_regions` span
//!   - Message: "no unsafe operation here"
//!
//! **Notes:**
//! - (none)

use std::collections::HashSet;

use kestrel_ast_builder::unsafety::{Unsafety, UnsafetyOf};
use kestrel_hecs::Entity;
use kestrel_hir::body::*;
use kestrel_span::Span;
use kestrel_type_infer::result::ResolvedTy;

use crate::body::restricted_use::display_name;
use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, BodyCheck, Describe};
use crate::util;

pub(crate) static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E641",
        name: "unsafe_use",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E642",
        name: "unused_unsafe",
        default_severity: Severity::Warning,
        category: Category::Usage,
    },
];

pub struct UnsafeUseAnalyzer;

impl Describe for UnsafeUseAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::UnsafeUse
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for UnsafeUseAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let regions = &cx.hir.unsafe_regions;
        let mut covered = vec![false; regions.len()];
        let mut seen: HashSet<(Entity, usize, usize)> = HashSet::new();
        let mut diags = Vec::new();
        for (id, expr) in cx.hir.exprs.iter() {
            if calls_c_value(cx, id, expr) {
                let span = util::expr_span(cx.hir, id);
                match innermost_region(regions, &span) {
                    Some(region) => covered[region] = true,
                    None if !span.is_synthetic() => diags.push(c_value_call(span)),
                    None => {},
                }
                continue;
            }
            let target = match expr {
                HirExpr::Def(entity, ..) => Some(*entity),
                _ => cx.typed.resolutions.get(&id).copied(),
            };
            let Some(target) = target else {
                continue;
            };
            let Some(unsafety) = cx.query.query(UnsafetyOf { entity: target }) else {
                continue;
            };
            let span = util::expr_span(cx.hir, id);
            // Compiler-generated code (synthesized accessors, shims) vouches
            // for itself: nobody wrote it, so nobody can mark it.
            if span.is_synthetic() {
                continue;
            }
            if let Some(region) = innermost_region(regions, &span) {
                covered[region] = true;
                continue;
            }
            kestrel_debug::ktrace!(
                "unsafe-use",
                "uncovered use of {:?} in {:?} at {:?} (regions {:?})",
                util::entity_name(cx.query, target),
                util::entity_name(cx.query, cx.entity),
                span,
                regions
            );
            if seen.insert((target, span.file_id, span.start)) {
                diags.push(unsafe_use(cx, target, unsafety, span));
            }
        }
        diags.extend(
            regions
                .iter()
                .zip(&covered)
                .filter(|(_, used)| !**used)
                .map(|(span, _)| unused_unsafe(span)),
        );
        diags
    }
}

/// A call through a C function value: `cb(x)` on a local or expression of
/// C function type, or `s.cb(x)` on such a field. Calling a function by
/// name is a direct call, safe unless the function itself is unsafe.
fn calls_c_value(cx: &BodyContext<'_>, id: HirExprId, expr: &HirExpr) -> bool {
    let callee_ty = match expr {
        HirExpr::Call { callee, .. } => {
            if let HirExpr::Def(entity, ..) = &cx.hir.exprs[*callee]
                && cx.query.get::<kestrel_ast_builder::NodeKind>(*entity)
                    == Some(&kestrel_ast_builder::NodeKind::Function)
            {
                return false;
            }
            cx.typed.expr_types.get(callee)
        },
        HirExpr::MethodCall { .. } => cx.typed.field_callee_types.get(&id),
        _ => None,
    };
    matches!(
        callee_ty,
        Some(ResolvedTy::Function {
            kind: kestrel_ast::FnTypeKind::C,
            ..
        })
    )
}

/// The smallest region whose extent holds `span`: a use marks only the
/// `unsafe` nearest it, so an outer `unsafe` around an inner one is unused.
fn innermost_region(regions: &[Span], span: &Span) -> Option<usize> {
    regions
        .iter()
        .enumerate()
        .filter(|(_, r)| r.file_id == span.file_id && r.start <= span.start && span.end <= r.end)
        .min_by_key(|(_, r)| r.end - r.start)
        .map(|(i, _)| i)
}

fn unsafe_use(
    cx: &BodyContext<'_>,
    target: Entity,
    unsafety: Unsafety,
    span: Span,
) -> AnalyzeDiagnostic {
    let (what, declared) = match unsafety {
        Unsafety::Declared => ("", "declared unsafe here"),
        Unsafety::Extern => ("extern function ", "declared `@extern` here"),
    };
    let mut labels = vec![DiagLabel {
        span,
        message: "unsafe use".into(),
        is_primary: true,
    }];
    // A `lang` intrinsic has no source to point at.
    let declared_at = util::entity_span(cx.query, target);
    if !declared_at.is_synthetic() {
        labels.push(DiagLabel {
            span: declared_at,
            message: declared.into(),
            is_primary: false,
        });
    }
    AnalyzeDiagnostic {
        descriptor_id: "E641",
        severity: Severity::Error,
        message: format!(
            "{what}'{}' is unsafe and requires an `unsafe` expression or block",
            display_name(cx.query, target)
        ),
        labels,
        notes: vec!["wrap the use in `unsafe` once you have checked its safety contract".into()],
    }
}

fn c_value_call(span: Span) -> AnalyzeDiagnostic {
    AnalyzeDiagnostic {
        descriptor_id: "E641",
        severity: Severity::Error,
        message:
            "calling a C function value is unsafe and requires an `unsafe` expression or block"
                .into(),
        labels: vec![DiagLabel {
            span,
            message: "unsafe call".into(),
            is_primary: true,
        }],
        notes: vec![
            "wrap the call in `unsafe` once you have checked the address is a live C function \
             of this type"
                .into(),
        ],
    }
}

fn unused_unsafe(span: &Span) -> AnalyzeDiagnostic {
    AnalyzeDiagnostic {
        descriptor_id: "E642",
        severity: Severity::Warning,
        message: "unnecessary `unsafe`: it covers no unsafe operation".into(),
        labels: vec![DiagLabel {
            span: span.clone(),
            message: "no unsafe operation here".into(),
            is_primary: true,
        }],
        notes: vec![],
    }
}
