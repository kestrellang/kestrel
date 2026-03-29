//! Fixpoint solver: iterates constraints until no more progress is made.
//!
//! Phases:
//! 1. Main solving — iterate until fixpoint
//! 2. Apply literal defaults for unconstrained literals
//! 3. Solve again with defaults applied
//! 4. Default remaining unconstrained TyVars to Error

use crate::constraint::{labels_match, CallArg, Constraint};
use crate::ctx::InferCtx;
use crate::error::InferError;
use kestrel_ast_builder::{Callable, Name, NodeKind, TypeParams};
use kestrel_hecs::Entity;
use kestrel_hir::Builtin;
use kestrel_hir_lower::{LowerCallableTypes, LowerTypeAnnotation};
use kestrel_span2::Span;
use crate::ty::{LiteralKind, TyKind, TySlot, TyVar};
use crate::unify::{self, UnifyError};

/// Run the full solver: fixpoint loop, literal defaults, final fixpoint,
/// then report any remaining unsolved constraints as errors.
pub fn solve(ctx: &mut InferCtx<'_>) {
    // Phase 1: main solving
    fixpoint(ctx);

    // Phase 2: apply literal defaults
    apply_literal_defaults(ctx);

    // Phase 3: solve again with defaults
    fixpoint(ctx);

    // Phase 4: report remaining unsolved constraints as errors
    report_unsolved(ctx);
}

/// Run rounds until no progress.
fn fixpoint(ctx: &mut InferCtx<'_>) {
    loop {
        let progress = solve_round(ctx);
        if !progress {
            break;
        }
    }
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
            }
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
        let err = match constraint {
            Constraint::Equal { a, b, span } => {
                if ctx.is_error(ctx.resolve(a)) || ctx.is_error(ctx.resolve(b)) {
                    continue;
                }
                InferError::TypeMismatch { expected: a, got: b, span }
            }
            Constraint::Coerce { from, to, span, .. } => {
                if ctx.is_error(ctx.resolve(from)) || ctx.is_error(ctx.resolve(to)) {
                    continue;
                }
                InferError::TypeMismatch { expected: to, got: from, span }
            }
            Constraint::Conforms { ty, protocol, span } => {
                if ctx.is_error(ctx.resolve(ty)) {
                    continue;
                }
                InferError::DoesNotConform { ty, protocol, span }
            }
            Constraint::Associated { container, name, span, .. } => {
                if ctx.is_error(ctx.resolve(container)) {
                    continue;
                }
                InferError::NoAssociatedType { container, name, span }
            }
            Constraint::Member { receiver, name, span, .. } => {
                if ctx.is_error(ctx.resolve(receiver)) {
                    continue;
                }
                InferError::NoMember { receiver, name, span }
            }
            Constraint::Call { callee, span, .. } => {
                if ctx.is_error(ctx.resolve(callee)) {
                    continue;
                }
                InferError::NoMember {
                    receiver: callee,
                    name: "(subscript)".into(),
                    span,
                }
            }
            Constraint::OverloadedCall { candidates, result, span, .. } => {
                if ctx.is_error(ctx.resolve(result)) {
                    continue;
                }
                let name = candidates
                    .first()
                    .and_then(|&e| ctx.query_ctx.get::<Name>(e))
                    .map(|n| n.0.clone())
                    .unwrap_or_else(|| "<overloaded>".into());
                InferError::NoMember { receiver: result, name, span }
            }
            Constraint::Implicit { expected, name, span, .. } => {
                if ctx.is_error(ctx.resolve(expected)) {
                    continue;
                }
                InferError::ImplicitMemberNotFound { expected, name, span }
            }
            // Pattern matching handles unresolved patterns at a higher level
            Constraint::ImplicitPat { .. } => continue,
            Constraint::TupleRestPat { .. } => continue,
        };
        ctx.report_error(err);
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
        Constraint::Equal { a, b, span } => solve_equal(ctx, a, b, span),
        Constraint::Coerce {
            from,
            to,
            expr,
            span,
        } => solve_coerce(ctx, from, to, expr, span),
        Constraint::Conforms { ty, protocol, span } => solve_conforms(ctx, ty, protocol, span),
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
        } => solve_call(ctx, callee, args, result, expr, span),
        Constraint::Member {
            receiver,
            name,
            args,
            result,
            expr,
            is_call,
            is_static_context,
            span,
        } => solve_member(ctx, receiver, &name, args, result, expr, is_call, is_static_context, span),
        Constraint::OverloadedCall {
            candidates,
            type_args,
            args,
            result,
            expr,
            span,
        } => solve_overloaded_call(ctx, candidates, type_args, args, result, expr, span),
        Constraint::Implicit {
            expected,
            name,
            args,
            result,
            expr,
            span,
        } => solve_implicit(ctx, expected, &name, args, result, expr, span),
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
    }
}

// ===== Per-constraint solvers =====

fn solve_equal(ctx: &mut InferCtx<'_>, a: TyVar, b: TyVar, span: Span) -> SolveResult {
    match unify::unify(ctx, a, b) {
        Ok(()) => SolveResult::Solved,
        Err(UnifyError::Mismatch) => SolveResult::Error(InferError::TypeMismatch {
            expected: a,
            got: b,
            span,
        }),
        Err(UnifyError::LiteralGuard) => {
            // Literal couldn't unify — could be deferred or error.
            // If both sides are concrete, it's an error.
            if ctx.is_concrete(a) && ctx.is_concrete(b) {
                SolveResult::Error(InferError::TypeMismatch {
                    expected: a,
                    got: b,
                    span,
                })
            } else {
                SolveResult::Deferred(Constraint::Equal { a, b, span })
            }
        }
        Err(UnifyError::OccursCheck) => {
            SolveResult::Error(InferError::InfiniteType { span })
        }
    }
}

fn solve_coerce(
    ctx: &mut InferCtx<'_>,
    from: TyVar,
    to: TyVar,
    expr: kestrel_hir::body::HirExprId,
    span: Span,
) -> SolveResult {
    // Try unification first (handles the common case)
    match unify::unify(ctx, from, to) {
        Ok(()) => return SolveResult::Solved,
        Err(UnifyError::LiteralGuard) => {
            // Literal couldn't unify — fall through to promotion
        }
        Err(UnifyError::Mismatch) => {
            // Types don't match structurally — try promotion
        }
        Err(UnifyError::OccursCheck) => {
            return SolveResult::Error(InferError::InfiniteType { span });
        }
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
    // the coercion is valid (protocol existential boxing handled at codegen)
    if let TyKind::Named { entity: to_entity, .. } = &to_kind {
        if ctx.query_ctx.get::<NodeKind>(*to_entity) == Some(&NodeKind::Protocol) {
            if ctx.resolver.conforms_to(&from_kind, *to_entity) {
                return SolveResult::Solved;
            }
        }
    }

    SolveResult::Error(InferError::TypeMismatch {
        expected: to,
        got: from,
        span,
    })
}

fn solve_conforms(
    ctx: &mut InferCtx<'_>,
    ty: TyVar,
    protocol: kestrel_hecs::Entity,
    span: Span,
) -> SolveResult {
    let resolved = ctx.resolve(ty);
    match ctx.slot(resolved) {
        TySlot::Unresolved { .. } => {
            SolveResult::Deferred(Constraint::Conforms { ty, protocol, span })
        }
        TySlot::Resolved(TyKind::Error) => SolveResult::Solved,
        TySlot::Resolved(kind) => {
            if ctx.resolver.conforms_to(kind, protocol) {
                SolveResult::Solved
            } else {
                SolveResult::Error(InferError::DoesNotConform { ty, protocol, span })
            }
        }
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
        return SolveResult::Solved;
    }

    // Get the concrete TyKind, clone to avoid borrow issues
    let kind = match ctx.slot(resolved) {
        TySlot::Resolved(k) => k.clone(),
        _ => unreachable!(),
    };

    match ctx.resolver.resolve_associated_type(&kind, name) {
        Some(assoc) => {
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
            let assoc_tv = if let kestrel_hir::ty::HirTy::Named { entity, args, .. } = &assoc.resolved {
                if args.is_empty() {
                    if let Some(&(_, tv)) = ctx.where_clause_assoc_subs.iter().find(|(e, _)| e == entity) {
                        if ctx.resolve(tv) == resolved_result {
                            // Self-referential: the where_clause_assoc_subs TyVar is the same
                            // as our result. Create a concrete Named type directly to break
                            // the cycle (lower_hir_ty_plain would also return the same TyVar).
                            ctx.named(*entity, vec![])
                        } else {
                            tv
                        }
                    } else if let Some(&(_, tv)) = ctx.where_clause_assoc_subs.iter().find(|(e, _)| {
                        // Name-based fallback: different protocols can define the same
                        // associated type (e.g., Iterator.Item vs Iterable.Item)
                        ctx.query_ctx.get::<kestrel_ast_builder::Name>(*e)
                            == ctx.query_ctx.get::<kestrel_ast_builder::Name>(*entity)
                    }) {
                        if ctx.resolve(tv) == resolved_result {
                            ctx.named(*entity, vec![])
                        } else {
                            tv
                        }
                    } else if let Some(&tv) = ctx.param_tyvars.get(entity) {
                        tv
                    } else {
                        lower_hir_ty_plain(ctx, &assoc.resolved)
                    }
                } else {
                    lower_hir_ty_plain(ctx, &assoc.resolved)
                }
            } else {
                lower_hir_ty_plain(ctx, &assoc.resolved)
            };

            // Emit where clause constraints from the resolved TypeAlias entity.
            // E.g., `type Iter: Iterator where Iter.Item = Item` — when we resolve
            // `T.Iter`, emit constraints equating `T.Iter.Item` with `T.Item`.
            if let kestrel_hir::ty::HirTy::Named { entity, .. } = &assoc.resolved {
                if ctx.query_ctx.get::<kestrel_ast_builder::NodeKind>(*entity)
                    == Some(&kestrel_ast_builder::NodeKind::TypeAlias)
                {
                    emit_type_alias_where_clauses(ctx, *entity, assoc_tv, &span);
                }
            }

            solve_equal(ctx, assoc_tv, result, span)
        }
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
        return SolveResult::Solved;
    }

    let kind = match ctx.slot(resolved) {
        TySlot::Resolved(k) => k.clone(),
        _ => unreachable!(),
    };

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
                ctx.coerce(arg.ty, *param, expr, span.clone());
            }
            ctx.equal(result, ret, span);
            SolveResult::Solved
        }
        TyKind::Named { ref entity, .. } | TyKind::Param { ref entity } => {
            // Check if callee is a type parameter — T() is an init call, not subscript.
            // Type params appear as Named(TypeParameter_entity), Param(entity), or Param.
            let is_type_param = matches!(kind, TyKind::Param { .. })
                || ctx.query_ctx.get::<kestrel_ast_builder::NodeKind>(*entity)
                    == Some(&kestrel_ast_builder::NodeKind::TypeParameter);

            if is_type_param {
                // Resolve init on the type parameter's protocol bounds.
                // The init's return annotation is () but the actual result
                // is an instance of the type param. Override the result:
                // solve_member will equate result with (), then we equate
                // result with callee to fix it.
                let init_result = ctx.fresh();
                let res = solve_member(ctx, callee, "init", args, init_result, expr, true, true, span.clone());
                // The result of T() is T, not the init's return type
                ctx.equal(result, callee, span);
                res
            } else if args.is_empty()
                && ctx.query_ctx.get::<kestrel_ast_builder::NodeKind>(*entity)
                    == Some(&kestrel_ast_builder::NodeKind::Enum)
            {
                // Zero-arg call on an enum value is a no-op (e.g., Color.Red())
                ctx.equal(result, callee, span);
                SolveResult::Solved
            } else {
                // Instance subscript call (e.g., dict(key))
                solve_member(ctx, callee, "(subscript)", args, result, expr, true, false, span)
            }
        }
        _ => {
            // Tuples, Never, etc. are not callable
            SolveResult::Error(InferError::NoMember {
                receiver: callee,
                name: "(subscript)".to_string(),
                span,
            })
        }
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
            let all_concrete = args
                .iter()
                .all(|a| ctx.is_concrete(ctx.resolve(a.ty)));
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
                    span,
                }),
                1 => emit_resolved_call(ctx, compatible[0], &type_args, args, result, expr, span),
                _ => SolveResult::Error(InferError::AmbiguousMember {
                    receiver: result,
                    name: overload_name,
                    span,
                }),
            }
        }
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
    if matches!(kind, Some(NodeKind::Initializer | NodeKind::EnumCase)) {
        if let Some(parent) = qctx.parent_of(entity) {
            let parent_tps: Vec<Entity> = qctx
                .get::<TypeParams>(parent)
                .map(|tp| tp.0.clone())
                .unwrap_or_default();
            for &tp in &parent_tps {
                if !subs.iter().any(|(e, _)| *e == tp) {
                    subs.push((tp, ctx.fresh()));
                }
            }
        }
    }

    // Record resolution
    ctx.resolutions.insert(expr, entity);

    // Store type args if any
    if !fresh_type_args.is_empty() {
        ctx.type_args.insert(expr, fresh_type_args);
    }

    // Emit where clause constraints
    for clause in ctx.resolver.where_clauses(entity) {
        match clause {
            crate::resolve::WhereClause::Bound { param, protocol, .. } => {
                if let Some(&(_, tv)) = subs.iter().find(|(e, _)| *e == param) {
                    ctx.conforms(tv, protocol, span.clone());
                }
            }
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
            }
            crate::resolve::WhereClause::DirectEquality { param, rhs } => {
                if let Some(&(_, tv)) = subs.iter().find(|(e, _)| *e == param) {
                    let rhs_tv = lower_hir_ty_sub(ctx, &rhs, None, TyVar(0), &subs);
                    ctx.types[tv.0 as usize] = crate::ty::TySlot::Redirect(rhs_tv);
                }
            }
        }
    }

    // Coerce args against param types
    if let Some(param_hir_tys) = qctx.query(LowerCallableTypes {
        entity,
        root,
    }) {
        for (arg, param_ty) in args.iter().zip(param_hir_tys.iter()) {
            if let Some(hir_ty) = param_ty {
                let param_tv = lower_hir_ty_sub(ctx, hir_ty, None, TyVar(0), &subs);
                ctx.coerce(arg.ty, param_tv, expr, span.clone());
            }
        }
    }

    // Equate result with return type
    let ret_tv = qctx
        .query(LowerTypeAnnotation { entity, root })
        .map(|hir_ty| lower_hir_ty_sub(ctx, &hir_ty, None, TyVar(0), &subs))
        .unwrap_or_else(|| ctx.fresh());

    // For inits and enum cases, result type is the parent type
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
            ctx.equal(result, parent_ty, span);
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
fn types_compatible(
    ctx: &InferCtx<'_>,
    entity: Entity,
    args: &[CallArg],
) -> bool {
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
    if matches!(kind, Some(NodeKind::Initializer)) {
        if let Some(parent) = qctx.parent_of(entity) {
            let parent_tps: Vec<Entity> = qctx
                .get::<TypeParams>(parent)
                .map(|tp| tp.0.clone())
                .unwrap_or_default();
            all_type_params.extend(parent_tps);
        }
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
            kestrel_hir::ty::HirTy::Named { entity: param_entity, .. } => {
                // Type parameter entity — always compatible (generic)
                if all_type_params.contains(param_entity) {
                    continue;
                }
                match arg_kind {
                    crate::ty::TyKind::Named { entity: arg_entity, .. } => {
                        if arg_entity != param_entity {
                            return false;
                        }
                    }
                    _ => return false,
                }
            }
            kestrel_hir::ty::HirTy::Param(_, _) => {
                // Type parameter — always compatible
                continue;
            }
            kestrel_hir::ty::HirTy::Tuple(elems, _) => {
                match arg_kind {
                    crate::ty::TyKind::Tuple(arg_elems) => {
                        if arg_elems.len() != elems.len() {
                            return false;
                        }
                    }
                    _ => return false,
                }
            }
            kestrel_hir::ty::HirTy::Function { params: p_params, .. } => {
                match arg_kind {
                    crate::ty::TyKind::Function { params: a_params, .. } => {
                        if a_params.len() != p_params.len() {
                            return false;
                        }
                    }
                    _ => return false,
                }
            }
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
    span: Span,
) -> SolveResult {
    let resolved = ctx.resolve(receiver);
    let recv_kind = if ctx.is_concrete(resolved) {
        if ctx.is_error(resolved) {
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
            span,
        });
    };

    // Tuple index access: "0", "1", etc. on a Tuple type
    if let TyKind::Tuple(ref elems) = recv_kind {
        if let Ok(idx) = name.parse::<usize>() {
            if idx < elems.len() {
                ctx.equal(result, elems[idx], span);
                return SolveResult::Solved;
            }
        }
    }

    // Resolve the member via the type resolver
    let resolution = match ctx.resolver.resolve_member(&recv_kind, name, &args) {
        Ok(res) => res,
        Err(crate::resolve::MemberError::NotFound) => {
            return SolveResult::Error(InferError::NoMember {
                receiver,
                name: name.to_string(),
                span,
            });
        }
        Err(crate::resolve::MemberError::Ambiguous(ranked_candidates)) => {
            // Try each candidate in specificity order, picking the first compatible one
            let compatible = ranked_candidates.iter().find(|(_, ext)| {
                match ext {
                    Some(ext) => {
                        extension_type_args_compatible(ctx, *ext, &recv_kind)
                            && extension_where_clauses_satisfied(ctx, *ext, &recv_kind)
                    }
                    None => true,
                }
            });
            if let Some(&(winner, ext)) = compatible {
                match ctx.resolver.resolve_single_member(&recv_kind, winner) {
                    Ok(mut res) => {
                        res.from_extension = ext;
                        res
                    }
                    Err(_) => {
                        return SolveResult::Error(InferError::AmbiguousMember {
                            receiver,
                            name: name.to_string(),
                            span,
                        });
                    }
                }
            } else {
                return SolveResult::Error(InferError::NoMember {
                    receiver,
                    name: name.to_string(),
                    span,
                });
            }
        }
        Err(crate::resolve::MemberError::NotVisible) => {
            return SolveResult::Error(InferError::MemberNotVisible {
                receiver,
                name: name.to_string(),
                span,
            });
        }
    };

    // Check extension type arg compatibility and where clause satisfaction
    if let Some(ext) = resolution.from_extension {
        if !extension_type_args_compatible(ctx, ext, &recv_kind) {
            return SolveResult::Error(InferError::NoMember {
                receiver,
                name: name.to_string(),
                span,
            });
        }
        if !extension_where_clauses_satisfied(ctx, ext, &recv_kind) {
            return SolveResult::Error(InferError::NoMember {
                receiver,
                name: name.to_string(),
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
        crate::resolve::MemberKind::Field { .. } | crate::resolve::MemberKind::ComputedProperty { .. }
    ) && (is_call || !args.is_empty())
    {
        ctx.resolutions.insert(expr, resolution.entity);
        // Get the field's type (with struct type param substitution)
        let recv_entity = match &recv_kind {
            TyKind::Named { entity, .. } => Some(*entity),
            _ => None,
        };
        let recv_type_args: Vec<TyVar> = match &recv_kind {
            TyKind::Named { args, .. } => args.clone(),
            _ => vec![],
        };
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

    // Instantiate the member's type parameters
    let fresh_params: Vec<TyVar> = resolution.type_params.iter().map(|_| ctx.fresh()).collect();

    if !fresh_params.is_empty() {
        ctx.type_args.insert(expr, fresh_params.clone());
    }

    // Build type param substitution map:
    // 1. Struct type params → receiver type args
    // 2. Method's own type params → fresh vars
    let mut subs: Vec<(kestrel_hecs::Entity, TyVar)> = Vec::new();

    // Map struct type params to the receiver's actual type args
    let recv_type_args: Vec<TyVar> = match &recv_kind {
        TyKind::Named { args, .. } => args.clone(),
        _ => vec![],
    };
    let recv_entity = match &recv_kind {
        TyKind::Named { entity, .. } => Some(*entity),
        _ => None,
    };
    if let Some(entity) = recv_entity {
        let struct_type_params: Vec<kestrel_hecs::Entity> = ctx.query_ctx
            .get::<TypeParams>(entity)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();
        for (&param, &arg) in struct_type_params.iter().zip(recv_type_args.iter()) {
            subs.push((param, arg));
        }
    }

    // Map method's own type params to fresh vars
    for (&param, &fresh) in resolution.type_params.iter().zip(&fresh_params) {
        subs.push((param, fresh));
    }

    // Map protocol type params when member comes from a protocol.
    // If protocol_type_args are provided (from where clause, e.g., F: Factory[i64]),
    // use those. Otherwise default to receiver (e.g., Addable[Rhs = Self]).
    if let Some(self_entity) = resolution.self_type {
        let proto_type_params: Vec<kestrel_hecs::Entity> = ctx.query_ctx
            .get::<TypeParams>(self_entity)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();
        for (i, &param) in proto_type_params.iter().enumerate() {
            if !subs.iter().any(|(e, _)| *e == param) {
                if let Some(hir_ty) = resolution.protocol_type_args.get(i) {
                    // Use the explicit type arg from the where clause bound
                    let tv = lower_hir_ty_sub(ctx, hir_ty, None, TyVar(0), &subs);
                    subs.push((param, tv));
                } else {
                    subs.push((param, receiver));
                }
            }
        }
    }

    // Emit where clause constraints
    for clause in &resolution.where_clauses {
        match clause {
            crate::resolve::WhereClause::Bound { param, protocol, .. } => {
                if let Some(idx) = resolution
                    .type_params
                    .iter()
                    .position(|&p| p == *param)
                {
                    ctx.conforms(fresh_params[idx], *protocol, span.clone());
                }
            }
            crate::resolve::WhereClause::TypeEquality {
                param,
                assoc_name,
                rhs,
            } => {
                if let Some(idx) = resolution
                    .type_params
                    .iter()
                    .position(|&p| p == *param)
                {
                    let assoc_result = ctx.fresh();
                    ctx.associated(fresh_params[idx], assoc_name, assoc_result, span.clone());
                    let rhs_tv = lower_hir_ty_sub(ctx, rhs, None, TyVar(0), &subs);
                    ctx.equal(assoc_result, rhs_tv, span.clone());
                }
            }
            crate::resolve::WhereClause::DirectEquality { param, rhs } => {
                // Direct type param equality: redirect the method type param to the RHS
                if let Some(idx) = resolution
                    .type_params
                    .iter()
                    .position(|&p| p == *param)
                {
                    let rhs_tv = lower_hir_ty_sub(ctx, rhs, None, TyVar(0), &subs);
                    ctx.types[fresh_params[idx].0 as usize] =
                        crate::ty::TySlot::Redirect(rhs_tv);
                }
            }
        }
    }

    // For protocol methods, Self in param/return types needs substitution
    // with the actual receiver type.
    let self_entity = resolution.self_type;

    // When resolved through a protocol conformance, emit a Conforms constraint
    // to verify the receiver conforms to this protocol with the inferred type args.
    if let Some(protocol) = resolution.via_protocol {
        ctx.conforms(receiver, protocol, span.clone());
    }

    // Validate argument count matches parameter count
    let required_count = resolution.param_types.len();
    if args.len() != required_count {
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
    if is_static_context && !ctx.query_ctx.has::<kestrel_ast_builder::Static>(resolution.entity) {
        // Allow inits (they don't have Static marker but are valid in static context)
        let is_init = ctx.query_ctx.get::<kestrel_ast_builder::NodeKind>(resolution.entity)
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
        ctx.coerce(arg.ty, param_tv, expr, span.clone());
    }

    // Equate result with return type
    let ret_tv = lower_hir_ty_sub(ctx, &resolution.return_type, self_entity, receiver, &subs);

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
                }
            }
        }
    };

    // Check labels match for enum case calls (e.g., .Circle(radius: 5.0))
    if ctx.query_ctx.get::<NodeKind>(resolution.entity) == Some(&NodeKind::EnumCase) {
        if let Some(callable) = ctx.query_ctx.get::<Callable>(resolution.entity) {
            let arg_labels: Vec<Option<&str>> = args.iter().map(|a| a.label.as_deref()).collect();
            if !crate::constraint::labels_match(&callable.params, &arg_labels) {
                let case_name = ctx.query_ctx.get::<Name>(resolution.entity)
                    .map(|n| n.0.clone())
                    .unwrap_or_else(|| name.to_string());
                return SolveResult::Error(InferError::NoMatchingOverload {
                    name: case_name,
                    span,
                });
            }
        }
    }

    ctx.resolutions.insert(expr, resolution.entity);

    // Build substitution map: enum type params → expected type args
    let mut subs: Vec<(Entity, TyVar)> = Vec::new();
    let recv_type_args: Vec<TyVar> = match &kind {
        TyKind::Named { args: ta, .. } => ta.clone(),
        _ => vec![],
    };
    let recv_entity = match &kind {
        TyKind::Named { entity, .. } => Some(*entity),
        _ => None,
    };
    if let Some(ent) = recv_entity {
        let struct_type_params: Vec<Entity> = ctx.query_ctx
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
        ctx.type_args.insert(expr, fresh_params.clone());
    }

    // Coerce argument types against parameter types
    let self_entity = resolution.self_type;
    for (arg, param_info) in args.iter().zip(&resolution.param_types) {
        let param_tv = lower_hir_ty_sub(ctx, &param_info.ty, self_entity, expected, &subs);
        ctx.coerce(arg.ty, param_tv, expr, span.clone());
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
        return SolveResult::Solved;
    }

    let kind = match ctx.slot(resolved) {
        TySlot::Resolved(k) => k.clone(),
        _ => unreachable!(),
    };

    // Find the enum entity and its type args from the scrutinee
    let (enum_entity, type_args) = match &kind {
        TyKind::Named { entity, args } => (*entity, args.clone()),
        _ => return SolveResult::Solved,
    };

    // Search children for an enum case with the matching name
    let children = ctx.query_ctx.children_of(enum_entity).to_vec();
    let case_entity = children.iter().copied().find(|&child| {
        ctx.query_ctx.get::<NodeKind>(child) == Some(&NodeKind::EnumCase)
            && ctx.query_ctx.get::<Name>(child).is_some_and(|n| n.0 == name)
    });

    let Some(case_entity) = case_entity else {
        // No matching case found
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
                let payload_tv =
                    crate::generate::lower_hir_ty_with_subs(ctx, hir_ty, &subs);
                ctx.equal(*arg_tv, payload_tv, span.clone());
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

    let TyKind::Named { args: recv_args, .. } = recv_kind else {
        return true;
    };

    for (i, ext_arg) in ext_args.iter().enumerate() {
        let Some(&recv_tv) = recv_args.get(i) else {
            continue;
        };

        // Skip generic (type parameter) positions — they match anything
        if let HirTy::Named { entity: ext_entity, .. } = ext_arg {
            if ctx.query_ctx.get::<NodeKind>(*ext_entity) == Some(&NodeKind::TypeParameter) {
                continue;
            }

            // Concrete extension arg — resolve receiver arg and compare entities
            let resolved_recv = ctx.resolve(recv_tv);
            match ctx.slot(resolved_recv) {
                TySlot::Resolved(TyKind::Named { entity: recv_entity, .. }) => {
                    if ext_entity != recv_entity {
                        return false;
                    }
                }
                TySlot::Resolved(_) => {
                    return false;
                }
                TySlot::Unresolved { literal: Some(lit) } => {
                    // Unresolved literal — check if the expected type is compatible
                    let ext_ty = TyKind::Named { entity: *ext_entity, args: vec![] };
                    if !crate::unify::conforms_to_literal_protocol(ctx, &ext_ty, *lit) {
                        return false;
                    }
                }
                _ => {
                    // Truly unresolved — allow
                    continue;
                }
            }
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

    // Resolve where clauses in the extension's own context (not the current method's context)
    let clauses = ctx.resolver.where_clauses_in_context(extension, extension);
    if clauses.is_empty() {
        return true;
    }

    let TyKind::Named { entity: target_entity, args: recv_args, .. } = recv_kind else {
        return true;
    };

    // Build map: type param entity → receiver TyVar
    let type_params: Vec<Entity> = ctx.query_ctx
        .get::<TypeParams>(*target_entity)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();
    let param_to_recv: Vec<(Entity, crate::ty::TyVar)> = type_params
        .iter()
        .zip(recv_args.iter())
        .map(|(&param, &tv)| (param, tv))
        .collect();

    // Get the extension's target entity for Self comparison
    let ext_target = ctx.query_ctx.query(kestrel_name_res::ExtensionTargetEntity {
        extension,
        root: ctx.root,
    });

    for clause in &clauses {
        if let WhereClause::Bound { param, protocol, .. } = clause {
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
                if let crate::ty::TySlot::Resolved(kind) = ctx.slot(resolved) {
                    if !ctx.resolver.conforms_to(kind, *protocol) {
                        return false;
                    }
                }
            }
        }
    }

    true
}

// ===== Literal defaults =====

/// Apply default types for unconstrained literal TyVars.
///
/// Before applying the default (Int64/Float64), check if context already
/// constrains the literal through a deferred Member chain. E.g., `-1` assigned
/// to an Int32 field: the negate result is already Int32, so the literal should
/// adopt Int32 instead of defaulting to Int64.
fn apply_literal_defaults(ctx: &mut InferCtx<'_>) {
    // First pass: collect context-driven types for literals that have deferred
    // Member constraints with already-resolved result TyVars.
    let mut context_types: Vec<(TyVar, TyVar)> = Vec::new();
    for constraint in &ctx.constraints {
        if let Constraint::Member { receiver, result, .. }
            | Constraint::Call { callee: receiver, result, .. } = constraint
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
        if matches!(&ctx.types[literal_tv.0 as usize], TySlot::Unresolved { literal: Some(_) }) {
            ctx.types[literal_tv.0 as usize] = TySlot::Redirect(*context_tv);
        }
    }

    // Second pass: apply defaults for remaining unconstrained literals
    for idx in 0..ctx.types.len() {
        let tv = TyVar(idx as u32);
        let resolved = ctx.resolve(tv);
        if resolved != tv {
            continue;
        }

        let literal = match &ctx.types[resolved.0 as usize] {
            TySlot::Unresolved {
                literal: Some(lit),
            } => *lit,
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
        };

        if let Some(entity) = ctx.resolver.builtin(feature) {
            let default_tv = ctx.named(entity, vec![]);
            ctx.types[resolved.0 as usize] = TySlot::Redirect(default_tv);
        }
    }
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
        TyKind::Named { entity, args } => {
            // Substitute Self type with receiver
            if self_entity == Some(*entity) {
                return recv_tv;
            }
            let arg_tvs: Vec<TyVar> = args.iter()
                .map(|a| kind_to_tyvar_sub(ctx, &resolve_kind(ctx, *a), self_entity, recv_tv))
                .collect();
            ctx.named(*entity, arg_tvs)
        }
        TyKind::Param { entity } => ctx.param(*entity),
        TyKind::Tuple(elems) => {
            let elem_tvs: Vec<TyVar> = elems.iter()
                .map(|e| kind_to_tyvar_sub(ctx, &resolve_kind(ctx, *e), self_entity, recv_tv))
                .collect();
            ctx.tuple(elem_tvs)
        }
        TyKind::Function { params, ret } => {
            let param_tvs: Vec<TyVar> = params.iter()
                .map(|p| kind_to_tyvar_sub(ctx, &resolve_kind(ctx, *p), self_entity, recv_tv))
                .collect();
            let ret_tv = kind_to_tyvar_sub(ctx, &resolve_kind(ctx, *ret), self_entity, recv_tv);
            ctx.function(param_tvs, ret_tv)
        }
        TyKind::Never => ctx.never(),
        TyKind::Error => {
            let idx = ctx.types.len() as u32;
            ctx.types.push(TySlot::Resolved(TyKind::Error));
            TyVar(idx)
        }
    }
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
    let clauses = ctx.resolver.where_clauses(alias_entity);
    for clause in clauses {
        match clause {
            crate::resolve::WhereClause::Bound { protocol, .. } => {
                // Emit conformance: e.g., `Iter: Iterator` → Conforms(alias_tv, Iterator)
                ctx.conforms(alias_tv, protocol, span.clone());
            }
            crate::resolve::WhereClause::TypeEquality { assoc_name, rhs, .. } => {
                // Emit associated type equality: e.g., `Iter.Item = Item`
                // → Associated(alias_tv, "Item", fresh) + Equal(fresh, rhs_tv)
                let fresh = ctx.fresh();
                ctx.associated(alias_tv, &assoc_name, fresh, span.clone());
                // Lower rhs using where_clause_assoc_subs so that `Item` resolves
                // to the existing TyVar for T.Item
                let rhs_tv = crate::generate::lower_hir_ty(ctx, &rhs);
                ctx.equal(fresh, rhs_tv, span.clone());
            }
            crate::resolve::WhereClause::DirectEquality { .. } => {
                // Direct equality on TypeAlias — rare, skip for now
            }
        }
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
        HirTy::Named { entity, args, .. } => {
            // Substitute Self type with receiver
            if self_entity == Some(*entity) {
                return recv_tv;
            }
            // Check substitution map (type params of the method/struct)
            if let Some(&(_, tv)) = subs.iter().find(|(e, _)| e == entity) {
                return tv;
            }
            // Check where clause associated type subs (e.g., Output → Item
            // from extension where clause `Item.Output = Item`).
            if args.is_empty() {
                if let Some(&(_, tv)) = ctx.where_clause_assoc_subs.iter().find(|(e, _)| e == entity) {
                    return tv;
                }
            }
            // Associated type resolution: if this entity is a TypeAlias
            // (e.g., `Iter` in `protocol Iterable { type Iter }`) and we have
            // a concrete receiver (not the protocol itself or a type param),
            // emit an Associated constraint so the solver resolves it via the
            // concrete type (e.g., ArrayIterator[T] for Array).
            if self_entity.is_some()
                && ctx.query_ctx.get::<kestrel_ast_builder::NodeKind>(*entity)
                    == Some(&kestrel_ast_builder::NodeKind::TypeAlias)
            {
                let recv_resolved = ctx.resolve(recv_tv);
                let is_concrete_non_self = match ctx.slot(recv_resolved) {
                    TySlot::Resolved(TyKind::Named { entity: recv_entity, .. }) => {
                        self_entity != Some(*recv_entity)
                    }
                    _ => false,
                };
                if is_concrete_non_self {
                    if let Some(name) = ctx.query_ctx.get::<kestrel_ast_builder::Name>(*entity) {
                        let result = ctx.fresh();
                        ctx.associated(recv_tv, &name.0, result, kestrel_span2::Span::synthetic(0));
                        return result;
                    }
                }
            }
            let arg_tvs: Vec<TyVar> = args.iter()
                .map(|a| lower_hir_ty_sub(ctx, a, self_entity, recv_tv, subs))
                .collect();
            ctx.named(*entity, arg_tvs)
        }
        HirTy::Param(entity, _) => {
            // Check substitution map
            if let Some(&(_, tv)) = subs.iter().find(|(e, _)| e == entity) {
                return tv;
            }
            ctx.param(*entity)
        }
        HirTy::Tuple(types, _) => {
            let elem_tvs: Vec<TyVar> = types.iter()
                .map(|t| lower_hir_ty_sub(ctx, t, self_entity, recv_tv, subs))
                .collect();
            ctx.tuple(elem_tvs)
        }
        HirTy::Function { params, ret, .. } => {
            let param_tvs: Vec<TyVar> = params.iter()
                .map(|p| lower_hir_ty_sub(ctx, p, self_entity, recv_tv, subs))
                .collect();
            let ret_tv = lower_hir_ty_sub(ctx, ret, self_entity, recv_tv, subs);
            ctx.function(param_tvs, ret_tv)
        }
        HirTy::Never(_) => ctx.never(),
        HirTy::Infer(_) => ctx.fresh(),
        HirTy::Error(_) => {
            let idx = ctx.types.len() as u32;
            ctx.types.push(TySlot::Resolved(TyKind::Error));
            TyVar(idx)
        }
    }
}

/// Convert HirTy to TyVar without substitution.
pub fn lower_hir_ty_plain(ctx: &mut InferCtx<'_>, ty: &kestrel_hir::ty::HirTy) -> TyVar {
    lower_hir_ty_sub(ctx, ty, None, TyVar(0), &[])
}
