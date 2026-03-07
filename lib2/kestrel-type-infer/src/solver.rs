//! Fixpoint solver: iterates constraints until no more progress is made.
//!
//! Phases:
//! 1. Main solving — iterate until fixpoint
//! 2. Apply literal defaults for unconstrained literals
//! 3. Solve again with defaults applied
//! 4. Default remaining unconstrained TyVars to Error

use crate::constraint::{CallArg, Constraint};
use crate::ctx::InferCtx;
use crate::error::InferError;
use kestrel_ast_builder::TypeParams;
use kestrel_hir::Builtin;
use crate::ty::{LiteralKind, TyKind, TySlot, TyVar};
use crate::unify::{self, UnifyError};

use kestrel_span2::Span;

/// Run the full solver: fixpoint loop, literal defaults, final fixpoint.
pub fn solve(ctx: &mut InferCtx<'_>) {
    // Phase 1: main solving
    fixpoint(ctx);

    // Phase 2: apply literal defaults
    apply_literal_defaults(ctx);

    // Phase 3: solve again with defaults
    fixpoint(ctx);
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
            span,
        } => solve_member(ctx, receiver, &name, args, result, expr, span),
        Constraint::Implicit {
            expected,
            name,
            args,
            result,
            expr,
            span,
        } => solve_implicit(ctx, expected, &name, args, result, expr, span),
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
            // Not concrete yet — defer
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
            // Convert the resolved HirTy into a TyVar
            let assoc_tv = lower_hir_ty_plain(ctx, &assoc.resolved);
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
            // Normal function call — unify params and return
            for (arg, param) in args.iter().zip(params.iter()) {
                ctx.coerce(arg.ty, *param, expr, span.clone());
            }
            ctx.equal(result, ret, span);
            SolveResult::Solved
        }
        TyKind::Named { ref entity, .. } | TyKind::Param { ref entity } => {
            // Check if callee is a type parameter — T() is an init call, not subscript.
            // Type params appear as Named(TypeParameter_entity) or Param(entity).
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
                let res = solve_member(ctx, callee, "init", args, init_result, expr, span.clone());
                // The result of T() is T, not the init's return type
                ctx.equal(result, callee, span);
                res
            } else {
                // Instance subscript call (e.g., dict(key))
                solve_member(ctx, callee, "(subscript)", args, result, expr, span)
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

fn solve_member(
    ctx: &mut InferCtx<'_>,
    receiver: TyVar,
    name: &str,
    args: Vec<CallArg>,
    result: TyVar,
    expr: kestrel_hir::body::HirExprId,
    span: Span,
) -> SolveResult {
    let resolved = ctx.resolve(receiver);
    if !ctx.is_concrete(resolved) {
        return SolveResult::Deferred(Constraint::Member {
            receiver,
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
    let recv_kind = match ctx.slot(resolved) {
        TySlot::Resolved(k) => k.clone(),
        _ => unreachable!(),
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
        Err(crate::resolve::MemberError::Ambiguous(_)) => {
            return SolveResult::Error(InferError::AmbiguousMember {
                receiver,
                name: name.to_string(),
                span,
            });
        }
        Err(crate::resolve::MemberError::NotVisible) => {
            return SolveResult::Error(InferError::MemberNotVisible {
                receiver,
                name: name.to_string(),
                span,
            });
        }
    };

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

    // Map protocol type params to receiver when member comes from a protocol.
    // E.g., Addable[Rhs = Self] — Rhs defaults to Self, so map Rhs → receiver.
    if let Some(self_entity) = resolution.self_type {
        let proto_type_params: Vec<kestrel_hecs::Entity> = ctx.query_ctx
            .get::<TypeParams>(self_entity)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();
        for &param in &proto_type_params {
            if !subs.iter().any(|(e, _)| *e == param) {
                subs.push((param, receiver));
            }
        }
    }

    // Emit where clause constraints
    for clause in &resolution.where_clauses {
        match clause {
            crate::resolve::WhereClause::Bound { param, protocol } => {
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
        }
    }

    // For protocol methods, Self in param/return types needs substitution
    // with the actual receiver type.
    let self_entity = resolution.self_type;

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

    // Resolve the member on the expected type
    match ctx.resolver.resolve_member(&kind, name, &args) {
        Ok(resolution) => {
            ctx.resolutions.insert(expr, resolution.entity);

            // Equate result with the expected type
            ctx.equal(result, expected, span);
            SolveResult::Solved
        }
        Err(_) => SolveResult::Error(InferError::ImplicitMemberNotFound {
            expected,
            name: name.to_string(),
            span,
        }),
    }
}

// ===== Literal defaults =====

/// Apply default types for unconstrained literal TyVars.
fn apply_literal_defaults(ctx: &mut InferCtx<'_>) {
    for idx in 0..ctx.types.len() {
        let tv = TyVar(idx as u32);
        let resolved = ctx.resolve(tv);
        if resolved != tv {
            continue; // skip redirects
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
            // Create the default type and bind the literal to it
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
