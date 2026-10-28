//! Associated-type projection (`Associated`) and type-alias reduction (`Reduce`).

use super::*;

/// Reduce a TypeAlias TyVar to its substituted definition.
///
/// Looks up the alias entity's TypeAnnotation, substitutes the user-provided
/// args for the alias's type params, and equates `result` with the substituted
/// TyVar. Also emits conformance obligations from the alias's TypeParam bounds
/// (e.g. `type Pair[T: Hashable] = (T, T)` requires `T: Hashable`).
pub(super) fn solve_reduce(
    ctx: &mut InferCtx<'_>,
    alias: TyVar,
    result: TyVar,
    span: Span,
) -> SolveResult {
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

    let arg_tvs: Vec<TyVar> = subs.iter().map(|&(_, tv)| tv).collect();
    emit_alias_bounds(ctx, entity, &arg_tvs, &span);

    ctx.equal(result, substituted_tv, span);
    SolveResult::Solved
}

/// The conformance obligations a use of type alias `alias` with `arg_tvs`
/// must meet: each parameter's inline bounds (`[T: Comparable]`) and the
/// alias's `where T: P` clauses, checked against the matching argument. The
/// single source for alias bounds (lowering emits them for `Alias`).
pub(crate) fn emit_alias_bounds(
    ctx: &mut InferCtx<'_>,
    alias: Entity,
    arg_tvs: &[TyVar],
    span: &Span,
) {
    let params: Vec<Entity> = ctx
        .query_ctx
        .get::<TypeParams>(alias)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();
    let arg_of = |p: Entity| {
        params
            .iter()
            .position(|&q| q == p)
            .and_then(|i| arg_tvs.get(i).copied())
    };
    for &param in &params {
        let Some(arg_tv) = arg_of(param) else {
            continue;
        };
        for protocol in direct_param_bound_protocols(ctx, param) {
            ctx.conforms_typearg(arg_tv, protocol, span.clone());
        }
    }
    for clause in ctx.query_ctx.query(crate::where_clauses::WhereClausesOf {
        entity: alias,
        root: ctx.root,
    }) {
        if let crate::resolve::WhereClause::Bound {
            subject: crate::resolve::WhereSubject::Param(param),
            protocol,
            ..
        } = clause
            && let Some(arg_tv) = arg_of(param)
        {
            ctx.conforms_typearg(arg_tv, protocol, span.clone());
        }
    }
}

pub(super) fn solve_associated(
    ctx: &mut InferCtx<'_>,
    container: TyVar,
    key: crate::constraint::AssocKey,
    result: TyVar,
    span: Span,
) -> SolveResult {
    let resolved = ctx.resolve(container);
    if !ctx.is_concrete(resolved) {
        return SolveResult::Deferred(Constraint::Associated {
            container,
            key,
            result,
            span,
        });
    }
    let name = &ctx.assoc_key_name(&key);

    if ctx.is_error(resolved) {
        ctx.poison(result);
        return SolveResult::Solved;
    }

    // Get the concrete TyKind, clone to avoid borrow issues
    let kind = match ctx.slot(resolved) {
        TySlot::Resolved(k) => k.clone(),
        _ => unreachable!(),
    };

    let found = match &key {
        crate::constraint::AssocKey::Entity(assoc) => ctx
            .resolver
            .resolve_associated_type_by_entity(&kind, *assoc, name),
        crate::constraint::AssocKey::Name(_) => ctx.resolver.resolve_associated_type(&kind, name),
        crate::constraint::AssocKey::Conformance {
            assoc,
            protocol_args,
        } => match conformance_binding(ctx, &kind, *assoc, protocol_args, name) {
            ConformancePick::Wait => {
                return SolveResult::Deferred(Constraint::Associated {
                    container,
                    key,
                    result,
                    span,
                });
            },
            ConformancePick::Found(res) => Some(res),
            ConformancePick::Unselected => ctx
                .resolver
                .resolve_associated_type_by_entity(&kind, *assoc, name),
        },
    };
    match found {
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
            // Abstract associated types come back as HirTy::AbstractAssoc; concrete
            // ones as Struct/Enum/Protocol/Tuple/etc. For the abstract case, consult
            // where_clause substitutions to find the TyVar already bound to this
            // associated type, otherwise fall through to lower_hir_ty_sub with
            // container_subs applied (handles `type Element = T` on Array[i32]).
            let assoc_tv = if let kestrel_hir::ty::HirTy::AbstractAssoc { entity, .. } =
                &assoc.resolved
            {
                {
                    // R3 — `container` is the receiver being projected off, and
                    // since C3 the memo only answers for that same receiver.
                    if let Some(tv) =
                        ctx.assoc_sub("solver:solve_associated", Some(container), *entity)
                    {
                        // Only an OPEN shared var is the no-op; one the
                        // where clause already bound (`I.Item = T` decided
                        // on the spot) is the answer itself.
                        if ctx.resolve(tv) == resolved_result
                            && matches!(ctx.slot(resolved_result), TySlot::Unresolved { .. })
                        {
                            // Self-referential: the where_clause_assoc_subs TyVar is the same
                            // as our result. Create an AssocProjection to preserve the base
                            // type for MIR lowering (needed for correct monomorphization
                            // when multiple type params conform to the same protocol).
                            ctx.assoc_projection(container, *entity)
                        } else {
                            tv
                        }
                    } else if let Some(&tv) = ctx.param_tyvars.get(entity) {
                        tv
                    } else if matches!(kind, TyKind::Param { .. } | TyKind::AssocProjection { .. })
                        || (is_self_rooted_assoc_alias(ctx, &kind)
                            && ctx
                                .assoc_sub("solver:solve_associated:self-rooted", None, *entity)
                                .is_none())
                    {
                        // Abstract container — a type param (`T.Item`) or an
                        // already-projected base (`T.TargetIterator.Item`) — with
                        // an abstract associated-type binding. `lower_hir_ty_sub`
                        // would collapse this to a baseless `TypeAlias(entity)`,
                        // dropping the base; that leak surfaces in MIR lowering as
                        // `AssocProjection { base: Self(protocol) }`, which
                        // monomorphization can't resolve (the protocol's Self is
                        // in no subst map → mangler panic). Preserve the base as a
                        // projection so mono resolves it via the witness for the
                        // concrete substituted base (e.g. `T.TargetIterator` →
                        // `Array[Int64].TargetIterator` → `ArraySliceIterator`).
                        //
                        // Intentionally excludes `SelfType` containers: projecting
                        // off `Self` yields a literal `Self.Item` that won't unify
                        // with the based form, so `Self.Item` collapses to the
                        // bare alias `TypeAlias(Item)`, which MIR's
                        // `lower_named_type` re-roots at its own protocol's
                        // `Self`. That collapse is exactly one level deep, so a
                        // projection OFF it (`Item.Sub` in `extend Outer`) must
                        // keep it as base — collapsing again would re-root `Sub`
                        // at `Inner`'s Self, which mono can't resolve (G26). Only
                        // when the baseless where-clause memo has no answer: that
                        // memo is the one thing the collapse below could still
                        // resolve it to (`Item.Output = Item` in `sum()`); with no
                        // answer the collapse ends in the bare alias, which is the
                        // base loss. See `is_self_rooted_assoc_alias`.
                        ctx.assoc_projection(container, *entity)
                    } else {
                        lower_hir_ty_sub(
                            ctx,
                            &assoc.resolved,
                            container_entity,
                            container,
                            &all_subs,
                        )
                    }
                }
            } else {
                lower_hir_ty_sub(ctx, &assoc.resolved, container_entity, container, &all_subs)
            };

            // Emit where clause constraints from the resolved TypeAlias entity.
            // E.g., `type Iter: Iterator where Iter.Item = Item` — when we resolve
            // `T.Iter`, emit constraints equating `T.Iter.Item` with `T.Item`.
            if let kestrel_hir::ty::HirTy::AbstractAssoc { entity, .. } = &assoc.resolved {
                emit_type_alias_where_clauses(ctx, *entity, assoc_tv, container, &span);
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

/// Is `kind` the bare alias a `Self.Item` projection collapses to — an
/// argument-less `TypeAlias` whose parent is a protocol? It stands for the
/// protocol's `Self.Item` (MIR's `lower_named_type` restores that base), so a
/// projection off it is off an abstract base and must keep it (G26).
pub(super) fn is_self_rooted_assoc_alias(ctx: &InferCtx<'_>, kind: &TyKind) -> bool {
    let TyKind::TypeAlias { entity, args } = kind else {
        return false;
    };
    args.is_empty()
        && ctx
            .query_ctx
            .parent_of(*entity)
            .is_some_and(|p| ctx.query_ctx.get::<NodeKind>(p) == Some(&NodeKind::Protocol))
}

/// What [`conformance_binding`] decided.
enum ConformancePick {
    /// The protocol args are still open; the pick waits for them.
    Wait,
    /// The binding declared by the one conformance the args select.
    Found(crate::resolve::AssociatedTypeResolution),
    /// Not a multi-conformance case (or no unique match): the plain
    /// by-entity lookup answers.
    Unselected,
}

/// `container.assoc` through the conformance `protocol_args` select, for a
/// container that conforms to `assoc`'s protocol more than once
/// (`extend Stamp: Subtractable[Meters] { type Output = Stamp }` +
/// `extend Stamp: Subtractable[Stamp] { type Output = Meters }`): each sibling
/// declares its own binding, and the by-name lookup would return the first.
/// Reads the binding from the selected conformance's own declaration — the
/// same rule the E632 checker uses (`find_associated_type_binding_entity`).
fn conformance_binding(
    ctx: &mut InferCtx<'_>,
    container: &TyKind,
    assoc: Entity,
    protocol_args: &[TyVar],
    name: &str,
) -> ConformancePick {
    let (Some(recv), Some(protocol)) = (container.entity(), ctx.query_ctx.parent_of(assoc)) else {
        return ConformancePick::Unselected;
    };
    let conformances = super::member::positive_conformances_to(ctx, recv, protocol);
    if conformances.len() < 2 {
        return ConformancePick::Unselected;
    }
    if protocol_args
        .iter()
        .any(|&tv| !ctx.is_concrete(ctx.resolve(tv)))
    {
        return ConformancePick::Wait;
    }

    let mut selected = conformances.into_iter().filter(|(source, ast_ty)| {
        let written = kestrel_name_res::extract_ast_type_args(ast_ty);
        written.len() == protocol_args.len()
            && written.iter().zip(protocol_args).all(|(arg, &tv)| {
                let hir = kestrel_hir_lower::lower_ast_type(ctx.query_ctx, *source, ctx.root, arg);
                written_arg_matches(ctx, &hir, tv)
            })
    });
    let (Some((source, _)), None) = (selected.next(), selected.next()) else {
        return ConformancePick::Unselected;
    };

    let binding = ctx
        .query_ctx
        .children_of(source)
        .iter()
        .copied()
        .find(|&child| {
            ctx.query_ctx.get::<NodeKind>(child) == Some(&NodeKind::TypeAlias)
                && ctx
                    .query_ctx
                    .get::<Name>(child)
                    .is_some_and(|n| n.0 == name)
        });
    let Some(resolved) = binding.and_then(|alias| {
        ctx.query_ctx.query(kestrel_hir_lower::LowerTypeAnnotation {
            entity: alias,
            root: ctx.root,
        })
    }) else {
        return ConformancePick::Unselected;
    };
    ConformancePick::Found(crate::resolve::AssociatedTypeResolution {
        resolved,
        source_extension: (source != recv).then_some(source),
    })
}

/// Whether a conformance's written protocol arg (`Meters` in
/// `Subtractable[Meters]`) is the solved `tv`. An extension's own type param
/// matches anything; an unresolved or unmodelled position does not match, so
/// the caller falls back rather than guessing.
fn written_arg_matches(ctx: &InferCtx<'_>, written: &kestrel_hir::ty::HirTy, tv: TyVar) -> bool {
    use kestrel_hir::ty::HirTy;
    let TySlot::Resolved(kind) = ctx.slot(ctx.resolve(tv)) else {
        return false;
    };
    match (written, kind) {
        (HirTy::Param(..), _) => true,
        (HirTy::Struct { entity, args, .. }, TyKind::Struct { entity: e, args: a })
        | (HirTy::Enum { entity, args, .. }, TyKind::Enum { entity: e, args: a }) => {
            entity == e
                && args.len() == a.len()
                && args
                    .iter()
                    .zip(a)
                    .all(|(w, &t)| written_arg_matches(ctx, w, t))
        },
        (HirTy::Tuple(ws, _), TyKind::Tuple(ts)) => {
            ws.len() == ts.len()
                && ws
                    .iter()
                    .zip(ts)
                    .all(|(w, &t)| written_arg_matches(ctx, w, t))
        },
        _ => false,
    }
}
