//! End-of-body rules: operator shape projection, Rule L, literal and
//! ref-decay defaults, type-parameter defaults (run by
//! [`crate::obligations::finish_body`]).

use super::*;

/// Rule L (bidi design §4 step 2; Rust's built-in binop rule): for
/// `lit op x`, an operator whose receiver is still an open literal var adopts
/// the right operand's type `B` when `B` is concrete, `B` can be written as
/// that literal, and `B` conforms to the operator's protocol. The mirror,
/// `x op lit`, already works through argument coercion. Returns true on
/// progress.
/// Rule L's test for one operator: the receiver is an open literal and the
/// right operand a concrete nominal that can be written as that literal and
/// conforms to the operator protocol. Shared by the walker (at the operator)
/// and the end-of-body pass (for operands that became known later).
pub(crate) fn rule_l_applies(
    ctx: &InferCtx<'_>,
    recv: TyVar,
    arg: TyVar,
    protocol: Entity,
) -> bool {
    let TySlot::Unresolved { literal: Some(lit) } = ctx.slot(ctx.resolve(recv)) else {
        return false;
    };
    let TySlot::Resolved(kind) = ctx.slot(ctx.resolve(arg)) else {
        return false;
    };
    kind.is_nominal_concrete()
        && crate::unify::conforms_to_literal_protocol(ctx, kind, *lit)
        && ctx.resolver.conforms_to(kind, protocol)
}

pub(crate) fn apply_operator_literal_adoption(ctx: &mut InferCtx<'_>) -> bool {
    let mut pairs: Vec<(TyVar, TyVar)> = Vec::new();
    for c in &ctx.constraints {
        let Constraint::Member {
            receiver,
            args,
            expr,
            ..
        } = c
        else {
            continue;
        };
        let Some(&protocol) = ctx.operator_members.get(expr) else {
            continue;
        };
        if args.len() != 1 {
            continue;
        }
        if rule_l_applies(ctx, *receiver, args[0].ty, protocol) {
            pairs.push((ctx.resolve(*receiver), ctx.resolve(args[0].ty)));
        }
    }
    let mut progress = false;
    for (recv, arg) in pairs {
        // An earlier pair may already have pinned this receiver.
        if matches!(
            ctx.slot(ctx.resolve(recv)),
            TySlot::Unresolved { literal: Some(_) }
        ) && crate::unify::unify(ctx, recv, arg).is_ok()
        {
            kestrel_debug::ktrace!("rule-l", "literal receiver adopts the right operand's type");
            progress = true;
        }
    }
    progress
}

/// Apply deferred type-parameter defaults for TyVars that are still
/// unconstrained. Returns true if any default was applied.
/// Stage-2b ref-decay defaulting: a deferred ref→unresolved `Coerce` whose
/// target NOTHING else pinned takes the stage-1.5 copy-out decay now — the
/// value position owns a copy of the POINTEE (`let y = r` gives the pointee
/// type). Runs inside the literal-relaxation loop so a decayed target can
/// still pin blocked literals before force-defaulting. The deferral itself
/// lives in `solve_coerce` arm 2: an annotation pinning the target to the
/// ref TYPE (`let o: Optional[&Int64] = .Some(r)`) must win the race.
pub(crate) fn apply_ref_decay_defaults(ctx: &mut InferCtx<'_>) -> bool {
    apply_ref_decay_defaults_into(ctx, None)
}

/// [`apply_ref_decay_defaults`], restricted to coercions whose target is
/// `only` when given: the checker needs one binding's type NOW (its use as
/// a receiver, bidi D3), and the copy-out decay is that binding's default.
pub(crate) fn apply_ref_decay_defaults_into(ctx: &mut InferCtx<'_>, only: Option<TyVar>) -> bool {
    let only = only.map(|t| ctx.resolve(t));
    let mut decays: Vec<(TyVar, TyVar)> = Vec::new();
    for c in &ctx.constraints {
        let (from, to) = match c {
            Constraint::Coerce { from, to, .. } => (from, to),
            // Arm results / tuple-literal elements wait on the same race.
            Constraint::EqualDecayed { value, target, .. } => (value, target),
            _ => continue,
        };
        let fr = ctx.resolve(*from);
        let TySlot::Resolved(TyKind::Ref { pointee, .. }) = ctx.slot(fr) else {
            continue;
        };
        if only.is_some_and(|o| ctx.resolve(*to) != o) {
            continue;
        }
        if matches!(
            ctx.slot(ctx.resolve(*to)),
            TySlot::Unresolved { literal: None }
        ) {
            decays.push((*to, *pointee));
        }
    }
    let mut progress = false;
    for (to, pointee) in decays {
        // Re-check: an earlier decay in this batch may have pinned it.
        if matches!(
            ctx.slot(ctx.resolve(to)),
            TySlot::Unresolved { literal: None }
        ) && unify::unify(ctx, to, pointee).is_ok()
        {
            progress = true;
        }
    }
    progress
}

/// Stage 2c: operator-result shape projection.
///
/// `100..=200` desugars to `100.inclusiveRange(to: 200)` — a Member on a
/// still-unresolved literal receiver, so the member defers and its result
/// TyVar stays a bare unknown. Any downstream information (an annotation
/// `ClosedRange[Int16]`, or a parameterized bound `RandomBounds[Int16]`
/// via `unify_bound_protocol_args`) can only flow back into the literal
/// through that result var — but only once it has a nominal *shape*.
///
/// This stage supplies the shape without picking an element type: for a
/// deferred operator-protocol member (name from the HIR desugar tables) on
/// a numeric-literal receiver, look the method up on the literal's DEFAULT
/// type and generalize its return type by substituting the default entity
/// with the receiver's own TyVar (`ClosedRange[Int64]` at receiver `?a` →
/// `ClosedRange[?a]`). Unification with the expectation then pins `?a`,
/// which un-defers the member — bidirectional flow through the operator.
///
/// Sound by construction: if the generalization is wrong for the type the
/// receiver ends up pinned to, the member's real return type fails to
/// unify with the projection — a type error, never a miscompile. Only the
/// generic-wrapper shape projects (return mentions the default entity
/// under a different head constructor); `Output = Self` operators
/// (`negate`, `add`, …) already flow through literal-marker unification
/// and the context-driven pass in `apply_literal_defaults`.
pub(crate) fn apply_operator_shape_projections(ctx: &mut InferCtx<'_>) -> bool {
    // Collect candidates immutably first; project after.
    let mut candidates: Vec<(TyVar, TyVar, String, Vec<crate::constraint::CallArg>)> = Vec::new();
    for c in &ctx.constraints {
        let Constraint::Member {
            receiver,
            name,
            args,
            result,
            expr,
            is_call: true,
            is_static_context: false,
            ..
        } = c
        else {
            continue;
        };
        // Operator desugars only — HIR marks them (ProtocolCall.from_operator)
        // and generate records the expr ids.
        if !ctx.operator_members.contains_key(expr) {
            continue;
        }
        let recv = ctx.resolve(*receiver);
        // Numeric literal receivers only — the shape generalizes from the
        // default type across the numeric family; other literal kinds have
        // a single conformer anyway and gain nothing.
        if !matches!(
            ctx.slot(recv),
            TySlot::Unresolved {
                literal: Some(LiteralKind::Integer | LiteralKind::Float)
            }
        ) {
            continue;
        }
        // Result: either a bare unknown (bound/annotation arrives later —
        // `low16(100..=200)`) or already pinned by a Coerce to a concrete
        // nominal (`let r: ClosedRange[Int16] = 100..=200` binds the result
        // during phase 1). Both unify against the projected shape; only a
        // literal-marked result (its own literal) is out of scope.
        let res = ctx.resolve(*result);
        match ctx.slot(res) {
            TySlot::Unresolved { literal: None } => {},
            TySlot::Resolved(TyKind::Struct { .. } | TyKind::Enum { .. }) => {},
            _ => continue,
        }
        kestrel_debug::ktrace!("op-shape", "candidate {name} recv={recv:?} res={res:?}");
        candidates.push((recv, res, name.clone(), args.clone()));
    }

    let mut progress = false;
    for (recv, res, name, args) in candidates {
        progress |= project_operator_shape(ctx, recv, res, &name, &args);
    }
    progress
}

/// The shape projection for one operator (see
/// [`apply_operator_shape_projections`]): give `res` the default type's
/// generic-wrapper return shape with the literal `recv` in the element
/// position. Called by the walker at the operator (bidi P5) and by the
/// end-of-body pass for operators it could not decide then. Returns true
/// when it added information.
pub(crate) fn project_operator_shape(
    ctx: &mut InferCtx<'_>,
    recv: TyVar,
    res: TyVar,
    name: &str,
    args: &[CallArg],
) -> bool {
    let TySlot::Unresolved { literal: Some(lit) } = ctx.slot(ctx.resolve(recv)) else {
        return false;
    };
    let feature = match lit {
        LiteralKind::Integer => Builtin::DefaultIntegerLiteralType,
        LiteralKind::Float => Builtin::DefaultFloatLiteralType,
        _ => return false,
    };
    let Some(default_entity) = ctx.resolver.builtin(feature) else {
        return false;
    };
    let default_kind = TyKind::Struct {
        entity: default_entity,
        args: vec![],
    };
    let Ok(resolution) = ctx.resolver.resolve_member(&default_kind, name, args) else {
        return false;
    };
    // Project only the generic-wrapper shape: a nominal head other than the
    // default type, whose args mention the default type. Anything else
    // (Self-typed returns, params, aliases, fn types) declines — this stage
    // must never guess where unanimity across the numeric family isn't
    // structurally plausible.
    if !projectable_wrapper_return(&resolution.return_type, default_entity) {
        return false;
    }
    let res_was_unresolved = matches!(
        ctx.slot(ctx.resolve(res)),
        TySlot::Unresolved { literal: None }
    );
    let subs = [(default_entity, ctx.resolve(recv))];
    let projected = lower_hir_ty_sub(ctx, &resolution.return_type, None, res, &subs);
    let ok = unify::unify(ctx, res, projected).is_ok();
    // Progress only when the projection ADDED information: the result gained
    // its shape, or the receiver literal got pinned through an already-shaped
    // result. A projection that merely re-unifies fresh vars must NOT count,
    // or the end-of-body loop re-fires it forever and never defaults.
    let recv_pinned = !matches!(
        ctx.slot(ctx.resolve(recv)),
        TySlot::Unresolved { literal: Some(_) }
    );
    kestrel_debug::ktrace!(
        "op-shape",
        "project {name}: ret={:?} ok={ok} res_new={res_was_unresolved} recv_pinned={recv_pinned}",
        resolution.return_type
    );
    ok && (res_was_unresolved || recv_pinned)
}

/// Shared body of the context-driven pass in `apply_literal_defaults`: a
/// literal receiver whose member/call result is already concrete adopts the
/// result type — valid only where result type ≈ receiver type (operator
/// members, subscript-style calls).
pub(super) fn collect_context_literal(
    ctx: &InferCtx<'_>,
    receiver: TyVar,
    result: TyVar,
    context_types: &mut Vec<(TyVar, TyVar)>,
) {
    let recv_resolved = ctx.resolve(receiver);
    let lit = match &ctx.types[recv_resolved.0 as usize] {
        TySlot::Unresolved { literal: Some(lit) } => *lit,
        _ => return,
    };
    let result_resolved = ctx.resolve(result);
    if let TySlot::Resolved(kind) = &ctx.types[result_resolved.0 as usize] {
        // Verify the concrete type conforms to the literal protocol.
        if unify::conforms_to_literal_protocol(ctx, kind, lit) {
            context_types.push((recv_resolved, result_resolved));
        }
    }
}

/// Is `ty` a nominal wrapper `Head[..default..]` safe to generalize — head a
/// Struct/Enum other than `default_entity`, all args recursively nominal, and
/// at least one arg mentioning `default_entity` (the Self-varying position)?
pub(super) fn projectable_wrapper_return(
    ty: &kestrel_hir::ty::HirTy,
    default_entity: Entity,
) -> bool {
    use kestrel_hir::ty::HirTy;
    let (entity, args) = match ty {
        HirTy::Struct { entity, args, .. } | HirTy::Enum { entity, args, .. } => (*entity, args),
        _ => return false,
    };
    if entity == default_entity || args.is_empty() {
        return false;
    }
    fn nominal_only(ty: &HirTy, default_entity: Entity, mentions: &mut bool) -> bool {
        match ty {
            HirTy::Struct { entity, args, .. } | HirTy::Enum { entity, args, .. } => {
                if *entity == default_entity {
                    *mentions = true;
                }
                args.iter()
                    .all(|a| nominal_only(a, default_entity, mentions))
            },
            _ => false,
        }
    }
    let mut mentions = false;
    args.iter()
        .all(|a| nominal_only(a, default_entity, &mut mentions))
        && mentions
}

pub(crate) fn apply_type_param_defaults(ctx: &mut InferCtx<'_>) -> bool {
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
pub(crate) fn apply_literal_defaults(ctx: &mut InferCtx<'_>, relax_level: u8) -> bool {
    // Context first, and alone: a context pick can unblock arguments the
    // solver should type from it (`100 + 27` against `Int8` pins `100`, then
    // `27` must flow from `add`'s parameter, not default to Int64).
    if apply_context_literals(ctx) {
        return true;
    }
    apply_literal_defaults_rest(ctx, relax_level)
}

/// The context-driven first pass of `apply_literal_defaults`.
/// Literals typed by their context before any default applies:
/// - an operator's open literal receiver takes the type Rule C picks from
///   the operator's expected result (`let y: Int8 = 100 + 27`,
///   `let r: ClosedRange[Int16] = 100..=200`, `-1` into an `Int32` slot,
///   `let c: Bool = 1 == 2` → `Int64`);
/// - a literal callee of a subscript-like `Call` takes the call's resolved
///   result type.
pub(crate) fn apply_context_literals(ctx: &mut InferCtx<'_>) -> bool {
    let mut picks: Vec<(TyVar, Entity)> = Vec::new();
    let mut context_types: Vec<(TyVar, TyVar)> = Vec::new();
    for constraint in &ctx.constraints {
        match constraint {
            Constraint::Member {
                receiver,
                result,
                expr,
                name,
                args,
                ..
            } if ctx.operator_members.contains_key(expr) => {
                let recv = ctx.resolve(*receiver);
                let TySlot::Unresolved { literal: Some(lit) } = ctx.slot(recv) else {
                    continue;
                };
                if let Some(c) = crate::solver_entail::rule_c_pick(ctx, *lit, *result, name, args) {
                    picks.push((recv, c));
                }
            },
            Constraint::Call {
                callee: receiver,
                result,
                ..
            } => collect_context_literal(ctx, *receiver, *result, &mut context_types),
            _ => {},
        }
    }
    let mut progress = false;
    for (recv, entity) in picks {
        if matches!(
            ctx.slot(ctx.resolve(recv)),
            TySlot::Unresolved { literal: Some(_) }
        ) {
            let ty = ctx.named(entity, Vec::new());
            progress |= unify::unify(ctx, recv, ty).is_ok();
        }
    }
    for (literal_tv, context_tv) in &context_types {
        if matches!(
            &ctx.types[literal_tv.0 as usize],
            TySlot::Unresolved { literal: Some(_) }
        ) {
            ctx.types[literal_tv.0 as usize] = TySlot::Redirect(*context_tv);
            progress = true;
        }
    }
    progress
}

pub(crate) fn apply_literal_defaults_rest(ctx: &mut InferCtx<'_>, relax_level: u8) -> bool {
    apply_literal_defaults_from(ctx, relax_level, 0)
}

/// Literal defaulting restricted to TyVars allocated at or after `start`.
/// Give the open literal var `resolved` its `Default<Kind>LiteralType`.
/// The single place a literal takes its default. Returns false when the
/// builtin is absent (`// stdlib: false`), leaving the literal open.
pub(crate) fn default_literal_var(
    ctx: &mut InferCtx<'_>,
    resolved: TyVar,
    literal: LiteralKind,
) -> bool {
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
    let Some(entity) = ctx.resolver.builtin(feature) else {
        return false;
    };
    // Fresh TyVars for the default's type params, so `DefaultArrayLiteralType[T]`
    // becomes `Array[?]` with a real inference slot that later Associated /
    // Coerce constraints flow element types into.
    let type_param_count = ctx
        .query_ctx
        .get::<TypeParams>(entity)
        .map(|tp| tp.0.len())
        .unwrap_or(0);
    let args: Vec<TyVar> = (0..type_param_count).map(|_| ctx.fresh()).collect();
    let default_tv = ctx.named(entity, args);
    ctx.types[resolved.0 as usize] = TySlot::Redirect(default_tv);
    true
}

pub(crate) fn apply_literal_defaults_from(
    ctx: &mut InferCtx<'_>,
    relax_level: u8,
    start: usize,
) -> bool {
    let mut progress = false;

    // Compute the set of literal TyVars that are "blocked" at this relax
    // level. Level 2 removes all blocking; levels 0–1 block arg-position
    // literals whose receiver is still unresolved; level 0 additionally
    // blocks InterpolationLink accumulators.
    let blocked: crate::collections::HashSet<TyVar> = {
        let mut set = crate::collections::HashSet::default();

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
    for idx in start..ctx.types.len() {
        let tv = TyVar(idx as u32);
        let resolved = ctx.resolve(tv);
        if resolved != tv && (start == 0 || resolved.0 as usize >= start) {
            continue;
        }
        if blocked.contains(&resolved) {
            continue;
        }

        let literal = match &ctx.types[resolved.0 as usize] {
            TySlot::Unresolved { literal: Some(lit) } => *lit,
            _ => continue,
        };

        if default_literal_var(ctx, resolved, literal) {
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
