//! Lowering HIR types into solver type vars, and the where-clause /
//! well-formedness obligations that lowering emits.

use super::*;

/// Emit constraints from a TypeAlias entity's where clauses.
///
/// When resolving an associated type like `T.Iter` where `Iter` has its own
/// constraints (e.g., `type Iter: Iterator where Iter.Item = Item`), we need
/// to propagate those constraints. This connects `T.Iter.Item` to `T.Item`
/// through the where clause equality.
pub(super) fn emit_type_alias_where_clauses(
    ctx: &mut InferCtx<'_>,
    alias_entity: kestrel_hecs::Entity,
    alias_tv: TyVar,
    container: TyVar,
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
            // `T: not P` emits nothing here: negative bounds gate member
            // applicability (the kernel) and copy semantics, not body solving.
            crate::resolve::WhereClause::NotBound { .. } => {},
            crate::resolve::WhereClause::Bound {
                subject,
                protocol,
                protocol_type_args,
            } => {
                // `Assoc.Inner: P` projection bounds on a TypeAlias — not
                // emitted here. TODO(G17 stage 3a): inert feature, not a live
                // bug, but the skip must stay explicit.
                if !matches!(subject, crate::resolve::WhereSubject::Param(_)) {
                    continue;
                }
                // Emit conformance: e.g., `Iter: Iterator` → Conforms(alias_tv, Iterator)
                ctx.conforms_typearg(alias_tv, protocol, span.clone());
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
            // About this alias (`Iter.Item = Item`); the root is not re-read.
            crate::resolve::WhereClause::Equality { subject, rhs } => {
                let Some(EqualityLhs::Assoc { assoc, .. }) = subject.equality_lhs() else {
                    continue;
                };
                // Emit associated type equality: e.g., `Iter.Item = Item`
                // → Associated(alias_tv, Iterator.Item, fresh) + Equal(fresh, rhs_tv)
                let fresh = ctx.fresh();
                ctx.associated_entity(alias_tv, assoc, fresh, span.clone());
                // Lower rhs using where_clause_assoc_subs so that `Item` resolves
                // to the existing TyVar for T.Item. The RHS's bare `Item` is
                // `Self.Item` of the alias's protocol, and that `Self` is the
                // `container` the alias was projected off (G25 step 1).
                let self_sub: Vec<(kestrel_hecs::Entity, TyVar)> = ctx
                    .query_ctx
                    .parent_of(alias_entity)
                    .map(|protocol| (protocol, container))
                    .into_iter()
                    .collect();
                let rhs_tv = crate::generate::lower_hir_ty_with_subs(ctx, &rhs, &self_sub);
                ctx.equal(fresh, rhs_tv, span.clone());
                // Register `alias.assoc → rhs` in the memo, same shape as the
                // protocol-side emitter: base = the alias's own TyVar, stored raw.
                ctx.push_assoc_sub(Some(alias_tv), assoc, rhs_tv);
            },
        }
    }
}

/// Lower a callee's return type, intercepting HirTy::Opaque to create TyKind::Opaque
/// with the correct origin entity and origin_args from the call-site substitution.
/// Passes through `self_entity` and `recv_tv` so Self-type substitution works
/// correctly for protocol method return types (e.g. `-> Item?`).
pub(super) fn lower_opaque_aware(
    ctx: &mut InferCtx<'_>,
    hir_ty: &kestrel_hir::ty::HirTy,
    callee: kestrel_hecs::Entity,
    self_entity: Option<kestrel_hecs::Entity>,
    recv_tv: TyVar,
    subs: &[(kestrel_hecs::Entity, TyVar)],
) -> TyVar {
    if let kestrel_hir::ty::HirTy::Opaque {
        bounds,
        not_copyable,
        ..
    } = hir_ty
    {
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
            not_copyable: *not_copyable,
        });
        tv
    } else {
        lower_hir_ty_sub(ctx, hir_ty, self_entity, recv_tv, subs)
    }
}

/// Solver-side twin of `generate::emit_copyable_wellformedness`, scoped to
/// the STATIC bound (the ref-containment soundness line). Pushes a
/// TypeArg-origin Conforms for each formed arg whose param carries the
/// (implicit or explicit) Static bound. Copyable/Cloneable stay
/// generate-side only — widening them here would newly flag code the
/// move checker already governs (orthogonal, larger change).
pub(super) fn emit_static_wellformedness(
    ctx: &mut InferCtx<'_>,
    entity: kestrel_hecs::Entity,
    arg_tvs: &[TyVar],
    span: &Span,
) {
    if arg_tvs.is_empty() {
        return;
    }
    let Some(static_proto) = ctx.query_ctx.query(ResolveBuiltin {
        builtin: Builtin::Static,
        root: ctx.root,
    }) else {
        return;
    };
    let Some(params) = ctx
        .query_ctx
        .get::<kestrel_ast_builder::TypeParams>(entity)
        .map(|tp| tp.0.clone())
    else {
        return;
    };
    let where_clauses = ctx.query_ctx.query(crate::where_clauses::WhereClausesOf {
        entity,
        root: ctx.root,
    });
    for clause in where_clauses {
        let crate::resolve::WhereClause::Bound {
            subject, protocol, ..
        } = clause
        else {
            continue;
        };
        // TODO(G17 stage 3a): projection subjects are skipped here, as they
        // were when `ProjectionBound` was a separate variant.
        let Some(param) = subject.as_param() else {
            continue;
        };
        if protocol != static_proto {
            continue;
        }
        let Some(pos) = params.iter().position(|p| *p == param) else {
            continue;
        };
        if let Some(&tv) = arg_tvs.get(pos) {
            kestrel_debug::ktrace!("static-wf", "emit Static wf for {entity:?} arg {tv:?}");
            // The formed type's own span (the declared signature site) —
            // a synthetic span here renders as NOTHING and the build
            // fails silently.
            ctx.conforms_typearg(tv, static_proto, span.clone());
        }
    }
}

/// Convert HirTy to TyVar with substitutions.
/// - Self entity → receiver TyVar
/// - Type params in `subs` → their mapped TyVars (struct type params + method type params)
pub(super) fn lower_hir_ty_sub(
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
        HirTy::Struct {
            entity, args, span, ..
        }
        | HirTy::Enum {
            entity, args, span, ..
        }
        | HirTy::Protocol {
            entity, args, span, ..
        } => {
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
            // Solver-side formation wellformedness, STATIC bound only: a
            // member-result instantiation is a formation site too —
            // `collect() -> Array[Item]` at `Item = &Int64` must fail
            // Array's implicit `T: Static` exactly like the annotation
            // route (`let xs: Array[&Int64]`), or heap storage of refs
            // materializes out of inference (the line that never moves).
            // Abstract args stay conservative: the obligation defers until
            // the arg resolves, and Param/assoc kinds pass through the
            // usual entailment arms.
            emit_static_wellformedness(ctx, *entity, &arg_tvs, span);
            ctx.named(*entity, arg_tvs)
        },
        // An alias arrives expanded; a bounded one also checks its bounds.
        HirTy::Alias {
            alias,
            args,
            expanded,
            span,
        } => {
            let arg_tvs: Vec<TyVar> = args
                .iter()
                .map(|a| lower_hir_ty_sub(ctx, a, self_entity, recv_tv, subs))
                .collect();
            super::assoc::emit_alias_bounds(ctx, *alias, &arg_tvs, span);
            lower_hir_ty_sub(ctx, expanded, self_entity, recv_tv, subs)
        },
        HirTy::AbstractAssoc { entity, .. } => {
            // Associated-type use: check substitution maps first.
            {
                if let Some(&(_, tv)) = subs.iter().find(|(e, _)| e == entity) {
                    return tv;
                }
                // R6 — reads as baseless, so C3's unambiguity rule applies: one
                // candidate answers, two or more bail to the paths below. A
                // bare `Item` in a protocol member signature lowered for
                // receiver `B` arguably *means* `B.Item`, and `recv_tv` is
                // right here — but that reading (C2) never diverges from the
                // baseless one across the corpus, so it stays an audit probe.
                ctx.probe_assoc_sub(
                    "solver:lower_hir_ty_sub:AbstractAssoc(recv_tv-probe)",
                    Some(recv_tv),
                    *entity,
                );
                if let Some(tv) =
                    ctx.assoc_sub("solver:lower_hir_ty_sub:AbstractAssoc", None, *entity)
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
                if is_concrete_non_self {
                    let result = ctx.fresh();
                    ctx.associated_entity(
                        recv_tv,
                        *entity,
                        result,
                        kestrel_span::Span::synthetic(0),
                    );
                    return result;
                }
            }

            ctx.type_alias(*entity, Vec::new())
        },
        HirTy::AssocProjection { base, assoc, span } => {
            let base_tv = lower_hir_ty_sub(ctx, base, self_entity, recv_tv, subs);
            // R7 — the `leak5.ks` site. Since C3 `base_tv` is the key, so a
            // clause about `A.Item` no longer answers a projection off `B`.
            if let Some(tv) = ctx.assoc_sub(
                "solver:lower_hir_ty_sub:AssocProjection",
                Some(base_tv),
                *assoc,
            ) {
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
        HirTy::Function {
            kind,
            params,
            param_conventions,
            ret,
            ..
        } => {
            let param_tvs: Vec<TyVar> = params
                .iter()
                .map(|p| lower_hir_ty_sub(ctx, p, self_entity, recv_tv, subs))
                .collect();
            let ret_tv = lower_hir_ty_sub(ctx, ret, self_entity, recv_tv, subs);
            ctx.function_conv(*kind, param_tvs, param_conventions.clone(), ret_tv)
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
        // Stage 1: a ref survives HIR lowering in return position only (the
        // E481 carve-out) — methods returning `&T` land here via
        // MemberResolution.return_type.
        HirTy::Ref {
            inner, mutating, ..
        } => {
            let pointee = lower_hir_ty_sub(ctx, inner, self_entity, recv_tv, subs);
            ctx.ref_ty(pointee, *mutating)
        },
        HirTy::Error(_) => {
            let idx = ctx.types.len() as u32;
            ctx.types.push(TySlot::Resolved(TyKind::Error));
            TyVar(idx)
        },
    }
}
