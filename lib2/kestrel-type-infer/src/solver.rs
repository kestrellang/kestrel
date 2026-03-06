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
            // Convert the resolved TyKind into a TyVar
            let assoc_tv = kind_to_tyvar(ctx, &assoc.resolved);
            solve_equal(ctx, assoc_tv, result, span)
        }
        None => SolveResult::Error(InferError::NoAssociatedType {
            container,
            name: name.to_string(),
            span,
        }),
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
                    let rhs_tv = kind_to_tyvar(ctx, rhs);
                    ctx.equal(assoc_result, rhs_tv, span.clone());
                }
            }
        }
    }

    // Equate argument types with parameter types
    for (arg, param_info) in args.iter().zip(&resolution.param_types) {
        let param_tv = kind_to_tyvar(ctx, &param_info.ty);
        ctx.coerce(arg.ty, param_tv, expr, span.clone());
    }

    // Equate result with return type
    let ret_tv = kind_to_tyvar(ctx, &resolution.return_type);
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
    match kind {
        TyKind::Named { entity, args } => {
            let arg_tvs: Vec<TyVar> = args.iter().map(|a| kind_to_tyvar(ctx, &resolve_kind(ctx, *a))).collect();
            ctx.named(*entity, arg_tvs)
        }
        TyKind::Param { entity } => ctx.param(*entity),
        TyKind::Tuple(elems) => {
            let elem_tvs: Vec<TyVar> = elems.iter().map(|e| kind_to_tyvar(ctx, &resolve_kind(ctx, *e))).collect();
            ctx.tuple(elem_tvs)
        }
        TyKind::Function { params, ret } => {
            let param_tvs: Vec<TyVar> = params.iter().map(|p| kind_to_tyvar(ctx, &resolve_kind(ctx, *p))).collect();
            let ret_tv = kind_to_tyvar(ctx, &resolve_kind(ctx, *ret));
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
