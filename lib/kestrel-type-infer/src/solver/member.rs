//! Member lookup: `Member`, `.Case` (`Implicit`, `ImplicitPat`), tuple
//! projection, interpolation links.

use super::*;

/// Resolve a string interpolation's accumulator type from the result type.
/// If the accumulator is already concrete (from literal defaults), this is a
/// no-op. If the result type is String (the default), also a no-op — literal
/// defaults handle it. Otherwise, delegates to `solve_associated` to resolve
/// `ResultType.Interpolation` and unify with the accumulator type variable.
pub(super) fn solve_interpolation_link(
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
    if let TySlot::Resolved(TyKind::Struct { entity, .. }) = ctx.slot(result_resolved)
        && ctx.resolver.builtin(Builtin::DefaultStringLiteralType) == Some(*entity)
    {
        return SolveResult::Solved;
    }

    // Delegate: resolve ResultType.Interpolation → accumulator type.
    solve_associated(
        ctx,
        result_tv,
        crate::constraint::AssocKey::Name("Interpolation".to_string()),
        acc_tv,
        span,
    )
}

/// Decide a `for` loop's iterator source (`Constraint::ForLoopIterator`).
/// Once the iterable resolves: an `Iterator` is consumed as the loop's
/// iterator (`result = iterable`, recorded in `direct_iterators`); any other
/// type takes the `Iterable.iter()` path — the poisoning conformance plus the
/// protocol member call the `ProtocolCall` arm would have emitted.
pub(super) fn solve_for_loop_iterator(
    ctx: &mut InferCtx<'_>,
    iterable: TyVar,
    result: TyVar,
    protocol: kestrel_hecs::Entity,
    expr: kestrel_hir::body::HirExprId,
    span: Span,
) -> SolveResult {
    let resolved = ctx.resolve(iterable);
    let is_iterator = match ctx.slot(resolved) {
        // A still-open literal (`for x in [1, 2]`, `for i in 0..<n`'s bound)
        // defaults to an Iterable collection, never an iterator: take the
        // `iter()` path now so the element type flows into the body.
        TySlot::Unresolved { literal: Some(_) } => false,
        TySlot::Unresolved { .. } | TySlot::Redirect(_) => {
            return SolveResult::Deferred(Constraint::ForLoopIterator {
                iterable,
                result,
                protocol,
                expr,
                span,
            });
        },
        // A reference iterates its pointee through `Iterable` (the Expr-origin
        // conformance rule judges a `&T` as its pointee); an iterator can't
        // be advanced through a shared borrow anyway.
        TySlot::Resolved(TyKind::Error | TyKind::Never | TyKind::Ref { .. }) => false,
        TySlot::Resolved(kind) => {
            let kind = kind.clone();
            // An `any Iterator` names the protocol it conforms to (`any P:
            // P`); the kernel below decides whether that holds.
            match ctx.resolver.builtin(Builtin::IteratorProtocol) {
                Some(iterator)
                    if ctx.resolver.conforms_to(&kind, iterator)
                        || crate::existential::names_protocol(ctx, resolved, iterator) =>
                {
                    match crate::solver_entail::conformance(ctx, resolved, iterator) {
                        kestrel_entail::Tri::Holds => true,
                        kestrel_entail::Tri::Fails => false,
                        kestrel_entail::Tri::Unknown => {
                            return SolveResult::Deferred(Constraint::ForLoopIterator {
                                iterable,
                                result,
                                protocol,
                                expr,
                                span,
                            });
                        },
                    }
                },
                _ => false,
            }
        },
    };
    kestrel_debug::ktrace!(
        "infer",
        "for-loop iterable {expr:?}: direct iterator = {is_iterator}"
    );
    if is_iterator {
        ctx.direct_iterators.insert(expr);
        ctx.equal(result, iterable, span);
        return SolveResult::Solved;
    }
    ctx.conforms_poisoning(iterable, protocol, span.clone());
    ctx.member(iterable, "iter", Vec::new(), result, expr, true, span);
    SolveResult::Solved
}

/// Resolve a tuple index access (`t.N`). Defers until `tuple` is concrete,
/// then either extracts the N-th element type or emits a specific error.
pub(super) fn solve_tuple_index(
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
    // Index through a reference like field access does: `r.0` on
    // `r: &(A, B)` reads the pointee's element (the `?.` binder is a ref).
    if let TyKind::Ref { pointee, .. } = kind {
        let pointee = *pointee;
        return solve_tuple_index(ctx, pointee, index, result, span);
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
pub(super) fn primitive_family(ctx: &InferCtx<'_>, kind: &TyKind) -> Option<PrimitiveFamily> {
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
pub(super) enum PrimitiveFamily {
    Int,
    Float,
    Bool,
    String,
}

/// Known primitive method names (mirrors lib1's `PrimitiveMethod` enum).
/// Used only to distinguish "user referenced a primitive method without
/// calling it" from "user accessed a nonexistent member on a primitive".
pub(super) fn is_known_primitive_method(family: PrimitiveFamily, name: &str) -> bool {
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
pub(super) fn member_not_found_error(
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
    if !is_call && let TyKind::Struct { entity, .. } | TyKind::Enum { entity, .. } = recv_kind {
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
    InferError::NoMember {
        receiver,
        name: name.to_string(),
        is_call,
        span,
    }
}

/// Lazy `Indirection` member peel. When member `name` was not found on the
/// concrete nominal `recv_kind`, but its entity conforms to the `Indirection`
/// protocol, this:
///   1. resolves the concrete `pointeeRef`/`pointeeMutRef` accessors on the
///      wrapper and records an `IndirectionPeel` for MIR (appending → the
///      outer→inner chain for nested wrappers), and
///   2. emits an `Associated` constraint binding a fresh TyVar to the
///      wrapper's `Indirection.Target` (reusing `solve_associated`),
///
/// returning that pointee TyVar for the caller to requeue the member on.
/// Returns `None` (no peel) when the receiver isn't a nominal or doesn't
/// conform to `Indirection`.
pub(crate) fn try_indirection_peel(
    ctx: &mut InferCtx<'_>,
    recv_kind: &TyKind,
    receiver: TyVar,
    expr: kestrel_hir::body::HirExprId,
    span: &Span,
) -> Option<TyVar> {
    // Only nominal wrappers peel.
    let _w_entity = match recv_kind {
        TyKind::Struct { entity, .. }
        | TyKind::Enum { entity, .. }
        | TyKind::Protocol { entity, .. } => *entity,
        _ => return None,
    };

    // Does the wrapper conform to `Indirection`? (bound-aware — a conditional
    // `extend CowBox[T]: MutableIndirection where T: Cloneable` is evaluated
    // here via `type_satisfies`.)
    let indirection = ctx.query_ctx.query(ResolveBuiltin {
        builtin: Builtin::Indirection,
        root: ctx.root,
    })?;
    let w_hir = reify_tv(ctx, receiver);
    if !crate::conformance::type_satisfies(ctx.query_ctx, &w_hir, indirection, ctx.root) {
        return None;
    }

    // Resolve the concrete pointee accessors on the wrapper (for MIR replay).
    // `pointeeRef` is required by `Indirection`; `pointeeMutRef` only exists on
    // `MutableIndirection` conformers (None ⇒ read-only ⇒ writes are D2).
    let read_method = ctx
        .resolver
        .resolve_member(recv_kind, "pointeeRef", &[])
        .ok()?
        .entity;
    let mut_method = ctx
        .resolver
        .resolve_member(recv_kind, "pointeeMutRef", &[])
        .ok()
        .map(|r| r.entity);

    // Pointee type = wrapper's `Indirection.Target`. Reuse the assoc-type
    // machinery via an `Associated` constraint on a fresh TyVar.
    let pointee_tv = ctx.fresh();
    ctx.associated(receiver, "Target", pointee_tv, span.clone());

    // Record the peel for MIR — append so nested wrappers build a chain.
    // `target_tv` is resolved in `build_result`.
    ctx.indirection_peels
        .entry(expr)
        .or_default()
        .push(crate::ctx::PendingPeel {
            read_method,
            mut_method,
            target_tv: pointee_tv,
        });

    Some(pointee_tv)
}

pub(super) fn solve_member(
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

    // A `Never` receiver is unreachable code: it conforms to every protocol,
    // so any member exists. Its result is never produced either, so it
    // coerces to whatever the context wants (`(return 1) + 2`).
    if matches!(recv_kind, TyKind::Never) {
        ctx.coerce(resolved, result, expr, span);
        return SolveResult::Solved;
    }

    // THE transparent-place peel: a ref-typed receiver requeues with the
    // POINTEE as the receiver, so nominal lookup, Self-substitution,
    // conformance and witness constraints all see `T`. One arm covers every
    // member-shaped form — field/method/paren-subscript/operators/for-in/
    // compound-assign/interpolation funnel through solve_member. The
    // receiver EXPRESSION's var is never rebound, so its recorded type
    // stays `&T`; mutability is policed in analyze (E207), like E203-E206.
    // Solved on the pointee right away: a re-queue is not fixpoint progress,
    // so it would wait for some unrelated constraint to move first.
    if let TyKind::Ref { pointee, .. } = &recv_kind {
        return solve_member(
            ctx,
            *pointee,
            name,
            args,
            result,
            expr,
            is_call,
            is_static_context,
            explicit_type_args,
            span,
        );
    }

    // Existential receiver (`B[rawany P]`): protocol members first, then
    // the box's own (existentials design §7). Operators and for-in go
    // through self-conformance instead, so they skip the peel.
    if !ctx.protocol_dispatch_members.contains(&expr)
        && let Some(peeled) = crate::existential::peel_for_member(
            ctx, receiver, &recv_kind, name, &args, is_call, expr, &span,
        )
    {
        return match peeled {
            Ok(payload) => solve_member(
                ctx,
                payload,
                name,
                args,
                result,
                expr,
                is_call,
                is_static_context,
                explicit_type_args,
                span,
            ),
            Err(err) => {
                ctx.poison(result);
                SolveResult::Error(err)
            },
        };
    }

    // If the receiver is an AssocProjection or a TypeAlias whose entity is
    // bound via a where-clause equality AND that bound resolves to a concrete
    // type, substitute the bound TyVar before member dispatch. If the bound is
    // itself unresolved (e.g. it's another abstract TypeAlias for the same
    // thing), fall through so resolve_member can do a protocol-bound search.
    // R5 — a projection receiver carries its base; a bare `TypeAlias` receiver
    // has already lost one, so the query is baseless there (G17 C1).
    let bound_entity = match &recv_kind {
        TyKind::AssocProjection { base, assoc } => Some((Some(*base), *assoc)),
        TyKind::TypeAlias { entity, args } if args.is_empty() => Some((None, *entity)),
        _ => None,
    };
    if let Some((base, entity)) = bound_entity
        && let Some(bound_tv) = ctx.assoc_sub("solver:solve_member", base, entity)
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

    // `escaping` closures are Cloneable (docs/design/closures.md §"escaping"),
    // and `clone()` is the environment's SHARE operation — a retain, not an
    // independent duplicate. A function type has no nominal entity to resolve
    // a protocol requirement against, so answer the requirement's shape
    // (`clone() -> Self`) directly here; MIR lowers it to the same `CopyValue`
    // an implicit copy emits (`body/call/mod.rs`).
    if let TyKind::Function { kind, .. } = &recv_kind
        && *kind == kestrel_ast::FnTypeKind::Escaping
        && name == "clone"
        && is_call
        && args.is_empty()
    {
        ctx.equal(result, receiver, span);
        return SolveResult::Solved;
    }

    // Resolve the member via the type resolver.
    // Try instance members first, fall back to static members for type-level calls
    // (e.g., Box.wrap() where wrap is a static method in an extension).
    // A projection receiver carries its base spine so the bound search sees
    // only bounds that name *this* receiver (G17 S4).
    let spine = projection_base_spine(ctx, &recv_kind);
    // On an existential, an extension method of its protocol (and anything
    // operators / for-in reach) resolves with the box itself as `Self`,
    // through `any P: P` (existentials design §6).
    let lookup = match crate::existential::self_conformance_member(
        ctx,
        receiver,
        &recv_kind,
        name,
        &args,
        ctx.protocol_dispatch_members.contains(&expr),
    ) {
        // The box stands in for `Self`, so it must self-conform to the
        // member's protocol — or the extension body could reach a static or
        // init requirement no payload-less box can answer. The obligation
        // goes to the solver, which reports E146 if `any P: P` fails.
        Some(res) => {
            if let Some(protocol) = ctx.declaring_protocol(res.entity) {
                ctx.conforms(receiver, protocol, span.clone());
            }
            Ok(res)
        },
        None => ctx
            .resolver
            .resolve_member_with_base(&recv_kind, spine.as_deref(), name, &args),
    };
    let resolution = match lookup {
        Ok(res) => res,
        Err(crate::resolve::MemberError::NotFound) => {
            // Fall back to static member search
            match ctx
                .resolver
                .resolve_static_member(&recv_kind, name, &args, is_call)
            {
                Ok(res) => res,
                Err(_) => {
                    // Lazy `Indirection` peel: the member is on neither the
                    // wrapper's instance nor static members, but if the wrapper
                    // conforms to `Indirection`, requeue the member on its
                    // pointee (`Target`). Skipped for protocol-dispatched
                    // members (operators/for-in) — those forward via `extend`,
                    // never the peel (R7). This is the LAZY twin of the eager
                    // `TyKind::Ref` arm above: same shape, fires only after the
                    // wrapper's own lookup misses (so the wrapper wins clashes).
                    if !ctx.protocol_dispatch_members.contains(&expr)
                        && let Some(pointee_tv) =
                            try_indirection_peel(ctx, &recv_kind, receiver, expr, &span)
                    {
                        return SolveResult::Deferred(Constraint::Member {
                            receiver: pointee_tv,
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
                    Some(ext) => !applicability(
                        ctx,
                        crate::constraint::ApplicableSource::Extension(*ext),
                        &recv_kind,
                    )
                    .fails(),
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

            // A pick that rests on an Unknown applicability (an open var in
            // the receiver) waits: once the var resolves, a more specific
            // candidate may turn out not to apply (bidi §5.1 step 6).
            let winner_unknown = top.iter().any(|(_, ext)| {
                ext.is_some_and(|e| {
                    applicability(
                        ctx,
                        crate::constraint::ApplicableSource::Extension(e),
                        &recv_kind,
                    ) == kestrel_entail::Tri::Unknown
                })
            });
            if winner_unknown && crate::solver_entail::has_open_var(ctx, receiver) {
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
            }

            if let [(winner, ext)] = top.as_slice() {
                match ctx.resolver.resolve_single_member(&recv_kind, *winner) {
                    Ok(mut res) => {
                        res.from_extension = *ext;
                        res
                    },
                    Err(_) => {
                        return SolveResult::Error(InferError::AmbiguousMember {
                            receiver: Some(receiver),
                            name: name.to_string(),
                            span,
                        });
                    },
                }
            } else {
                return SolveResult::Error(InferError::AmbiguousMember {
                    receiver: Some(receiver),
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
            match ctx
                .resolver
                .resolve_static_member(&recv_kind, name, &args, is_call)
            {
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

    // An enum case names a value of the enum TYPE (`E.b`); reaching one
    // through a value (`E.a.b`, `x.b`) is not member access (it used to
    // type-check as the case's constructor result).
    if ctx.query_ctx.get::<NodeKind>(resolution.entity) == Some(&NodeKind::EnumCase) {
        ctx.poison(result);
        return SolveResult::Error(InferError::MemberIsStatic {
            receiver,
            name: name.to_string(),
            span,
        });
    }

    // Applicability (bidi §5.1 steps 4 and 6): the declaring extension's
    // target pattern and where clauses — equality clauses included (N2) —
    // must hold for this receiver. A member resolved THROUGH a protocol (the
    // multi-match collapse in `try_resolve_through_protocol`) carries no
    // single `from_extension`; the receiver must then genuinely conform to
    // that protocol for THIS instantiation (`Box(0).show()` over
    // `extend Box[lang.i64]: Show` + `extend Box[lang.i32]: Show`).
    // `Fails` is "no member". `Unknown` (an open var in the receiver)
    // commits the member *tagged*: an `Applicable` obligation re-checks it
    // once the var resolves, and is never permitted on the guess (N1).
    let sources = resolution
        .from_extension
        .map(crate::constraint::ApplicableSource::Extension)
        .into_iter()
        .chain(
            resolution
                .via_protocol
                .map(crate::constraint::ApplicableSource::Protocol),
        );
    for source in sources {
        match applicability(ctx, source, &recv_kind) {
            kestrel_entail::Tri::Holds => {},
            kestrel_entail::Tri::Fails if !crate::solver_entail::has_open_var(ctx, receiver) => {
                return SolveResult::Error(InferError::NoMember {
                    receiver,
                    name: name.to_string(),
                    is_call,
                    span,
                });
            },
            kestrel_entail::Tri::Fails | kestrel_entail::Tri::Unknown => {
                ctx.constraints.push(Constraint::Applicable {
                    receiver,
                    source,
                    name: name.to_string(),
                    is_call,
                    span: span.clone(),
                })
            },
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
        // Zero-arg call syntax on a field is only valid for a function-valued
        // field (e.g. `separator: () -> Item`). A method call on a name that
        // resolves to a non-callable field — `slice.len()` where `len: Int64` —
        // is really a missing method. Don't forward into solve_call, which would
        // misreport "no matching subscript on type '<field type>'" against the
        // field's type instead of "no member 'len' on type '<receiver>'".
        if is_call && args.is_empty() {
            let resolved = ctx.resolve(field_tv);
            if ctx.is_concrete(resolved)
                && !matches!(
                    ctx.slot(resolved),
                    TySlot::Resolved(TyKind::Function { .. })
                )
            {
                return SolveResult::Error(InferError::NoMember {
                    receiver,
                    name: name.to_string(),
                    is_call,
                    span,
                });
            }
        }
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
            } else if recv_entity
                .map(|r| receiver_conformance_count(ctx, r, self_entity) >= 2)
                .unwrap_or(false)
            {
                // The receiver conforms to this protocol multiple times with
                // different args (`S: Producer[Int64]` + `S: Producer[Int32]`).
                // Pinning the proto param to the first conformance mis-dispatches
                // every call; leave it a fresh var so the call's expected return
                // type (unified at `ctx.equal(result, ret_tv)`) — or an
                // argument's type — selects the instantiation. The resolved type
                // is recorded via `record_type_args` for witness dispatch.
                let tv = ctx.fresh();
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
    let mut recorded_type_args = proto_type_args_tvs.clone();
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
            // `T: not P` emits nothing here: negative bounds gate member
            // applicability (the kernel) and copy semantics, not body solving.
            crate::resolve::WhereClause::NotBound { .. } => {},
            crate::resolve::WhereClause::Bound {
                subject,
                protocol,
                protocol_type_args,
            } => {
                let bound_tv = match subject.as_param() {
                    // Bare `T: P`. This loop runs on every member resolution
                    // (~12k times compiling a 50-line program), so the common
                    // shape keeps its allocation-free two-stage lookup: the
                    // method's own type params first, then the receiver's.
                    Some(param) => resolution
                        .type_params
                        .iter()
                        .position(|&p| p == param)
                        .map(|idx| fresh_params[idx])
                        .or_else(|| subs.iter().find(|(e, _)| *e == param).map(|&(_, tv)| tv)),
                    // G17 C11: `T.Assoc: P` — the member/method call-site
                    // obligation, previously skipped outright. `SubjectRoot`
                    // models one substitution, not two, so flatten them: `find`
                    // is first-wins, which *is* the two-stage semantics above.
                    // Built only for a projection (or `Self`) subject, so the
                    // hot path above never allocates.
                    //
                    // `Reduce`, not `Opaque`: the base is a concrete call
                    // type-arg here, so at `h.render(StrSrc(…))` the obligation
                    // is really about `String`. An unreduced `StrSrc.Item` would
                    // be judged by `solve_conforms` as an opaque projection,
                    // which permits. `lower_subject` still returns `None` when
                    // the root maps to nothing — the conservative permit for an
                    // abstract position.
                    None => {
                        let merged: Vec<(Entity, TyVar)> = resolution
                            .type_params
                            .iter()
                            .copied()
                            .zip(fresh_params.iter().copied())
                            .chain(subs.iter().copied())
                            .collect();
                        crate::generate::lower_subject(
                            ctx,
                            subject,
                            SubjectRoot::Subs(&merged),
                            crate::generate::ProjectionPolicy::Reduce(&span),
                        )
                    },
                };
                if let Some(tv) = bound_tv {
                    let arg_tvs: Vec<TyVar> = protocol_type_args
                        .iter()
                        .map(|hir_ty| lower_hir_ty_sub(ctx, hir_ty, self_entity, receiver, &subs))
                        .collect();
                    ctx.bound_typearg(tv, *protocol, arg_tvs, span.clone());
                }
            },
            crate::resolve::WhereClause::Equality { subject, rhs } => {
                match subject.equality_lhs() {
                    Some(EqualityLhs::Assoc { root, assoc }) => {
                        let param = &root;
                        let param_tv = if let Some(idx) =
                            resolution.type_params.iter().position(|&p| p == *param)
                        {
                            Some(fresh_params[idx])
                        } else {
                            subs.iter().find(|(e, _)| e == param).map(|&(_, tv)| tv)
                        };
                        if let Some(tv) = param_tv {
                            let assoc_result = ctx.fresh();
                            ctx.associated_entity(tv, assoc, assoc_result, span.clone());
                            let rhs_tv = lower_hir_ty_sub(ctx, rhs, self_entity, receiver, &subs);
                            ctx.equal(assoc_result, rhs_tv, span.clone());
                        }
                    },
                    Some(EqualityLhs::Direct(param)) => {
                        let param = &param;
                        let rhs_tv = lower_hir_ty_sub(ctx, rhs, self_entity, receiver, &subs);
                        if let Some(idx) = resolution.type_params.iter().position(|&p| p == *param)
                        {
                            // Method's own type param — redirect directly
                            ctx.types[fresh_params[idx].0 as usize] =
                                crate::ty::TySlot::Redirect(rhs_tv);
                        } else if let Some(&(_, tv)) = subs.iter().find(|(e, _)| e == param) {
                            // Struct/extension type param — equate with RHS
                            ctx.equal(tv, rhs_tv, span.clone());
                        } else if ctx.query_ctx.get::<kestrel_ast_builder::NodeKind>(*param)
                            == Some(&kestrel_ast_builder::NodeKind::TypeAlias)
                        {
                            // Associated type (e.g. `Item` in `where Item = (A, B)`) —
                            // resolve on receiver and equate with RHS
                            let assoc_tv = ctx.fresh();
                            ctx.associated_entity(receiver, *param, assoc_tv, span.clone());
                            ctx.equal(assoc_tv, rhs_tv, span.clone());
                        }
                    },
                    None => {},
                }
            },
        }
    }

    // For protocol methods, Self in param/return types needs substitution
    // with the actual receiver type. `self_entity` is bound above the
    // where-clause loop so RHS lowering can see it too.

    // When resolved through a protocol conformance, emit a Conforms constraint
    // to verify the receiver conforms to this protocol with the inferred type args.
    // The protocol's args ride along: `Int8(from: x)` resolves through
    // `Convertible[From]`, and `Int8: Convertible` alone holds for ANY
    // declared `From` — the obligation is `Int8: Convertible[typeof x]`.
    if let Some(protocol) = resolution.via_protocol {
        ctx.conforms_with_args(receiver, protocol, proto_type_args_tvs, span.clone());
    }

    // Bind arguments to parameters in declaration order, allowing defaulted
    // params to be skipped anywhere. Single source of truth: `arg_binding`.
    let bind_params: Vec<BindParam> = resolution
        .param_types
        .iter()
        .map(|p| BindParam::new(p.label.as_deref(), p.has_default))
        .collect();
    let arg_labels: Vec<Option<&str>> = args.iter().map(|a| a.label.as_deref()).collect();
    // An UNAPPLIED static method (`let f = Box[Int64].make`) binds nothing:
    // it becomes a function value below.
    let is_unapplied_static = !is_call
        && resolution.kind == crate::resolve::MemberKind::Method
        && ctx
            .query_ctx
            .has::<kestrel_ast_builder::Static>(resolution.entity);
    let bound = if is_unapplied_static {
        Ok(Vec::new())
    } else {
        bind_arguments(&bind_params, &arg_labels)
    };
    let plan = match bound {
        Ok(plan) => plan,
        Err(BindError::LabelMismatch {
            arg_index,
            expected,
            ..
        }) => {
            return SolveResult::Error(InferError::LabelMismatch {
                expected,
                got: args[arg_index].label.clone(),
                span,
            });
        },
        Err(BindError::MissingArgument { .. } | BindError::TooManyArguments { .. }) => {
            let required_count = bind_params.iter().filter(|p| !p.has_default).count();
            return SolveResult::Error(InferError::ArgCountMismatch {
                expected: required_count,
                got: args.len(),
                span,
            });
        },
    };

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

    // Equate each explicitly-provided argument's type with the type of the
    // parameter it bound to (per `plan`). Skipped params take their defaults,
    // which are checked at their definition site.
    for (param_idx, binding) in plan.iter().enumerate() {
        if let Binding::Arg(ai) = binding {
            let param_info = &resolution.param_types[param_idx];
            let param_tv = lower_hir_ty_sub(ctx, &param_info.ty, self_entity, receiver, &subs);
            ctx.coerce(args[*ai].ty, param_tv, args[*ai].value, span.clone());
        }
    }
    let param_hir_tys: Vec<Option<kestrel_hir::HirTy>> =
        resolution.param_types.iter().map(|p| Some(p.ty.clone())).collect();
    for (param_ty, default_ty) in
        super::call::skipped_default_types(ctx, resolution.entity, &plan, &param_hir_tys)
    {
        let param_tv = lower_hir_ty_sub(ctx, &param_ty, self_entity, receiver, &subs);
        let default_tv = lower_hir_ty_sub(ctx, &default_ty, self_entity, receiver, &subs);
        ctx.equal(param_tv, default_tv, span.clone());
    }

    // An UNAPPLIED static method (`let f = T.zero`) is a function value:
    // its type is `(params) -> ret`, not its call result. `Box.stat` reaches
    // the same type through name resolution; a static requirement named
    // through a type parameter or projection (`T.zero`, `B.Item.zero`)
    // arrives here as a field access (G26).
    if is_unapplied_static {
        let params: Vec<TyVar> = resolution
            .param_types
            .iter()
            .map(|p| lower_hir_ty_sub(ctx, &p.ty, self_entity, receiver, &subs))
            .collect();
        let conventions = vec![kestrel_ast::ParamConvention::Consuming; params.len()];
        let ret = lower_opaque_aware(
            ctx,
            &resolution.return_type,
            resolution.entity,
            self_entity,
            receiver,
            &subs,
        );
        let fn_tv = ctx.function_conv(kestrel_ast::FnTypeKind::Normal, params, conventions, ret);
        ctx.equal(result, fn_tv, span);
        return SolveResult::Solved;
    }

    // Equate result with return type (ref-aware: scrutinee decay / pinned
    // result — see bind_call_result)
    let ret_tv = lower_opaque_aware(
        ctx,
        &resolution.return_type,
        resolution.entity,
        self_entity,
        receiver,
        &subs,
    );

    // Stage 1.5 read-provider typing: when a subscript/computed property
    // declares a `ref { … }` accessor, READS of the member produce `&T` —
    // wrap the declared type so all stage-1 decay machinery (binding/
    // scrutinee/assign-target/arm decay, E492 validation, E497) applies
    // unchanged. The wrap is read-only: RMW receivers are never typed
    // `&mutating T` — analyze and mir-lower route writes and RMW by ENTITY
    // (the PlaceAccessors query), not by this expression type.
    let ret_tv = if matches!(
        resolution.kind,
        crate::resolve::MemberKind::Subscript | crate::resolve::MemberKind::ComputedProperty { .. }
    ) && ctx
        .query_ctx
        .query(kestrel_hir_lower::PlaceAccessors {
            entity: resolution.entity,
        })
        .is_some_and(|info| info.ref_accessor.is_some())
    {
        ctx.ref_ty(ret_tv, false)
    } else {
        ret_tv
    };
    bind_call_result(ctx, result, ret_tv, expr, span.clone());

    SolveResult::Solved
}

/// Whether `kind` is an enum with a case named `name`.
fn has_enum_case(ctx: &InferCtx<'_>, kind: &TyKind, name: &str) -> bool {
    let Some(entity) = kind.entity() else {
        return false;
    };
    ctx.query_ctx.children_of(entity).iter().any(|&child| {
        ctx.query_ctx.get::<NodeKind>(child) == Some(&NodeKind::EnumCase)
            && ctx
                .query_ctx
                .get::<Name>(child)
                .is_some_and(|n| n.0 == name)
    })
}

pub(super) fn solve_implicit(
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
            // `Implicit` does not record whether `()` was written; with args
            // it is a call, without them a read (only a var/func name clash
            // can tell the two apart).
            let is_call = !args.is_empty();
            match ctx
                .resolver
                .resolve_static_member(&kind, name, &args, is_call)
            {
                Ok(res) => res,
                // The case exists but its labels/arity don't fit the args
                // (`Shape.Rectangle(width: 5.0)`): an overload mismatch, not
                // a missing member.
                Err(_) if has_enum_case(ctx, &kind, name) => {
                    return SolveResult::Error(InferError::NoMatchingOverload {
                        name: name.to_string(),
                        span,
                    });
                },
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

    // `.name` has no receiver, so it can only name a static member, an enum
    // case or an initializer. An instance method/field has nothing to bind
    // `self` to (it used to be called with no `self` argument).
    let is_instance_member = matches!(
        ctx.query_ctx.get::<NodeKind>(resolution.entity),
        Some(NodeKind::Function | NodeKind::Field)
    ) && ctx
        .query_ctx
        .get::<kestrel_ast_builder::Static>(resolution.entity)
        .is_none();
    if is_instance_member {
        return SolveResult::Error(InferError::InstanceMethodAsStatic {
            name: name.to_string(),
            span,
        });
    }

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
pub(super) fn solve_implicit_pat(
    ctx: &mut InferCtx<'_>,
    scrutinee: TyVar,
    name: &str,
    arg_tys: Vec<TyVar>,
    labels: Vec<Option<String>>,
    span: Span,
) -> SolveResult {
    let resolved = ctx.resolve(scrutinee);
    if !ctx.is_concrete(resolved) {
        return SolveResult::Deferred(Constraint::ImplicitPat {
            scrutinee,
            name: name.to_string(),
            arg_tys,
            labels,
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

    crate::generate::check_variant_pattern_labels(ctx, case_entity, &labels, &span);

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
pub(super) fn solve_tuple_rest_pat(
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
            reason: Reason::Unspecified,
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

/// Recover the protocol entity from an extension's first positive
/// conformance. `extend Slot: Indexable[T]` returns Indexable's entity.
/// Used by `solve_associated` to look up the witness's protocol args
/// (cached by `(container, protocol)` key) when projecting through a
/// binding declared inside an extension block.
pub(super) fn find_extension_conformance_protocol(
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
pub(super) fn resolve_proto_param_via_conformance(
    ctx: &mut InferCtx<'_>,
    recv_entity: Option<kestrel_hecs::Entity>,
    protocol: kestrel_hecs::Entity,
    proto_param_idx: usize,
    proto_param_count: usize,
    subs: &[(kestrel_hecs::Entity, TyVar)],
) -> Option<TyVar> {
    let recv = recv_entity?;
    // The first positive conformance to `protocol` wins.
    let (source, ast_ty) = positive_conformances_to(ctx, recv, protocol)
        .into_iter()
        .next()?;
    let type_args = kestrel_name_res::extract_ast_type_args(&ast_ty);

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
    let hir_ty = kestrel_hir_lower::lower_ast_type(ctx.query_ctx, source, ctx.root, arg_ast);
    let subs_vec = subs.to_vec();
    Some(lower_hir_ty_sub(ctx, &hir_ty, None, TyVar(0), &subs_vec))
}

/// Every positive conformance of `recv` (its own declaration, then its
/// extensions) to `protocol`, as `(declaring entity, written conformance type)`.
/// The one walk behind the multi-conformance questions below.
pub(super) fn positive_conformances_to(
    ctx: &InferCtx<'_>,
    recv: kestrel_hecs::Entity,
    protocol: kestrel_hecs::Entity,
) -> Vec<(kestrel_hecs::Entity, kestrel_ast_builder::AstType)> {
    use kestrel_ast_builder::{ConformanceItem, Conformances};
    if ctx.query_ctx.get::<NodeKind>(recv) != Some(&NodeKind::Struct)
        && ctx.query_ctx.get::<NodeKind>(recv) != Some(&NodeKind::Enum)
    {
        return Vec::new();
    }
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
    let mut out = Vec::new();
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
            if let kestrel_name_res::TypeResolution::Found(resolved) =
                ctx.query_ctx.query(kestrel_name_res::ResolveTypePath {
                    segments: seg_names,
                    context: source,
                    root: ctx.root,
                })
                && resolved == protocol
            {
                out.push((source, ast_ty.clone()));
            }
        }
    }
    out
}

/// Count `recv`'s positive conformances to `protocol` (direct + extensions).
///
/// Used to detect the ambiguous same-protocol multi-conformance case
/// (`S: Producer[Int64]` + `S: Producer[Int32]`): when ≥2, the protocol's type
/// param must be left to unification rather than pinned to the first
/// conformance by `resolve_proto_param_via_conformance`.
pub(super) fn receiver_conformance_count(
    ctx: &InferCtx<'_>,
    recv: kestrel_hecs::Entity,
    protocol: kestrel_hecs::Entity,
) -> usize {
    positive_conformances_to(ctx, recv, protocol).len()
}
