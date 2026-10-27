//! Equality, coercion and closure-kind reconciliation: `Equal`, `Coerce`,
//! `EqualDecayed`, `AssignTarget`, `BorrowPointee`.

use super::*;

pub(super) fn solve_equal(ctx: &mut InferCtx<'_>, a: TyVar, b: TyVar, span: Span) -> SolveResult {
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
                reason: Reason::Unspecified,
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
        // Structurally identical function types that disagree only on the
        // closure tier — E624, not a generic "type mismatch".
        Err(UnifyError::KindMismatch { expected, actual }) => {
            SolveResult::Error(InferError::KindMismatch {
                expected,
                actual,
                span,
            })
        },
        Err(UnifyError::LiteralGuard) => {
            // Literal couldn't unify — could be deferred or error.
            // If both sides are concrete, it's an error.
            if ctx.is_concrete(a) && ctx.is_concrete(b) {
                SolveResult::Error(mismatch_error(ctx, a, b, span))
            } else {
                SolveResult::Deferred(Constraint::Equal {
                    a,
                    b,
                    span,
                    reason: Reason::Unspecified,
                })
            }
        },
        Err(UnifyError::OccursCheck) => SolveResult::Error(InferError::InfiniteType { span }),
    }
}

/// Report an error (captures detail against pristine TyVars), then poison
/// both sides so downstream constraints absorb. Order matters: `report_error`
/// must run first so the detail reflects pre-poison types.
pub(super) fn report_and_poison(ctx: &mut InferCtx<'_>, err: InferError, a: TyVar, b: TyVar) {
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
pub(crate) fn target_accepts_promotion(ctx: &InferCtx<'_>, to: TyVar) -> bool {
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

pub(super) fn mismatch_error(ctx: &InferCtx<'_>, a: TyVar, b: TyVar, span: Span) -> InferError {
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
        reason: Reason::Unspecified,
    }
}

pub(super) fn try_literal_mismatch(
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
            protocol_args: Vec::new(),
            span,
        },
        None => InferError::LiteralNotAccepted {
            ty: ty_side,
            literal,
            span,
        },
    })
}

/// `&inner` pointee resolution — see `Constraint::BorrowPointee`. The
/// Borrow expr's own var is already a resolved `Ref { pointee }`; this
/// fills the pointee: a ref inner re-borrows (same pointee), a value
/// inner borrows the place directly. Mutability legality (`&mutating` of
/// a shared ref / immutable place) is analyze's job, not typing's.
pub(super) fn solve_borrow_pointee(
    ctx: &mut InferCtx<'_>,
    inner: TyVar,
    pointee: TyVar,
    span: Span,
) -> SolveResult {
    let ir = ctx.resolve(inner);
    let target = match ctx.slot(ir) {
        // Re-borrow: `&r` where r is already `&T` — borrow the same place.
        TySlot::Resolved(TyKind::Ref { pointee: p, .. }) => *p,
        // Concrete value type: borrow the place itself.
        TySlot::Resolved(_) => ir,
        // A literal can never become a ref — pin the pointee now so
        // literal defaulting can settle it (mirrors solve_coerce's arm).
        TySlot::Unresolved { literal: Some(_) } => ir,
        // Unresolved (e.g. a pending ref-returning accessor call): WAIT —
        // eagerly unifying would pin the call's in-flight `&T` binding.
        _ => {
            return SolveResult::Deferred(Constraint::BorrowPointee {
                inner,
                pointee,
                span,
            });
        },
    };
    match unify::unify(ctx, pointee, target) {
        Ok(()) => SolveResult::Solved,
        Err(_) => SolveResult::Error(mismatch_error(ctx, pointee, target, span)),
    }
}

/// Equal with the ref-decay dimension (see `Constraint::EqualDecayed`):
/// arm results and tuple-literal elements decay a REF value to its pointee
/// UNLESS the target position is itself a ref (an annotation or `-> &T`
/// pinned it) — and wait while the target is unresolved so the annotation
/// can win the race (unpinned targets decay in `apply_ref_decay_defaults`).
pub(super) fn solve_equal_decayed(
    ctx: &mut InferCtx<'_>,
    value: TyVar,
    target: TyVar,
    span: Span,
) -> SolveResult {
    let vr = ctx.resolve(value);
    let (pointee, value_is_ref) = match ctx.slot(vr) {
        TySlot::Resolved(TyKind::Ref { pointee, .. }) => (*pointee, true),
        TySlot::Resolved(_) | TySlot::Unresolved { literal: Some(_) } => (vr, false),
        // Unresolved value (e.g. a pattern-payload binding whose scrutinee
        // hasn't resolved): WAIT.
        _ => {
            return SolveResult::Deferred(Constraint::EqualDecayed {
                value,
                target,
                span,
                reason: Reason::Unspecified,
            });
        },
    };
    if value_is_ref {
        match ctx.slot(ctx.resolve(target)) {
            // Ref slot pinned by the position: ref-to-ref equality.
            TySlot::Resolved(TyKind::Ref { .. }) => {
                return match unify::unify(ctx, value, target) {
                    Ok(()) => SolveResult::Solved,
                    Err(_) => SolveResult::Error(mismatch_error(ctx, target, value, span)),
                };
            },
            // Nothing pinned the target yet: wait for the annotation race.
            TySlot::Unresolved { literal: None } => {
                return SolveResult::Deferred(Constraint::EqualDecayed {
                    value,
                    target,
                    span,
                    reason: Reason::Unspecified,
                });
            },
            // Non-ref (or literal) target: decay to the pointee below.
            _ => {},
        }
    }
    match unify::unify(ctx, pointee, target) {
        Ok(()) => SolveResult::Solved,
        Err(_) => SolveResult::Error(mismatch_error(ctx, target, pointee, span)),
    }
}

/// Assignment into a LOCAL target (see `Constraint::AssignTarget`): a
/// ref-typed target is store-through — the RHS coerces to the POINTEE; a
/// resolved non-ref (or literal-kinded) target is a plain Coerce; an
/// unresolved target WAITS for whatever owns its type (a deferred
/// ImplicitPat's payload, an annotation).
pub(super) fn solve_assign_target(
    ctx: &mut InferCtx<'_>,
    value: TyVar,
    target: TyVar,
    expr: kestrel_hir::body::HirExprId,
    span: Span,
) -> SolveResult {
    match ctx.slot(ctx.resolve(target)) {
        TySlot::Resolved(TyKind::Ref { pointee, .. }) => {
            let pointee = *pointee;
            solve_coerce(ctx, value, pointee, expr, span)
        },
        TySlot::Resolved(_) | TySlot::Unresolved { literal: Some(_) } => {
            solve_coerce(ctx, value, target, expr, span)
        },
        _ => SolveResult::Deferred(Constraint::AssignTarget {
            value,
            target,
            expr,
            span,
        }),
    }
}

/// Post-relaxation stall-breaker for `AssignTarget`: a target nothing
/// resolved (no pattern fired, no annotation pinned it) takes the plain
/// Coerce — the deferral existed only so a deferred pattern could deliver
/// the target's ref-ness first; once the literal-relaxation loop exhausts,
/// the system is at a fixpoint and waiting longer cannot help. This keeps
/// assignment-driven inference (`x = 5` pinning `x`) working for locals
/// whose type has no other source.
pub(super) fn solve_coerce(
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

    // Stage-1 transparent place: the two ref coercion arms, BEFORE the unify
    // attempt (unify(unresolved_from, Ref_to) would bind the body var to the
    // ref wholesale).
    //
    // 1. `to` is a ref — only reachable from return-position coercion (params
    //    never carry ref types; E480 is permanent). From-ref: §10.1's one-way
    //    `&mutating → &` plus pointee unification. From non-ref/unresolved:
    //    the return-position implicit borrow — body exprs type as `T`; the
    //    borrow is MIR's, the escape checker judges the root.
    if let TySlot::Resolved(TyKind::Ref {
        pointee: tp,
        mutating: tm,
    }) = ctx.slot(tr)
    {
        let (tp, tm) = (*tp, *tm);
        match ctx.slot(fr) {
            TySlot::Resolved(TyKind::Ref {
                pointee: fp,
                mutating: fm,
            }) => {
                let (fp, fm) = (*fp, *fm);
                // Equal mutability, or mut→shared decay. Shared→mutating is
                // the const-cast mismatch.
                if fm == tm || (fm && !tm) {
                    return match unify::unify(ctx, fp, tp) {
                        Ok(()) => SolveResult::Solved,
                        Err(_) => SolveResult::Error(mismatch_error(ctx, to, from, span)),
                    };
                }
                return SolveResult::Error(mismatch_error(ctx, to, from, span));
            },
            TySlot::Resolved(_) => {
                // Return-position implicit borrow: a `T`-typed body expr
                // satisfies `-> &T`; the borrow is MIR's, the escape checker
                // judges the root.
                return match unify::unify(ctx, from, tp) {
                    Ok(()) => SolveResult::Solved,
                    Err(_) => SolveResult::Error(mismatch_error(ctx, to, from, span)),
                };
            },
            TySlot::Unresolved { literal: Some(_) } => {
                // A literal can never be a pending ref-call result, so pin it
                // to the POINTEE now (deferring would strand it: in no-stdlib
                // bodies defaulting can't rescue it and the retry would
                // bottom out against the whole `&T`).
                return match unify::unify(ctx, from, tp) {
                    Ok(()) => SolveResult::Solved,
                    Err(_) => SolveResult::Error(mismatch_error(ctx, to, from, span)),
                };
            },
            _ => {
                // Unresolved source: WAIT. Eagerly unifying with the pointee
                // would pin a ref-returning call's pending result var to the
                // pointee and break its in-flight `result = &T` binding
                // (order-fragile).
                return SolveResult::Deferred(Constraint::Coerce {
                    from,
                    to,
                    expr,
                    span,
                    reason: Reason::Unspecified,
                });
            },
        }
    }
    // 2. `from` is a ref, `to` is not — copy-out DECAY (binding decay,
    //    consuming args, assignment RHS, return-of-T): the value read is of
    //    the POINTEE type; continue the whole coercion (incl. FromValue
    //    promotion — a promotion reads the place) with the pointee.
    if let TySlot::Resolved(TyKind::Ref { pointee: fp, .. }) = ctx.slot(fr) {
        // Stage 2b: a FULLY-unresolved target WAITS — an annotation
        // (`let o: Optional[&Int64] = .Some(r)`) may be about to pin it
        // through the ref type itself; decaying now loses that race and
        // manufactures a mismatch. Literal targets can't be refs (arm-1
        // twin) and decay immediately. Targets nothing ever pins are
        // decayed by `apply_ref_decay_defaults` between fixpoint phases.
        if matches!(
            ctx.slot(ctx.resolve(to)),
            TySlot::Unresolved { literal: None }
        ) {
            return SolveResult::Deferred(Constraint::Coerce {
                from,
                to,
                expr,
                span,
                reason: Reason::Unspecified,
            });
        }
        let fp = *fp;
        return solve_coerce(ctx, fp, to, expr, span);
    }

    // Check-mode leaf with a pending synthesized type (see
    // `InferCtx::synth_pending_exprs`): wait for it, so promotion can apply.
    if ctx.synth_gate
        && ctx.synth_pending_exprs.contains(&expr)
        && matches!(ctx.slot(fr), TySlot::Unresolved { literal: None })
        && matches!(ctx.slot(tr), TySlot::Resolved(_))
    {
        return SolveResult::Deferred(Constraint::Coerce {
            from,
            to,
            expr,
            span,
            reason: Reason::Unspecified,
        });
    }

    // Closure KINDS (closures.md "Passing: What Fits Where"). Whole-type scalar
    // check, so it must sit OUTSIDE the per-param convention loop below (which
    // never executes for a 0-param flex closure) and ABOVE the
    // `closure_flex`/`closure_it` arity-adaptation arm further down (which
    // early-returns `Solved` after equating return types only). Handles the
    // literal retrofit, the bare-value adoption, and the directional table;
    // returns `Some` only when it has fully decided the coercion.
    if let Some(result) = reconcile_fn_kinds(ctx, from, to, expr, &span) {
        return result;
    }

    // #178: a closure literal coerced to an expected function type
    // (`let f: (mutating T) -> () = { (x) in … }`) upgrades its param
    // conventions exactly like a call argument does (reconcile_fn_convention at
    // the call site). `unify` ignores conventions, so without this the closure
    // keeps its default `Consuming` convention and a `mutating`-requiring body
    // is falsely rejected with E201. Only fires once both sides resolve to
    // Functions (else it no-ops and the unify below defers, re-running coerce).
    if ctx.closure_literal_exprs.contains(&expr)
        && let Some(err) = reconcile_closure_conventions(ctx, from, to, &span)
    {
        ctx.errored_coerce_exprs.insert(expr);
        return SolveResult::Error(err);
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
        // A NESTED kind disagreement (`(mutating (T) -> R) -> U` vs
        // `((T) -> R) -> U`): the shapes match and only the tier differs, so
        // report E624 straight away. No `FromValue` promotion can rescue a
        // function type, so there is nothing to fall through to.
        Err(UnifyError::KindMismatch { expected, actual }) => {
            ctx.errored_coerce_exprs.insert(expr);
            return SolveResult::Error(InferError::KindMismatch {
                expected,
                actual,
                span,
            });
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
            reason: Reason::Unspecified,
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
                        reason: Reason::Unspecified,
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
            ..
        },
        TyKind::Function {
            ret: ret_b,
            params: pb,
            ..
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

/// Whether a closure of kind `from` may be passed where kind `to` is expected —
/// the directional passing table of closures.md ("Passing: What Fits Where"):
///
/// | have ↓ \ expected → | normal | mutating | consuming | escaping |
/// |---|---|---|---|---|
/// | normal    | ✓ | ✓ | ✗ frame view is not owned | ✗ frame-bound |
/// | mutating  | ✗ | ✓ | ✗ frame view is not owned | ✗ frame-bound |
/// | consuming | ✗ | ✗ | ✓ | ✗ one-shot |
/// | escaping  | ✓ | ✗ shared, not exclusive | ✓ | ✓ |
///
/// The diagonal is handled by the caller (equal kinds never reach here).
pub(super) fn kind_passes(from: kestrel_ast::FnTypeKind, to: kestrel_ast::FnTypeKind) -> bool {
    use kestrel_ast::FnTypeKind::*;
    matches!(
        (from, to),
        (Normal, Mutating) | (Escaping, Normal) | (Escaping, Consuming)
    )
}

/// Reconcile a closure/function value's KIND against an expected function type.
///
/// Three cases, in order:
/// 1. **Literal retrofit** — a closure literal is always built at `Normal`, and
///    the expected type selects the environment it is built with, so the
///    literal's kind is rewritten in place. Whether the *body* supports that
///    kind (assigning to a capture in a normal body, moving a capture out of a
///    non-`consuming` body) is a later analyzer's judgement, not this one's.
/// 2. **Bare value adoption** — a `kind_flex` value (named `Def` / enum-case
///    ctor) has no environment and satisfies every kind, so it adopts the
///    expected kind. A cross-kind adoption is still a representation change
///    (2 words → an owning layout), so it is recorded in `kind_coercions`.
/// 3. **The passing table** — for a real closure value. Accepted cross-kind
///    cells unify params/return pairwise and return `Solved` *without*
///    structurally unifying the two function types (whole-type `unify` demands
///    kind equality) and record `kind_coercions` for MIR; the source value is
///    never re-labelled (`escaping → normal` produces a non-owning view, and
///    bit-copying a relabelled shared handle would skip the share operation).
///
/// Returns `None` when the coercion is not (or not yet) a kind decision — the
/// caller then proceeds with the ordinary convention/unify path. Like
/// `reconcile_closure_conventions`, an unresolved side yields `None`: `unify`
/// below either binds it or defers the whole `Coerce`, which re-runs this.
pub(super) fn reconcile_fn_kinds(
    ctx: &mut InferCtx<'_>,
    from: TyVar,
    to: TyVar,
    expr: kestrel_hir::body::HirExprId,
    span: &Span,
) -> Option<SolveResult> {
    let (fr, tr) = (ctx.resolve(from), ctx.resolve(to));
    let (from_kind, from_params, from_ret) = match ctx.slot(fr) {
        TySlot::Resolved(TyKind::Function {
            kind, params, ret, ..
        }) => (*kind, params.clone(), *ret),
        _ => return None,
    };
    let (to_kind, to_params, to_ret) = match ctx.slot(tr) {
        TySlot::Resolved(TyKind::Function {
            kind, params, ret, ..
        }) => (*kind, params.clone(), *ret),
        _ => return None,
    };

    // (1) Literal retrofit. The gate is `closure_literal_exprs`, which
    // `gen_expr` propagates outward through `Block`/`Sugar` wrappers — a
    // trailing-closure call site carries the WRAPPER's id in the `Coerce`.
    if ctx.closure_literal_exprs.contains(&expr) {
        if from_kind != to_kind {
            ctx.set_function_kind(fr, to_kind);
        }
        return None;
    }

    // (2) Bare-value adoption (either direction; both bare stays bare).
    let (from_flex, to_flex) = (ctx.kind_flex.contains(&fr), ctx.kind_flex.contains(&tr));
    if from_flex || to_flex {
        if from_flex && !to_flex && from_kind != to_kind {
            ctx.set_function_kind(fr, to_kind);
            ctx.kind_coercions.insert(expr, (from_kind, to_kind));
        } else if to_flex && !from_flex && from_kind != to_kind {
            ctx.set_function_kind(tr, from_kind);
        }
        return None;
    }

    // (3) The passing table. The diagonal falls through to the ordinary path.
    if from_kind == to_kind {
        return None;
    }
    if !kind_passes(from_kind, to_kind) {
        ctx.errored_coerce_exprs.insert(expr);
        return Some(SolveResult::Error(InferError::KindMismatch {
            expected: to_kind,
            actual: from_kind,
            span: span.clone(),
        }));
    }

    // Accepted cross-kind cell: settle params/return ourselves.
    if from_params.len() != to_params.len() {
        // A 0-param flex closure adapts to any expected arity (the same rule the
        // arity-adaptation arm applies); anything else is a real arity mismatch.
        if from_params.is_empty() && ctx.closure_flex.contains(&fr) {
            ctx.equal(from_ret, to_ret, span.clone());
            ctx.kind_coercions.insert(expr, (from_kind, to_kind));
            return Some(SolveResult::Solved);
        }
        ctx.errored_coerce_exprs.insert(expr);
        return Some(SolveResult::Error(mismatch_error(
            ctx,
            to,
            from,
            span.clone(),
        )));
    }
    for (a, b) in from_params.into_iter().zip(to_params) {
        if unify::unify(ctx, a, b).is_err() {
            ctx.errored_coerce_exprs.insert(expr);
            return Some(SolveResult::Error(mismatch_error(
                ctx,
                to,
                from,
                span.clone(),
            )));
        }
    }
    if unify::unify(ctx, from_ret, to_ret).is_err() {
        ctx.errored_coerce_exprs.insert(expr);
        return Some(SolveResult::Error(mismatch_error(
            ctx,
            to,
            from,
            span.clone(),
        )));
    }
    ctx.kind_coercions.insert(expr, (from_kind, to_kind));
    Some(SolveResult::Solved)
}

/// Reconcile a function-typed call argument's param conventions against the
/// expected parameter type (#106).
///
/// There is deliberately NO kind twin here. Conventions need this hook because
/// `solve_coerce`'s convention reconcile is gated on `closure_literal_exprs`, so
/// an opaque function VALUE would otherwise never be checked. The kind check in
/// `reconcile_fn_kinds` is ungated, and `solve_call`'s `Function` arm routes
/// every argument through `ctx.coerce` — so calls through a function-typed value
/// already get the passing table. Adding a second call here would double-report.
///
/// For a closure *literal*, a non-`MutBorrow`
/// param is upgraded to `MutBorrow` when the expected type demands it (the
/// no-annotation `arr.modify { it.len += 1 }` inference). Passing a `MutBorrow`
/// closure where a non-mutating param is expected is a hard error. Returns
/// `Some(err)` only on that mismatch; the upgrade is an in-place side effect.
pub(super) fn reconcile_fn_convention(
    ctx: &mut InferCtx<'_>,
    arg: &CallArg,
    param: TyVar,
    span: &Span,
) -> Option<InferError> {
    reconcile_closure_conventions(ctx, arg.ty, param, span)
}

/// Check a closure/function value's param conventions against an expected
/// function type: passing a `MutBorrow` closure where a non-mutating param is
/// expected is a hard error. (A closure *literal* no longer needs an in-place
/// `MutBorrow` upgrade here: `check_closure` builds it with the expected
/// type's conventions — bidi §8. The upgrade never fired over the corpus
/// after P3 and was removed in P4.)
pub(super) fn reconcile_closure_conventions(
    ctx: &mut InferCtx<'_>,
    actual_ty: TyVar,
    expected_ty: TyVar,
    span: &Span,
) -> Option<InferError> {
    use kestrel_ast::ParamConvention::MutBorrow;

    // Expected per-param conventions come from the expected function type.
    let expected = match ctx.slot(expected_ty) {
        TySlot::Resolved(TyKind::Function { conventions, .. }) => conventions.clone(),
        _ => return None,
    };
    // Actual conventions from the value's (closure/function) type.
    let actual = match ctx.slot(actual_ty) {
        TySlot::Resolved(TyKind::Function { conventions, .. }) => conventions.clone(),
        _ => return None,
    };

    // A mutating closure can't be passed where the callee promises no
    // mutable place.
    let mismatch = expected
        .iter()
        .zip(&actual)
        .any(|(e, a)| *e != MutBorrow && *a == MutBorrow);
    mismatch.then(|| InferError::ConventionMismatch { span: span.clone() })
}
