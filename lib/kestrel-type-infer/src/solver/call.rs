//! Calls: `Call`, `OverloadedCall`, and binding a resolved callee's signature.

use super::*;

pub(super) fn solve_call(
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

    // Transparent-place peel for callees: paren-subscripting a ref-typed
    // value (`h.view()(1)`) calls the POINTEE in place.
    if let TyKind::Ref { pointee, .. } = &kind {
        return SolveResult::Deferred(Constraint::Call {
            callee: *pointee,
            args,
            result,
            expr,
            span,
        });
    }

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
        TyKind::Function { params, ret, .. } => {
            // Reject excess arguments (too few is OK — defaults may fill in)
            if args.len() > params.len() {
                return SolveResult::Error(InferError::ArgCountMismatch {
                    expected: params.len(),
                    got: args.len(),
                    span,
                });
            }
            // Normal function call — unify params and return. Reconcile each
            // function-typed arg's conventions first (its `MutBorrow` upgrade is
            // an in-place side effect), capturing the first mismatch. We still
            // run every `coerce` so the param/return types unify regardless —
            // reporting the convention error without a spurious cascade.
            let mut conv_err: Option<InferError> = None;
            for (index, (arg, param)) in args.iter().zip(params.iter()).enumerate() {
                let e = reconcile_fn_convention(ctx, arg, *param, &span);
                if conv_err.is_none() {
                    conv_err = e;
                }
                // Only the function type is known here, not its declaration.
                let reason = Reason::Param { index, decl: None };
                ctx.coerce_because(arg.ty, *param, arg.value, span.clone(), reason);
            }
            // Ref-aware: a direct callee's Function type carries `ret: &T`
            // (intrinsics like lang.ptr_ref); the result may already be
            // pinned to the pointee by an earlier return-position coerce.
            bind_call_result(ctx, result, ret, expr, span);
            if let Some(err) = conv_err {
                ctx.report_error(err);
            }
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
pub(super) fn solve_overloaded_call(
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
                // Receiver-less: an overloaded module-level function call has
                // no receiver type — `result` is the call result, not a
                // receiver, so don't render it (#210).
                _ => SolveResult::Error(InferError::AmbiguousMember {
                    receiver: None,
                    name: overload_name,
                    span,
                }),
            }
        },
    }
}

/// Emit constraints for a resolved overloaded call: instantiate the selected
/// function/init entity, coerce args, equate return, record resolution.
/// Compute the argument→parameter binding plan for callable `entity` and call
/// `args`, using `arg_binding`. `None` if the entity has no `Callable` or the
/// arguments don't bind (wrong labels / arity). Indexed by parameter slot.
pub(super) fn binding_plan_for(
    qctx: &kestrel_hecs::QueryContext<'_>,
    entity: Entity,
    args: &[CallArg],
) -> Option<Vec<Binding>> {
    let callable = qctx.get::<Callable>(entity)?;
    let bind_params: Vec<BindParam> = callable
        .params
        .iter()
        .map(|p| BindParam::new(p.label.as_deref(), p.default_entity.is_some()))
        .collect();
    let arg_labels: Vec<Option<&str>> = args.iter().map(|a| a.label.as_deref()).collect();
    bind_arguments(&bind_params, &arg_labels).ok()
}

/// Bind a call's result var to the signature return type, ref-aware:
/// - a match-SCRUTINEE call decays: a scrutinee is a value context, so the
///   result binds to the POINTEE and patterns never see a ref;
/// - an if/match ARM-VALUE call decays: refs cannot cross merges, so arm
///   values always produce owned values (the merge `Equal`s stay untouched
///   and only ever see owned types);
/// - a result already pinned to a non-ref (an early Coerce ran before the
///   member resolved — `let x: Int = late.peek()`) unifies with the pointee
///   instead of erroring.
///
/// Consequence: a ref-returning call's recorded type is `&T` in the common
/// solve order but `T` in the pinned/scrutinee orders — consumers key
/// place-ness on `CallableRefReturn` (entity), never on expr_types alone.
pub(super) fn bind_call_result(
    ctx: &mut InferCtx<'_>,
    result: TyVar,
    ret_tv: TyVar,
    expr: kestrel_hir::body::HirExprId,
    span: Span,
) {
    let rr = ctx.resolve(ret_tv);
    if matches!(ctx.slot(rr), TySlot::Resolved(TyKind::Ref { .. })) {
        kestrel_debug::ktrace!(
            "arm-decay",
            "bind_call_result REF expr {expr:?} owner={:?} arm_set={} scrut={} bind={} assign={}",
            ctx.owner,
            ctx.always_decay_exprs.contains(&expr),
            ctx.scrutinee_exprs.contains(&expr),
            ctx.binding_init_exprs.contains(&expr),
            ctx.assign_target_exprs.contains(&expr),
        );
    }
    if let TySlot::Resolved(TyKind::Ref { pointee, .. }) = ctx.slot(rr) {
        let pointee = *pointee;
        let pinned_non_ref = matches!(
            ctx.slot(ctx.resolve(result)),
            TySlot::Resolved(k) if !matches!(k, TyKind::Ref { .. })
        );
        if ctx.scrutinee_exprs.contains(&expr)
            || ctx.binding_init_exprs.contains(&expr)
            || ctx.assign_target_exprs.contains(&expr)
            || ctx.always_decay_exprs.contains(&expr)
            || pinned_non_ref
        {
            ctx.equal(result, pointee, span);
            return;
        }
    }
    ctx.equal(result, ret_tv, span);
}

pub(super) fn emit_resolved_call(
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
            // `T: not P` emits nothing here: negative bounds gate member
            // applicability (the kernel) and copy semantics, not body solving.
            crate::resolve::WhereClause::NotBound { .. } => {},
            crate::resolve::WhereClause::Bound {
                subject,
                protocol,
                protocol_type_args,
            } => {
                // G17 C10: a projection subject (`A.Item: P`) gets its call-site
                // obligation here too, same as a bare `A: P`. This is the
                // *overload-resolution* twin of the site C7 fixed in
                // `generate::emit_where_clause_constraints_with_subs`;
                // `emit_resolved_call`'s only callers are inside
                // `solve_overloaded_call`, so an unambiguous call never reaches
                // it. `Reduce`, not `Opaque`: the base is a call type-arg, so at
                // `good(src: StrSrc(…))` the obligation is really about `String`,
                // and an unreduced `StrSrc.Item` would be judged as an opaque
                // projection — which permits. `lower_subject` returns `None`
                // when the subject's root is not one of `subs`, and that skip is
                // the deliberate conservative permit for an abstract position.
                if let Some(tv) = crate::generate::lower_subject(
                    ctx,
                    &subject,
                    SubjectRoot::Subs(&subs),
                    crate::generate::ProjectionPolicy::Reduce(&span),
                ) {
                    ctx.conforms_typearg(tv, protocol, span.clone());
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
            crate::resolve::WhereClause::Equality { subject, rhs } => {
                match subject.equality_lhs() {
                    Some(EqualityLhs::Assoc { root: param, assoc }) => {
                        if let Some(&(_, tv)) = subs.iter().find(|(e, _)| *e == param) {
                            let assoc_result = ctx.fresh();
                            ctx.associated_entity(tv, assoc, assoc_result, span.clone());
                            let rhs_tv = lower_hir_ty_sub(ctx, &rhs, None, TyVar(0), &subs);
                            ctx.equal(assoc_result, rhs_tv, span.clone());
                        }
                    },
                    Some(EqualityLhs::Direct(param)) => {
                        if let Some(&(_, tv)) = subs.iter().find(|(e, _)| *e == param) {
                            let rhs_tv = lower_hir_ty_sub(ctx, &rhs, None, TyVar(0), &subs);
                            ctx.types[tv.0 as usize] = crate::ty::TySlot::Redirect(rhs_tv);
                        }
                    },
                    None => {},
                }
            },
        }
    }

    // Coerce each explicitly-provided arg against the type of the param it bound
    // to (per plan). Skipped params take their defaults, checked at their def site.
    if let Some(param_hir_tys) = qctx.query(LowerCallableTypes { entity, root }) {
        let plan = binding_plan_for(qctx, entity, &args);
        match plan {
            Some(plan) => {
                for (slot, binding) in plan.iter().enumerate() {
                    if let Binding::Arg(ai) = binding
                        && let Some(Some(hir_ty)) = param_hir_tys.get(slot)
                    {
                        let param_tv = lower_hir_ty_sub(ctx, hir_ty, None, TyVar(0), &subs);
                        ctx.coerce(args[*ai].ty, param_tv, args[*ai].value, span.clone());
                    }
                }
            },
            // No bindable plan (e.g. no Callable): fall back to positional coercion.
            None => {
                for (arg, param_ty) in args.iter().zip(param_hir_tys.iter()) {
                    if let Some(hir_ty) = param_ty {
                        let param_tv = lower_hir_ty_sub(ctx, hir_ty, None, TyVar(0), &subs);
                        ctx.coerce(arg.ty, param_tv, arg.value, span.clone());
                    }
                }
            },
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
            bind_call_result(ctx, result, ret_tv, expr, span);
        }
    } else {
        bind_call_result(ctx, result, ret_tv, expr, span);
    }

    SolveResult::Solved
}

/// Check if a candidate's param types are compatible with concrete arg types.
/// Does not mutate the type table — only inspects resolved types structurally.
pub(super) fn types_compatible(ctx: &InferCtx<'_>, entity: Entity, args: &[CallArg]) -> bool {
    let qctx = ctx.query_ctx;
    let root = ctx.root;

    let Some(param_hir_tys) = qctx.query(LowerCallableTypes { entity, root }) else {
        return false;
    };

    // Bind args to params (defaults skippable); reject if they don't bind.
    let Some(plan) = binding_plan_for(qctx, entity, args) else {
        return false;
    };

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

    for (slot, binding) in plan.iter().enumerate() {
        let Binding::Arg(ai) = binding else {
            continue; // defaulted-and-skipped param — nothing to check
        };
        let arg = &args[*ai];
        let Some(param_ty) = param_hir_tys.get(slot) else {
            continue;
        };
        let Some(hir_ty) = param_ty else {
            continue; // unannotated param — compatible with anything
        };

        // Get the concrete arg type
        let mut arg_resolved = ctx.resolve(arg.ty);
        // Transparent place: a ref-typed argument disambiguates as its
        // POINTEE — borrow params receive the place, consuming params the
        // decayed copy, so `f(box.peek())` matches `f(x: T)` overloads.
        if let crate::ty::TySlot::Resolved(crate::ty::TyKind::Ref { pointee, .. }) =
            &ctx.types[arg_resolved.0 as usize]
        {
            arg_resolved = ctx.resolve(*pointee);
        }
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
            // Arity only — deliberately NOT kind-aware. A closure literal is
            // always generated at `Normal` and only learns its real kind from
            // the expected type during coercion, so filtering candidates on the
            // kind here would reject every literal offered to an `escaping` /
            // `consuming` / `mutating` parameter before the retrofit could run.
            // Overloads that differ ONLY in a parameter's closure kind stay
            // ambiguous at this stage and are settled by the argument coercion.
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

/// Wrap a TyVar for an effectful init call site: `Self` → `Self?` or `Self throws E`.
pub(super) fn wrap_init_call_result(
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
