//! Conformance and applicability obligations (`Conforms`, `Applicable`),
//! decided through the `kestrel-entail` kernel, plus the solver's copy and
//! staticness layers.

use super::*;

/// Re-evaluate a tagged member commitment (see `Constraint::Applicable`).
pub(super) fn solve_applicable(
    ctx: &mut InferCtx<'_>,
    receiver: TyVar,
    source: crate::constraint::ApplicableSource,
    name: String,
    is_call: bool,
    span: Span,
) -> SolveResult {
    let verdict = match ctx.slot(ctx.resolve(receiver)) {
        TySlot::Resolved(kind) => applicability(ctx, source, &kind.clone()),
        _ => kestrel_entail::Tri::Unknown,
    };
    match verdict {
        kestrel_entail::Tri::Holds => SolveResult::Solved,
        // A failure decided by a still-open literal waits for the literal to
        // settle, so the diagnostic names `Box[String]`, not a literal.
        kestrel_entail::Tri::Fails if !crate::solver_entail::has_open_var(ctx, receiver) => {
            SolveResult::Error(InferError::NoMember {
                receiver,
                name,
                is_call,
                span,
            })
        },
        kestrel_entail::Tri::Fails | kestrel_entail::Tri::Unknown => {
            SolveResult::Deferred(Constraint::Applicable {
                receiver,
                source,
                name,
                is_call,
                span,
            })
        },
    }
}

/// Is a member reached through `source` applicable to a receiver of kind
/// `recv`? One kernel question either way.
pub(super) fn applicability(
    ctx: &InferCtx<'_>,
    source: crate::constraint::ApplicableSource,
    recv: &TyKind,
) -> kestrel_entail::Tri {
    use crate::constraint::ApplicableSource;
    match source {
        ApplicableSource::Extension(ext) => {
            crate::solver_entail::extension_applicability(ctx, ext, recv)
        },
        ApplicableSource::Protocol(proto) => {
            crate::solver_entail::kind_conformance(ctx, recv, proto)
        },
    }
}

/// Collect protocols `param` is DIRECTLY bound to via its `Conformances`
/// component. Does not walk inheritance or extension-added conformances —
/// downstream `solve_conforms` runs each emitted obligation through
/// `ConformingProtocols`, so the transitive closure is handled there.
pub(super) fn direct_param_bound_protocols(ctx: &InferCtx<'_>, param: Entity) -> Vec<Entity> {
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

/// For a TYPE-ARGUMENT conformance FAILURE, record `(structural type,
/// protocol)` and return whether this exact violation was already reported.
/// The same wellformedness obligation is emitted from the call-site
/// where-clause, the annotation formation, and the return-position formation,
/// so a single concrete violation can fail several times over independent
/// TyVar trees — key structurally and report once. Expr-origin conforms are
/// never deduped (false), so genuine value-level conformance errors are
/// unaffected.
pub(super) fn typearg_failure_is_duplicate(
    ctx: &mut InferCtx<'_>,
    resolved: TyVar,
    protocol: kestrel_hecs::Entity,
    origin: ConformsOrigin,
) -> bool {
    if origin != ConformsOrigin::TypeArg {
        return false;
    }
    let key = (crate::result::describe_tyvar(ctx, resolved), protocol);
    // `insert` returns false when the key was already present.
    !ctx.reported_typearg_conformance.insert(key)
}

pub(super) fn solve_conforms(
    ctx: &mut InferCtx<'_>,
    ty: TyVar,
    protocol: kestrel_hecs::Entity,
    span: Span,
    poison_ty_on_failure: bool,
    origin: ConformsOrigin,
    protocol_args: Vec<TyVar>,
) -> SolveResult {
    let resolved = ctx.resolve(ty);
    match ctx.slot(resolved) {
        TySlot::Unresolved { .. } => SolveResult::Deferred(Constraint::Conforms {
            ty,
            protocol,
            span,
            poison_ty_on_failure,
            origin,
            protocol_args: protocol_args.clone(),
        }),
        // `Never` has no values, so it vacuously conforms to every protocol
        // (`(return 1) + 2` needs `Never: Addable`).
        TySlot::Resolved(TyKind::Error | TyKind::Never) => SolveResult::Solved,
        // The ref matrix (stage 2b, conformances via `extend &T`):
        // - Static: a ref is the one thing that is never Static — reject
        //   regardless of origin (judging the pointee would let `&T` slip
        //   through containment bounds).
        // - Expr origin: transparent place — a ref conforms as its POINTEE
        //   (protocol dispatch through `&T` borrows the place: for-in's
        //   Iterable bound, operator protocols).
        // - TypeArg origin: the ref ITSELF is judged. Refs bit-copy, so
        //   Copyable holds (not known Cloneable); every other bound holds
        //   iff `extend &T: P` / `extend &mutating T: P` declares it AND the
        //   extension's `where` clauses hold at the pointee — the same
        //   declared+satisfies pair as the nominal arm, routed through the
        //   synthetic `lang.&` entities. No declaring extension → clean
        //   DoesNotConform (an unguarded permit would instantiate a pointee
        //   witness at `&U` — the witness_instantiation_collapse ICE class).
        TySlot::Resolved(TyKind::Ref { pointee, .. }) => {
            // A reference is two words even to an unsized pointee.
            if is_sized_builtin(ctx, protocol) {
                return SolveResult::Solved;
            }
            if is_static_builtin(ctx, protocol) {
                if poison_ty_on_failure {
                    ctx.poison(ty);
                }
                return SolveResult::Error(InferError::DoesNotConform {
                    ty,
                    protocol,
                    protocol_args: protocol_args.clone(),
                    span,
                });
            }
            if origin == ConformsOrigin::TypeArg {
                let conforms = match copyable_builtin_kind(ctx, protocol) {
                    Some(want_cloneable) => !want_cloneable,
                    None => {
                        let kind = match ctx.slot(resolved) {
                            TySlot::Resolved(k) => k.clone(),
                            _ => unreachable!(),
                        };
                        if !ctx.resolver.conforms_to(&kind, protocol) {
                            false
                        } else {
                            match crate::solver_entail::conformance(ctx, resolved, protocol) {
                                kestrel_entail::Tri::Holds => true,
                                kestrel_entail::Tri::Fails => false,
                                kestrel_entail::Tri::Unknown => {
                                    return SolveResult::Deferred(Constraint::Conforms {
                                        ty,
                                        protocol,
                                        span,
                                        poison_ty_on_failure,
                                        origin,
                                        protocol_args: protocol_args.clone(),
                                    });
                                },
                            }
                        }
                    },
                };
                if conforms {
                    return SolveResult::Solved;
                }
                if typearg_failure_is_duplicate(ctx, resolved, protocol, origin) {
                    return SolveResult::Solved;
                }
                if poison_ty_on_failure {
                    ctx.poison(ty);
                }
                return SolveResult::Error(InferError::DoesNotConform {
                    ty,
                    protocol,
                    protocol_args: protocol_args.clone(),
                    span,
                });
            }
            let pointee = *pointee;
            SolveResult::Deferred(Constraint::Conforms {
                ty: pointee,
                protocol,
                span,
                poison_ty_on_failure,
                origin,
                protocol_args: protocol_args.clone(),
            })
        },
        TySlot::Resolved(_) => {
            // Per-instantiation Copyable/Cloneable: evaluate conditional
            // `extend X: Copyable where T: Copyable` against the resolved type
            // args, so `Box[Int]` is Copyable while `Box[File]` is move-only.
            // Static first: structural (never declared), so the declared-
            // conformance path below can't answer it.
            let conforms = if is_static_builtin(ctx, protocol) {
                solver_ty_is_static(ctx, resolved, 0)
            } else if is_sized_builtin(ctx, protocol) {
                solver_ty_is_sized(ctx, resolved, 0)
            } else if let Some(want_cloneable) = copyable_builtin_kind(ctx, protocol) {
                type_conforms_copyable(ctx, resolved, want_cloneable)
            } else {
                let kind = match ctx.slot(resolved) {
                    TySlot::Resolved(k) => k.clone(),
                    _ => unreachable!(),
                };
                // Declared AND — for a conditional conformance — its `where`
                // clauses hold for the concrete args. The second check turns a
                // concrete bound violation (e.g. `Result[NotExitable, E]:
                // Exitable`, which the unconditional `conforms_to` accepts) into a
                // clean `DoesNotConform` rather than a mono `report()` witness ICE.
                // Conservative: `type_satisfies` permits abstract/generic
                // positions, so this never rejects a body whose bound holds
                // abstractly — only concrete violations. That same permissiveness
                // is why `projection_base_admits` has to sit here: for an
                // associated-type projection `conforms_to` is the SOLE decider,
                // and it answers declares-only (G17 C4).
                // An existential that names `protocol` declares it in effect
                // (`any P: P`); the kernel decides whether it holds.
                let declared = ctx.resolver.conforms_to(&kind, protocol)
                    || crate::existential::names_protocol(ctx, resolved, protocol);
                if !(declared && projection_base_admits(ctx, &kind, protocol)) {
                    false
                } else {
                    match crate::solver_entail::conformance(ctx, resolved, protocol) {
                        kestrel_entail::Tri::Holds => true,
                        kestrel_entail::Tri::Fails => false,
                        // An open var in the type args: re-queue until it
                        // resolves — never permit on the guess (N1). The
                        // bound's protocol args still pin it now (the element
                        // of `100..=200` against `RandomBounds[Int16]`),
                        // before literal defaulting can pick Int64.
                        kestrel_entail::Tri::Unknown => {
                            if unify_bound_protocol_args(ctx, resolved, protocol, &protocol_args)
                                .is_err()
                            {
                                if poison_ty_on_failure {
                                    ctx.poison(ty);
                                }
                                return SolveResult::Error(InferError::DoesNotConform {
                                    ty,
                                    protocol,
                                    protocol_args: protocol_args.clone(),
                                    span,
                                });
                            }
                            return SolveResult::Deferred(Constraint::Conforms {
                                ty,
                                protocol,
                                span,
                                poison_ty_on_failure,
                                origin,
                                protocol_args: protocol_args.clone(),
                            });
                        },
                    }
                }
            };
            if conforms {
                // Parameterized bounds: `where R: P[Int16]` must also pin the
                // matched conformance's declared protocol args against the
                // bound's args — dropping them lets `R = ClosedRange[Int64]`
                // sail through a `RandomBounds[Int16]` bound and mono then
                // reads the range at the wrong layout (silent miscompile).
                // Unification also drives inference: an unresolved literal
                // arg (the element of `100..=200`) gets pinned by the bound
                // before literal defaulting runs.
                match unify_bound_protocol_args(ctx, resolved, protocol, &protocol_args) {
                    Ok(BoundArgs::Decided) => SolveResult::Solved,
                    Ok(BoundArgs::Undecided) => SolveResult::Deferred(Constraint::Conforms {
                        ty,
                        protocol,
                        span,
                        poison_ty_on_failure,
                        origin,
                        protocol_args,
                    }),
                    Err(()) => {
                        if poison_ty_on_failure {
                            ctx.poison(ty);
                        }
                        SolveResult::Error(InferError::DoesNotConform {
                            ty,
                            protocol,
                            protocol_args: protocol_args.clone(),
                            span,
                        })
                    },
                }
            } else if typearg_failure_is_duplicate(ctx, resolved, protocol, origin) {
                SolveResult::Solved
            } else {
                // E157: `?.` on a type with no payload names that type, so
                // keep a copy of it before poisoning overwrites the slot.
                if ctx.resolver.builtin(Builtin::OptionalChainProtocol) == Some(protocol) {
                    let shown = ctx.fresh();
                    ctx.types[shown.0 as usize] = ctx.slot(resolved).clone();
                    if poison_ty_on_failure {
                        ctx.poison(ty);
                    }
                    return SolveResult::Error(InferError::OptionalChainNotConforming {
                        ty: shown,
                        span,
                    });
                }
                if poison_ty_on_failure {
                    ctx.poison(ty);
                }
                // E146 names what blocks `any P: P`.
                SolveResult::Error(
                    crate::existential::self_conformance_error(ctx, resolved, protocol, &span)
                        .unwrap_or_else(|| InferError::DoesNotConform {
                            ty,
                            protocol,
                            protocol_args: protocol_args.clone(),
                            span,
                        }),
                )
            }
        },
        TySlot::Redirect(_) => unreachable!("resolve() follows redirects"),
    }
}

/// G17 C4 — the base-aware half of a projection's conformance answer.
///
/// `WorldResolver::conforms_to`'s `TyKind::AssocProjection` arm answers
/// *declares-only*: it collects the bounds filed against the alias **entity**
/// and discards `base`, because a `WorldResolver` holds no `InferCtx` and so
/// cannot resolve a `TyVar`. Here it can, so this is where the two halves meet.
///
/// Non-projections answer `true` unchanged — a `Param`, `SelfType`, or nominal
/// subject is not what this narrows.
pub(super) fn projection_base_admits(ctx: &InferCtx<'_>, kind: &TyKind, protocol: Entity) -> bool {
    let TyKind::AssocProjection { assoc, .. } = kind else {
        return true;
    };
    // No comparable receiver (a concrete base, an unresolved one, `Self`) —
    // permit, matching this module's contract for abstract positions. An
    // unknown must never manufacture a rejection.
    let Some(spine) = projection_base_spine(ctx, kind) else {
        return true;
    };
    ctx.resolver
        .assoc_projection_base_admits(*assoc, protocol, &spine)
}

/// The base spine of a `TyKind::AssocProjection`, or `None` for any other
/// kind or a base that cannot be compared. Shared by the conformance answer
/// (C4) and member lookup (S4) so both judge the same receiver.
pub(super) fn projection_base_spine(ctx: &InferCtx<'_>, kind: &TyKind) -> Option<Vec<Entity>> {
    let TyKind::AssocProjection { base, .. } = kind else {
        return None;
    };
    base_spine(ctx, *base, 0)
}

/// Flatten a projection base to the receiver spine
/// `WhereSubject::spine` produces, so the two are directly comparable:
/// `B` is `[B]`, `T.Iter` is `[T, Iter]`.
///
/// `None` for any base that is not a chain of projections over a type
/// parameter — the caller reads that as "cannot compare".
pub(super) fn base_spine(ctx: &InferCtx<'_>, base: TyVar, depth: u32) -> Option<Vec<Entity>> {
    // Projection chains are shallow by construction; the cap is only so a
    // cyclic slot graph cannot spin here.
    if depth > 16 {
        return None;
    }
    match ctx.slot(ctx.resolve(base)) {
        TySlot::Resolved(TyKind::Param { entity }) => Some(vec![*entity]),
        TySlot::Resolved(TyKind::AssocProjection { base, assoc }) => {
            let (base, assoc) = (*base, *assoc);
            let mut spine = base_spine(ctx, base, depth + 1)?;
            spine.push(assoc);
            Some(spine)
        },
        _ => None,
    }
}

/// Unify an obligation's protocol args (`Constraint::Conforms::protocol_args`)
/// with the declared
/// conformance's protocol args instantiated at the concrete receiver.
///
/// `extend Foo[T]: P[T]` at receiver `Foo[X]` declares `P[X]`; a bound
/// `R: P[Int16]` therefore requires `X = Int16`. When `X` is still an
/// unresolved (literal) TyVar this PINS it — bound-driven inference; when
/// `X` is already concrete and different, this reports the mismatch that
/// was previously silently accepted (and miscompiled at mono).
///
/// Overlapping specializations (one concrete conformance per element type)
/// are first narrowed by the bound's args; a unique survivor also PINS the
/// receiver's own args via the extension's concrete target positions, so
/// `Int8.random(in: 3..=9)` infers `ClosedRange[Int8]` instead of letting
/// the literal default to `Int64`.
///
/// Conservative by design: no args, non-nominal receivers,
/// arg-less conformance declarations, or genuinely ambiguous multi-source
/// matches all permit (returning `Ok`) — only a definite arg conflict
/// (including "no declared instantiation is compatible with the bound")
/// rejects.
pub(super) fn unify_bound_protocol_args(
    ctx: &mut InferCtx<'_>,
    resolved: TyVar,
    protocol: Entity,
    explicit_args: &[TyVar],
) -> Result<BoundArgs, ()> {
    // The obligation carries the protocol args it requires (a call through
    // a generic protocol member, or a `where T: P[Args]` bound — see
    // `InferCtx::bound_typearg`); an obligation without args judges the
    // protocol alone.
    let want_args = explicit_args.to_vec();
    if want_args.is_empty() {
        return Ok(BoundArgs::Decided);
    }

    // Only nominal receivers carry declared conformance instantiations.
    let (entity, recv_args) = match ctx.slot(resolved) {
        TySlot::Resolved(TyKind::Struct { entity, args })
        | TySlot::Resolved(TyKind::Enum { entity, args }) => (*entity, args.clone()),
        _ => return Ok(BoundArgs::Decided),
    };

    // Find the conformance declaration(s) for this protocol whose target
    // instantiation applies to the receiver.
    let insts = ctx
        .query_ctx
        .query(kestrel_name_res::ConformingProtocolInstantiations {
            entity,
            root: ctx.root,
        });
    let mut applicable: Vec<ApplicableConformance> = Vec::new();
    for (proto, source, ast_args) in &insts {
        if *proto != protocol || ast_args.is_empty() {
            continue;
        }
        // Map the source's free type params to the receiver's concrete args:
        // decl-header conformances map the type's own params positionally;
        // extension conformances map the Param positions of the extension's
        // target args (skipping extensions whose concrete target args don't
        // match this instantiation). Applicability is judged against the
        // receiver's arg TyVars directly (not a reified HirTy): an UNRESOLVED
        // receiver arg — a still-defaultable literal — must keep concrete
        // extensions in play so the bound can pin it, whereas reification
        // maps it to `Error` which matches nothing.
        let (subs, target_args): (Vec<(Entity, TyVar)>, Vec<kestrel_hir::ty::HirTy>) =
            if *source == entity {
                let subs = ctx
                    .query_ctx
                    .get::<TypeParams>(entity)
                    .map(|tp| tp.0.clone())
                    .unwrap_or_default()
                    .into_iter()
                    .zip(recv_args.iter().copied())
                    .collect();
                (subs, Vec::new())
            } else {
                let target_args = ctx
                    .query_ctx
                    .query(kestrel_hir_lower::LowerExtensionTargetTypeArgs {
                        extension: *source,
                        root: ctx.root,
                    })
                    .unwrap_or_default();
                if target_args
                    .iter()
                    .zip(recv_args.iter())
                    .any(|(t, &r)| tv_definitely_incompatible(ctx, r, t))
                {
                    continue;
                }
                let mut subs: Vec<(Entity, TyVar)> = target_args
                    .iter()
                    .zip(recv_args.iter().copied())
                    .filter_map(|(t, c)| match t {
                        kestrel_hir::ty::HirTy::Param(e, _) => Some((*e, c)),
                        _ => None,
                    })
                    .collect();
                // Blanket free params (`extend Int64: SeqIndex[T]` — T appears
                // only on the protocol side): the conformance holds for every T,
                // so map them to fresh TyVars that unify freely with the bound's
                // args instead of leaking as rigid `Param`s.
                for p in ctx
                    .query_ctx
                    .get::<TypeParams>(*source)
                    .map(|tp| tp.0.clone())
                    .unwrap_or_default()
                {
                    if !subs.iter().any(|(e, _)| *e == p) {
                        let fresh = ctx.fresh();
                        subs.push((p, fresh));
                    }
                }
                (subs, target_args)
            };
        // Lower the declared protocol args (in the source's scope) to HirTy.
        let decl_args: Vec<kestrel_hir::ty::HirTy> = ast_args
            .iter()
            .map(|a| kestrel_hir_lower::lower_ast_type(ctx.query_ctx, *source, ctx.root, a))
            .collect();
        applicable.push(ApplicableConformance {
            decl_args,
            subs,
            target_args,
        });
    }
    if applicable.is_empty() {
        // No parameterized declaration — permit.
        return Ok(BoundArgs::Decided);
    }
    // Several applicable sources (overlapping specializations — e.g. one
    // concrete conformance per element type): narrow by the bound's args.
    // A source whose DECLARED protocol args definitely contradict the bound's
    // can never be the one mono selects, so drop it. Exactly one survivor →
    // proceed (this is what pins a literal receiver arg to the only viable
    // instantiation); several survivors → permit (mono picks most-specific);
    // zero → every declared instantiation contradicts the bound: reject.
    if applicable.len() > 1 {
        applicable.retain(|c| {
            !want_args
                .iter()
                .zip(c.decl_args.iter())
                .any(|(&w, d)| tv_definitely_incompatible(ctx, w, d))
        });
        if applicable.is_empty() {
            return Err(());
        }
        if applicable.len() > 1 {
            // A call's own protocol args (`Int8(from: x)` → `Convertible[?]`)
            // that are still open must be waited for: permitting now lets an
            // instantiation with no conformance reach mono (fuzz H20). Bound
            // args recorded from where clauses keep the permit.
            if !explicit_args.is_empty()
                && want_args
                    .iter()
                    .any(|&w| crate::solver_entail::has_open_var(ctx, w))
            {
                return Ok(BoundArgs::Undecided);
            }
            return Ok(BoundArgs::Decided);
        }
    }
    let ApplicableConformance {
        decl_args,
        subs,
        target_args,
    } = applicable.swap_remove(0);

    // Pin the receiver's args against the selected source's CONCRETE target
    // positions (`extend ClosedRange[Int8]: P[Int8]` at receiver
    // `ClosedRange[?lit]` pins `?lit = Int8`). A resolved-but-conflicting
    // receiver arg fails here — the conformance provably doesn't apply.
    for (t, &r) in target_args.iter().zip(recv_args.iter()) {
        if matches!(t, kestrel_hir::ty::HirTy::Param(..)) {
            continue;
        }
        let t_tv = lower_hir_ty_sub(ctx, t, None, resolved, &subs);
        if unify::unify(ctx, r, t_tv).is_err() {
            return Err(());
        }
    }

    for (want, decl) in want_args.iter().zip(decl_args.iter()) {
        let decl_tv = lower_hir_ty_sub(ctx, decl, None, resolved, &subs);
        if unify::unify(ctx, *want, decl_tv).is_err() {
            return Err(());
        }
    }
    Ok(BoundArgs::Decided)
}

/// Outcome of [`unify_bound_protocol_args`] when no contradiction was found.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum BoundArgs {
    /// The args were checked (or there was nothing to check).
    Decided,
    /// Several declared instantiations still fit open args: retry later.
    Undecided,
}

/// One conformance declaration of (receiver-entity, protocol) that structurally
/// applies to the receiver instantiation, as collected by
/// `unify_bound_protocol_args`.
pub(super) struct ApplicableConformance {
    /// The declared protocol args, lowered in the source's scope.
    decl_args: Vec<kestrel_hir::ty::HirTy>,
    /// Source free param → receiver arg (or fresh) substitution.
    subs: Vec<(Entity, TyVar)>,
    /// The extension's own target args (empty for decl-header conformances).
    target_args: Vec<kestrel_hir::ty::HirTy>,
}

/// Does the (resolved state of) `tv` PROVABLY conflict with `hir`? Only a
/// nominal-vs-nominal entity mismatch (recursively through args) counts;
/// unresolved vars, `Param` patterns, and every non-nominal shape are
/// "can't rule out" — this must never reject on incompleteness.
pub(super) fn tv_definitely_incompatible(
    ctx: &InferCtx<'_>,
    tv: TyVar,
    hir: &kestrel_hir::ty::HirTy,
) -> bool {
    use kestrel_hir::ty::HirTy;
    let (h_entity, h_args) = match hir {
        HirTy::Struct { entity, args, .. } | HirTy::Enum { entity, args, .. } => (*entity, args),
        _ => return false,
    };
    let resolved = ctx.resolve(tv);
    let (t_entity, t_args) = match ctx.slot(resolved) {
        TySlot::Resolved(TyKind::Struct { entity, args })
        | TySlot::Resolved(TyKind::Enum { entity, args }) => (*entity, args.clone()),
        _ => return false,
    };
    t_entity != h_entity
        || t_args
            .iter()
            .zip(h_args.iter())
            .any(|(&t, h)| tv_definitely_incompatible(ctx, t, h))
}

/// Reify a fully-resolved `TyVar` into a `HirTy` for the bound-aware conformance
/// check (`crate::conformance::type_satisfies`). Only the cases that check
/// inspects (nominal / `()` / `!` / type-param) are reified faithfully; anything
/// else maps to `HirTy::Error`, which `type_satisfies` permits. Unresolved slots
/// likewise map to `Error` (permit) — this only runs on otherwise-resolved types.
pub(crate) fn reify_tv(ctx: &InferCtx<'_>, tv: TyVar) -> kestrel_hir::ty::HirTy {
    let resolved = ctx.resolve(tv);
    match ctx.slot(resolved) {
        TySlot::Resolved(kind) => reify_kind(ctx, &kind.clone()),
        _ => kestrel_hir::ty::HirTy::Error(Span::synthetic(0)),
    }
}

pub(crate) fn reify_kind(ctx: &InferCtx<'_>, kind: &TyKind) -> kestrel_hir::ty::HirTy {
    use kestrel_hir::ty::HirTy;
    let span = Span::synthetic(0);
    let reify_args = |args: &[TyVar]| args.iter().map(|&a| reify_tv(ctx, a)).collect();
    match kind {
        TyKind::Struct { entity, args } => HirTy::Struct {
            entity: *entity,
            args: reify_args(args),
            span,
        },
        TyKind::Enum { entity, args } => HirTy::Enum {
            entity: *entity,
            args: reify_args(args),
            span,
        },
        TyKind::Protocol { entity, args } => HirTy::Protocol {
            entity: *entity,
            args: reify_args(args),
            span,
        },
        TyKind::Tuple(elems) => HirTy::Tuple(reify_args(elems), span),
        TyKind::Never => HirTy::Never(span),
        TyKind::Param { entity } => HirTy::Param(*entity, span),
        // Faithful Ref reification (stage 2b): `type_satisfies` must see
        // `Optional[&T]`, not `Optional[Error]`.
        TyKind::Ref { pointee, mutating } => HirTy::Ref {
            inner: Box::new(reify_tv(ctx, *pointee)),
            mutating: *mutating,
            span,
        },
        // Keep the bounds: conformance of an existential is decided from
        // them (`any P: P`, existentials design §6).
        TyKind::RawExistential { bounds, bindings } => HirTy::RawExistential {
            bounds: bounds
                .iter()
                .map(|(entity, args)| HirTy::Protocol {
                    entity: *entity,
                    args: reify_args(args),
                    span: span.clone(),
                })
                .collect(),
            bindings: bindings
                .iter()
                .map(|&(assoc, ty)| kestrel_hir::ty::HirAssocBinding {
                    assoc,
                    ty: reify_tv(ctx, ty),
                })
                .collect(),
            span,
        },
        _ => HirTy::Error(span),
    }
}

/// Returns `Some(want_cloneable)` if `protocol` is the Copyable (`false`) or
/// Cloneable (`true`) builtin, else `None`.
pub(crate) fn copyable_builtin_kind(ctx: &InferCtx<'_>, protocol: Entity) -> Option<bool> {
    if ctx.query_ctx.query(ResolveBuiltin {
        builtin: Builtin::Copyable,
        root: ctx.root,
    }) == Some(protocol)
    {
        return Some(false);
    }
    if ctx.query_ctx.query(ResolveBuiltin {
        builtin: Builtin::Cloneable,
        root: ctx.root,
    }) == Some(protocol)
    {
        return Some(true);
    }
    None
}

/// `CopyLayer` over `TyVar` — the solver's hooks into the shared decision tree
/// (`kestrel_copy_fold::instance_semantics`, the single source of truth for
/// per-instantiation copy semantics across semantics / solver / analyze /
/// MIR). Layer-specific plumbing: the recursion-depth bookkeeping threaded
/// through `member_semantics`.
pub(super) struct SolverCopyLayer<'a, 'i> {
    ctx: &'a InferCtx<'i>,
    depth: u32,
}

impl CopyLayer for SolverCopyLayer<'_, '_> {
    type Ty = TyVar;
    type Sem = CopySemantics;

    fn base_semantics(&self, entity: Entity) -> CopySemantics {
        // Plain NominalCopySemantics query (no cycle guard needed mid-inference).
        let sem = self
            .ctx
            .query_ctx
            .query(NominalCopySemantics {
                entity,
                root: self.ctx.root,
            })
            .semantics;
        kestrel_debug::ktrace!("copyable", "nominal {:?} sem={:?}", entity, sem);
        sem
    }

    fn gating_positions(&self, entity: Entity) -> Cow<'_, [usize]> {
        Cow::Owned(self.ctx.query_ctx.query(ConditionalCopyableParams {
            entity,
            root: self.ctx.root,
        }))
    }

    fn sem_from_class(&self, _: Entity, class: CopySemantics) -> CopySemantics {
        class
    }

    fn member_semantics(&self, &tv: &TyVar) -> CopySemantics {
        // Depth bookkeeping = layer plumbing.
        solver_copy_class(self.ctx, tv, self.depth + 1)
    }
}

/// Per-instantiation Copyable/Cloneable classification. Evaluates conditional
/// `extend X: Copyable where T: Copyable` conformances against the resolved
/// type args (via `kestrel_copy_fold::instance_semantics`), recursing into
/// args / tuple elements, so `Box[Int]` is Copyable while `Box[File]` is
/// move-only. Answer-equivalent to the former boolean `want_cloneable`
/// recursion: per node, `(conforms(Copyable), conforms(Cloneable)) ==
/// (class != NotCopyable, class == Cloneable)` — the "never block" arms
/// return `Cloneable` so BOTH questions pass.
///
/// WARNING: `Cloneable` from the never-block arms (unresolved vars,
/// `TyKind::Error`, the depth guard) is a stand-in for "don't block the
/// solver", not a real classification. Consume only via
/// `type_conforms_copyable`; reading the class directly for clone-vs-bitcopy
/// decisions or diagnostics would inherit a wrong answer.
pub(crate) fn solver_copy_class(ctx: &InferCtx<'_>, tv: TyVar, depth: u32) -> CopySemantics {
    if depth > 64 {
        return CopySemantics::Cloneable; // recursion guard — never block (both questions)
    }
    let resolved = ctx.resolve(tv);
    let kind = match ctx.slot(resolved) {
        TySlot::Resolved(k) => k.clone(),
        _ => return CopySemantics::Cloneable, // unresolved: never block (both questions)
    };
    match kind {
        TyKind::Error => CopySemantics::Cloneable, // never block (both questions)
        TyKind::Struct { entity, args } | TyKind::Enum { entity, args } => {
            instance_semantics(&SolverCopyLayer { ctx, depth }, entity, &args)
        },
        // A protocol's abstract `Self` is only as copyable as its refinements;
        // a concrete `Self` is its nominal (empty args: a gated base folds to
        // NotCopyable).
        TyKind::SelfType { entity }
            if ctx.query_ctx.get::<kestrel_ast_builder::NodeKind>(entity)
                == Some(&kestrel_ast_builder::NodeKind::Protocol) =>
        {
            ctx.query_ctx
                .query(kestrel_semantics::ProtocolSelfCopySemantics {
                    protocol: entity,
                    context: ctx.owner,
                    root: ctx.root,
                })
        },
        TyKind::SelfType { entity } => {
            instance_semantics(&SolverCopyLayer { ctx, depth }, entity, &[])
        },
        // HOOK: structural parent context scopes the bound lookup.
        TyKind::Param { entity } => {
            let context = ctx.query_ctx.parent_of(entity).unwrap_or(entity);
            ctx.query_ctx
                .query(TypeParamCopyRequirement {
                    param: entity,
                    context,
                    root: ctx.root,
                })
                .into()
        },
        // Canonical fold (copy-drift #2 resolved 2026-06-10): a mixed tuple
        // like `(Int64, RcBox)` is Cloneable — it satisfies `T: Copyable` AND
        // `T: Cloneable` bounds (MIR already clone-elaborates its copies).
        // `()` folds to Copyable, no longer vacuously Cloneable.
        TyKind::Tuple(elems) => {
            fold_members(elems.iter().map(|&e| solver_copy_class(ctx, e, depth + 1)))
        },
        // A ref-as-TYPE is Copyable (stage 2b ruling: payload/field refs
        // bit-copy — the copy aliases the same storage, may-alias model).
        // Not known Cloneable. The pointee question ("can a read through
        // this ref place copy?") is the Expr-origin peel in solve_conforms,
        // never this fold — this classifies the ref VALUE for gating-arg
        // folds (`Optional[&File]` is Copyable even though File isn't).
        TyKind::Ref { .. } => CopySemantics::Copyable,
        // `some P and not Copyable`: the underlier may be move-only, so use
        // sites must move, never bit-copy. Plain `some P` guarantees a
        // duplicable underlier (enforced post-solve on the defining body).
        TyKind::Opaque { not_copyable, .. } => {
            if not_copyable {
                CopySemantics::NotCopyable
            } else {
                CopySemantics::Copyable
            }
        },
        TyKind::RawExistential { bounds, .. } => kestrel_semantics::raw_existential_copy_semantics(
            ctx.query_ctx,
            bounds.iter().map(|(protocol, _)| *protocol),
            ctx.root,
        ),
        // A closure value's copy class is fixed by its KIND (plan D6): normal
        // views bit-copy, `mutating`/`consuming` are move-only, `escaping`
        // shares its environment through the box's clone.
        TyKind::Function { kind, .. } => kestrel_copy_fold::fn_kind_semantics(kind),
        // Mirror `hir_type_copy_semantics`: protocol existentials are Copyable
        // (not known Cloneable).
        TyKind::Protocol { .. } | TyKind::Never | TyKind::TypeAlias { .. } => {
            CopySemantics::Copyable
        },
        // Associated projection (`I.Item`) is Copyable-by-default (implicit
        // bound), like a type param — matches `hir_type_copy_semantics` and MIR
        // `ty_query`. Not known to be Cloneable, so only satisfies Copyable.
        TyKind::AssocProjection { .. } => CopySemantics::Copyable,
    }
}

/// Boundary mapping from the tri-state classifier to the solver's boolean
/// conformance question — sole caller is `solve_conforms`. `want_cloneable`
/// selects the Cloneable (`true`) vs Copyable (`false`) question.
pub(super) fn type_conforms_copyable(ctx: &InferCtx<'_>, tv: TyVar, want_cloneable: bool) -> bool {
    // A `Copyable` bound promises an implicitly copyable *value*, which needs
    // a compile-time size: a Cloneable raw existential is Cloneable but not
    // Copyable (and `T: Copyable` implies `T: Sized`, see
    // `TypeParamSizeRequirement`).
    if !want_cloneable && !solver_ty_is_sized(ctx, tv, 0) {
        return false;
    }
    match solver_copy_class(ctx, tv, 0) {
        CopySemantics::NotCopyable => false,
        CopySemantics::Cloneable => true,
        CopySemantics::Copyable => !want_cloneable,
    }
}

/// Is `protocol` the `Sized` builtin (existentials)?
pub(crate) fn is_sized_builtin(ctx: &InferCtx<'_>, protocol: Entity) -> bool {
    ctx.query_ctx.query(ResolveBuiltin {
        builtin: Builtin::Sized,
        root: ctx.root,
    }) == Some(protocol)
}

/// `SizedLayer` over `TyVar` — the solver's face of the per-instantiation
/// sizedness rule (`kestrel_semantics::instance_is_sized`).
struct SolverSizedLayer<'a, 'i> {
    ctx: &'a InferCtx<'i>,
    depth: u32,
}

impl kestrel_semantics::SizedLayer for SolverSizedLayer<'_, '_> {
    type Ty = TyVar;

    fn nominal_sizedness(&self, entity: Entity) -> kestrel_semantics::Sizedness {
        self.ctx
            .query_ctx
            .query(kestrel_semantics::NominalSizedness {
                entity,
                root: self.ctx.root,
            })
    }

    fn member_is_sized(&self, &tv: &TyVar) -> bool {
        solver_ty_is_sized(self.ctx, tv, self.depth + 1)
    }
}

/// Is `tv` Sized (existentials design §2)? The twin of
/// `kestrel_semantics::hir_type_is_sized`. A type parameter is judged from
/// the body being inferred (`ctx.owner`), so a member of an
/// `extend Box[T] where T: Sized` extension sees `T` as Sized. Unresolved
/// never blocks.
pub(crate) fn solver_ty_is_sized(ctx: &InferCtx<'_>, tv: TyVar, depth: u32) -> bool {
    if depth > 64 {
        return true;
    }
    let resolved = ctx.resolve(tv);
    let TySlot::Resolved(kind) = ctx.slot(resolved) else {
        return true;
    };
    match kind.clone() {
        TyKind::RawExistential { .. } => false,
        TyKind::Struct { entity, args } => {
            kestrel_semantics::instance_is_sized(&SolverSizedLayer { ctx, depth }, entity, &args)
        },
        TyKind::Param { entity } => {
            ctx.query_ctx
                .query(kestrel_semantics::TypeParamSizeRequirement {
                    param: entity,
                    context: ctx.owner,
                    root: ctx.root,
                })
                == kestrel_semantics::SizeRequirement::RequiresSized
        },
        TyKind::AssocProjection { assoc, .. } => {
            !kestrel_semantics::assoc_may_be_unsized(ctx.query_ctx, assoc, ctx.root)
        },
        TyKind::Enum { .. }
        | TyKind::Protocol { .. }
        | TyKind::SelfType { .. }
        | TyKind::TypeAlias { .. }
        | TyKind::Tuple(_)
        | TyKind::Function { .. }
        | TyKind::Ref { .. }
        | TyKind::Opaque { .. }
        | TyKind::Never
        | TyKind::Error => true,
    }
}

/// Is `protocol` the `Static` builtin (references 2a)?
pub(crate) fn is_static_builtin(ctx: &InferCtx<'_>, protocol: Entity) -> bool {
    ctx.query_ctx.query(ResolveBuiltin {
        builtin: Builtin::Static,
        root: ctx.root,
    }) == Some(protocol)
}

/// `StaticLayer` over `TyVar` — the solver's hooks into the per-instantiation
/// staticness rule (`kestrel_semantics::instance_is_static`, the single
/// source of truth across semantics / solver / analyze). Depth bookkeeping
/// mirrors `SolverCopyLayer`.
pub(super) struct SolverStaticLayer<'a, 'i> {
    ctx: &'a InferCtx<'i>,
    depth: u32,
}

impl StaticLayer for SolverStaticLayer<'_, '_> {
    type Ty = TyVar;

    fn nominal_staticness(&self, entity: Entity) -> Staticness {
        self.ctx
            .query_ctx
            .query(NominalStaticness {
                entity,
                root: self.ctx.root,
            })
            .staticness
    }

    fn member_is_static(&self, &tv: &TyVar) -> bool {
        solver_ty_is_static(self.ctx, tv, self.depth + 1)
    }
}

/// Per-instantiation staticness for the solver: structural contains-ref over
/// resolved type vars. Never-block arms (unresolved / Error / depth guard)
/// answer `true` — in 2a nothing unresolved can become non-Static, and a
/// missed rejection is a mono-stage gap, not a miscompile (Static has no
/// runtime semantics). The universal fast path: `Staticness::Static` bases
/// return without touching args.
pub(crate) fn solver_ty_is_static(ctx: &InferCtx<'_>, tv: TyVar, depth: u32) -> bool {
    if depth > 64 {
        return true; // recursion guard — never block
    }
    let resolved = ctx.resolve(tv);
    let kind = match ctx.slot(resolved) {
        TySlot::Resolved(k) => k.clone(),
        _ => return true, // unresolved: never block
    };
    match kind {
        // The axiom: a reference is never Static.
        TyKind::Ref { .. } => false,
        TyKind::Struct { entity, args } | TyKind::Enum { entity, args } => {
            instance_is_static(&SolverStaticLayer { ctx, depth }, entity, &args)
        },
        // Empty args: a conditional base is unprovable here (mirrors the
        // copy layer's empty-args rule).
        TyKind::SelfType { entity } => {
            instance_is_static(&SolverStaticLayer { ctx, depth }, entity, &[])
        },
        TyKind::Param { entity } => {
            let context = ctx.query_ctx.parent_of(entity).unwrap_or(entity);
            ctx.query_ctx.query(TypeParamStaticRequirement {
                param: entity,
                context,
                root: ctx.root,
            }) == StaticRequirement::RequiresStatic
        },
        TyKind::Tuple(elems) => elems
            .iter()
            .all(|&e| solver_ty_is_static(ctx, e, depth + 1)),
        TyKind::RawExistential { bounds, .. } => kestrel_semantics::raw_existential_is_static(
            ctx.query_ctx,
            bounds.iter().map(|(protocol, _)| *protocol),
            ctx.root,
        ),
        // A view-kind closure is not Static (references 2c); owning kinds are.
        TyKind::Function { kind, .. } => !kind.is_view(),
        // Protocol / opaque / alias / assoc-projection / Never / Error:
        // conservative never-block leaves — refs can't reach them in 2a.
        TyKind::Error
        | TyKind::Protocol { .. }
        | TyKind::Opaque { .. }
        | TyKind::Never
        | TyKind::TypeAlias { .. }
        | TyKind::AssocProjection { .. } => true,
    }
}
