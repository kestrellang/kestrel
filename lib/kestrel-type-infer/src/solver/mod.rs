//! Constraint solving: `try_solve` for each constraint kind, `solve_now` for
//! the walker resolving one constraint where it meets it, and `fixpoint`
//! for flushing what is still queued. The end of a body — the flush, the
//! end-of-body defaults and the reports — is orchestrated by
//! [`crate::obligations::finish_body`].
//!
//! The rules live in one submodule per concern: `equality` (Equal, Coerce,
//! closure-kind reconciliation), `conformance` (Conforms / Applicable via the
//! kernel, copy and staticness layers), `assoc` (Associated, Reduce), `call`
//! (Call, OverloadedCall), `member` (Member, `.Case`, tuple projection,
//! interpolation), `defaults` (end-of-body rules) and `lower` (HIR types into
//! type vars). This file keeps the dispatch (`try_solve`), the queue
//! (`solve_now`, `fixpoint`) and the end-of-body reports.

use std::borrow::Cow;

use crate::constraint::{CallArg, ConformsOrigin, Constraint, Reason, labels_match};
use crate::ctx::InferCtx;
use crate::error::InferError;
use crate::generate::SubjectRoot;
use crate::resolve::EqualityLhs;
use crate::ty::{LiteralKind, TyKind, TySlot, TyVar};
use crate::unify::{self, UnifyError};
use kestrel_ast_builder::arg_binding::{BindError, BindParam, Binding, bind_arguments};
use kestrel_ast_builder::{Callable, InitEffect, Intrinsic, Name, NodeKind, TypeParams};
use kestrel_copy_fold::{CopyLayer, fold_members, instance_semantics};
use kestrel_hecs::Entity;
use kestrel_hir::Builtin;
use kestrel_hir::body::{HirBody, HirExpr};
use kestrel_hir_lower::{LowerCallableTypes, LowerTypeAnnotation};
use kestrel_name_res::ResolveBuiltin;
use kestrel_semantics::{
    ConditionalCopyableParams, CopySemantics, NominalCopySemantics, NominalStaticness, StaticLayer,
    StaticRequirement, Staticness, TypeParamCopyRequirement, TypeParamStaticRequirement,
    instance_is_static,
};
use kestrel_span::Span;

pub(crate) mod assoc;
mod call;
mod conformance;
mod defaults;
mod equality;
mod lower;
mod member;
use assoc::*;
use call::*;
pub(crate) use conformance::*;
pub(crate) use defaults::*;
pub(crate) use equality::*;
use lower::*;
use member::*;

/// Diagnose type parameters at generic call sites that inference couldn't
/// pin down. Lib1 silently defaulted these to `Never` (RFC 1216-style
/// fallback) — we deliberately don't, to avoid silent witness-selection
/// drift. Each unresolved slot becomes an `UnresolvedTypeParam` error
/// pointing at the call site, with the param's name in the label.
pub(crate) fn report_unresolved_type_params(ctx: &mut InferCtx<'_>) {
    // Snapshot to avoid borrow conflicts when we call `report_error`/`poison`.
    let entries: Vec<(kestrel_hir::body::HirExprId, Vec<TyVar>, Span)> = ctx
        .type_args
        .iter()
        .filter_map(|(&expr, tvs)| {
            ctx.type_arg_spans
                .get(&expr)
                .map(|sp| (expr, tvs.clone(), sp.clone()))
        })
        .collect();

    for (expr, tvs, span) in entries {
        // Map each TyVar back to its originating TypeParameter entity
        // via the resolved callee's `TypeParams` component. Without a
        // resolution (e.g. the call errored upstream) we can't name
        // the param, so skip — the upstream error already covers it.
        let Some(&callee) = ctx.resolutions.get(&expr) else {
            continue;
        };
        let params: Vec<Entity> = ctx
            .query_ctx
            .get::<TypeParams>(callee)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();

        for (i, &tv) in tvs.iter().enumerate() {
            let resolved = ctx.resolve(tv);
            // Only truly unconstrained slots — skip literals (they get
            // defaults in phase 2) and anything already poisoned.
            if !matches!(
                &ctx.types[resolved.0 as usize],
                TySlot::Unresolved { literal: None }
            ) {
                continue;
            }
            let Some(&param) = params.get(i) else {
                continue;
            };
            ctx.report_error(InferError::UnresolvedTypeParam {
                param,
                span: span.clone(),
            });
            ctx.poison(tv);
        }
    }
}

/// Bind any TyVar that `unify` saw meet `Never` (with the Unresolved
/// side deliberately left unbound) to `Never`, if fixpoint settled with
/// no other constraint pinning it. Rust's `never_type_fallback`.
pub(crate) fn default_never_fallback(ctx: &mut InferCtx<'_>) {
    let targets: Vec<TyVar> = ctx.never_fallback_targets.iter().copied().collect();
    for tv in targets {
        let resolved = ctx.resolve(tv);
        if matches!(&ctx.types[resolved.0 as usize], TySlot::Unresolved { .. }) {
            ctx.types[resolved.0 as usize] = TySlot::Resolved(TyKind::Never);
        }
    }
}

/// Diagnose expressions and locals whose TyVar never got pinned down by the
/// time solving finished. These are the slots that would otherwise silently
/// become `MirTy::Error` in lowering (e.g. from `ResolvedTy::Error` or a
/// leaked `HirTy::Infer`) and produce a Cranelift type-mismatch panic during
/// codegen. Points the diagnostic at the expression or local binding's span
/// so the user can add an annotation.
///
/// Skips slots that are already poisoned with `TyKind::Error` — an earlier
/// error already covers them.
pub(crate) fn report_unresolved_slots(ctx: &mut InferCtx<'_>, hir: &HirBody) {
    // Snapshot the entries so we can mutate `ctx` while iterating, in source
    // order (start, then the innermost span first). Order decides which
    // expression reports: reporting poisons the shared var, which silences
    // every later expression typed by it. In map order that winner — and
    // whether one or two errors came out — changed from run to run.
    let mut expr_entries: Vec<(kestrel_hir::body::HirExprId, TyVar)> =
        ctx.expr_types.iter().map(|(&id, &tv)| (id, tv)).collect();
    expr_entries.sort_by_key(|&(id, _)| {
        let span = expr_span(&hir.exprs[id]);
        (span.start, span.end, id.raw())
    });
    let mut local_entries: Vec<(kestrel_hir::res::LocalId, TyVar)> =
        ctx.local_types.iter().map(|(&id, &tv)| (id, tv)).collect();
    local_entries.sort_by_key(|&(id, _)| {
        let span = &hir.locals[id].span;
        (span.start, span.end, id.raw())
    });

    // Only report one diagnostic per statement/expression tree — otherwise a
    // single unresolvable subexpression produces a cascade of errors across
    // every parent expression that inherited its type.
    let mut seen_spans: crate::collections::HashSet<(usize, std::ops::Range<usize>)> =
        crate::collections::HashSet::default();

    for (id, tv) in expr_entries {
        let resolved = ctx.resolve(tv);
        // Skip literal root slots (`Unresolved { literal: Some(_) }`) — those
        // get a primitive fallback in `apply_literal_defaults`, and when no
        // default exists at all (stdlib absent, no lang primitive) the silent
        // Error-lowering is what diagnostics-only tests rely on. Do report
        // concrete containers with unresolved type args, e.g. `Dictionary[?, ?]`.
        let unresolved = matches!(
            &ctx.types[resolved.0 as usize],
            TySlot::Unresolved { literal: None }
        ) || contains_unresolved_type_args(ctx, resolved);
        if !unresolved {
            continue;
        }
        // Skip expressions whose HIR is `Error` — `FromHir` already covers them.
        let expr = &hir.exprs[id];
        if matches!(expr, HirExpr::Error { .. }) {
            continue;
        }
        let span = expr_span(expr);
        let key = (span.file_id, span.range());
        if !seen_spans.insert(key) {
            continue;
        }
        ctx.report_error(InferError::CannotInferType { span: span.clone() });
        ctx.poison(tv);
    }

    for (id, tv) in local_entries {
        let resolved = ctx.resolve(tv);
        let unresolved = matches!(
            &ctx.types[resolved.0 as usize],
            TySlot::Unresolved { literal: None }
        ) || contains_unresolved_type_args(ctx, resolved);
        if !unresolved {
            continue;
        }
        let local = &hir.locals[id];
        let key = (local.span.file_id, local.span.range());
        if !seen_spans.insert(key) {
            continue;
        }
        ctx.report_error(InferError::CannotInferType {
            span: local.span.clone(),
        });
        ctx.poison(tv);
    }
}

/// Poison `tv` if its resolved root is still fully Unresolved
/// (`Unresolved { literal: None }`). Used after a constraint error so
/// cascading "could not infer type" diagnostics don't fire on the same slot.
///
/// Intentionally skips TyVars with a literal marker: those carry real type
/// info (e.g. `Unresolved { literal: Some(Bool) }` from a `true` expression),
/// and subsequent coerce/equal constraints can still produce meaningful
/// errors against them. Poisoning would hide those legitimate errors.
pub(super) fn poison_if_unresolved(ctx: &mut InferCtx<'_>, tv: TyVar) {
    if matches!(
        ctx.slot(ctx.resolve(tv)),
        TySlot::Unresolved { literal: None }
    ) {
        ctx.poison(tv);
    }
}

/// Recursively poison any Unresolved type arguments nested inside `tv`'s
/// resolved type. Leaves the outer type intact — we only silence inner
/// slots so `contains_unresolved_type_args` doesn't cascade after a
/// structural mismatch (e.g. `null` default → `Optional[?]` vs concrete
/// `i64`).
pub(super) fn poison_unresolved_type_args(ctx: &mut InferCtx<'_>, tv: TyVar) {
    let mut seen = crate::collections::HashSet::default();
    poison_inner(ctx, tv, &mut seen);

    fn poison_inner(
        ctx: &mut InferCtx<'_>,
        tv: TyVar,
        seen: &mut crate::collections::HashSet<TyVar>,
    ) {
        let resolved = ctx.resolve(tv);
        if !seen.insert(resolved) {
            return;
        }
        let args: Vec<TyVar> = match ctx.slot(resolved).clone() {
            TySlot::Resolved(TyKind::Struct { args, .. })
            | TySlot::Resolved(TyKind::Enum { args, .. })
            | TySlot::Resolved(TyKind::Protocol { args, .. })
            | TySlot::Resolved(TyKind::TypeAlias { args, .. }) => args,
            TySlot::Resolved(TyKind::Tuple(elements)) => elements,
            TySlot::Resolved(TyKind::Function { params, ret, .. }) => {
                let mut v = params;
                v.push(ret);
                v
            },
            _ => return,
        };
        for arg in args {
            let arg_root = ctx.resolve(arg);
            if matches!(ctx.slot(arg_root), TySlot::Unresolved { .. }) {
                ctx.poison(arg_root);
            } else {
                poison_inner(ctx, arg_root, seen);
            }
        }
    }
}

pub(super) fn contains_unresolved_type_args(ctx: &InferCtx<'_>, tv: TyVar) -> bool {
    fn walk(ctx: &InferCtx<'_>, tv: TyVar, seen: &mut crate::collections::HashSet<TyVar>) -> bool {
        let resolved = ctx.resolve(tv);
        if !seen.insert(resolved) {
            return false;
        }
        match &ctx.types[resolved.0 as usize] {
            // Wildcards (from explicit `_` type args) are intentionally unresolved —
            // don't report them as inference failures.
            TySlot::Unresolved { literal: None } if ctx.wildcard_tvars.contains(&resolved) => false,
            TySlot::Unresolved { literal: None } => true,
            TySlot::Unresolved { literal: Some(_) } => false,
            TySlot::Redirect(target) => walk(ctx, *target, seen),
            TySlot::Resolved(TyKind::Struct { args, .. })
            | TySlot::Resolved(TyKind::Enum { args, .. })
            | TySlot::Resolved(TyKind::Protocol { args, .. })
            | TySlot::Resolved(TyKind::TypeAlias { args, .. }) => {
                args.iter().any(|&arg| walk(ctx, arg, seen))
            },
            TySlot::Resolved(TyKind::Tuple(elements)) => {
                elements.iter().any(|&elem| walk(ctx, elem, seen))
            },
            TySlot::Resolved(TyKind::Function { params, ret, .. }) => {
                params.iter().any(|&param| walk(ctx, param, seen)) || walk(ctx, *ret, seen)
            },
            TySlot::Resolved(TyKind::Ref { pointee, .. }) => walk(ctx, *pointee, seen),
            TySlot::Resolved(TyKind::AssocProjection { base, .. }) => walk(ctx, *base, seen),
            TySlot::Resolved(TyKind::Opaque {
                bounds,
                origin_args,
                ..
            }) => {
                bounds
                    .iter()
                    .any(|(_, args)| args.iter().any(|&a| walk(ctx, a, seen)))
                    || origin_args.iter().any(|&a| walk(ctx, a, seen))
            },
            TySlot::Resolved(
                TyKind::Param { .. } | TyKind::SelfType { .. } | TyKind::Never | TyKind::Error,
            ) => false,
        }
    }

    let mut seen = crate::collections::HashSet::default();
    match ctx.slot(tv) {
        TySlot::Resolved(_) => walk(ctx, tv, &mut seen),
        _ => false,
    }
}

/// Extract the `Span` from any `HirExpr` variant.
pub(super) fn expr_span(expr: &HirExpr) -> &Span {
    match expr {
        HirExpr::Literal { span, .. }
        | HirExpr::Tuple { span, .. }
        | HirExpr::Array { span, .. }
        | HirExpr::Dict { span, .. }
        | HirExpr::Closure { span, .. }
        | HirExpr::Local(_, span)
        | HirExpr::Def(_, _, span)
        | HirExpr::TypeRef { span, .. }
        | HirExpr::OverloadSet { span, .. }
        | HirExpr::Borrow { span, .. }
        | HirExpr::Field { span, .. }
        | HirExpr::TupleIndex { span, .. }
        | HirExpr::ImplicitMember { span, .. }
        | HirExpr::Call { span, .. }
        | HirExpr::MethodCall { span, .. }
        | HirExpr::ProtocolCall { span, .. }
        | HirExpr::If { span, .. }
        | HirExpr::Loop { span, .. }
        | HirExpr::Match { span, .. }
        | HirExpr::Break { span, .. }
        | HirExpr::Continue { span, .. }
        | HirExpr::Return { span, .. }
        | HirExpr::Assign { span, .. }
        | HirExpr::Block { span, .. }
        | HirExpr::Error { span }
        | HirExpr::Sugar { span, .. } => span,
    }
}

/// Run rounds until no progress, with a safety cap to prevent unbounded spins
/// from misbehaving constraint interactions (e.g. reducible types that keep
/// marking "progress" without actually converging).
/// Where a ref was found relative to the expression's type root.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum RefPos {
    Top,
    /// A nominal type argument or tuple element — a LEGAL ref position
    /// since stage 2b (ref-bearing aggregates); recorded so the walk can
    /// keep hunting deeper for the still-illegal positions.
    Slot,
    Nested,
    InFn,
}

/// Depth-first search for an illegally-placed `TyKind::Ref` in a type tree.
/// Returns the violating position, or None. Legal positions (stage 2b): the
/// top level (a ref-returning call result) and aggregate slots (nominal
/// type args, tuple elements — the HIR formation walk checks annotations;
/// inference can't mint them elsewhere). Still-illegal backstops: function
/// types (E491), protocol args / opaque bounds / assoc-projection bases
/// (E492 — positions the formation walk also still rejects).
pub(super) fn find_ref_violation(
    ctx: &InferCtx<'_>,
    tv: TyVar,
    pos: RefPos,
    seen: &mut crate::collections::HashSet<TyVar>,
) -> Option<RefPos> {
    let r = ctx.resolve(tv);
    if !seen.insert(r) {
        return None;
    }
    let kind = match ctx.slot(r) {
        TySlot::Resolved(k) => k.clone(),
        _ => return None,
    };
    match kind {
        TyKind::Ref { pointee, .. } => match pos {
            RefPos::Top | RefPos::Slot => find_ref_violation(ctx, pointee, RefPos::Slot, seen),
            _ => Some(pos),
        },
        TyKind::Struct { args, .. }
        | TyKind::Enum { args, .. }
        | TyKind::TypeAlias { args, .. } => args
            .iter()
            .find_map(|&a| find_ref_violation(ctx, a, RefPos::Slot, seen)),
        TyKind::Protocol { args, .. } => args
            .iter()
            .find_map(|&a| find_ref_violation(ctx, a, RefPos::Nested, seen)),
        TyKind::Tuple(elems) => elems
            .iter()
            .find_map(|&e| find_ref_violation(ctx, e, RefPos::Slot, seen)),
        TyKind::Function { params, ret, .. } => params
            .iter()
            .chain(std::iter::once(&ret))
            .find_map(|&p| find_ref_violation(ctx, p, RefPos::InFn, seen)),
        TyKind::AssocProjection { base, .. } => find_ref_violation(ctx, base, RefPos::Nested, seen),
        TyKind::Opaque {
            bounds,
            origin_args,
            ..
        } => bounds
            .iter()
            .flat_map(|(_, args)| args.iter())
            .chain(origin_args.iter())
            .find_map(|&a| find_ref_violation(ctx, a, RefPos::Nested, seen)),
        _ => None,
    }
}

/// Stage-1 second-class enforcement over the solved expression types.
pub(crate) fn validate_ref_placement(ctx: &mut InferCtx<'_>, hir: &HirBody) {
    let entries: Vec<(kestrel_hir::body::HirExprId, TyVar)> =
        ctx.expr_types.iter().map(|(k, v)| (*k, *v)).collect();
    for (expr_id, tv) in entries {
        if ctx.direct_callee_exprs.contains(&expr_id) {
            continue;
        }
        let mut seen = crate::collections::HashSet::default();
        let Some(pos) = find_ref_violation(ctx, tv, RefPos::Top, &mut seen) else {
            continue;
        };
        let span = expr_span(&hir.exprs[expr_id]).clone();
        let err = match pos {
            RefPos::InFn => InferError::RefFunctionAsValue { span },
            _ => InferError::RefInTypeArgument { span },
        };
        ctx.report_error(err);
    }
}

pub(crate) fn fixpoint(ctx: &mut InferCtx<'_>) {
    // The cap only guards against a non-converging solver. A long dependency
    // chain (512 nested operators) can need one round per link, so it scales
    // with the body: a fixed 256 rejected valid deep expressions.
    const MIN_ROUNDS: usize = 256;
    let max_rounds = MIN_ROUNDS.max(ctx.constraints.len() * 2);
    for _ in 0..max_rounds {
        let progress = solve_round(ctx);
        if !progress {
            return;
        }
    }
    // Hit the cap — leave remaining constraints for report_unsolved to error out on.
}

/// One round of constraint solving: drain all constraints, try to solve each.
/// Returns true if any progress was made.
/// Solve one constraint right away (the checker resolving a member as it
/// walks, bidi §2.3): solved, re-queued if it still waits, or reported.
pub(crate) fn solve_now(ctx: &mut InferCtx<'_>, constraint: Constraint) {
    let start = ctx.constraints.len();
    let result = try_solve(ctx, constraint);
    // Follow-ups the decision queued (a member's declared type, a call's
    // parameter equalities). From the walker they are settled before it
    // moves on, so the next constraint sees what this one implied (bidi P5);
    // each is tried once and one still waiting stays queued.
    let followups = if ctx.solving_depth == 0 {
        ctx.constraints.split_off(start)
    } else {
        Vec::new()
    };
    match result {
        SolveResult::Solved => {},
        SolveResult::Deferred(c) => ctx.constraints.push(c),
        SolveResult::Error(err) => {
            ctx.report_error(err);
        },
    }
    for c in followups {
        solve_now(ctx, c);
    }
}

pub(super) fn solve_round(ctx: &mut InferCtx<'_>) -> bool {
    let mut progress = false;
    let constraints = std::mem::take(&mut ctx.constraints);

    for constraint in constraints {
        match try_solve(ctx, constraint) {
            SolveResult::Solved => progress = true,
            SolveResult::Deferred(c) => ctx.constraints.push(c),
            SolveResult::Error(err) => {
                ctx.report_error(err);
                progress = true; // error counts as progress (removes constraint)
            },
        }
    }

    progress
}

/// Report remaining unsolved constraints as errors.
///
/// After the fixpoint loop completes, any constraints still in `ctx.constraints`
/// could never be solved (typically because one side stayed unresolved — e.g.,
/// a literal without stdlib to provide a default type). Each constraint maps to
/// an appropriate InferError variant.
///
/// To prevent cascading errors, we skip constraints where a key TyVar is already
/// poisoned with `TyKind::Error` (meaning an earlier error already covers it).
pub(crate) fn report_unsolved(ctx: &mut InferCtx<'_>) {
    let constraints = std::mem::take(&mut ctx.constraints);

    for constraint in constraints {
        // `poison_tv` is the TyVar to mark as Error after reporting so the
        // downstream `report_unresolved_slots` (phase 4.5) doesn't double-
        // diagnose the same slot with the generic "could not infer type".
        let (err, poison_tv) = match constraint {
            Constraint::Equal { a, b, span, reason } => {
                let a_err = ctx.is_error(ctx.resolve(a));
                let b_err = ctx.is_error(ctx.resolve(b));
                if a_err || b_err {
                    // Propagate poison to the other side if it's still Unresolved,
                    // so report_unresolved_slots doesn't cascade "could not infer type".
                    if a_err && matches!(ctx.slot(ctx.resolve(b)), TySlot::Unresolved { .. }) {
                        ctx.poison(b);
                    }
                    if b_err && matches!(ctx.slot(ctx.resolve(a)), TySlot::Unresolved { .. }) {
                        ctx.poison(a);
                    }
                    continue;
                }
                let err = mismatch_error(ctx, a, b, span).with_reason(&reason);
                report_and_poison(ctx, err, a, b);
                continue;
            },
            Constraint::Coerce {
                from,
                to,
                expr,
                span,
                reason,
            } => {
                let from_err = ctx.is_error(ctx.resolve(from));
                let to_err = ctx.is_error(ctx.resolve(to));
                if from_err || to_err {
                    // Propagate poison to unresolved sides to suppress cascading errors.
                    if from_err && matches!(ctx.slot(ctx.resolve(to)), TySlot::Unresolved { .. }) {
                        ctx.poison(to);
                    }
                    if to_err && matches!(ctx.slot(ctx.resolve(from)), TySlot::Unresolved { .. }) {
                        ctx.poison(from);
                    }
                    continue;
                }
                if ctx.errored_coerce_exprs.contains(&expr) {
                    continue;
                }
                let err = mismatch_error(ctx, to, from, span).with_reason(&reason);
                ctx.errored_coerce_exprs.insert(expr);
                report_and_poison(ctx, err, from, to);
                continue;
            },
            Constraint::BorrowPointee { pointee, .. } => {
                // Unsolved = `inner` never resolved; its own diagnosis (or
                // the generic could-not-infer) covers it. Poison the pointee
                // so the enclosing Ref doesn't cascade a second error.
                if matches!(ctx.slot(ctx.resolve(pointee)), TySlot::Unresolved { .. }) {
                    ctx.poison(pointee);
                }
                continue;
            },
            Constraint::EqualDecayed {
                value,
                target,
                span,
                reason,
            } => {
                // `apply_ref_decay_defaults` settles every deferred decay,
                // so an unsolved one means a side never resolved — report
                // like Equal (poison-propagation included).
                let v_err = ctx.is_error(ctx.resolve(value));
                let t_err = ctx.is_error(ctx.resolve(target));
                if v_err || t_err {
                    if v_err && matches!(ctx.slot(ctx.resolve(target)), TySlot::Unresolved { .. }) {
                        ctx.poison(target);
                    }
                    if t_err && matches!(ctx.slot(ctx.resolve(value)), TySlot::Unresolved { .. }) {
                        ctx.poison(value);
                    }
                    continue;
                }
                let err = mismatch_error(ctx, target, value, span).with_reason(&reason);
                report_and_poison(ctx, err, value, target);
                continue;
            },
            Constraint::AssignTarget {
                value,
                target,
                span,
                ..
            } => {
                // An unsolved AssignTarget means its target never resolved
                // (the former stall-breaker that turned it into a plain
                // Coerce never fired over the corpus and was removed in
                // bidi P4) — report like Equal (poison-propagation
                // included), as EqualDecayed does.
                let v_err = ctx.is_error(ctx.resolve(value));
                let t_err = ctx.is_error(ctx.resolve(target));
                if v_err || t_err {
                    if v_err && matches!(ctx.slot(ctx.resolve(target)), TySlot::Unresolved { .. }) {
                        ctx.poison(target);
                    }
                    if t_err && matches!(ctx.slot(ctx.resolve(value)), TySlot::Unresolved { .. }) {
                        ctx.poison(value);
                    }
                    continue;
                }
                let err = mismatch_error(ctx, target, value, span);
                report_and_poison(ctx, err, value, target);
                continue;
            },
            Constraint::Conforms {
                ty,
                protocol,
                span,
                poison_ty_on_failure,
                origin: _,
            } => {
                if ctx.is_error(ctx.resolve(ty)) {
                    continue;
                }
                // A literal default's well-formedness check (`Array[?E]: E:
                // Static`) on an element var that never resolved (`[]` with
                // no context): the var gets its own "could not infer".
                if matches!(ctx.slot(ctx.resolve(ty)), TySlot::Unresolved { .. })
                    && ctx
                        .literal_default_args
                        .iter()
                        .any(|&a| ctx.resolve(a) == ctx.resolve(ty))
                {
                    continue;
                }
                // Re-queued as Unknown because a type arg never resolved: the
                // open var gets its own "could not infer" diagnostic; a
                // "does not conform" on top would blame the wrong thing.
                if matches!(ctx.slot(ctx.resolve(ty)), TySlot::Resolved(_))
                    && crate::solver_entail::has_open_var(ctx, ty)
                {
                    kestrel_debug::ktrace!("entail", "conformance left undecided by an open var");
                    continue;
                }
                if poison_ty_on_failure {
                    ctx.poison(ty);
                }
                (InferError::DoesNotConform { ty, protocol, span }, Some(ty))
            },
            Constraint::Associated {
                container,
                key,
                result,
                span,
            } => {
                let name = ctx.assoc_key_name(&key);
                let resolved = ctx.resolve(container);
                if ctx.is_error(resolved) {
                    ctx.poison(result);
                    continue;
                }
                // Container stayed an unresolved literal TyVar — the only way
                // this happens is when no `Default<Kind>LiteralType` builtin
                // exists (e.g. stdlib disabled in a diagnostics test) or the
                // literal was already flagged in a prior mismatch. Either way,
                // any associated-type lookup is a cascading non-answer.
                if matches!(ctx.slot(resolved), TySlot::Unresolved { literal: Some(_) }) {
                    continue;
                }
                (
                    InferError::NoAssociatedType {
                        container,
                        name,
                        span,
                    },
                    Some(result),
                )
            },
            Constraint::Member {
                receiver,
                name,
                span,
                is_call,
                result,
                ..
            } => {
                if ctx.is_error(ctx.resolve(receiver)) {
                    ctx.poison(result);
                    continue;
                }
                (
                    InferError::NoMember {
                        receiver,
                        name,
                        is_call,
                        span,
                    },
                    Some(result),
                )
            },
            Constraint::Call {
                callee,
                result,
                span,
                ..
            } => {
                if ctx.is_error(ctx.resolve(callee)) {
                    ctx.poison(result);
                    continue;
                }
                (
                    InferError::NoMember {
                        receiver: callee,
                        name: "subscript".into(),
                        is_call: true,
                        span,
                    },
                    Some(result),
                )
            },
            Constraint::OverloadedCall {
                candidates,
                result,
                span,
                ..
            } => {
                if ctx.is_error(ctx.resolve(result)) {
                    continue;
                }
                let name = candidates
                    .first()
                    .and_then(|&e| ctx.query_ctx.get::<Name>(e))
                    .map(|n| n.0.clone())
                    .unwrap_or_else(|| "<overloaded>".into());
                (
                    InferError::NoMember {
                        receiver: result,
                        name,
                        is_call: true,
                        span,
                    },
                    Some(result),
                )
            },
            Constraint::Implicit {
                expected,
                name,
                span,
                ..
            } => {
                if ctx.is_error(ctx.resolve(expected)) {
                    continue;
                }
                (
                    InferError::ImplicitMemberNotFound {
                        expected,
                        name,
                        span,
                    },
                    Some(expected),
                )
            },
            // Pattern matching handles unresolved patterns at a higher level —
            // but poison any bound arg TyVars so cascading "could not infer type"
            // doesn't fire on pattern bindings whose payload type was never
            // resolved (e.g. `.Some(x)` on a scrutinee that stayed unresolved).
            Constraint::ImplicitPat { arg_tys, .. } => {
                for tv in &arg_tys {
                    poison_if_unresolved(ctx, *tv);
                }
                continue;
            },
            Constraint::TupleRestPat {
                prefix_tys,
                suffix_tys,
                ..
            } => {
                for tv in prefix_tys.iter().chain(suffix_tys.iter()) {
                    poison_if_unresolved(ctx, *tv);
                }
                continue;
            },
            // An unresolved Reduce means the alias never became concrete.
            // Absorb silently — any error will surface through the downstream
            // constraint that needed the reduction.
            Constraint::Reduce { .. } => continue,
            // Unresolved TupleIndex: the base never became concrete. Poison
            // the result so downstream `could not infer type` doesn't cascade.
            Constraint::TupleIndex { result, .. } => {
                ctx.poison(result);
                continue;
            },
            // Unresolved InterpolationLink: the accumulator was resolved by
            // literal defaults. Absorb silently.
            Constraint::InterpolationLink { .. } => continue,
            // Still Unknown at body end: the receiver's open var is reported
            // by its own "could not infer" diagnostic, so this obligation
            // adds nothing a user could act on.
            Constraint::Applicable {
                receiver,
                source,
                name,
                is_call,
                span,
            } => {
                let verdict = match ctx.slot(ctx.resolve(receiver)) {
                    TySlot::Resolved(kind) => applicability(ctx, source, &kind.clone()),
                    _ => kestrel_entail::Tri::Unknown,
                };
                if !verdict.fails() || ctx.is_error(ctx.resolve(receiver)) {
                    kestrel_debug::ktrace!("entail", "applicability of '{name}' never decided");
                    continue;
                }
                (
                    InferError::NoMember {
                        receiver,
                        name,
                        is_call,
                        span,
                    },
                    None,
                )
            },
        };
        ctx.report_error(err);
        if let Some(tv) = poison_tv {
            ctx.poison(tv);
        }
    }
}

/// Result of attempting to solve a single constraint.
pub(super) enum SolveResult {
    /// Constraint was fully resolved.
    Solved,
    /// Not enough info yet — put back for later.
    Deferred(Constraint),
    /// Constraint is unsatisfiable — report error.
    Error(InferError),
}

/// Carry a constraint's `Reason` into what solving it produced: a type
/// mismatch takes it as its cause, and a deferred Equal / Coerce /
/// EqualDecayed — the same check, retried later — keeps it.
pub(super) fn carry_reason(r: SolveResult, reason: &Reason) -> SolveResult {
    match r {
        SolveResult::Error(e) => SolveResult::Error(e.with_reason(reason)),
        SolveResult::Deferred(mut c) => {
            if let Constraint::Equal { reason: r, .. }
            | Constraint::Coerce { reason: r, .. }
            | Constraint::EqualDecayed { reason: r, .. } = &mut c
                && *r == Reason::Unspecified
            {
                *r = reason.clone();
            }
            SolveResult::Deferred(c)
        },
        SolveResult::Solved => SolveResult::Solved,
    }
}

/// Dispatch a constraint to the appropriate solver.
pub(super) fn try_solve(ctx: &mut InferCtx<'_>, c: Constraint) -> SolveResult {
    ctx.solving_depth += 1;
    kestrel_debug::ktrace!("p5-solve", "d={} {:?}", ctx.solving_depth, c);
    let r = try_solve_inner(ctx, c);
    kestrel_debug::ktrace!(
        "p5-solve",
        "  -> {}",
        match &r {
            SolveResult::Solved => "solved",
            SolveResult::Deferred(_) => "deferred",
            SolveResult::Error(_) => "ERROR",
        }
    );
    ctx.solving_depth -= 1;
    r
}

pub(super) fn try_solve_inner(ctx: &mut InferCtx<'_>, c: Constraint) -> SolveResult {
    match c {
        Constraint::Equal { a, b, span, reason } => {
            let r = carry_reason(solve_equal(ctx, a, b, span), &reason);
            if matches!(r, SolveResult::Error(_)) {
                poison_if_unresolved(ctx, a);
                poison_if_unresolved(ctx, b);
                poison_unresolved_type_args(ctx, a);
                poison_unresolved_type_args(ctx, b);
            }
            r
        },
        Constraint::Coerce {
            from,
            to,
            expr,
            span,
            reason,
        } => {
            let r = carry_reason(solve_coerce(ctx, from, to, expr, span), &reason);
            if matches!(r, SolveResult::Error(_)) {
                // Cascade suppression: poison unresolved sides AND inner type
                // args so expressions with a literal default (e.g. `null` →
                // `Optional[?]`) don't report "could not infer type" for the
                // unbound inner slot after an outer mismatch.
                poison_if_unresolved(ctx, from);
                poison_if_unresolved(ctx, to);
                poison_unresolved_type_args(ctx, from);
                poison_unresolved_type_args(ctx, to);
            }
            r
        },
        Constraint::BorrowPointee {
            inner,
            pointee,
            span,
        } => solve_borrow_pointee(ctx, inner, pointee, span),
        Constraint::EqualDecayed {
            value,
            target,
            span,
            reason,
        } => carry_reason(solve_equal_decayed(ctx, value, target, span), &reason),
        Constraint::AssignTarget {
            value,
            target,
            expr,
            span,
        } => solve_assign_target(ctx, value, target, expr, span),
        Constraint::Conforms {
            ty,
            protocol,
            span,
            poison_ty_on_failure,
            origin,
        } => solve_conforms(ctx, ty, protocol, span, poison_ty_on_failure, origin),
        Constraint::Associated {
            container,
            key,
            result,
            span,
        } => solve_associated(ctx, container, key, result, span),
        Constraint::Call {
            callee,
            args,
            result,
            expr,
            span,
        } => {
            let r = solve_call(ctx, callee, args, result, expr, span);
            if matches!(r, SolveResult::Error(_)) {
                ctx.poison(result);
            }
            r
        },
        Constraint::Member {
            receiver,
            name,
            args,
            result,
            expr,
            is_call,
            is_static_context,
            explicit_type_args,
            span,
        } => {
            let r = solve_member(
                ctx,
                receiver,
                &name,
                args,
                result,
                expr,
                is_call,
                is_static_context,
                &explicit_type_args,
                span,
            );
            if matches!(r, SolveResult::Error(_)) {
                ctx.poison(result);
            }
            r
        },
        Constraint::OverloadedCall {
            candidates,
            type_args,
            args,
            result,
            expr,
            span,
        } => {
            let r = solve_overloaded_call(ctx, candidates, type_args, args, result, expr, span);
            if matches!(r, SolveResult::Error(_)) {
                ctx.poison(result);
            }
            r
        },
        Constraint::Implicit {
            expected,
            name,
            args,
            result,
            expr,
            span,
        } => {
            let r = solve_implicit(ctx, expected, &name, args, result, expr, span);
            if matches!(r, SolveResult::Error(_)) {
                ctx.poison(result);
            }
            r
        },
        Constraint::ImplicitPat {
            scrutinee,
            name,
            arg_tys,
            labels,
            span,
        } => solve_implicit_pat(ctx, scrutinee, &name, arg_tys, labels, span),
        Constraint::TupleRestPat {
            scrutinee,
            prefix_tys,
            suffix_tys,
            span,
        } => solve_tuple_rest_pat(ctx, scrutinee, prefix_tys, suffix_tys, span),
        Constraint::Reduce {
            alias,
            result,
            span,
        } => solve_reduce(ctx, alias, result, span),
        Constraint::TupleIndex {
            tuple,
            index,
            result,
            span,
        } => {
            let r = solve_tuple_index(ctx, tuple, index, result, span);
            if matches!(r, SolveResult::Error(_)) {
                ctx.poison(result);
            }
            r
        },

        Constraint::InterpolationLink {
            result_tv,
            acc_tv,
            span,
        } => solve_interpolation_link(ctx, result_tv, acc_tv, span),

        Constraint::Applicable {
            receiver,
            source,
            name,
            is_call,
            span,
        } => solve_applicable(ctx, receiver, source, name, is_call, span),
    }
}
