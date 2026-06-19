//! # Dangling Pointer-Derived Reference Lint
//!
//! Warns on the provably-silly dangling-reference shape: a ref-returning
//! function whose returned reference is fabricated from `Pointer(to: x)`
//! where `x` is a local of the SAME function — the storage dies at return,
//! so every use of the reference is use-after-free.
//!
//! ```kestrel
//! func dangle() -> &Int64 {
//!     var x = 42;
//!     Pointer(to: x).value   // WARNING: x's storage dies at return
//! }
//! ```
//!
//! Scope is deliberately narrow (`references-gaps.md` §10.3): the lint
//! claims NOTHING beyond this shape. Pointer-derived refs inherit the
//! pointer's safety contract — heap-backed chains (`self.ptr().….value`),
//! param-rooted captures, and pointers received from elsewhere are the
//! user's responsibility and stay silent. Tracing follows the returned
//! expression through block tails and single-assignment `let` pointers to
//! a literal `Pointer(to: <local>)` construction; anything else ends the
//! trace.
//!
//! ## Diagnostics
//!
//! ### E504 — `dangling_pointer_ref` (Warning, Correctness)
//!
//! **Message:** "returned reference points into local '{name}', whose
//! storage dies when the function returns"
//!
//! **Labels:**
//! - Primary: the returned expression — "reference to dead storage escapes here"
//! - Secondary: the `Pointer(to: x)` argument — "address of '{name}' taken here"
//!
//! **Notes:** the inherited-contract explanation.

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, BodyCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Name, NodeKind};
use kestrel_hir::body::{HirBody, HirExpr, HirExprId, HirStmt};
use kestrel_hir::res::LocalId;
use kestrel_hir_lower::CallableRefReturn;
use kestrel_type_infer::RetRefPointerDerived;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E504",
    name: "dangling_pointer_ref",
    default_severity: Severity::Warning,
    category: Category::Correctness,
}];

pub struct DangleRefAnalyzer;

impl Describe for DangleRefAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::DangleRef
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for DangleRefAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Only ref-returning bodies can dangle a returned reference.
        if cx
            .query
            .query(CallableRefReturn {
                entity: cx.entity,
                root: cx.root,
            })
            .is_none()
        {
            return vec![];
        }
        // Inference errors mean the resolutions the trace relies on may be
        // missing or wrong; a warning on broken code is noise.
        if !cx.typed.errors.is_empty() {
            return vec![];
        }

        // Return-position exprs: tail + explicit `return v`. Flat scan over
        // the arena (the RetRefPointerDerived precedent): a Return inside a
        // closure over-collects, but a closure cannot legally return a ref,
        // so that code already errors and the early-out above silences us.
        let mut rets: Vec<HirExprId> = cx.hir.tail_expr.into_iter().collect();
        for (_, expr) in cx.hir.exprs.iter() {
            if let HirExpr::Return { value: Some(v), .. } = expr {
                rets.push(*v);
            }
        }

        let mut diags = Vec::new();
        for ret in rets {
            let ret = peel_block_tail(cx.hir, ret);
            // The returned expr must read a pointer-derived wrapper member
            // (`.value` / `.mutatingValue` — anything whose getter is a
            // ptr_ref/ptr_mut_ref thin wrapper).
            let HirExpr::Field { base, .. } = &cx.hir.exprs[ret] else {
                continue;
            };
            let Some(&member) = cx.typed.resolutions.get(&ret) else {
                continue;
            };
            if !cx.query.query(RetRefPointerDerived {
                entity: member,
                root: cx.root,
            }) {
                kestrel_debug::ktrace!("dangle", "member {member:?} not ptr-derived wrapper");
                continue;
            }
            let Some((local, arg)) = trace_pointer_to_local(cx, *base) else {
                kestrel_debug::ktrace!("dangle", "base {base:?} trace failed: {:?}", &cx.hir.exprs[*base]);
                continue;
            };
            let name = &cx.hir.locals[local].name;
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[0].id,
                severity: DESCRIPTORS[0].default_severity,
                message: format!(
                    "returned reference points into local '{name}', whose storage dies when the function returns"
                ),
                labels: vec![
                    DiagLabel {
                        span: util::expr_span(cx.hir, ret),
                        message: "reference to dead storage escapes here".into(),
                        is_primary: true,
                    },
                    DiagLabel {
                        span: util::expr_span(cx.hir, arg),
                        message: format!("address of '{name}' taken here"),
                        is_primary: false,
                    },
                ],
                notes: vec![
                    "`Pointer(to:)` captures a raw address without extending the local's \
                     lifetime; the reference inherits the pointer's contract"
                        .to_string(),
                ],
            });
        }
        diags
    }
}

/// Peel `{ …; tail }` wrappers down to the value-producing tail expr.
fn peel_block_tail(hir: &HirBody, mut id: HirExprId) -> HirExprId {
    while let HirExpr::Block { body, .. } = &hir.exprs[id] {
        let Some(tail) = body.tail_expr else { break };
        id = tail;
    }
    id
}

/// Trace a pointer-typed expr to a literal `Pointer(to: <same-fn local>)`
/// construction. Follows single-assignment `let` pointers; params and `var`
/// pointers (reassignable) end the trace. Returns the address-taken local
/// and the argument expr for the secondary label.
fn trace_pointer_to_local(
    cx: &BodyContext<'_>,
    expr: HirExprId,
) -> Option<(LocalId, HirExprId)> {
    let expr = peel_block_tail(cx.hir, expr);
    match &cx.hir.exprs[expr] {
        HirExpr::Call { callee, args, .. } => {
            // Init-call resolutions key on the CALL expr; plain calls on the
            // callee (the emit_resolved_call lookup order).
            let init = cx
                .typed
                .resolutions
                .get(&expr)
                .or_else(|| cx.typed.resolutions.get(callee))
                .copied()
                .or_else(|| match &cx.hir.exprs[*callee] {
                    HirExpr::Def(e, _, _) => Some(*e),
                    _ => None,
                })?;
            if !is_pointer_to_init(cx, init) {
                return None;
            }
            let arg = args.iter().find(|a| a.label.as_deref() == Some("to"))?;
            let HirExpr::Local(local, _) = &cx.hir.exprs[arg.value] else {
                return None;
            };
            // Params are caller-owned storage — out of this lint's claim.
            if cx.hir.params.contains(local) {
                return None;
            }
            Some((*local, arg.value))
        },
        HirExpr::Local(local, _) => {
            // Follow `let p = Pointer(to: x); … p.value` — single-assignment
            // only; a `var` pointer could be reassigned between init and use.
            if cx.hir.params.contains(local) || cx.hir.locals[*local].is_mut {
                return None;
            }
            let init = cx.hir.stmts.iter().find_map(|(_, stmt)| match stmt {
                HirStmt::Let {
                    local: l,
                    value: Some(v),
                    ..
                } if l == local => Some(*v),
                _ => None,
            })?;
            trace_pointer_to_local(cx, init)
        },
        _ => None,
    }
}

/// Is this entity the stdlib `Pointer.init(to:)`?
fn is_pointer_to_init(cx: &BodyContext<'_>, entity: kestrel_hecs::Entity) -> bool {
    if !matches!(cx.query.get::<NodeKind>(entity), Some(NodeKind::Initializer)) {
        return false;
    }
    cx.query
        .parent_of(entity)
        .and_then(|p| cx.query.get::<Name>(p))
        .is_some_and(|n| n.0 == "Pointer")
}
