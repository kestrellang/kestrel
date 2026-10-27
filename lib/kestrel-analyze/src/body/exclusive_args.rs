//! # Exclusive Arguments Analyzer
//!
//! A `mutating` argument or receiver borrows its place for the whole call,
//! so a `mutating` closure literal argument of the SAME call that writes an
//! overlapping place would be a second writer the borrow cannot coexist with
//! (fuzz H12: it used to ICE in OSSA verification).
//!
//! The same place passed to two `mutating` PARAMETERS (`swap(s, s)`) is
//! deliberately allowed: the callee sees two aliasing pointers and must cope
//! (as `swap`'s self-swap guard does).
//!
//! A READ-only access overlapping a `mutating` one is NOT reported: plain
//! borrowed arguments and read captures of closure literals see the value at
//! the call (MIR lowering snapshots them — `snapshot_overlapping_borrows`,
//! `prepare_closure_arg_snapshots`).
//!
//! Place identity is `kestrel_type_infer::captures::place_key_of`, the same
//! function the capture analysis uses, so `t(p.a, p.b)` (disjoint stored
//! fields) passes, and so does a closure writing a field disjoint from a
//! `mutating` argument (a view closure captures written fields by address).
//!
//! ## Diagnostics
//!
//! ### E635 — `overlapping_exclusive_access` (Error, Correctness)
//!
//! **Message:** "overlapping exclusive access in one call"
//!
//! **Labels:**
//! - Primary: the later exclusive access
//!   - Span source: `util::expr_span` on the argument / receiver / closure
//!     literal `HirExprId`
//!   - Message: "this writes a place already exclusively accessed in this call"
//! - Secondary: the earlier exclusive access
//!   - Span source: `util::expr_span` on that access's `HirExprId`
//!   - Message: "first exclusive access here"
//!
//! **Notes:**
//! - "a `mutating` argument and a closure that writes the same variable
//!   would both change it during the call; split the call"

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, BodyCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Callable, NodeKind, ReceiverKind};
use kestrel_hir::body::*;
use kestrel_type_infer::captures::{CaptureKind, PlaceKey, place_key_of};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E635",
    name: "overlapping_exclusive_access",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ExclusiveArgsAnalyzer;

impl Describe for ExclusiveArgsAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::ExclusiveArgs
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for ExclusiveArgsAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let captures = cx.query.query(kestrel_type_infer::ClosureCaptures {
            entity: cx.entity,
            root: cx.root,
        });
        let mut diags = Vec::new();
        for (expr_id, expr) in cx.hir.exprs.iter() {
            let (callee, receiver, args) = match expr {
                HirExpr::Call { callee, args, .. } => {
                    let entity = match &cx.hir.exprs[*callee] {
                        HirExpr::Def(entity, _, _) => Some(*entity),
                        _ => cx.typed.resolutions.get(callee).copied(),
                    };
                    (entity, None, args)
                },
                HirExpr::MethodCall { receiver, args, .. } => (
                    cx.typed.resolutions.get(&expr_id).copied(),
                    Some(*receiver),
                    args,
                ),
                HirExpr::ProtocolCall {
                    receiver,
                    protocol,
                    method,
                    args,
                    ..
                } => (
                    util::children_named_of_kind(
                        cx.query,
                        *protocol,
                        method.as_str_or_empty(),
                        NodeKind::Function,
                    )
                    .first()
                    .copied(),
                    Some(*receiver),
                    args,
                ),
                _ => continue,
            };
            let Some(callable) = callee.and_then(|c| cx.query.get::<Callable>(c)) else {
                continue;
            };
            let accesses = exclusive_accesses(cx, &captures, callable, receiver, args);
            report_first_overlap(cx, &accesses, &mut diags);
        }
        diags
    }
}

/// Every exclusive access the call makes, as (place, the expression making
/// it, whether it is a writing closure).
fn exclusive_accesses(
    cx: &BodyContext<'_>,
    captures: &kestrel_type_infer::captures::ClosureCaptureMap,
    callable: &Callable,
    receiver: Option<HirExprId>,
    args: &[HirCallArg],
) -> Vec<(PlaceKey, HirExprId, bool)> {
    let place = |e: HirExprId| place_key_of(cx.query, cx.typed, cx.hir, e);
    let mut out = Vec::new();
    if let Some(recv) = receiver
        && matches!(callable.receiver, Some(ReceiverKind::Mutating))
        && let Some(key) = place(recv)
    {
        out.push((key, recv, false));
    }
    for (i, arg) in args.iter().enumerate() {
        if matches!(cx.hir.exprs[arg.value], HirExpr::Closure { .. }) {
            // A closure literal's WRITE captures are exclusive accesses of
            // exactly the written place: a view closure captures a projected
            // place by its own address (`plan_slots`), so `w(p.a, { p.b = 3 })`
            // touches only `p.b`.
            for cp in captures.get(arg.value) {
                if cp.kind == CaptureKind::Write {
                    out.push((cp.key.clone(), arg.value, true));
                }
            }
            continue;
        }
        let Some(param) = callable.params.get(i) else {
            continue;
        };
        if param.is_mut
            && !param.is_consuming
            && let Some(key) = place(arg.value)
        {
            out.push((key, arg.value, false));
        }
    }
    out
}

/// One diagnostic per call: the first writing-closure access overlapping a
/// `mutating` argument/receiver (either order).
fn report_first_overlap(
    cx: &BodyContext<'_>,
    accesses: &[(PlaceKey, HirExprId, bool)],
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    for (i, (key, expr, closure)) in accesses.iter().enumerate() {
        let Some((_, first, _)) = accesses[..i].iter().find(|(k, e, c)| {
            c != closure && e != expr && (k.is_prefix_of(key) || key.is_prefix_of(k))
        }) else {
            continue;
        };
        let d = &DESCRIPTORS[0];
        diags.push(AnalyzeDiagnostic {
            descriptor_id: d.id,
            severity: d.default_severity,
            message: "overlapping exclusive access in one call".to_string(),
            labels: vec![
                DiagLabel {
                    span: util::expr_span(cx.hir, *expr),
                    message: "this writes a place already exclusively accessed in this call".into(),
                    is_primary: true,
                },
                DiagLabel {
                    span: util::expr_span(cx.hir, *first),
                    message: "first exclusive access here".into(),
                    is_primary: false,
                },
            ],
            notes: vec![
                "a `mutating` argument and a closure that writes the same variable would both \
                 change it during the call; split the call"
                    .to_string(),
            ],
        });
        return;
    }
}
