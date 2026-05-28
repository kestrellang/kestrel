//! Fixpoint solver: iterates constraints until no more progress is made.
//!
//! Phases:
//! 1. Main solving — iterate until fixpoint
//! 2. Apply literal defaults for unconstrained literals
//! 3. Solve again with defaults applied
//! 4. Report any unresolved generic type parameters at call sites
//! 5. Report any remaining unsolved constraints as errors

use crate::constraint::{CallArg, Constraint, labels_match};
use crate::ctx::InferCtx;
use crate::error::InferError;
use crate::ty::{LiteralKind, TyKind, TySlot, TyVar};
use crate::unify::{self, UnifyError};
use kestrel_ast_builder::{Callable, InitEffect, Intrinsic, Name, NodeKind, TypeParams};
use kestrel_hecs::Entity;
use kestrel_hir::Builtin;
use kestrel_hir::body::{HirBody, HirExpr};
use kestrel_hir_lower::{LowerCallableTypes, LowerTypeAnnotation};
use kestrel_span::Span;

/// Run the full solver: fixpoint loop, literal defaults, final fixpoint,
/// then report any remaining unsolved constraints as errors.
pub fn solve(ctx: &mut InferCtx<'_>, hir: &HirBody) {
    // Phase 1: main solving
    fixpoint(ctx);

    // Phase 2: literal defaults with graduated blocking relaxation.
    //
    // Level 0 (strict): InterpolationLink accumulators and arg-position
    //   literals in deferred Member/Call constraints are both blocked.
    // Level 1 (relaxed): InterpolationLink blocking lifted — string
    //   interpolation literals default to their type, unblocking
    //   downstream chains (bytes subscript → UInt8 → == context).
    // Level 2 (force): all blocking removed — genuinely unconstrained
    //   literals get their fallback default.
    let mut relax_level = 0u8;
    loop {
        let progress = apply_literal_defaults(ctx, relax_level);
        if progress {
            fixpoint(ctx);
            relax_level = 0;
            continue;
        }
        relax_level += 1;
        if relax_level > 2 {
            break;
        }
    }

    // Phase 3: solve again with defaults
    fixpoint(ctx);

    // Phase 3.5: apply deferred type-parameter defaults (e.g. H = DefaultHasher).
    // Only unconstrained TyVars get their default — inside a generic body
    // like Set.init(), the TyVar for H is already constrained by `self.dict`'s
    // field type and stays untouched.
    if apply_type_param_defaults(ctx) {
        fixpoint(ctx);
    }

    // Phase 4: report unresolved type parameters at generic call sites.
    // Runs before `report_unsolved` so unresolved TyVars are poisoned
    // and downstream cascade constraints get suppressed silently.
    report_unresolved_type_params(ctx);

    // Phase 4.25: never-fallback. Any TyVar that `unify` saw meet `Never`
    // while still unresolved (so the Never-branch intentionally didn't
    // bind it — see `unify.rs`) but which fixpoint hasn't since pinned to
    // anything else gets defaulted to `Never` now. Mirror of Rust's
    // `never_type_fallback`: divergent match arms, branches that always
    // return, loops whose only break values are `!`, etc.
    default_never_fallback(ctx);

    // Phase 5: report remaining unsolved constraints as errors. Runs before
    // phase 4.5 so real `TypeMismatch` / `DoesNotConform` errors fire on
    // deferred `Equal` / `Coerce` constraints (and poison their TyVars via
    // `report_and_poison`) before the generic "could not infer type" fallback
    // gets a chance to eat them.
    report_unsolved(ctx);

    // Phase 4.5: report any expression or local whose TyVar stayed unresolved.
    // These slots would otherwise surface as `MirTy::Error` downstream and
    // trigger misleading Cranelift type-mismatch panics. Runs after phase 5
    // so constraints that can name a better error already have.
    report_unresolved_slots(ctx, hir);
}

/// Diagnose type parameters at generic call sites that inference couldn't
/// pin down. Lib1 silently defaulted these to `Never` (RFC 1216-style
/// fallback) — we deliberately don't, to avoid silent witness-selection
/// drift. Each unresolved slot becomes an `UnresolvedTypeParam` error
/// pointing at the call site, with the param's name in the label.
fn report_unresolved_type_params(ctx: &mut InferCtx<'_>) {
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
fn default_never_fallback(ctx: &mut InferCtx<'_>) {
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
fn report_unresolved_slots(ctx: &mut InferCtx<'_>, hir: &HirBody) {
    // Snapshot the entries so we can mutate `ctx` while iterating.
    let expr_entries: Vec<(kestrel_hir::body::HirExprId, TyVar)> =
        ctx.expr_types.iter().map(|(&id, &tv)| (id, tv)).collect();
    let local_entries: Vec<(kestrel_hir::res::LocalId, TyVar)> =
        ctx.local_types.iter().map(|(&id, &tv)| (id, tv)).collect();

    // Only report one diagnostic per statement/expression tree — otherwise a
    // single unresolvable subexpression produces a cascade of errors across
    // every parent expression that inherited its type.
    let mut seen_spans: std::collections::HashSet<(usize, std::ops::Range<usize>)> =
        std::collections::HashSet::new();

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
fn poison_if_unresolved(ctx: &mut InferCtx<'_>, tv: TyVar) {
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
fn poison_unresolved_type_args(ctx: &mut InferCtx<'_>, tv: TyVar) {
    let mut seen = std::collections::HashSet::new();
    poison_inner(ctx, tv, &mut seen);

    fn poison_inner(
        ctx: &mut InferCtx<'_>,
        tv: TyVar,
        seen: &mut std::collections::HashSet<TyVar>,
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
            TySlot::Resolved(TyKind::Function { params, ret }) => {
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

fn contains_unresolved_type_args(ctx: &InferCtx<'_>, tv: TyVar) -> bool {
    fn walk(ctx: &InferCtx<'_>, tv: TyVar, seen: &mut std::collections::HashSet<TyVar>) -> bool {
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
            TySlot::Resolved(TyKind::Function { params, ret }) => {
                params.iter().any(|&param| walk(ctx, param, seen)) || walk(ctx, *ret, seen)
            },
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

    let mut seen = std::collections::HashSet::new();
    match ctx.slot(tv) {
        TySlot::Resolved(_) => walk(ctx, tv, &mut seen),
        _ => false,
    }
}

/// Extract the `Span` from any `HirExpr` variant.
fn expr_span(expr: &HirExpr) -> &Span {
    match expr {
        HirExpr::Literal { span, .. }
        | HirExpr::Tuple { span, .. }
        | HirExpr::Array { span, .. }
        | HirExpr::Dict { span, .. }
        | HirExpr::Closure { span, .. }
        | HirExpr::Local(_, span)
        | HirExpr::Def(_, _, span)
        | HirExpr::OverloadSet { span, .. }
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
fn fixpoint(ctx: &mut InferCtx<'_>) {
    const MAX_ROUNDS: usize = 256;
    for _ in 0..MAX_ROUNDS {
        let progress = solve_round(ctx);
        if !progress {
            return;
        }
    }
    // Hit the cap — leave remaining constraints for report_unsolved to error out on.
}

/// One round of constraint solving: drain all constraints, try to solve each.
/// Returns true if any progress was made.
fn solve_round(ctx: &mut InferCtx<'_>) -> bool {
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
fn report_unsolved(ctx: &mut InferCtx<'_>) {
    let constraints = std::mem::take(&mut ctx.constraints);

    for constraint in constraints {
        // `poison_tv` is the TyVar to mark as Error after reporting so the
        // downstream `report_unresolved_slots` (phase 4.5) doesn't double-
        // diagnose the same slot with the generic "could not infer type".
        let (err, poison_tv) = match constraint {
            Constraint::Equal { a, b, span } => {
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
                let err = mismatch_error(ctx, a, b, span);
                report_and_poison(ctx, err, a, b);
                continue;
            },
            Constraint::Coerce {
                from,
                to,
                expr,
                span,
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
                let err = mismatch_error(ctx, to, from, span);
                ctx.errored_coerce_exprs.insert(expr);
                report_and_poison(ctx, err, from, to);
                continue;
            },
            Constraint::Conforms {
                ty,
                protocol,
                span,
                poison_ty_on_failure,
            } => {
                if ctx.is_error(ctx.resolve(ty)) {
                    continue;
                }
                if poison_ty_on_failure {
                    ctx.poison(ty);
                }
                (InferError::DoesNotConform { ty, protocol, span }, Some(ty))
            },
            Constraint::Associated {
                container,
                name,
                result,
                span,
            } => {
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
        };
        ctx.report_error(err);
        if let Some(tv) = poison_tv {
            ctx.poison(tv);
        }
    }
}

/// Result of attempting to solve a single constraint.
enum SolveResult {
    /// Constraint was fully resolved.
    Solved,
    /// Not enough info yet — put back for later.
    Deferred(Constraint),
    /// Constraint is unsatisfiable — report error.
    Error(InferError),
}

/// Dispatch a constraint to the appropriate solver.
fn try_solve(ctx: &mut InferCtx<'_>, c: Constraint) -> SolveResult {
    match c {
        Constraint::Equal { a, b, span } => {
            let r = solve_equal(ctx, a, b, span);
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
        } => {
            let r = solve_coerce(ctx, from, to, expr, span);
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
        Constraint::Conforms {
            ty,
            protocol,
            span,
            poison_ty_on_failure,
        } => solve_conforms(ctx, ty, protocol, span, poison_ty_on_failure),
        Constraint::Associated {
            container,
            name,
            result,
            span,
        } => solve_associated(ctx, container, &name, result, span),
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
            span,
        } => solve_implicit_pat(ctx, scrutinee, &name, arg_tys, span),
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
    }
}

/// Resolve a string interpolation's accumulator type from the result type.
/// If the accumulator is already concrete (from literal defaults), this is a
/// no-op. If the result type is String (the default), also a no-op — literal
/// defaults handle it. Otherwise, delegates to `solve_associated` to resolve
/// `ResultType.Interpolation` and unify with the accumulator type variable.
fn solve_interpolation_link(
    ctx: &mut InferCtx<'_>,
    result_tv: TyVar,
    acc_tv: TyVar,
    span: Span,
) -> SolveResult {
    // Accumulator already resolved (from literal defaults) — nothing to do.
    if ctx.is_concrete(ctx.resolve(acc_tv)) || ctx.is_error(ctx.resolve(acc_tv)) {
        return SolveResult::Solved;
    }

    // Wait until the result type is concrete.
    let result_resolved = ctx.resolve(result_tv);
    if !ctx.is_concrete(result_resolved) {
        return SolveResult::Deferred(Constraint::InterpolationLink {
            result_tv,
            acc_tv,
            span,
        });
    }

    if ctx.is_error(result_resolved) {
        ctx.poison(acc_tv);
        return SolveResult::Solved;
    }

    // If the result type is String (the default path), let literal defaults
    // handle the accumulator — String's conformance to
    // ExpressibleByStringInterpolation is compiler-synthesized.
    if let TySlot::Resolved(TyKind::Struct { entity, .. }) = ctx.slot(result_resolved) {
        if ctx.resolver.builtin(Builtin::DefaultStringLiteralType) == Some(*entity) {
            return SolveResult::Solved;
        }
    }

    // Delegate: resolve ResultType.Interpolation → accumulator type.
    solve_associated(ctx, result_tv, "Interpolation", acc_tv, span)
}

/// Resolve a tuple index access (`t.N`). Defers until `tuple` is concrete,
/// then either extracts the N-th element type or emits a specific error.
fn solve_tuple_index(
    ctx: &mut InferCtx<'_>,
    tuple: TyVar,
    index: usize,
    result: TyVar,
    span: Span,
) -> SolveResult {
    let slot = ctx.slot(tuple);
    // Literal-unresolved receivers (e.g. integer literal) are definitionally
    // not tuples — flag immediately instead of deferring indefinitely, which
    // otherwise silently drops the error when no literal default is available
    // (e.g. in a `stdlib: false` test).
    if let TySlot::Unresolved { literal: Some(_) } = slot {
        return SolveResult::Error(InferError::TupleIndexOnNonTuple {
            receiver: tuple,
            index,
            span,
        });
    }
    let TySlot::Resolved(kind) = slot else {
        return SolveResult::Deferred(Constraint::TupleIndex {
            tuple,
            index,
            result,
            span,
        });
    };
    // Silently absorb — upstream error will have been reported already.
    if matches!(kind, TyKind::Error) {
        return SolveResult::Solved;
    }
    if let TyKind::Tuple(elems) = kind {
        if index < elems.len() {
            let elem = elems[index];
            ctx.equal(result, elem, span);
            return SolveResult::Solved;
        }
        return SolveResult::Error(InferError::TupleIndexOutOfBounds {
            arity: elems.len(),
            index,
            span,
        });
    }
    SolveResult::Error(InferError::TupleIndexOnNonTuple {
        receiver: tuple,
        index,
        span,
    })
}

/// Classify a receiver TyKind as a primitive intrinsic by its simple name.
/// Returns the primitive "family" suitable for looking up known method names.
fn primitive_family(ctx: &InferCtx<'_>, kind: &TyKind) -> Option<PrimitiveFamily> {
    let TyKind::Struct { entity, .. } = kind else {
        return None;
    };
    ctx.query_ctx.get::<Intrinsic>(*entity)?;
    let name = ctx.query_ctx.get::<Name>(*entity)?;
    match name.0.as_str() {
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" => Some(PrimitiveFamily::Int),
        "f32" | "f64" => Some(PrimitiveFamily::Float),
        "i1" => Some(PrimitiveFamily::Bool),
        "str" => Some(PrimitiveFamily::String),
        _ => None,
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum PrimitiveFamily {
    Int,
    Float,
    Bool,
    String,
}

/// Known primitive method names (mirrors lib1's `PrimitiveMethod` enum).
/// Used only to distinguish "user referenced a primitive method without
/// calling it" from "user accessed a nonexistent member on a primitive".
fn is_known_primitive_method(family: PrimitiveFamily, name: &str) -> bool {
    match family {
        PrimitiveFamily::Int => matches!(
            name,
            "toString"
                | "abs"
                | "add"
                | "subtract"
                | "multiply"
                | "divide"
                | "modulo"
                | "negate"
                | "equal"
                | "notEqual"
                | "lessThan"
                | "lessThanOrEqual"
                | "greaterThan"
                | "greaterThanOrEqual"
                | "bitwiseAnd"
                | "bitwiseOr"
                | "bitwiseXor"
                | "bitwiseNot"
                | "shiftLeft"
                | "shiftRight"
        ),
        PrimitiveFamily::Float => matches!(
            name,
            "add"
                | "subtract"
                | "multiply"
                | "divide"
                | "negate"
                | "equal"
                | "notEqual"
                | "lessThan"
                | "lessThanOrEqual"
                | "greaterThan"
                | "greaterThanOrEqual"
        ),
        PrimitiveFamily::Bool => {
            matches!(
                name,
                "logicalAnd" | "logicalOr" | "logicalNot" | "equal" | "notEqual"
            )
        },
        PrimitiveFamily::String => {
            matches!(
                name,
                "length" | "isEmpty" | "equal" | "notEqual" | "unsafePtr"
            )
        },
    }
}

/// Pick the appropriate "member not found" diagnostic for `recv_kind`.
/// On primitive receivers, distinguish known-primitive-method (must-call) from
/// generic nonexistent-member so the user gets a targeted suggestion.
fn member_not_found_error(
    ctx: &InferCtx<'_>,
    receiver: TyVar,
    recv_kind: &TyKind,
    name: &str,
    is_call: bool,
    span: Span,
) -> InferError {
    if let Some(family) = primitive_family(ctx, recv_kind) {
        if !is_call && is_known_primitive_method(family, name) {
            return InferError::MethodNotCalled {
                receiver,
                method: name.to_string(),
                span,
            };
        }
        if !is_call {
            return InferError::MemberAccessOnPrimitive {
                receiver,
                name: name.to_string(),
                span,
            };
        }
    }
    // When accessed without `()`, check if there's a method with this name.
    if !is_call {
        if let TyKind::Struct { entity, .. } | TyKind::Enum { entity, .. } = recv_kind {
            let has_callable = |parent: Entity| -> bool {
                ctx.query_ctx.children_of(parent).iter().any(|&child| {
                    ctx.query_ctx
                        .get::<Name>(child)
                        .is_some_and(|n| n.0 == name)
                        && ctx.query_ctx.get::<Callable>(child).is_some()
                })
            };
            let mut found = has_callable(*entity);
            if !found {
                let extensions = ctx.query_ctx.query(kestrel_name_res::ExtensionsFor {
                    target: *entity,
                    root: ctx.root,
                });
                found = extensions.iter().any(|&ext| has_callable(ext));
            }
            if found {
                return InferError::MethodNotCalled {
                    receiver,
                    method: name.to_string(),
                    span,
                };
            }
        }
    }
    InferError::NoMember {
        receiver,
        name: name.to_string(),
        is_call,
        span,
    }
}

/// Reduce a TypeAlias TyVar to its substituted definition.
///
/// Looks up the alias entity's TypeAnnotation, substitutes the user-provided
/// args for the alias's type params, and equates `result` with the substituted
/// TyVar. Also emits conformance obligations from the alias's TypeParam bounds
/// (e.g. `type Pair[T: Hashable] = (T, T)` requires `T: Hashable`).
fn solve_reduce(ctx: &mut InferCtx<'_>, alias: TyVar, result: TyVar, span: Span) -> SolveResult {
    let resolved = ctx.resolve(alias);
    if !ctx.is_concrete(resolved) {
        return SolveResult::Deferred(Constraint::Reduce {
            alias,
            result,
            span,
        });
    }
    if ctx.is_error(resolved) {
        return SolveResult::Solved;
    }

    let (entity, args) = match ctx.slot(resolved) {
        TySlot::Resolved(TyKind::TypeAlias { entity, args }) => (*entity, args.clone()),
        _ => {
            // Not actually a TypeAlias — nothing to reduce.
            ctx.equal(result, alias, span);
            return SolveResult::Solved;
        },
    };

    // Look up the alias's TypeAnnotation. Abstract (no annotation) aliases stay
    // as TypeAlias — member resolution consults protocol bounds instead.
    let Some(ann_hir) = ctx.query_ctx.query(kestrel_hir_lower::LowerTypeAnnotation {
        entity,
        root: ctx.root,
    }) else {
        // Abstract associated type — leave alias in place, result equals alias.
        ctx.equal(result, alias, span);
        return SolveResult::Solved;
    };

    // Build substitution map: alias's TypeParams → user args.
    let type_params: Vec<Entity> = ctx
        .query_ctx
        .get::<TypeParams>(entity)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();
    let subs: Vec<(Entity, TyVar)> = type_params
        .iter()
        .zip(args.iter())
        .map(|(&p, &tv)| (p, tv))
        .collect();

    // Lower the annotation with substitutions applied.
    let substituted_tv = crate::generate::lower_hir_ty_with_subs(ctx, &ann_hir, &subs);

    // Emit conformance obligations from the alias's TypeParam bounds.
    for &param in &type_params {
        if let Some(&(_, arg_tv)) = subs.iter().find(|(e, _)| *e == param) {
            let bound_protocols = direct_param_bound_protocols(ctx, param);
            for protocol in bound_protocols {
                ctx.conforms(arg_tv, protocol, span.clone());
            }
        }
    }

    ctx.equal(result, substituted_tv, span);
    SolveResult::Solved
}

/// Collect protocols `param` is DIRECTLY bound to via its `Conformances`
/// component. Does not walk inheritance or extension-added conformances —
/// downstream `solve_conforms` runs each emitted obligation through
/// `ConformingProtocols`, so the transitive closure is handled there.
fn direct_param_bound_protocols(ctx: &InferCtx<'_>, param: Entity) -> Vec<Entity> {
    use kestrel_ast_builder::{ConformanceItem, Conformances};
    use kestrel_name_res::ResolveTypePath;

    let Some(confs) = ctx.query_ctx.get::<Conformances>(param) else {
        return Vec::new();
    };
    let mut protocols = Vec::new();
    for item in &confs.0 {
        let ConformanceItem::Positive(ast_ty, _) = item else {
            continue;
        };
        let kestrel_ast_builder::AstType::Named { segments, .. } = ast_ty else {
            continue;
        };
        let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
        let scope = ctx.query_ctx.parent_of(param).unwrap_or(param);
        let result = ctx.query_ctx.query(ResolveTypePath {
            segments: seg_names,
            context: scope,
            root: ctx.root,
        });
        if let kestrel_name_res::TypeResolution::Found(proto) = result
            && ctx.query_ctx.get::<kestrel_ast_builder::NodeKind>(proto)
                == Some(&kestrel_ast_builder::NodeKind::Protocol)
        {
            protocols.push(proto);
        }
    }
    protocols
}

// ===== Per-constraint solvers =====

fn solve_equal(ctx: &mut InferCtx<'_>, a: TyVar, b: TyVar, span: Span) -> SolveResult {
    // Pre-reduce concrete TypeAliases ONLY if the other side is NOT also a
    // TypeAlias referring to the same entity (that case unifies structurally).
    // Without this guard, solve_reduce re-emits Equal every round → infinite
    // progress-without-work loop.
    let ra = ctx.resolve(a);
    let rb = ctx.resolve(b);
    let alias_a = matches!(ctx.slot(ra), TySlot::Resolved(TyKind::TypeAlias { .. }));
    let alias_b = matches!(ctx.slot(rb), TySlot::Resolved(TyKind::TypeAlias { .. }));
    if alias_a != alias_b {
        // Exactly one side is a TypeAlias — reduce it.
        let (alias_side, alias_is_a) = if alias_a { (a, true) } else { (b, false) };
        let alias_entity = match ctx.slot(ctx.resolve(alias_side)) {
            TySlot::Resolved(TyKind::TypeAlias { entity, .. }) => *entity,
            _ => unreachable!(),
        };
        let has_ann = ctx
            .query_ctx
            .query(kestrel_hir_lower::LowerTypeAnnotation {
                entity: alias_entity,
                root: ctx.root,
            })
            .is_some();
        if has_ann {
            let reduced = ctx.fresh();
            ctx.reduce(alias_side, reduced, span.clone());
            let (new_a, new_b) = if alias_is_a {
                (reduced, b)
            } else {
                (a, reduced)
            };
            return SolveResult::Deferred(Constraint::Equal {
                a: new_a,
                b: new_b,
                span,
            });
        }
    }

    // If either side is Error, propagate poison to any Unresolved side.
    // unify() silently absorbs Error without binding the other TyVar.
    let ra = ctx.resolve(a);
    let rb = ctx.resolve(b);
    if ctx.is_error(ra) || ctx.is_error(rb) {
        if ctx.is_error(ra) && matches!(ctx.slot(rb), TySlot::Unresolved { .. }) {
            ctx.poison(b);
        }
        if ctx.is_error(rb) && matches!(ctx.slot(ra), TySlot::Unresolved { .. }) {
            ctx.poison(a);
        }
        return SolveResult::Solved;
    }

    match unify::unify(ctx, a, b) {
        Ok(()) => SolveResult::Solved,
        Err(UnifyError::Mismatch) => SolveResult::Error(mismatch_error(ctx, a, b, span)),
        Err(UnifyError::LiteralGuard) => {
            // Literal couldn't unify — could be deferred or error.
            // If both sides are concrete, it's an error.
            if ctx.is_concrete(a) && ctx.is_concrete(b) {
                SolveResult::Error(mismatch_error(ctx, a, b, span))
            } else {
                SolveResult::Deferred(Constraint::Equal { a, b, span })
            }
        },
        Err(UnifyError::OccursCheck) => SolveResult::Error(InferError::InfiniteType { span }),
    }
}

/// Report an error (captures detail against pristine TyVars), then poison
/// both sides so downstream constraints absorb. Order matters: `report_error`
/// must run first so the detail reflects pre-poison types.
fn report_and_poison(ctx: &mut InferCtx<'_>, err: InferError, a: TyVar, b: TyVar) {
    ctx.report_error(err);
    ctx.poison(a);
    ctx.poison(b);
}

/// Build the right flavor of mismatch error for an Equal/Coerce failure.
///
/// If one side is an unresolved-literal TyVar and the other is concrete,
/// the failure is "concrete type doesn't accept this literal kind" — surface
/// as `DoesNotConform` (when `ExpressibleBy*Literal` exists) or
/// `LiteralNotAccepted` (stdlib disabled). Both render with wording that
/// includes the phrase "type mismatch" so tests asserting either wording
/// ("type mismatch" and "does not conform to protocol") both match.
/// Check whether `to` could accept a coerced value via FromValue promotion
/// once the source defaults to a concrete type. Used to decide whether a
/// LiteralGuard failure in `solve_coerce` is definitely unsalvageable (and
/// can surface `DoesNotConform` now) or might still succeed after literal
/// defaulting.
fn target_accepts_promotion(ctx: &InferCtx<'_>, to: TyVar) -> bool {
    let resolved = ctx.resolve(to);
    let to_kind = match ctx.slot(resolved) {
        TySlot::Resolved(k) => k,
        _ => return true, // unresolved — can't decide, stay conservative
    };
    let Some(from_value) = ctx.resolver.builtin(Builtin::FromValueProtocol) else {
        return false;
    };
    ctx.resolver.conforms_to(to_kind, from_value)
}

fn mismatch_error(ctx: &InferCtx<'_>, a: TyVar, b: TyVar, span: Span) -> InferError {
    if let Some(err) = try_literal_mismatch(ctx, a, b, span.clone()) {
        return err;
    }
    if let Some(err) = try_literal_mismatch(ctx, b, a, span.clone()) {
        return err;
    }
    InferError::TypeMismatch {
        expected: a,
        got: b,
        span,
    }
}

fn try_literal_mismatch(
    ctx: &InferCtx<'_>,
    lit_side: TyVar,
    ty_side: TyVar,
    span: Span,
) -> Option<InferError> {
    let lit_resolved = ctx.resolve(lit_side);
    let literal = match &ctx.types[lit_resolved.0 as usize] {
        TySlot::Unresolved { literal: Some(lit) } => *lit,
        _ => return None,
    };
    if !ctx.is_concrete(ty_side) {
        return None;
    }
    let feature = match literal {
        LiteralKind::Integer => Builtin::ExpressibleByIntegerLiteral,
        LiteralKind::Float => Builtin::ExpressibleByFloatLiteral,
        LiteralKind::String => Builtin::ExpressibleByStringLiteral,
        LiteralKind::Bool => Builtin::ExpressibleByBoolLiteral,
        LiteralKind::Char => Builtin::ExpressibleByCharLiteral,
        LiteralKind::Null => Builtin::ExpressibleByNullLiteral,
        LiteralKind::Array => Builtin::InternalExpressibleByArrayLiteral,
        LiteralKind::Dictionary => Builtin::InternalExpressibleByDictionaryLiteral,
        // Accumulator type variable — not directly coerced against user types
        LiteralKind::StringInterpolation => return None,
    };
    Some(match ctx.resolver.builtin(feature) {
        Some(protocol) => InferError::DoesNotConform {
            ty: ty_side,
            protocol,
            span,
        },
        None => InferError::LiteralNotAccepted {
            ty: ty_side,
            literal,
            span,
        },
    })
}

fn solve_coerce(
    ctx: &mut InferCtx<'_>,
    from: TyVar,
    to: TyVar,
    expr: kestrel_hir::body::HirExprId,
    span: Span,
) -> SolveResult {
    // If either side is Error, propagate poison to any Unresolved side so
    // downstream expressions don't cascade into "could not infer type".
    // unify() silently absorbs Error but does NOT bind the other TyVar, so
    // we must do this explicitly before falling into the unify path.
    let fr = ctx.resolve(from);
    let tr = ctx.resolve(to);
    if ctx.is_error(fr) || ctx.is_error(tr) {
        if ctx.is_error(fr) && matches!(ctx.slot(tr), TySlot::Unresolved { .. }) {
            ctx.poison(to);
        }
        if ctx.is_error(tr) && matches!(ctx.slot(fr), TySlot::Unresolved { .. }) {
            ctx.poison(from);
        }
        return SolveResult::Solved;
    }
    // Try unification first (handles the common case)
    match unify::unify(ctx, from, to) {
        Ok(()) => return SolveResult::Solved,
        Err(UnifyError::LiteralGuard) => {
            // Literal couldn't unify with a concrete target. If the target
            // can't be rescued by FromValue promotion, emit the literal's
            // "does not conform" diagnostic now — deferring would let
            // literal defaulting overwrite the literal slot (e.g. null →
            // Optional[?]) and we'd lose the correct wording on retry.
            if !target_accepts_promotion(ctx, to)
                && let Some(err) = try_literal_mismatch(ctx, from, to, span.clone())
                    .or_else(|| try_literal_mismatch(ctx, to, from, span.clone()))
            {
                ctx.errored_coerce_exprs.insert(expr);
                // Poison the literal side so phase 4.5 doesn't cascade
                // a "could not infer type" on the unresolved literal slot.
                if matches!(ctx.slot(ctx.resolve(from)), TySlot::Unresolved { .. }) {
                    ctx.poison(from);
                }
                return SolveResult::Error(err);
            }
            // Otherwise fall through to promotion (possibly after defaulting).
        },
        Err(UnifyError::Mismatch) => {
            // Types don't match structurally — try promotion
        },
        Err(UnifyError::OccursCheck) => {
            return SolveResult::Error(InferError::InfiniteType { span });
        },
    }

    // Check if both sides are concrete enough for promotion check
    let from_resolved = ctx.resolve(from);
    let to_resolved = ctx.resolve(to);

    if !ctx.is_concrete(from_resolved) || !ctx.is_concrete(to_resolved) {
        // Can't check promotion yet — defer
        return SolveResult::Deferred(Constraint::Coerce {
            from,
            to,
            expr,
            span,
        });
    }

    // Check FromValue promotion: does the target type conform to FromValue[source]?
    let from_kind = match ctx.slot(from_resolved) {
        TySlot::Resolved(k) => k.clone(),
        _ => unreachable!(),
    };
    let to_kind = match ctx.slot(to_resolved) {
        TySlot::Resolved(k) => k.clone(),
        _ => unreachable!(),
    };

    if let Some(method) = ctx.resolver.check_promotion(&from_kind, &to_kind) {
        // Verify the source type matches the `from` method's `value: T` parameter
        // once T is bound to the target's type args. Without this, any source
        // type silently promotes to any FromValue-conforming target (e.g.
        // `let x: String? = 5` would succeed).
        let target_entity = to_kind.entity();
        let target_args: Vec<TyVar> = to_kind.args().to_vec();
        let qctx = ctx.query_ctx;
        let root = ctx.root;
        if let Some(target_e) = target_entity {
            let target_tps: Vec<Entity> = qctx
                .get::<TypeParams>(target_e)
                .map(|tp| tp.0.clone())
                .unwrap_or_default();
            let subs: Vec<(Entity, TyVar)> = target_tps
                .iter()
                .copied()
                .zip(target_args.iter().copied())
                .collect();

            if let Some(param_hir_tys) = qctx.query(LowerCallableTypes {
                entity: method,
                root,
            }) && let Some(Some(value_hir_ty)) = param_hir_tys.first()
            {
                let param_tv = lower_hir_ty_sub(ctx, value_hir_ty, None, TyVar(0), &subs);
                if unify::unify(ctx, from, param_tv).is_err() {
                    ctx.errored_coerce_exprs.insert(expr);
                    return SolveResult::Error(InferError::TypeMismatch {
                        expected: param_tv,
                        got: from,
                        span,
                    });
                }
            }
        }

        // Record the promotion for codegen
        ctx.promotions.insert(
            expr,
            crate::ctx::PromotionInfo {
                method,
                target_ty: to,
            },
        );
        return SolveResult::Solved;
    }

    // Check protocol conformance: if the target is a protocol and the source conforms,
    // the coercion is valid (protocol existential boxing handled at codegen).
    // `SelfType { entity: P }` acts as an abstract `P` for this purpose.
    let to_protocol_entity = match &to_kind {
        TyKind::Protocol { entity, .. } | TyKind::SelfType { entity } => Some(*entity),
        _ => None,
    };
    if let Some(to_entity) = to_protocol_entity
        && ctx.resolver.conforms_to(&from_kind, to_entity)
    {
        return SolveResult::Solved;
    }

    // Flex closure adaptation: 0-param closure coerced to N-param function.
    // Also catches implicit `it` in wrong-arity context.
    if let (
        TyKind::Function {
            ret: ret_a,
            params: pa,
        },
        TyKind::Function {
            ret: ret_b,
            params: pb,
        },
    ) = (&from_kind, &to_kind)
    {
        let from_root = ctx.resolve(from);
        if pa.is_empty() && ctx.closure_flex.contains(&from_root) {
            // Adapt: ignore expected params, just equate return types
            ctx.equal(*ret_a, *ret_b, span);
            return SolveResult::Solved;
        }
        if pa.len() == 1 && ctx.closure_it.contains(&from_root) && pb.len() != 1 {
            return SolveResult::Error(InferError::ItWrongArity {
                expected: pb.len(),
                span,
            });
        }
    }

    // Cascade suppression: if an earlier arg of the same call already
    // reported a Coerce error, skip this one.
    if ctx.errored_coerce_exprs.contains(&expr) {
        return SolveResult::Solved;
    }
    ctx.errored_coerce_exprs.insert(expr);
    SolveResult::Error(mismatch_error(ctx, to, from, span))
}

fn solve_conforms(
    ctx: &mut InferCtx<'_>,
    ty: TyVar,
    protocol: kestrel_hecs::Entity,
    span: Span,
    poison_ty_on_failure: bool,
) -> SolveResult {
    let resolved = ctx.resolve(ty);
    match ctx.slot(resolved) {
        TySlot::Unresolved { .. } => SolveResult::Deferred(Constraint::Conforms {
            ty,
            protocol,
            span,
            poison_ty_on_failure,
        }),
        TySlot::Resolved(TyKind::Error) => SolveResult::Solved,
        TySlot::Resolved(kind) => {
            if ctx.resolver.conforms_to(kind, protocol) {
                SolveResult::Solved
            } else {
                if poison_ty_on_failure {
                    ctx.poison(ty);
                }
                SolveResult::Error(InferError::DoesNotConform { ty, protocol, span })
            }
        },
        TySlot::Redirect(_) => unreachable!("resolve() follows redirects"),
    }
}

fn solve_associated(
    ctx: &mut InferCtx<'_>,
    container: TyVar,
    name: &str,
    result: TyVar,
    span: Span,
) -> SolveResult {
    let resolved = ctx.resolve(container);
    if !ctx.is_concrete(resolved) {
        return SolveResult::Deferred(Constraint::Associated {
            container,
            name: name.to_string(),
            result,
            span,
        });
    }

    if ctx.is_error(resolved) {
        ctx.poison(result);
        return SolveResult::Solved;
    }

    // Get the concrete TyKind, clone to avoid borrow issues
    let kind = match ctx.slot(resolved) {
        TySlot::Resolved(k) => k.clone(),
        _ => unreachable!(),
    };

    match ctx.resolver.resolve_associated_type(&kind, name) {
        Some(assoc) => {
            // Build substitution map from the container's type parameters → its
            // concrete type args, so e.g. `Array[i32].Element = T` resolves to
            // `i32` rather than a bare `Param{T}`. The map is empty for tuple,
            // function, never, etc. containers.
            let container_subs: Vec<(Entity, TyVar)> = match &kind {
                TyKind::Struct { entity, args }
                | TyKind::Enum { entity, args }
                | TyKind::Protocol { entity, args }
                | TyKind::TypeAlias { entity, args } => {
                    let type_params: Vec<Entity> = ctx
                        .query_ctx
                        .get::<TypeParams>(*entity)
                        .map(|tp| tp.0.clone())
                        .unwrap_or_default();
                    type_params
                        .iter()
                        .zip(args.iter())
                        .map(|(&p, &tv)| (p, tv))
                        .collect()
                },
                _ => Vec::new(),
            };

            // Extension free-param substitution: when the binding came from
            // `extend ConcreteType: Proto[FreeParams]`, the resolved HirTy may
            // reference TypeParams that live on the extension, not on the
            // container. Pair them with the witness's protocol args (cached at
            // where-clause emission time) so `lower_hir_ty_sub` substitutes
            // them with the call-site's concrete types.
            let extension_subs: Vec<(Entity, TyVar)> = (|| {
                let ext = assoc.source_extension?;
                // Recover the protocol from the extension's conformances —
                // bindings inside `extend Type: Proto[...]` are scoped to
                // the conformance protocol, not the extension's parent.
                let proto = find_extension_conformance_protocol(ctx.query_ctx, ext, ctx.root)?;
                // Look up the witness's protocol args by the *exact* container
                // TyVar that the constraint was generated with. Each call site
                // creates a fresh TyVar for the protocol-bound type parameter
                // and `record_witness_args` stores under that fresh TyVar, so
                // the constraint's `container` matches the cache key directly.
                //
                // Resolving to the canonical here would merge entries from
                // independent call sites that later got unified to the same
                // concrete type (e.g. multiple `arr(unchecked: i)` calls where
                // every `I` collapses to `Int64`), and the fallback scan would
                // pick whichever entry came first in HashMap iteration order.
                let proto_args = ctx
                    .witness_protocol_args
                    .get(&(container, proto))
                    .cloned()
                    .or_else(|| {
                        // The container may have been recorded under a
                        // different-but-redirected TyVar (the body-level
                        // emitter in `lib.rs` records against the body-scoped
                        // param TyVar). Fall back to canonical match, but
                        // only when it is unambiguous — multiple matches
                        // mean separate call sites collapsed to the same
                        // canonical and cannot be disambiguated by canonical
                        // alone.
                        let canonical = ctx.resolve(container);
                        if let Some(args) = ctx.witness_protocol_args.get(&(canonical, proto)) {
                            return Some(args.clone());
                        }
                        let mut iter =
                            ctx.witness_protocol_args
                                .iter()
                                .filter(|((k_tv, k_proto), _)| {
                                    *k_proto == proto && ctx.resolve(*k_tv) == canonical
                                });
                        let first = iter.next()?;
                        if iter.next().is_some() {
                            // Ambiguous — bail rather than pick arbitrarily.
                            return None;
                        }
                        Some(first.1.clone())
                    })?;
                let ext_params: Vec<Entity> = ctx
                    .query_ctx
                    .get::<TypeParams>(ext)
                    .map(|tp| tp.0.clone())
                    .unwrap_or_default();
                Some(ext_params.into_iter().zip(proto_args).collect::<Vec<_>>())
            })()
            .unwrap_or_default();

            let mut all_subs = container_subs;
            all_subs.extend(extension_subs);

            // Check where_clause_assoc_subs first — if a where clause directly
            // equated this associated type (e.g., `where Item = Optional[T]`),
            // use that TyVar instead of creating a new one.
            // Check where_clause_assoc_subs and param_tyvars before creating
            // a new TyVar — ensures we reuse the TyVar from where clause equalities.
            //
            // Skip entries that resolve to the same TyVar as `result` — that
            // would create a self-referential no-op (unify(tv, tv) = trivially Ok,
            // leaving the TyVar permanently Unresolved). This happens when the
            // Associated constraint was generated by the same where clause that
            // populated where_clause_assoc_subs.
            let resolved_result = ctx.resolve(result);
            // The `assoc.resolved` type may contain `HirTy::SelfType` — e.g.
            // `extend Iterator: Iterable { type Iterable.Iter = Self }`. In
            // that case Self refers to the concrete container whose Iter we're
            // resolving. Pass the container's entity and TyVar so `lower_hir_ty_sub`
            // substitutes SelfType → container (via its `recv_tv` path).
            let container_entity = kind.entity();
            // Abstract associated types come back as HirTy::AliasUse; concrete ones
            // as Struct/Enum/Protocol/Tuple/etc. For the abstract case, consult
            // where_clause substitutions to find the TyVar already bound to this
            // associated type, otherwise fall through to lower_hir_ty_sub with
            // container_subs applied (handles `type Element = T` on Array[i32]).
            let assoc_tv = if let kestrel_hir::ty::HirTy::AliasUse { entity, args, .. } =
                &assoc.resolved
            {
                if args.is_empty() {
                    if let Some(&(_, tv)) = ctx
                        .where_clause_assoc_subs
                        .iter()
                        .find(|(e, _)| e == entity)
                    {
                        if ctx.resolve(tv) == resolved_result {
                            // Self-referential: the where_clause_assoc_subs TyVar is the same
                            // as our result. Create an AssocProjection to preserve the base
                            // type for MIR lowering (needed for correct monomorphization
                            // when multiple type params conform to the same protocol).
                            ctx.assoc_projection(container, *entity)
                        } else {
                            tv
                        }
                    } else if let Some(&(_, tv)) =
                        ctx.where_clause_assoc_subs.iter().find(|(e, _)| {
                            // Name-based fallback: different protocols can define the same
                            // associated type (e.g., Iterator.Item vs Iterable.Item)
                            ctx.query_ctx.get::<kestrel_ast_builder::Name>(*e)
                                == ctx.query_ctx.get::<kestrel_ast_builder::Name>(*entity)
                        })
                    {
                        if ctx.resolve(tv) == resolved_result {
                            ctx.assoc_projection(container, *entity)
                        } else {
                            tv
                        }
                    } else if let Some(&tv) = ctx.param_tyvars.get(entity) {
                        tv
                    } else {
                        lower_hir_ty_sub(
                            ctx,
                            &assoc.resolved,
                            container_entity,
                            container,
                            &all_subs,
                        )
                    }
                } else {
                    lower_hir_ty_sub(ctx, &assoc.resolved, container_entity, container, &all_subs)
                }
            } else {
                lower_hir_ty_sub(ctx, &assoc.resolved, container_entity, container, &all_subs)
            };

            // Emit where clause constraints from the resolved TypeAlias entity.
            // E.g., `type Iter: Iterator where Iter.Item = Item` — when we resolve
            // `T.Iter`, emit constraints equating `T.Iter.Item` with `T.Item`.
            if let kestrel_hir::ty::HirTy::AliasUse { entity, .. } = &assoc.resolved {
                emit_type_alias_where_clauses(ctx, *entity, assoc_tv, &span);
            }

            solve_equal(ctx, assoc_tv, result, span)
        },
        None => SolveResult::Error(InferError::NoAssociatedType {
            container,
            name: name.to_string(),
            span,
        }),
    }
}

fn solve_call(
    ctx: &mut InferCtx<'_>,
    callee: TyVar,
    args: Vec<CallArg>,
    result: TyVar,
    expr: kestrel_hir::body::HirExprId,
    span: Span,
) -> SolveResult {
    let resolved = ctx.resolve(callee);
    if !ctx.is_concrete(resolved) {
        return SolveResult::Deferred(Constraint::Call {
            callee,
            args,
            result,
            expr,
            span,
        });
    }
    if ctx.is_error(resolved) {
        // Poison result so downstream "could not infer type" is suppressed.
        ctx.poison(result);
        return SolveResult::Solved;
    }

    let kind = match ctx.slot(resolved) {
        TySlot::Resolved(k) => k.clone(),
        _ => unreachable!(),
    };

    // If the callee is a concrete TypeAlias, reduce before dispatch.
    // This handles `type C = Counter; C(42)` — the callee is TypeAlias{C}
    // which reduces to Struct{Counter}, and then init-call dispatch proceeds.
    if let TyKind::TypeAlias { entity, .. } = &kind
        && ctx
            .query_ctx
            .query(kestrel_hir_lower::LowerTypeAnnotation {
                entity: *entity,
                root: ctx.root,
            })
            .is_some()
    {
        let reduced = ctx.fresh();
        ctx.reduce(callee, reduced, span.clone());
        return SolveResult::Deferred(Constraint::Call {
            callee: reduced,
            args,
            result,
            expr,
            span,
        });
    }

    match kind {
        TyKind::Function { params, ret } => {
            // Reject excess arguments (too few is OK — defaults may fill in)
            if args.len() > params.len() {
                return SolveResult::Error(InferError::ArgCountMismatch {
                    expected: params.len(),
                    got: args.len(),
                    span,
                });
            }
            // Normal function call — unify params and return
            for (arg, param) in args.iter().zip(params.iter()) {
                ctx.coerce(arg.ty, *param, arg.value, span.clone());
            }
            ctx.equal(result, ret, span);
            SolveResult::Solved
        },
        TyKind::Param { .. } | TyKind::SelfType { .. } => {
            // `T(...)` where T is a generic param, or `Self()` whose callee
            // already resolved to SelfType — dispatch as init on the bound.
            // The init's declared return is () but the actual result is an
            // instance of the type / Self.
            let init_result = ctx.fresh();
            let res = solve_member(
                ctx,
                callee,
                "init",
                args,
                init_result,
                expr,
                true,
                true,
                &[],
                span.clone(),
            );
            // Effectful inits: wrap T → T? or T throws E
            let final_result = if let Some(&init_entity) = ctx.resolutions.get(&expr) {
                wrap_init_call_result(ctx, init_entity, callee, &[], &span)
            } else {
                callee
            };
            ctx.equal(result, final_result, span);
            res
        },
        TyKind::Protocol { entity, .. } => {
            // `Self()` in a protocol body or protocol extension reaches
            // here with the protocol entity as callee. Treat the call as
            // an init on `Self` (the conforming type, abstract here), so
            // monomorphization gets `MirTy::SelfType` and substitutes the
            // caller's concrete type. The protocol entity itself is never
            // constructible — there's no other path that lands `Def(Protocol)`
            // as a Call callee.
            let self_tv = ctx.self_type_ty(entity);
            let init_result = ctx.fresh();
            let res = solve_member(
                ctx,
                self_tv,
                "init",
                args,
                init_result,
                expr,
                true,
                true,
                &[],
                span.clone(),
            );
            let final_result = if let Some(&init_entity) = ctx.resolutions.get(&expr) {
                wrap_init_call_result(ctx, init_entity, self_tv, &[], &span)
            } else {
                self_tv
            };
            ctx.equal(result, final_result, span);
            res
        },
        TyKind::Enum { .. } if args.is_empty() => {
            // Zero-arg call on an enum value is a no-op (e.g., Color.Red())
            ctx.equal(result, callee, span);
            SolveResult::Solved
        },
        TyKind::Struct { .. }
        | TyKind::Enum { .. }
        | TyKind::TypeAlias { .. }
        | TyKind::AssocProjection { .. } => {
            // Instance subscript call (e.g. dict(key)).
            solve_member(
                ctx,
                callee,
                "subscript",
                args,
                result,
                expr,
                true,
                false,
                &[],
                span,
            )
        },
        _ => {
            // Tuples, Never, etc. are not callable
            SolveResult::Error(InferError::NoMember {
                receiver: callee,
                name: "subscript".to_string(),
                is_call: true,
                span,
            })
        },
    }
}

/// Resolve an overloaded call by label/arity filtering, then type disambiguation.
fn solve_overloaded_call(
    ctx: &mut InferCtx<'_>,
    candidates: Vec<Entity>,
    type_args: Vec<kestrel_hir::ty::HirTy>,
    args: Vec<CallArg>,
    result: TyVar,
    expr: kestrel_hir::body::HirExprId,
    span: Span,
) -> SolveResult {
    // Get a readable name for error messages from the first candidate
    let overload_name = candidates
        .first()
        .and_then(|&e| ctx.query_ctx.get::<Name>(e))
        .map(|n| n.0.clone())
        .unwrap_or_else(|| "<overloaded>".into());

    let arg_labels: Vec<Option<&str>> = args.iter().map(|a| a.label.as_deref()).collect();

    // Step 1: filter candidates by label/arity
    let matched: Vec<Entity> = candidates
        .iter()
        .copied()
        .filter(|&c| {
            let Some(callable) = ctx.query_ctx.get::<Callable>(c) else {
                return false;
            };
            labels_match(&callable.params, &arg_labels)
        })
        .collect();

    match matched.len() {
        0 => SolveResult::Error(InferError::NoMatchingOverload {
            name: overload_name,
            span,
        }),
        1 => emit_resolved_call(ctx, matched[0], &type_args, args, result, expr, span),
        _ => {
            // Multiple label matches — need type-based disambiguation.
            // Defer until all arg types are concrete.
            let all_concrete = args.iter().all(|a| ctx.is_concrete(ctx.resolve(a.ty)));
            if !all_concrete {
                return SolveResult::Deferred(Constraint::OverloadedCall {
                    candidates: matched,
                    type_args,
                    args,
                    result,
                    expr,
                    span,
                });
            }

            // Check each candidate's param types against concrete arg types
            let compatible: Vec<Entity> = matched
                .iter()
                .copied()
                .filter(|&c| types_compatible(ctx, c, &args))
                .collect();

            match compatible.len() {
                0 => SolveResult::Error(InferError::NoMember {
                    receiver: result,
                    name: overload_name,
                    is_call: true,
                    span,
                }),
                1 => emit_resolved_call(ctx, compatible[0], &type_args, args, result, expr, span),
                _ => SolveResult::Error(InferError::AmbiguousMember {
                    receiver: result,
                    name: overload_name,
                    span,
                }),
            }
        },
    }
}

/// Emit constraints for a resolved overloaded call: instantiate the selected
/// function/init entity, coerce args, equate return, record resolution.
fn emit_resolved_call(
    ctx: &mut InferCtx<'_>,
    entity: Entity,
    explicit_type_args: &[kestrel_hir::ty::HirTy],
    args: Vec<CallArg>,
    result: TyVar,
    expr: kestrel_hir::body::HirExprId,
    span: Span,
) -> SolveResult {
    let qctx = ctx.query_ctx;
    let root = ctx.root;

    // Read type params and create fresh TyVars (or use explicit type args)
    let type_param_entities: Vec<Entity> = qctx
        .get::<TypeParams>(entity)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();
    let fresh_type_args: Vec<TyVar> = if !explicit_type_args.is_empty()
        && explicit_type_args.len() == type_param_entities.len()
    {
        explicit_type_args
            .iter()
            .map(|t| crate::generate::lower_hir_ty(ctx, t))
            .collect()
    } else {
        type_param_entities.iter().map(|_| ctx.fresh()).collect()
    };

    // Build substitution map: type param entity → fresh TyVar
    let mut subs: Vec<(Entity, TyVar)> = type_param_entities
        .iter()
        .zip(fresh_type_args.iter())
        .map(|(&e, &tv)| (e, tv))
        .collect();

    // For initializers and enum cases, also add parent type params
    let kind = qctx.get::<NodeKind>(entity);
    if matches!(kind, Some(NodeKind::Initializer | NodeKind::EnumCase))
        && let Some(parent) = qctx.parent_of(entity)
    {
        let parent_tps: Vec<Entity> = qctx
            .get::<TypeParams>(parent)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();
        for &tp in &parent_tps {
            if !subs.iter().any(|(e, _)| *e == tp) {
                let tv = ctx.fresh();
                // Defer default: apply only if unconstrained after solving
                if let Some(default_ty) = qctx.query(LowerTypeAnnotation { entity: tp, root }) {
                    ctx.type_param_defaults.push((tv, default_ty));
                }
                subs.push((tp, tv));
            }
        }
    }

    // Record resolution
    ctx.resolutions.insert(expr, entity);

    // Store type args if any
    if !fresh_type_args.is_empty() {
        ctx.record_type_args(expr, fresh_type_args, span.clone());
    }

    // Emit where clause constraints
    let where_clauses = qctx.query(crate::where_clauses::WhereClausesOf { entity, root });
    for clause in where_clauses {
        match clause {
            crate::resolve::WhereClause::Bound {
                param,
                protocol,
                protocol_type_args,
            } => {
                if let Some(&(_, tv)) = subs.iter().find(|(e, _)| *e == param) {
                    ctx.conforms(tv, protocol, span.clone());
                    // Cache the protocol args so solve_associated can substitute
                    // an extension's free TypeParams when projecting through
                    // `extend ConcreteType: Proto[FreeParams]`.
                    let arg_tvs: Vec<TyVar> = protocol_type_args
                        .iter()
                        .map(|hir_ty| lower_hir_ty_sub(ctx, hir_ty, None, TyVar(0), &subs))
                        .collect();
                    if !arg_tvs.is_empty() {
                        ctx.record_witness_args(tv, protocol, arg_tvs);
                    }
                }
            },
            crate::resolve::WhereClause::TypeEquality {
                param,
                assoc_name,
                rhs,
            } => {
                if let Some(&(_, tv)) = subs.iter().find(|(e, _)| *e == param) {
                    let assoc_result = ctx.fresh();
                    ctx.associated(tv, &assoc_name, assoc_result, span.clone());
                    let rhs_tv = lower_hir_ty_sub(ctx, &rhs, None, TyVar(0), &subs);
                    ctx.equal(assoc_result, rhs_tv, span.clone());
                }
            },
            crate::resolve::WhereClause::DirectEquality { param, rhs } => {
                if let Some(&(_, tv)) = subs.iter().find(|(e, _)| *e == param) {
                    let rhs_tv = lower_hir_ty_sub(ctx, &rhs, None, TyVar(0), &subs);
                    ctx.types[tv.0 as usize] = crate::ty::TySlot::Redirect(rhs_tv);
                }
            },
        }
    }

    // Coerce args against param types
    if let Some(param_hir_tys) = qctx.query(LowerCallableTypes { entity, root }) {
        for (arg, param_ty) in args.iter().zip(param_hir_tys.iter()) {
            if let Some(hir_ty) = param_ty {
                let param_tv = lower_hir_ty_sub(ctx, hir_ty, None, TyVar(0), &subs);
                ctx.coerce(arg.ty, param_tv, arg.value, span.clone());
            }
        }
    }

    // Equate result with return type. A function with no return annotation
    // returns `()` (Swift/Rust semantics) — defaulting to a fresh TyVar would
    // leak an unresolved slot at every no-return-type call site.
    let ret_tv = qctx
        .query(LowerTypeAnnotation { entity, root })
        .map(|hir_ty| lower_opaque_aware(ctx, &hir_ty, entity, None, TyVar(0), &subs))
        .unwrap_or_else(|| ctx.tuple(Vec::new()));

    // For inits and enum cases, result type is the parent type.
    // Effectful inits wrap through type operators: Self? or Self throws E.
    if matches!(kind, Some(NodeKind::Initializer | NodeKind::EnumCase)) {
        if let Some(parent) = qctx.parent_of(entity) {
            let parent_tps: Vec<Entity> = qctx
                .get::<TypeParams>(parent)
                .map(|tp| tp.0.clone())
                .unwrap_or_default();
            let parent_args: Vec<TyVar> = parent_tps
                .iter()
                .filter_map(|tp| subs.iter().find(|(e, _)| e == tp).map(|&(_, tv)| tv))
                .collect();
            let parent_ty = ctx.named(parent, parent_args);
            let final_ty = wrap_init_call_result(ctx, entity, parent_ty, &subs, &span);
            ctx.equal(result, final_ty, span);
        } else {
            ctx.equal(result, ret_tv, span);
        }
    } else {
        ctx.equal(result, ret_tv, span);
    }

    SolveResult::Solved
}

/// Check if a candidate's param types are compatible with concrete arg types.
/// Does not mutate the type table — only inspects resolved types structurally.
fn types_compatible(ctx: &InferCtx<'_>, entity: Entity, args: &[CallArg]) -> bool {
    let qctx = ctx.query_ctx;
    let root = ctx.root;

    let Some(param_hir_tys) = qctx.query(LowerCallableTypes { entity, root }) else {
        return false;
    };

    if param_hir_tys.len() != args.len() {
        return false;
    }

    // Build substitution map for type params
    let type_param_entities: Vec<Entity> = qctx
        .get::<TypeParams>(entity)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();

    // For inits, also include parent struct type params
    let mut all_type_params = type_param_entities;
    let kind = qctx.get::<NodeKind>(entity);
    if matches!(kind, Some(NodeKind::Initializer))
        && let Some(parent) = qctx.parent_of(entity)
    {
        let parent_tps: Vec<Entity> = qctx
            .get::<TypeParams>(parent)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();
        all_type_params.extend(parent_tps);
    }

    for (arg, param_ty) in args.iter().zip(param_hir_tys.iter()) {
        let Some(hir_ty) = param_ty else {
            continue; // unannotated param — compatible with anything
        };

        // Get the concrete arg type
        let arg_resolved = ctx.resolve(arg.ty);
        let arg_kind = match &ctx.types[arg_resolved.0 as usize] {
            crate::ty::TySlot::Resolved(k) => k,
            _ => return false, // not concrete — shouldn't happen (caller checks)
        };

        // Check compatibility by inspecting the HirTy directly
        match hir_ty {
            kestrel_hir::ty::HirTy::Struct {
                entity: param_entity,
                ..
            }
            | kestrel_hir::ty::HirTy::Enum {
                entity: param_entity,
                ..
            }
            | kestrel_hir::ty::HirTy::Protocol {
                entity: param_entity,
                ..
            }
            | kestrel_hir::ty::HirTy::AliasUse {
                entity: param_entity,
                ..
            } => {
                // Type parameter entity — always compatible (generic)
                if all_type_params.contains(param_entity) {
                    continue;
                }
                match arg_kind.entity() {
                    Some(arg_entity) => {
                        if arg_entity != *param_entity {
                            return false;
                        }
                    },
                    None => return false,
                }
            },
            kestrel_hir::ty::HirTy::AssocProjection { .. } => {
                // Treat projections as always compatible for disambiguation —
                // member matching happens downstream after projection resolves.
                continue;
            },
            kestrel_hir::ty::HirTy::Param(_, _) => {
                // Type parameter — always compatible
                continue;
            },
            kestrel_hir::ty::HirTy::Tuple(elems, _) => match arg_kind {
                crate::ty::TyKind::Tuple(arg_elems) => {
                    if arg_elems.len() != elems.len() {
                        return false;
                    }
                },
                _ => return false,
            },
            kestrel_hir::ty::HirTy::Function {
                params: p_params, ..
            } => match arg_kind {
                crate::ty::TyKind::Function {
                    params: a_params, ..
                } => {
                    if a_params.len() != p_params.len() {
                        return false;
                    }
                },
                _ => return false,
            },
            // Never, Infer, Error — don't use for disambiguation
            _ => continue,
        }
    }

    true
}

fn solve_member(
    ctx: &mut InferCtx<'_>,
    receiver: TyVar,
    name: &str,
    args: Vec<CallArg>,
    result: TyVar,
    expr: kestrel_hir::body::HirExprId,
    is_call: bool,
    is_static_context: bool,
    explicit_type_args: &[kestrel_hir::ty::HirTy],
    span: Span,
) -> SolveResult {
    let resolved = ctx.resolve(receiver);
    let recv_kind = if ctx.is_concrete(resolved) {
        if ctx.is_error(resolved) {
            // Poison result so downstream "could not infer type" is suppressed.
            ctx.poison(result);
            return SolveResult::Solved;
        }
        match ctx.slot(resolved) {
            TySlot::Resolved(k) => k.clone(),
            _ => unreachable!(),
        }
    } else {
        return SolveResult::Deferred(Constraint::Member {
            receiver,
            name: name.to_string(),
            args,
            result,
            expr,
            is_call,
            is_static_context,
            explicit_type_args: explicit_type_args.to_vec(),
            span,
        });
    };

    // If the receiver is an AssocProjection or a TypeAlias whose entity is
    // bound via a where-clause equality AND that bound resolves to a concrete
    // type, substitute the bound TyVar before member dispatch. If the bound is
    // itself unresolved (e.g. it's another abstract TypeAlias for the same
    // thing), fall through so resolve_member can do a protocol-bound search.
    let bound_entity = match &recv_kind {
        TyKind::AssocProjection { assoc, .. } => Some(*assoc),
        TyKind::TypeAlias { entity, args } if args.is_empty() => Some(*entity),
        _ => None,
    };
    if let Some(entity) = bound_entity
        && let Some(&(_, bound_tv)) = ctx
            .where_clause_assoc_subs
            .iter()
            .find(|(e, _)| *e == entity)
    {
        let bound_resolved = ctx.resolve(bound_tv);
        // Only substitute if bound_tv resolves to something other than the
        // same (or equivalent) TypeAlias — otherwise the substitution loses
        // information the bound-search path needs.
        let should_substitute = match ctx.slot(bound_resolved) {
            TySlot::Resolved(TyKind::TypeAlias { entity: e, .. }) if *e == entity => false,
            // AssocProjection whose assoc matches = same abstract type, don't substitute
            TySlot::Resolved(TyKind::AssocProjection { assoc: a, .. }) if *a == entity => false,
            TySlot::Resolved(_) => true,
            _ => false,
        };
        if should_substitute {
            return SolveResult::Deferred(Constraint::Member {
                receiver: bound_tv,
                name: name.to_string(),
                args,
                result,
                expr,
                is_call,
                is_static_context,
                explicit_type_args: explicit_type_args.to_vec(),
                span,
            });
        }
    }

    // If the receiver is a TypeAlias with a concrete TypeAnnotation, reduce it
    // before member dispatch. Abstract aliases (no annotation) fall through to
    // the resolver's bound-search path (handled in resolve_member).
    if let TyKind::TypeAlias { entity, .. } = &recv_kind
        && ctx
            .query_ctx
            .query(kestrel_hir_lower::LowerTypeAnnotation {
                entity: *entity,
                root: ctx.root,
            })
            .is_some()
    {
        let reduced = ctx.fresh();
        ctx.reduce(receiver, reduced, span.clone());
        return SolveResult::Deferred(Constraint::Member {
            receiver: reduced,
            name: name.to_string(),
            args,
            result,
            expr,
            is_call,
            is_static_context,
            explicit_type_args: explicit_type_args.to_vec(),
            span,
        });
    }

    // Tuple index access: "0", "1", etc. on a Tuple type
    if let TyKind::Tuple(ref elems) = recv_kind
        && let Ok(idx) = name.parse::<usize>()
        && idx < elems.len()
    {
        ctx.equal(result, elems[idx], span);
        return SolveResult::Solved;
    }

    // (debug removed)
    // Resolve the member via the type resolver.
    // Try instance members first, fall back to static members for type-level calls
    // (e.g., Box.wrap() where wrap is a static method in an extension).
    let resolution = match ctx.resolver.resolve_member(&recv_kind, name, &args) {
        Ok(res) => res,
        Err(crate::resolve::MemberError::NotFound) => {
            // Fall back to static member search
            match ctx.resolver.resolve_static_member(&recv_kind, name, &args) {
                Ok(res) => res,
                Err(_) => {
                    return SolveResult::Error(member_not_found_error(
                        ctx, receiver, &recv_kind, name, is_call, span,
                    ));
                },
            }
        },
        Err(crate::resolve::MemberError::Ambiguous(ranked_candidates)) => {
            // Filter to candidates whose extension (if any) is compatible with
            // the receiver type. Direct members (ext=None) always pass.
            let compatible: Vec<(Entity, Option<Entity>, usize)> = ranked_candidates
                .iter()
                .copied()
                .filter(|(_, ext, _)| match ext {
                    Some(ext) => {
                        extension_type_args_compatible(ctx, *ext, &recv_kind)
                            && extension_where_clauses_satisfied(ctx, *ext, &recv_kind)
                    },
                    None => true,
                })
                .collect();

            if compatible.is_empty() {
                return SolveResult::Error(InferError::NoMember {
                    receiver,
                    name: name.to_string(),
                    is_call,
                    span,
                });
            }

            // Keep only candidates at the highest specificity. A uniquely most-
            // specific extension wins; ties at the top (including all-direct-
            // member ties, which share specificity 0) are a real ambiguity.
            let max_spec = compatible.iter().map(|(_, _, s)| *s).max().unwrap();
            let top: Vec<(Entity, Option<Entity>)> = compatible
                .into_iter()
                .filter(|(_, _, s)| *s == max_spec)
                .map(|(c, e, _)| (c, e))
                .collect();

            if let [(winner, ext)] = top.as_slice() {
                match ctx.resolver.resolve_single_member(&recv_kind, *winner) {
                    Ok(mut res) => {
                        res.from_extension = *ext;
                        res
                    },
                    Err(_) => {
                        return SolveResult::Error(InferError::AmbiguousMember {
                            receiver,
                            name: name.to_string(),
                            span,
                        });
                    },
                }
            } else {
                return SolveResult::Error(InferError::AmbiguousMember {
                    receiver,
                    name: name.to_string(),
                    span,
                });
            }
        },
        Err(crate::resolve::MemberError::NotVisible { visibility, .. }) => {
            ctx.poison(result);
            return SolveResult::Error(InferError::MemberNotVisible {
                receiver,
                name: name.to_string(),
                visibility,
                span,
            });
        },
        Err(crate::resolve::MemberError::IsStatic { .. }) => {
            // The member exists but is static — fall back to static resolution.
            // Only error if static resolution also fails (the instance-on-static
            // diagnostic is for cases where no static call is possible).
            match ctx.resolver.resolve_static_member(&recv_kind, name, &args) {
                Ok(res) => res,
                Err(_) => {
                    ctx.poison(result);
                    return SolveResult::Error(InferError::MemberIsStatic {
                        receiver,
                        name: name.to_string(),
                        span,
                    });
                },
            }
        },
    };

    // Check extension type arg compatibility and where clause satisfaction
    if let Some(ext) = resolution.from_extension {
        if !extension_type_args_compatible(ctx, ext, &recv_kind) {
            return SolveResult::Error(InferError::NoMember {
                receiver,
                name: name.to_string(),
                is_call,
                span,
            });
        }
        if !extension_where_clauses_satisfied(ctx, ext, &recv_kind) {
            return SolveResult::Error(InferError::NoMember {
                receiver,
                name: name.to_string(),
                is_call,
                span,
            });
        }
    }

    // Field/property used as a call → field access + call on the field value.
    // Handles both function-typed fields (e.g., `self.transform(item)`, `self.separator()`)
    // and subscriptable fields (e.g., `self.data(unchecked: i)` where data is Array[T]).
    // The Call constraint dispatches correctly: Function → direct call, Named → subscript.
    // Triggers when: (a) args are provided, or (b) is_call=true (from MethodCall syntax).
    // Case (b) handles zero-arg function fields like `separator: () -> Item`.
    if matches!(
        resolution.kind,
        crate::resolve::MemberKind::Field { .. }
            | crate::resolve::MemberKind::ComputedProperty { .. }
    ) && (is_call || !args.is_empty())
    {
        ctx.resolutions.insert(expr, resolution.entity);
        ctx.field_subscripts.insert(expr, resolution.entity);
        // Get the field's type (with struct type param substitution)
        let recv_entity = recv_kind.entity();
        let recv_type_args: Vec<TyVar> = recv_kind.args().to_vec();
        let mut field_subs: Vec<(kestrel_hecs::Entity, TyVar)> = Vec::new();
        if let Some(entity) = recv_entity {
            let struct_type_params: Vec<kestrel_hecs::Entity> = ctx
                .query_ctx
                .get::<TypeParams>(entity)
                .map(|tp| tp.0.clone())
                .unwrap_or_default();
            for (&param, &arg) in struct_type_params.iter().zip(recv_type_args.iter()) {
                field_subs.push((param, arg));
            }
        }
        let self_entity = resolution.self_type;
        let field_tv = lower_hir_ty_sub(
            ctx,
            &resolution.return_type,
            self_entity,
            receiver,
            &field_subs,
        );
        // Dispatch via solve_call — handles both function calls and subscript calls
        return solve_call(ctx, field_tv, args, result, expr, span);
    }

    // Record the resolved entity
    ctx.resolutions.insert(expr, resolution.entity);

    // Instantiate the member's type parameters.
    // Use explicit type args from the call site when they match the method's
    // type param count (e.g., `x.flatMap[Int](...)`). Otherwise create fresh vars.
    let fresh_params: Vec<TyVar> = if !explicit_type_args.is_empty()
        && explicit_type_args.len() == resolution.type_params.len()
    {
        explicit_type_args
            .iter()
            .map(|t| crate::generate::lower_hir_ty(ctx, t))
            .collect()
    } else {
        resolution.type_params.iter().map(|_| ctx.fresh()).collect()
    };

    // Note: record_type_args is moved after subs is fully populated (below)
    // so we can prepend protocol type args from the where clause / receiver
    // — the MIR witness dispatch reads these as the leading type_args.

    // Build type param substitution map:
    // 1. Struct type params → receiver type args
    // 2. Method's own type params → fresh vars
    let mut subs: Vec<(kestrel_hecs::Entity, TyVar)> = Vec::new();

    // Map struct type params to the receiver's actual type args.
    // SelfType has no args, so for protocol extension bodies we synthesize
    // them from the owner's parent (the extension or protocol whose type
    // params are positionally equivalent to the protocol's).
    let recv_type_args: Vec<TyVar> =
        if matches!(&recv_kind, TyKind::SelfType { .. }) && recv_kind.args().is_empty() {
            // Walk from the owner (function) to its parent (protocol, extension,
            // or struct). For extensions, type params live on the target type, not
            // the extension entity itself.
            // Use accessor_enclosing_container to skip through Subscript/Field
            // parents — a setter's direct parent is the subscript (which has its
            // own TypeParams [I]), not the extension. Without this, the protocol's
            // T gets mapped to the subscript's I.
            let parent = crate::accessor_enclosing_container(ctx.query_ctx, ctx.owner)
                .or_else(|| ctx.query_ctx.parent_of(ctx.owner));
            let params_source = parent.and_then(|p| {
                if ctx.query_ctx.get::<TypeParams>(p).is_some() {
                    Some(p)
                } else if ctx.query_ctx.get::<kestrel_ast_builder::NodeKind>(p)
                    == Some(&kestrel_ast_builder::NodeKind::Extension)
                {
                    ctx.query_ctx
                        .query(kestrel_name_res::ExtensionTargetEntity {
                            extension: p,
                            root: ctx.root,
                        })
                } else {
                    None
                }
            });
            params_source
                .and_then(|src| ctx.query_ctx.get::<TypeParams>(src))
                .map(|tp| tp.0.iter().map(|&p| ctx.param(p)).collect::<Vec<TyVar>>())
                .unwrap_or_default()
        } else {
            recv_kind.args().to_vec()
        };
    let recv_entity = recv_kind.entity();
    if let Some(entity) = recv_entity {
        let struct_type_params: Vec<kestrel_hecs::Entity> = ctx
            .query_ctx
            .get::<TypeParams>(entity)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();
        for (&param, &arg) in struct_type_params.iter().zip(recv_type_args.iter()) {
            subs.push((param, arg));
        }
    }

    // Map extension target type params to receiver type args.
    // Extension type params are different entities from the struct's type params,
    // but represent the same types (e.g., extend Box[T] has its own T entity).
    if let Some(ext) = resolution.from_extension {
        let ext_type_params: Vec<kestrel_hecs::Entity> = ctx
            .query_ctx
            .get::<TypeParams>(ext)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();
        for (&param, &arg) in ext_type_params.iter().zip(recv_type_args.iter()) {
            if !subs.iter().any(|(e, _)| *e == param) {
                subs.push((param, arg));
            }
        }
    }

    // Map method's own type params to fresh vars
    for (&param, &fresh) in resolution.type_params.iter().zip(&fresh_params) {
        subs.push((param, fresh));
    }

    // Map protocol type params when member comes from a protocol.
    // Only fires when `self_entity` is a Protocol — for inits and struct
    // members `self_type` is the struct itself, whose type params are already
    // mapped above via the recv_type_args loop. Prepending those again would
    // double the recorded type args (regression: init in generic extension).
    // If protocol_type_args are provided (from where clause, e.g., F: Factory[i64]),
    // use those. Otherwise default to receiver (e.g., Addable[Rhs = Self]).
    let mut proto_type_args_tvs: Vec<TyVar> = Vec::new();
    let self_is_protocol = resolution
        .self_type
        .map(|e| ctx.query_ctx.get::<NodeKind>(e) == Some(&NodeKind::Protocol))
        .unwrap_or(false);
    if self_is_protocol && let Some(self_entity) = resolution.self_type {
        let proto_type_params: Vec<kestrel_hecs::Entity> = ctx
            .query_ctx
            .get::<TypeParams>(self_entity)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();
        // When the method comes from a protocol extension, the extension has
        // its own type params that are positionally equivalent to the protocol's.
        // These already have TyVars from body inference. Use them instead of
        // falling back to `receiver` (which would collapse T → Self).
        let ext_type_params: Vec<kestrel_hecs::Entity> = resolution
            .from_extension
            .and_then(|ext| ctx.query_ctx.get::<TypeParams>(ext).map(|tp| tp.0.clone()))
            .unwrap_or_default();
        for (i, &param) in proto_type_params.iter().enumerate() {
            let tv = if let Some(&(_, existing)) = subs.iter().find(|(e, _)| *e == param) {
                existing
            } else if let Some(hir_ty) = resolution.protocol_type_args.get(i) {
                // Use the explicit type arg from the where clause bound
                let tv = lower_hir_ty_sub(ctx, hir_ty, None, TyVar(0), &subs);
                subs.push((param, tv));
                tv
            } else if let Some(&ext_param) = ext_type_params.get(i) {
                // Protocol extension with RHS free type params: map the protocol's
                // type param through the extension's positionally-corresponding
                // type param. If the extension param was already substituted to
                // a concrete TyVar (e.g. recv `Array[Int64]` mapped SliceExt's
                // T → Int64), use *that* — otherwise the abstract
                // `ctx.param(ext_param)` would collapse `T` to a free Param.
                let tv = subs
                    .iter()
                    .find(|(e, _)| *e == ext_param)
                    .map(|&(_, tv)| tv)
                    .unwrap_or_else(|| ctx.param(ext_param));
                subs.push((param, tv));
                tv
            } else if let Some(tv) = resolve_proto_param_via_conformance(
                ctx,
                recv_entity,
                self_entity,
                i,
                proto_type_params.len(),
                &subs,
            ) {
                // Protocol-default extension (e.g. `extend Slice[T]`): the
                // extension's `T` resolves to the protocol's T entity itself,
                // so `ext_type_params` is empty. Recover the binding from the
                // receiver's conformance to the protocol — explicit type args
                // (`Dictionary[K,V]: Slice[V]`) lower through subs; positional
                // (`Array[T]: Slice`) maps proto-param i → recv struct param i.
                subs.push((param, tv));
                tv
            } else {
                subs.push((param, receiver));
                receiver
            };
            proto_type_args_tvs.push(tv);
        }
    }

    // Record type args for the MIR witness dispatch: protocol-level args come
    // first (the dispatcher slices `method_type_args[..proto_count]` out as
    // `expected_proto_args`), then the method's own fresh params. Without
    // the proto args here, dispatch through `extend Type: Proto[FreeParams]`
    // can't recover the extension's free TypeParams from the witness.
    let mut recorded_type_args = proto_type_args_tvs;
    recorded_type_args.extend(fresh_params.iter().copied());
    if !recorded_type_args.is_empty() {
        ctx.record_type_args(expr, recorded_type_args, span.clone());
    }

    // Emit where clause constraints.
    // Where clauses may reference the method's own type params OR the
    // receiver/extension type params (e.g. `flatten[U]() where T = Optional[U]`
    // has T from Optional). We check method type params first, then fall back
    // to the full subs map which includes struct/extension type params.
    // `self_entity`/`receiver` must be threaded into RHS lowering so that
    // `HirTy::SelfType` (e.g. bare `Item` in `extend Iterator`'s where clause
    // lowers to `AssocProjection { base: SelfType(Iterator), .. }`) resolves
    // against the concrete receiver rather than becoming a fresh unresolved
    // TyVar — which would leave the Associated constraint permanently deferred.
    let self_entity = resolution.self_type;
    for clause in &resolution.where_clauses {
        match clause {
            crate::resolve::WhereClause::Bound {
                param,
                protocol,
                protocol_type_args,
            } => {
                let bound_tv =
                    if let Some(idx) = resolution.type_params.iter().position(|&p| p == *param) {
                        ctx.conforms(fresh_params[idx], *protocol, span.clone());
                        Some(fresh_params[idx])
                    } else if let Some(&(_, tv)) = subs.iter().find(|(e, _)| e == param) {
                        ctx.conforms(tv, *protocol, span.clone());
                        Some(tv)
                    } else {
                        None
                    };
                if let Some(tv) = bound_tv {
                    let arg_tvs: Vec<TyVar> = protocol_type_args
                        .iter()
                        .map(|hir_ty| lower_hir_ty_sub(ctx, hir_ty, self_entity, receiver, &subs))
                        .collect();
                    if !arg_tvs.is_empty() {
                        ctx.record_witness_args(tv, *protocol, arg_tvs);
                    }
                }
            },
            crate::resolve::WhereClause::TypeEquality {
                param,
                assoc_name,
                rhs,
            } => {
                let param_tv =
                    if let Some(idx) = resolution.type_params.iter().position(|&p| p == *param) {
                        Some(fresh_params[idx])
                    } else {
                        subs.iter().find(|(e, _)| e == param).map(|&(_, tv)| tv)
                    };
                if let Some(tv) = param_tv {
                    let assoc_result = ctx.fresh();
                    ctx.associated(tv, assoc_name, assoc_result, span.clone());
                    let rhs_tv = lower_hir_ty_sub(ctx, rhs, self_entity, receiver, &subs);
                    ctx.equal(assoc_result, rhs_tv, span.clone());
                }
            },
            crate::resolve::WhereClause::DirectEquality { param, rhs } => {
                let rhs_tv = lower_hir_ty_sub(ctx, rhs, self_entity, receiver, &subs);
                if let Some(idx) = resolution.type_params.iter().position(|&p| p == *param) {
                    // Method's own type param — redirect directly
                    ctx.types[fresh_params[idx].0 as usize] = crate::ty::TySlot::Redirect(rhs_tv);
                } else if let Some(&(_, tv)) = subs.iter().find(|(e, _)| e == param) {
                    // Struct/extension type param — equate with RHS
                    ctx.equal(tv, rhs_tv, span.clone());
                } else if ctx.query_ctx.get::<kestrel_ast_builder::NodeKind>(*param)
                    == Some(&kestrel_ast_builder::NodeKind::TypeAlias)
                {
                    // Associated type (e.g. `Item` in `where Item = (A, B)`) —
                    // resolve on receiver and equate with RHS
                    if let Some(name) = ctx.query_ctx.get::<kestrel_ast_builder::Name>(*param) {
                        let assoc_tv = ctx.fresh();
                        ctx.associated(receiver, &name.0, assoc_tv, span.clone());
                        ctx.equal(assoc_tv, rhs_tv, span.clone());
                    }
                }
            },
        }
    }

    // For protocol methods, Self in param/return types needs substitution
    // with the actual receiver type. `self_entity` is bound above the
    // where-clause loop so RHS lowering can see it too.

    // When resolved through a protocol conformance, emit a Conforms constraint
    // to verify the receiver conforms to this protocol with the inferred type args.
    if let Some(protocol) = resolution.via_protocol {
        ctx.conforms(receiver, protocol, span.clone());
    }

    // Validate argument count: must be between required (no default) and total params
    let required_count = resolution
        .param_types
        .iter()
        .filter(|p| !p.has_default)
        .count();
    let total_count = resolution.param_types.len();
    if args.len() < required_count || args.len() > total_count {
        return SolveResult::Error(InferError::ArgCountMismatch {
            expected: required_count,
            got: args.len(),
            span,
        });
    }

    // Validate argument labels match parameter labels
    for (arg, param_info) in args.iter().zip(&resolution.param_types) {
        if arg.label.as_deref() != param_info.label.as_deref() {
            return SolveResult::Error(InferError::LabelMismatch {
                expected: param_info.label.clone(),
                got: arg.label.clone(),
                span,
            });
        }
    }

    // Check static/instance mismatch: instance methods can't be called in static context
    if is_static_context
        && !ctx
            .query_ctx
            .has::<kestrel_ast_builder::Static>(resolution.entity)
    {
        // Allow inits (they don't have Static marker but are valid in static context)
        let is_init = ctx
            .query_ctx
            .get::<kestrel_ast_builder::NodeKind>(resolution.entity)
            == Some(&kestrel_ast_builder::NodeKind::Initializer);
        if !is_init {
            return SolveResult::Error(InferError::InstanceMethodAsStatic {
                name: name.to_string(),
                span,
            });
        }
    }

    // Equate argument types with parameter types
    for (arg, param_info) in args.iter().zip(&resolution.param_types) {
        let param_tv = lower_hir_ty_sub(ctx, &param_info.ty, self_entity, receiver, &subs);
        ctx.coerce(arg.ty, param_tv, arg.value, span.clone());
    }

    // Equate result with return type
    let ret_tv = lower_opaque_aware(
        ctx,
        &resolution.return_type,
        resolution.entity,
        self_entity,
        receiver,
        &subs,
    );

    ctx.equal(result, ret_tv, span.clone());

    SolveResult::Solved
}

fn solve_implicit(
    ctx: &mut InferCtx<'_>,
    expected: TyVar,
    name: &str,
    args: Vec<CallArg>,
    result: TyVar,
    expr: kestrel_hir::body::HirExprId,
    span: Span,
) -> SolveResult {
    let resolved = ctx.resolve(expected);
    if !ctx.is_concrete(resolved) {
        return SolveResult::Deferred(Constraint::Implicit {
            expected,
            name: name.to_string(),
            args,
            result,
            expr,
            span,
        });
    }

    if ctx.is_error(resolved) {
        return SolveResult::Solved;
    }

    // Get the concrete TyKind, clone to avoid borrow issues
    let kind = match ctx.slot(resolved) {
        TySlot::Resolved(k) => k.clone(),
        _ => unreachable!(),
    };

    // Resolve the member on the expected type.
    // First try instance members (enum cases, etc.), then fall back to static
    // members (e.g., .fromResidual for try operator desugaring).
    let resolution = match ctx.resolver.resolve_member(&kind, name, &args) {
        Ok(res) => res,
        Err(_) => {
            // Fall back to static member search (e.g., Result.fromResidual
            // is a static method defined in a FromResidual extension)
            match ctx.resolver.resolve_static_member(&kind, name, &args) {
                Ok(res) => res,
                Err(_) => {
                    return SolveResult::Error(InferError::ImplicitMemberNotFound {
                        expected,
                        name: name.to_string(),
                        span,
                    });
                },
            }
        },
    };

    // Check labels match for enum case calls (e.g., .Circle(radius: 5.0))
    if ctx.query_ctx.get::<NodeKind>(resolution.entity) == Some(&NodeKind::EnumCase)
        && let Some(callable) = ctx.query_ctx.get::<Callable>(resolution.entity)
    {
        let arg_labels: Vec<Option<&str>> = args.iter().map(|a| a.label.as_deref()).collect();
        if !crate::constraint::labels_match(&callable.params, &arg_labels) {
            let case_name = ctx
                .query_ctx
                .get::<Name>(resolution.entity)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| name.to_string());
            return SolveResult::Error(InferError::NoMatchingOverload {
                name: case_name,
                span,
            });
        }
    }

    ctx.resolutions.insert(expr, resolution.entity);

    // Build substitution map: enum type params → expected type args
    let mut subs: Vec<(Entity, TyVar)> = Vec::new();
    let recv_type_args: Vec<TyVar> = kind.args().to_vec();
    let recv_entity = kind.entity();
    if let Some(ent) = recv_entity {
        let struct_type_params: Vec<Entity> = ctx
            .query_ctx
            .get::<TypeParams>(ent)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();
        for (&param, &arg) in struct_type_params.iter().zip(recv_type_args.iter()) {
            subs.push((param, arg));
        }
    }

    // Instantiate method's own type params as fresh vars
    let fresh_params: Vec<TyVar> = resolution.type_params.iter().map(|_| ctx.fresh()).collect();
    for (&param, &fresh) in resolution.type_params.iter().zip(&fresh_params) {
        subs.push((param, fresh));
    }
    if !fresh_params.is_empty() {
        ctx.record_type_args(expr, fresh_params.clone(), span.clone());
    }

    // Coerce argument types against parameter types
    let self_entity = resolution.self_type;
    for (arg, param_info) in args.iter().zip(&resolution.param_types) {
        let param_tv = lower_hir_ty_sub(ctx, &param_info.ty, self_entity, expected, &subs);
        ctx.coerce(arg.ty, param_tv, arg.value, span.clone());
    }

    // Equate result with the expected type
    ctx.equal(result, expected, span);
    SolveResult::Solved
}

/// Solve an implicit variant pattern: `.CaseName(bindings)` in pattern position.
/// Deferred until the scrutinee type is concrete, then looks up the enum case
/// by name and equates each binding TyVar with the substituted payload type.
fn solve_implicit_pat(
    ctx: &mut InferCtx<'_>,
    scrutinee: TyVar,
    name: &str,
    arg_tys: Vec<TyVar>,
    span: Span,
) -> SolveResult {
    let resolved = ctx.resolve(scrutinee);
    if !ctx.is_concrete(resolved) {
        return SolveResult::Deferred(Constraint::ImplicitPat {
            scrutinee,
            name: name.to_string(),
            arg_tys,
            span,
        });
    }

    if ctx.is_error(resolved) {
        // Poison any bound arg TyVars so pattern bindings don't cascade
        // into "could not infer type".
        for tv in &arg_tys {
            if matches!(ctx.slot(ctx.resolve(*tv)), TySlot::Unresolved { .. }) {
                ctx.poison(*tv);
            }
        }
        return SolveResult::Solved;
    }

    let kind = match ctx.slot(resolved) {
        TySlot::Resolved(k) => k.clone(),
        _ => unreachable!(),
    };

    // Find the enum entity and its type args from the scrutinee
    let (enum_entity, type_args) = match &kind {
        TyKind::Enum { entity, args } => (*entity, args.clone()),
        _ => {
            // Scrutinee isn't an enum — poison arg TyVars to suppress cascades.
            for tv in &arg_tys {
                if matches!(ctx.slot(ctx.resolve(*tv)), TySlot::Unresolved { .. }) {
                    ctx.poison(*tv);
                }
            }
            return SolveResult::Solved;
        },
    };

    // Search children for an enum case with the matching name
    let children = ctx.query_ctx.children_of(enum_entity).to_vec();
    let case_entity = children.iter().copied().find(|&child| {
        ctx.query_ctx.get::<NodeKind>(child) == Some(&NodeKind::EnumCase)
            && ctx
                .query_ctx
                .get::<Name>(child)
                .is_some_and(|n| n.0 == name)
    });

    let Some(case_entity) = case_entity else {
        // No matching case found — poison arg TyVars to suppress cascades.
        for tv in &arg_tys {
            if matches!(ctx.slot(ctx.resolve(*tv)), TySlot::Unresolved { .. }) {
                ctx.poison(*tv);
            }
        }
        return SolveResult::Solved;
    };

    // Build substitution map: enum type params → scrutinee type args
    let type_params: Vec<Entity> = ctx
        .query_ctx
        .get::<TypeParams>(enum_entity)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();
    let subs: Vec<(Entity, TyVar)> = type_params
        .iter()
        .zip(type_args.iter())
        .map(|(&e, &tv)| (e, tv))
        .collect();

    // Get the case's payload types and substitute with the scrutinee's type args
    let root = ctx.root;
    if let Some(payload_hir_tys) = ctx.query_ctx.query(LowerCallableTypes {
        entity: case_entity,
        root,
    }) {
        for (arg_tv, param_ty) in arg_tys.iter().zip(payload_hir_tys.iter()) {
            if let Some(hir_ty) = param_ty {
                let payload_tv = crate::generate::lower_hir_ty_with_subs(ctx, hir_ty, &subs);
                // Solve inline, not via ctx.equal: the binding's TyVar must
                // be resolved in this same round, before any Equal constraints
                // from enclosing branches merge it with other TyVars.
                match solve_equal(ctx, *arg_tv, payload_tv, span.clone()) {
                    SolveResult::Solved => {},
                    SolveResult::Deferred(c) => ctx.constraints.push(c),
                    SolveResult::Error(err) => {
                        ctx.report_error(err);
                    },
                }
            }
        }
    }

    SolveResult::Solved
}

/// Solve a tuple rest pattern: `(prefix.., suffix..)`.
/// Deferred until scrutinee resolves to a concrete tuple, then equates
/// prefix TyVars against the first N elements, suffix against the last M.
fn solve_tuple_rest_pat(
    ctx: &mut InferCtx<'_>,
    scrutinee: TyVar,
    prefix_tys: Vec<TyVar>,
    suffix_tys: Vec<TyVar>,
    span: Span,
) -> SolveResult {
    let resolved = ctx.resolve(scrutinee);
    if !ctx.is_concrete(resolved) {
        return SolveResult::Deferred(Constraint::TupleRestPat {
            scrutinee,
            prefix_tys,
            suffix_tys,
            span,
        });
    }

    if ctx.is_error(resolved) {
        return SolveResult::Solved;
    }

    let kind = match ctx.slot(resolved) {
        TySlot::Resolved(k) => k.clone(),
        _ => return SolveResult::Solved,
    };

    let elems = match &kind {
        TyKind::Tuple(elems) => elems.clone(),
        _ => return SolveResult::Solved,
    };

    let min_needed = prefix_tys.len() + suffix_tys.len();
    if elems.len() < min_needed {
        // Pattern has more fixed elements than the tuple — type mismatch
        return SolveResult::Error(InferError::TypeMismatch {
            expected: scrutinee,
            got: scrutinee,
            span,
        });
    }

    // Equate prefix elements against the first N tuple elements
    for (pat_tv, &elem_tv) in prefix_tys.iter().zip(elems.iter()) {
        ctx.equal(*pat_tv, elem_tv, span.clone());
    }

    // Equate suffix elements against the last M tuple elements
    let suffix_start = elems.len() - suffix_tys.len();
    for (pat_tv, &elem_tv) in suffix_tys.iter().zip(elems[suffix_start..].iter()) {
        ctx.equal(*pat_tv, elem_tv, span.clone());
    }

    SolveResult::Solved
}

/// Check if an extension's explicit type args are compatible with the receiver's type args.
/// Returns false only when we can definitively prove incompatibility.
fn extension_type_args_compatible(
    ctx: &InferCtx<'_>,
    extension: Entity,
    recv_kind: &TyKind,
) -> bool {
    use crate::ty::TySlot;
    use kestrel_hir::ty::HirTy;
    use kestrel_hir_lower::LowerExtensionTargetTypeArgs;

    let Some(ext_args) = ctx.query_ctx.query(LowerExtensionTargetTypeArgs {
        extension,
        root: ctx.root,
    }) else {
        return true;
    };

    if ext_args.is_empty() {
        return true; // Generic extension
    }

    let recv_args = recv_kind.args();
    if recv_args.is_empty() && !recv_kind.is_nominal_concrete() && !recv_kind.is_type_alias() {
        return true;
    }

    for (i, ext_arg) in ext_args.iter().enumerate() {
        let Some(&recv_tv) = recv_args.get(i) else {
            continue;
        };

        // Determine the extension arg's entity (if it's a nominal type).
        let ext_entity_opt: Option<Entity> = match ext_arg {
            HirTy::Struct { entity, .. }
            | HirTy::Enum { entity, .. }
            | HirTy::Protocol { entity, .. }
            | HirTy::AliasUse { entity, .. } => Some(*entity),
            HirTy::SelfType(entity, _) => Some(*entity),
            _ => None,
        };
        let Some(ext_entity) = ext_entity_opt else {
            continue;
        };

        // Skip generic (type parameter) positions — they match anything.
        // (TypeParameter entities shouldn't appear via the nominal variants,
        // but allow a defensive check.)
        if ctx.query_ctx.get::<NodeKind>(ext_entity) == Some(&NodeKind::TypeParameter) {
            continue;
        }

        // Concrete extension arg — resolve receiver arg and compare entities.
        let resolved_recv = ctx.resolve(recv_tv);
        match ctx.slot(resolved_recv) {
            TySlot::Resolved(k) if k.entity() == Some(ext_entity) => continue,
            TySlot::Resolved(k) if k.entity().is_some() => return false,
            TySlot::Resolved(_) => return false,
            TySlot::Unresolved { literal: Some(lit) } => {
                // Unresolved literal — check if the expected type is compatible.
                let ext_ty = TyKind::Struct {
                    entity: ext_entity,
                    args: vec![],
                };
                if !crate::unify::conforms_to_literal_protocol(ctx, &ext_ty, *lit) {
                    return false;
                }
            },
            _ => continue, // Truly unresolved — allow
        }
    }

    true
}

/// Check if an extension's where clause constraints are satisfied by the receiver type.
/// For `extend Box[T] where T: Equatable`, verifies that the receiver's T arg conforms.
fn extension_where_clauses_satisfied(
    ctx: &InferCtx<'_>,
    extension: Entity,
    recv_kind: &TyKind,
) -> bool {
    use crate::resolve::WhereClause;

    // Resolve where clauses in the extension's own scope. Scope walking from
    // the extension entity sees its own type params and enclosing scope.
    let clauses = ctx.query_ctx.query(crate::where_clauses::WhereClausesOf {
        entity: extension,
        root: ctx.root,
    });
    if clauses.is_empty() {
        return true;
    }

    let Some(target_entity) = recv_kind.entity() else {
        return true;
    };
    let recv_args = recv_kind.args().to_vec();

    // Build map: type param entity → receiver TyVar
    let type_params: Vec<Entity> = ctx
        .query_ctx
        .get::<TypeParams>(target_entity)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();
    let param_to_recv: Vec<(Entity, crate::ty::TyVar)> = type_params
        .iter()
        .zip(recv_args.iter())
        .map(|(&param, &tv)| (param, tv))
        .collect();

    // Get the extension's target entity for Self comparison
    let ext_target = ctx
        .query_ctx
        .query(kestrel_name_res::ExtensionTargetEntity {
            extension,
            root: ctx.root,
        });

    for clause in &clauses {
        if let WhereClause::Bound {
            param, protocol, ..
        } = clause
        {
            // Case 1: `Self: Protocol` — param is the extension target entity
            if ext_target == Some(*param) {
                // Check if the receiver type conforms to the protocol
                if !ctx.resolver.conforms_to(recv_kind, *protocol) {
                    return false;
                }
                continue;
            }

            // Case 2: `T: Protocol` — param is a type parameter
            if let Some(&(_, recv_tv)) = param_to_recv.iter().find(|(p, _)| p == param) {
                let resolved = ctx.resolve(recv_tv);
                if let crate::ty::TySlot::Resolved(kind) = ctx.slot(resolved)
                    && !ctx.resolver.conforms_to(kind, *protocol)
                {
                    return false;
                }
            }
        }
    }

    true
}

// ===== Literal defaults =====

/// Apply deferred type-parameter defaults for TyVars that are still
/// unconstrained. Returns true if any default was applied.
fn apply_type_param_defaults(ctx: &mut InferCtx<'_>) -> bool {
    let mut progress = false;
    let defaults = std::mem::take(&mut ctx.type_param_defaults);
    for (tv, hir_ty) in &defaults {
        let resolved = ctx.resolve(*tv);
        if matches!(
            ctx.types[resolved.0 as usize],
            TySlot::Unresolved { literal: None }
        ) {
            let default_tv = crate::generate::lower_hir_ty(ctx, hir_ty);
            ctx.types[resolved.0 as usize] = TySlot::Redirect(default_tv);
            progress = true;
        }
    }
    progress
}

/// Apply default types for unconstrained literal TyVars.
///
/// Before applying the default (Int64/Float64), check if context already
/// constrains the literal through a deferred Member chain. E.g., `-1` assigned
/// to an Int32 field: the negate result is already Int32, so the literal should
/// adopt Int32 instead of defaulting to Int64.
///
/// Blocking is controlled by `relax_level`:
///   0 — strict: InterpolationLink accumulators and arg-position literals
///       in deferred Member/Call with unresolved receivers are both blocked.
///   1 — relaxed: InterpolationLink blocking lifted (string interpolation
///       literals default normally, unblocking downstream chains).
///   2 — force: all blocking removed.
///
/// Returns `true` when this call made any change.
fn apply_literal_defaults(ctx: &mut InferCtx<'_>, relax_level: u8) -> bool {
    let mut progress = false;

    // First pass: collect context-driven types for literals that have deferred
    // Member constraints with already-resolved result TyVars.
    let mut context_types: Vec<(TyVar, TyVar)> = Vec::new();
    for constraint in &ctx.constraints {
        if let Constraint::Member {
            receiver, result, ..
        }
        | Constraint::Call {
            callee: receiver,
            result,
            ..
        } = constraint
        {
            let recv_resolved = ctx.resolve(*receiver);
            let lit = match &ctx.types[recv_resolved.0 as usize] {
                TySlot::Unresolved { literal: Some(lit) } => *lit,
                _ => continue,
            };
            // Check if the result TyVar is already concrete (constrained by context)
            let result_resolved = ctx.resolve(*result);
            if let TySlot::Resolved(kind) = &ctx.types[result_resolved.0 as usize] {
                // Verify the concrete type conforms to the literal protocol
                if unify::conforms_to_literal_protocol(ctx, kind, lit) {
                    context_types.push((recv_resolved, result_resolved));
                }
            }
        }
    }

    // Apply context-driven types
    for (literal_tv, context_tv) in &context_types {
        if matches!(
            &ctx.types[literal_tv.0 as usize],
            TySlot::Unresolved { literal: Some(_) }
        ) {
            ctx.types[literal_tv.0 as usize] = TySlot::Redirect(*context_tv);
            progress = true;
        }
    }

    // Compute the set of literal TyVars that are "blocked" at this relax
    // level. Level 2 removes all blocking; levels 0–1 block arg-position
    // literals whose receiver is still unresolved; level 0 additionally
    // blocks InterpolationLink accumulators.
    let blocked: std::collections::HashSet<TyVar> = if relax_level >= 2 {
        std::collections::HashSet::new()
    } else {
        let mut set = std::collections::HashSet::new();

        // InterpolationLink blocking — only at level 0. At level 1+,
        // string interpolation accumulators default normally, unblocking
        // downstream Member/Call chains that depend on the string type
        // (e.g. interpolation → bytes subscript → UInt8 → == context).
        if relax_level == 0 {
            for constraint in &ctx.constraints {
                if let Constraint::InterpolationLink { acc_tv, .. } = constraint {
                    let acc_resolved = ctx.resolve(*acc_tv);
                    if matches!(
                        &ctx.types[acc_resolved.0 as usize],
                        TySlot::Unresolved {
                            literal: Some(LiteralKind::StringInterpolation)
                        }
                    ) {
                        set.insert(acc_resolved);
                    }
                }
            }
        }

        for constraint in &ctx.constraints {
            let (receiver, args) = match constraint {
                Constraint::Member { receiver, args, .. } => (*receiver, args),
                Constraint::Call { callee, args, .. } => (*callee, args),
                _ => continue,
            };
            // Receiver/callee unresolved → dispatch may still happen, deferring
            // arg defaults gives it a chance to bind them. If the receiver is
            // *concrete*, dispatch already had its shot during the prior
            // fixpoint, so any literal arg here is genuinely undetermined and
            // can be defaulted normally.
            let recv_resolved = ctx.resolve(receiver);
            if matches!(
                &ctx.types[recv_resolved.0 as usize],
                TySlot::Unresolved { literal: None }
            ) {
                for a in args {
                    let arg_resolved = ctx.resolve(a.ty);
                    if matches!(
                        &ctx.types[arg_resolved.0 as usize],
                        TySlot::Unresolved { literal: Some(_) }
                    ) {
                        set.insert(arg_resolved);
                    }
                }
            }
        }
        set
    };

    // Second pass: apply defaults for remaining unconstrained literals
    for idx in 0..ctx.types.len() {
        let tv = TyVar(idx as u32);
        let resolved = ctx.resolve(tv);
        if resolved != tv {
            continue;
        }
        if blocked.contains(&resolved) {
            continue;
        }

        let literal = match &ctx.types[resolved.0 as usize] {
            TySlot::Unresolved { literal: Some(lit) } => *lit,
            _ => continue,
        };

        let feature = match literal {
            LiteralKind::Integer => Builtin::DefaultIntegerLiteralType,
            LiteralKind::Float => Builtin::DefaultFloatLiteralType,
            LiteralKind::String => Builtin::DefaultStringLiteralType,
            LiteralKind::Bool => Builtin::DefaultBooleanLiteralType,
            LiteralKind::Char => Builtin::DefaultCharLiteralType,
            LiteralKind::Null => Builtin::DefaultNullLiteralType,
            LiteralKind::Array => Builtin::DefaultArrayLiteralType,
            LiteralKind::Dictionary => Builtin::DefaultDictionaryLiteralType,
            LiteralKind::StringInterpolation => Builtin::DefaultStringInterpolation,
        };

        if let Some(entity) = ctx.resolver.builtin(feature) {
            // Create fresh TyVars for each of the default entity's type parameters
            // so e.g. `DefaultArrayLiteralType[T]` becomes `Array[?]` with a real
            // inference slot — letting later Associated/Coerce constraints flow
            // element types into T. `ctx.named(entity, vec![])` would leave args
            // empty, breaking substitution in solve_associated.
            let type_param_count = ctx
                .query_ctx
                .get::<TypeParams>(entity)
                .map(|tp| tp.0.len())
                .unwrap_or(0);
            let args: Vec<TyVar> = (0..type_param_count).map(|_| ctx.fresh()).collect();
            let default_tv = ctx.named(entity, args);
            ctx.types[resolved.0 as usize] = TySlot::Redirect(default_tv);
            progress = true;
        }
        // Without the `Default<Kind>LiteralType` builtin (e.g. `// stdlib: false`
        // tests), leave the literal as `Unresolved { literal: Some(_) }`. Phase
        // 4.5 (`report_unresolved_slots`) already skips literal-marked slots, so
        // orphan literals don't surface a spurious "could not infer type", and
        // any deferred `Equal`/`Coerce` against a concrete non-conforming type
        // produces the preferred "does not conform to protocol" / "type mismatch"
        // wording via `try_literal_mismatch` in `mismatch_error`.
    }

    progress
}

// ===== Helpers =====

/// Convert a TyKind to a new TyVar.
pub fn kind_to_tyvar(ctx: &mut InferCtx<'_>, kind: &TyKind) -> TyVar {
    kind_to_tyvar_sub(ctx, kind, None, TyVar(0))
}

/// Convert a TyKind to a TyVar, substituting `self_entity` with `recv_tv`.
/// Used for protocol method dispatch where Self needs to become the concrete receiver.
pub fn kind_to_tyvar_sub(
    ctx: &mut InferCtx<'_>,
    kind: &TyKind,
    self_entity: Option<kestrel_hecs::Entity>,
    recv_tv: TyVar,
) -> TyVar {
    match kind {
        TyKind::Struct { entity, args }
        | TyKind::Enum { entity, args }
        | TyKind::Protocol { entity, args }
        | TyKind::TypeAlias { entity, args } => {
            // Substitute Self type with receiver
            if self_entity == Some(*entity) {
                return recv_tv;
            }
            let arg_tvs: Vec<TyVar> = args
                .iter()
                .map(|a| kind_to_tyvar_sub(ctx, &resolve_kind(ctx, *a), self_entity, recv_tv))
                .collect();
            ctx.named(*entity, arg_tvs)
        },
        TyKind::SelfType { entity } => {
            // SelfType(P) substitutes to the concrete receiver at call time,
            // same as the `self_entity == Some(entity)` branch above.
            if self_entity == Some(*entity) {
                return recv_tv;
            }
            ctx.self_type_ty(*entity)
        },
        TyKind::Param { entity } => ctx.param(*entity),
        TyKind::AssocProjection { base, assoc } => {
            let base_tv = kind_to_tyvar_sub(ctx, &resolve_kind(ctx, *base), self_entity, recv_tv);
            // TyKind has no span; this rematerialization path runs when
            // instantiating an already-resolved type back into a fresh TyVar
            // set, so a synthetic span is acceptable for the emitted constraint.
            ctx.project_associated(base_tv, *assoc, kestrel_span::Span::synthetic(0))
        },
        TyKind::Tuple(elems) => {
            let elem_tvs: Vec<TyVar> = elems
                .iter()
                .map(|e| kind_to_tyvar_sub(ctx, &resolve_kind(ctx, *e), self_entity, recv_tv))
                .collect();
            ctx.tuple(elem_tvs)
        },
        TyKind::Function { params, ret } => {
            let param_tvs: Vec<TyVar> = params
                .iter()
                .map(|p| kind_to_tyvar_sub(ctx, &resolve_kind(ctx, *p), self_entity, recv_tv))
                .collect();
            let ret_tv = kind_to_tyvar_sub(ctx, &resolve_kind(ctx, *ret), self_entity, recv_tv);
            ctx.function(param_tvs, ret_tv)
        },
        TyKind::Opaque {
            origin,
            bounds,
            origin_args,
            index,
        } => {
            // Remap bound args and origin_args through the substitution
            let new_bounds: Vec<(Entity, Vec<TyVar>)> = bounds
                .iter()
                .map(|(proto, args)| {
                    let new_args: Vec<TyVar> = args
                        .iter()
                        .map(|a| {
                            kind_to_tyvar_sub(ctx, &resolve_kind(ctx, *a), self_entity, recv_tv)
                        })
                        .collect();
                    (*proto, new_args)
                })
                .collect();
            let new_origin_args: Vec<TyVar> = origin_args
                .iter()
                .map(|a| kind_to_tyvar_sub(ctx, &resolve_kind(ctx, *a), self_entity, recv_tv))
                .collect();
            let idx = ctx.types.len() as u32;
            ctx.types.push(TySlot::Resolved(TyKind::Opaque {
                origin: *origin,
                bounds: new_bounds,
                origin_args: new_origin_args,
                index: *index,
            }));
            TyVar(idx)
        },
        TyKind::Never => ctx.never(),
        TyKind::Error => {
            let idx = ctx.types.len() as u32;
            ctx.types.push(TySlot::Resolved(TyKind::Error));
            TyVar(idx)
        },
    }
}

/// Recover the protocol entity from an extension's first positive
/// conformance. `extend Slot: Indexable[T]` returns Indexable's entity.
/// Used by `solve_associated` to look up the witness's protocol args
/// (cached by `(container, protocol)` key) when projecting through a
/// binding declared inside an extension block.
fn find_extension_conformance_protocol(
    ctx: &kestrel_hecs::QueryContext<'_>,
    extension: Entity,
    root: Entity,
) -> Option<Entity> {
    let confs = ctx.get::<kestrel_ast_builder::Conformances>(extension)?;
    for item in &confs.0 {
        let kestrel_ast_builder::ConformanceItem::Positive(ast_ty, _) = item else {
            continue;
        };
        let kestrel_ast_builder::AstType::Named { segments, .. } = ast_ty else {
            continue;
        };
        let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
        if let kestrel_name_res::TypeResolution::Found(proto) =
            ctx.query(kestrel_name_res::ResolveTypePath {
                segments: seg_names,
                context: extension,
                root,
            })
            && matches!(ctx.get::<NodeKind>(proto), Some(NodeKind::Protocol))
        {
            return Some(proto);
        }
    }
    None
}

/// Recover the protocol's `i`-th type-param binding from the receiver's
/// conformance to that protocol. Returns `None` when no positive conformance
/// from `recv_entity` (or its extensions) to `protocol` is found.
///
/// Two shapes:
/// - Implicit-positional (`extend Array[T]: Slice` / `struct Array[T]: Slice`):
///   the conformance has no AST type args. Maps proto param i → recv struct
///   param i, looked up via `subs`. Falls back to `recv_type_args` indexing
///   if the struct param entity isn't in `subs` (shouldn't happen in practice).
/// - Explicit (`extend Dictionary[K, V]: Slice[V]`): lowers the i-th AST type
///   arg through `subs` so identifiers like `V` resolve to whatever K/V map to.
///
/// Used when the protocol-default extension (`extend Slice[T] { ... }`) doesn't
/// register its own TypeParams — its `T` IS the protocol's T entity, so the
/// extension-positional fallback finds nothing.
fn resolve_proto_param_via_conformance(
    ctx: &mut InferCtx<'_>,
    recv_entity: Option<kestrel_hecs::Entity>,
    protocol: kestrel_hecs::Entity,
    proto_param_idx: usize,
    proto_param_count: usize,
    subs: &[(kestrel_hecs::Entity, TyVar)],
) -> Option<TyVar> {
    use kestrel_ast_builder::{ConformanceItem, Conformances};
    let recv = recv_entity?;
    if ctx.query_ctx.get::<NodeKind>(recv) != Some(&NodeKind::Struct)
        && ctx.query_ctx.get::<NodeKind>(recv) != Some(&NodeKind::Enum)
    {
        return None;
    }

    // Search direct conformances + extension conformances. The first positive
    // conformance whose resolved entity is `protocol` wins.
    let mut sources: Vec<kestrel_hecs::Entity> = vec![recv];
    sources.extend(
        ctx.query_ctx
            .query(kestrel_name_res::ExtensionsFor {
                target: recv,
                root: ctx.root,
            })
            .iter()
            .copied(),
    );

    for source in sources {
        let Some(confs) = ctx.query_ctx.get::<Conformances>(source) else {
            continue;
        };
        for item in &confs.0 {
            let ConformanceItem::Positive(ast_ty, _) = item else {
                continue;
            };
            let kestrel_ast_builder::AstType::Named { segments, .. } = ast_ty else {
                continue;
            };
            let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
            let kestrel_name_res::TypeResolution::Found(resolved) =
                ctx.query_ctx.query(kestrel_name_res::ResolveTypePath {
                    segments: seg_names,
                    context: source,
                    root: ctx.root,
                })
            else {
                continue;
            };
            if resolved != protocol {
                continue;
            }
            let type_args = kestrel_name_res::extract_ast_type_args(ast_ty);

            // Implicit-positional: empty args means "match struct params positionally".
            if type_args.is_empty() {
                let struct_params = ctx
                    .query_ctx
                    .get::<TypeParams>(recv)
                    .map(|tp| tp.0.clone())
                    .unwrap_or_default();
                if struct_params.len() != proto_param_count {
                    return None;
                }
                let struct_param = struct_params.get(proto_param_idx).copied()?;
                return subs
                    .iter()
                    .find(|(e, _)| *e == struct_param)
                    .map(|&(_, tv)| tv);
            }

            // Explicit: lower the i-th conformance type arg through subs so
            // Param references (`V`) resolve to the receiver's TyVar.
            let arg_ast = type_args.get(proto_param_idx)?;
            let hir_ty =
                kestrel_hir_lower::lower_ast_type(ctx.query_ctx, source, ctx.root, arg_ast);
            let subs_vec = subs.to_vec();
            return Some(lower_hir_ty_sub(ctx, &hir_ty, None, TyVar(0), &subs_vec));
        }
    }
    None
}

/// Helper: get the TyKind of a TyVar (or return the TyVar as-is if unresolved).
fn resolve_kind(ctx: &InferCtx<'_>, tv: TyVar) -> TyKind {
    let resolved = ctx.resolve(tv);
    match &ctx.types[resolved.0 as usize] {
        TySlot::Resolved(k) => k.clone(),
        _ => TyKind::Error, // unresolved — shouldn't happen in well-formed member types
    }
}

/// Emit constraints from a TypeAlias entity's where clauses.
///
/// When resolving an associated type like `T.Iter` where `Iter` has its own
/// constraints (e.g., `type Iter: Iterator where Iter.Item = Item`), we need
/// to propagate those constraints. This connects `T.Iter.Item` to `T.Item`
/// through the where clause equality.
fn emit_type_alias_where_clauses(
    ctx: &mut InferCtx<'_>,
    alias_entity: kestrel_hecs::Entity,
    alias_tv: TyVar,
    span: &Span,
) {
    // Resolve where clause names in the alias's own scope. Scope walking
    // from the alias visits sibling associated types on the parent protocol
    // (verified by name-res::resolve_sibling_assoc_type_from_alias_scope).
    let clauses = ctx.query_ctx.query(crate::where_clauses::WhereClausesOf {
        entity: alias_entity,
        root: ctx.root,
    });
    for clause in clauses {
        match clause {
            crate::resolve::WhereClause::Bound {
                protocol,
                protocol_type_args,
                ..
            } => {
                // Emit conformance: e.g., `Iter: Iterator` → Conforms(alias_tv, Iterator)
                ctx.conforms(alias_tv, protocol, span.clone());
                // Cache protocol args so projecting through this alias's bound
                // protocol can substitute extension free TypeParams.
                let arg_tvs: Vec<TyVar> = protocol_type_args
                    .iter()
                    .map(|hir_ty| lower_hir_ty_sub(ctx, hir_ty, None, TyVar(0), &[]))
                    .collect();
                if !arg_tvs.is_empty() {
                    ctx.record_witness_args(alias_tv, protocol, arg_tvs);
                }
            },
            crate::resolve::WhereClause::TypeEquality {
                assoc_name, rhs, ..
            } => {
                // Emit associated type equality: e.g., `Iter.Item = Item`
                // → Associated(alias_tv, "Item", fresh) + Equal(fresh, rhs_tv)
                let fresh = ctx.fresh();
                ctx.associated(alias_tv, &assoc_name, fresh, span.clone());
                // Lower rhs using where_clause_assoc_subs so that `Item` resolves
                // to the existing TyVar for T.Item
                let rhs_tv = crate::generate::lower_hir_ty(ctx, &rhs);
                ctx.equal(fresh, rhs_tv, span.clone());
            },
            crate::resolve::WhereClause::DirectEquality { .. } => {
                // Direct equality on TypeAlias — rare, skip for now
            },
        }
    }
}

/// Lower a callee's return type, intercepting HirTy::Opaque to create TyKind::Opaque
/// with the correct origin entity and origin_args from the call-site substitution.
/// Passes through `self_entity` and `recv_tv` so Self-type substitution works
/// correctly for protocol method return types (e.g. `-> Item?`).
fn lower_opaque_aware(
    ctx: &mut InferCtx<'_>,
    hir_ty: &kestrel_hir::ty::HirTy,
    callee: kestrel_hecs::Entity,
    self_entity: Option<kestrel_hecs::Entity>,
    recv_tv: TyVar,
    subs: &[(kestrel_hecs::Entity, TyVar)],
) -> TyVar {
    if let kestrel_hir::ty::HirTy::Opaque { bounds, .. } = hir_ty {
        if ctx.owner == callee {
            return ctx.return_ty;
        }

        let mut opaque_bounds = Vec::new();
        for bound in bounds {
            let bound_tv = lower_hir_ty_sub(ctx, bound, self_entity, recv_tv, subs);
            let resolved = ctx.resolve(bound_tv);
            if let crate::ty::TySlot::Resolved(crate::ty::TyKind::Protocol { entity, args }) =
                &ctx.types[resolved.0 as usize]
            {
                opaque_bounds.push((*entity, args.clone()));
            }
        }

        let origin_args: Vec<TyVar> = subs.iter().map(|(_, tv)| *tv).collect();

        let tv = ctx.fresh();
        let resolved = ctx.resolve(tv);
        ctx.types[resolved.0 as usize] = crate::ty::TySlot::Resolved(crate::ty::TyKind::Opaque {
            origin: callee,
            bounds: opaque_bounds,
            origin_args,
            index: 0,
        });
        tv
    } else {
        lower_hir_ty_sub(ctx, hir_ty, self_entity, recv_tv, subs)
    }
}

/// Convert HirTy to TyVar with substitutions.
/// - Self entity → receiver TyVar
/// - Type params in `subs` → their mapped TyVars (struct type params + method type params)
fn lower_hir_ty_sub(
    ctx: &mut InferCtx<'_>,
    ty: &kestrel_hir::ty::HirTy,
    self_entity: Option<kestrel_hecs::Entity>,
    recv_tv: TyVar,
    subs: &[(kestrel_hecs::Entity, TyVar)],
) -> TyVar {
    use kestrel_hir::ty::HirTy;
    match ty {
        HirTy::SelfType(_, _) => {
            // Substitute Self with the receiver TyVar when we know what Self is.
            // Outside a body scope (plain HirTy walk, no self_entity supplied),
            // fall back to a fresh TyVar — the solver pins it via `recv_tv` at
            // the relevant call site.
            if self_entity.is_some() {
                recv_tv
            } else {
                ctx.fresh()
            }
        },
        HirTy::Struct { entity, args, .. }
        | HirTy::Enum { entity, args, .. }
        | HirTy::Protocol { entity, args, .. } => {
            // Pre-SelfType compatibility: some sites still explicitly construct
            // `HirTy::Protocol(P)` meaning "Self where Self: P". Keep the old
            // guard as a safety net until every Self emission is confirmed to
            // go through `HirTy::SelfType`.
            if args.is_empty() && self_entity == Some(*entity) {
                return recv_tv;
            }
            if let Some(&(_, tv)) = subs.iter().find(|(e, _)| e == entity) {
                return tv;
            }
            let arg_tvs: Vec<TyVar> = args
                .iter()
                .map(|a| lower_hir_ty_sub(ctx, a, self_entity, recv_tv, subs))
                .collect();
            ctx.named(*entity, arg_tvs)
        },
        HirTy::AliasUse { entity, args, .. } => {
            // Associated-type style alias use: check substitution maps first.
            if args.is_empty() {
                if let Some(&(_, tv)) = subs.iter().find(|(e, _)| e == entity) {
                    return tv;
                }
                if let Some(&(_, tv)) = ctx
                    .where_clause_assoc_subs
                    .iter()
                    .find(|(e, _)| e == entity)
                {
                    return tv;
                }
            }

            // If this alias is a protocol associated type (parent is a Protocol)
            // AND we have a concrete receiver (not the protocol itself), emit an
            // Associated constraint so the solver resolves via the concrete type.
            let parent_is_protocol = ctx.query_ctx.parent_of(*entity).and_then(|p| {
                ctx.query_ctx
                    .get::<kestrel_ast_builder::NodeKind>(p)
                    .cloned()
            }) == Some(kestrel_ast_builder::NodeKind::Protocol);
            if parent_is_protocol && self_entity.is_some() {
                let recv_resolved = ctx.resolve(recv_tv);
                let is_concrete_non_self = match ctx.slot(recv_resolved) {
                    TySlot::Resolved(k) => match k.entity() {
                        Some(recv_entity) => self_entity != Some(recv_entity),
                        None => false,
                    },
                    _ => false,
                };
                if is_concrete_non_self
                    && let Some(name) = ctx.query_ctx.get::<kestrel_ast_builder::Name>(*entity)
                {
                    let result = ctx.fresh();
                    ctx.associated(recv_tv, &name.0, result, kestrel_span::Span::synthetic(0));
                    return result;
                }
            }

            let arg_tvs: Vec<TyVar> = args
                .iter()
                .map(|a| lower_hir_ty_sub(ctx, a, self_entity, recv_tv, subs))
                .collect();
            ctx.type_alias(*entity, arg_tvs)
        },
        HirTy::AssocProjection { base, assoc, span } => {
            let base_tv = lower_hir_ty_sub(ctx, base, self_entity, recv_tv, subs);
            if let Some(&(_, tv)) = ctx.where_clause_assoc_subs.iter().find(|(e, _)| e == assoc) {
                let _ = base_tv;
                return tv;
            }
            ctx.project_associated(base_tv, *assoc, span.clone())
        },
        HirTy::Param(entity, _) => {
            if let Some(&(_, tv)) = subs.iter().find(|(e, _)| e == entity) {
                return tv;
            }
            ctx.param(*entity)
        },
        HirTy::Tuple(types, _) => {
            let elem_tvs: Vec<TyVar> = types
                .iter()
                .map(|t| lower_hir_ty_sub(ctx, t, self_entity, recv_tv, subs))
                .collect();
            ctx.tuple(elem_tvs)
        },
        HirTy::Function { params, ret, .. } => {
            let param_tvs: Vec<TyVar> = params
                .iter()
                .map(|p| lower_hir_ty_sub(ctx, p, self_entity, recv_tv, subs))
                .collect();
            let ret_tv = lower_hir_ty_sub(ctx, ret, self_entity, recv_tv, subs);
            ctx.function(param_tvs, ret_tv)
        },
        // Opaque types at call sites: create a fresh TyVar for now.
        // Full `TyKind::Opaque` creation requires the callee entity (origin),
        // which is not available in `lower_hir_ty_sub`. The callee entity
        // lives at the `solve_member`/`resolve_overload` level. A future
        // pass will thread it through so external call sites produce proper
        // `TyKind::Opaque` with origin/bounds/origin_args.
        HirTy::Opaque { .. } => ctx.fresh(),
        HirTy::Never(_) => ctx.never(),
        HirTy::Infer(_) => ctx.fresh(),
        HirTy::Error(_) => {
            let idx = ctx.types.len() as u32;
            ctx.types.push(TySlot::Resolved(TyKind::Error));
            TyVar(idx)
        },
    }
}

/// Convert HirTy to TyVar without substitution.
pub fn lower_hir_ty_plain(ctx: &mut InferCtx<'_>, ty: &kestrel_hir::ty::HirTy) -> TyVar {
    lower_hir_ty_sub(ctx, ty, None, TyVar(0), &[])
}

/// Wrap a TyVar for an effectful init call site: `Self` → `Self?` or `Self throws E`.
fn wrap_init_call_result(
    ctx: &mut InferCtx<'_>,
    init_entity: kestrel_hecs::Entity,
    inner_tv: TyVar,
    subs: &[(kestrel_hecs::Entity, TyVar)],
    _span: &kestrel_span::Span,
) -> TyVar {
    let qctx = ctx.query_ctx;
    let root = ctx.root;

    if qctx.get::<InitEffect>(init_entity).is_none() {
        return inner_tv;
    }

    let Some(hir_ty) = qctx.query(kestrel_hir_lower::LowerTypeAnnotation {
        entity: init_entity,
        root,
    }) else {
        return inner_tv;
    };

    match &hir_ty {
        kestrel_hir::ty::HirTy::Enum { entity, args, .. }
        | kestrel_hir::ty::HirTy::Struct { entity, args, .. } => {
            let mut wrapped_args = Vec::with_capacity(args.len());
            wrapped_args.push(inner_tv);
            for arg in args.iter().skip(1) {
                wrapped_args.push(lower_hir_ty_sub(ctx, arg, None, TyVar(0), subs));
            }
            ctx.named(*entity, wrapped_args)
        },
        _ => inner_tv,
    }
}
