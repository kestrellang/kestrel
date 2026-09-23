//! `WhereClausesOf` query — resolves an entity's raw AST where clauses into
//! structured, typed constraints with entities resolved and HIR-lowered RHS.
//!
//! Contract: names in the where clause resolve in the given entity's own
//! scope (via scope walking). No separate `context` parameter — the entity
//! IS the context. The query is memoized per `(entity, root)`.
//!
//! Implemented as free functions (not methods on a stateful resolver) so
//! there is no ambient `self.owner` to accidentally leak into name lookup.

use kestrel_ast_builder::{
    AstType, Intrinsic, NodeKind, TypeParams, WhereClause as AstWhereClause, WhereConstraint,
};
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir::Builtin;
use kestrel_hir::ty::HirTy;
use kestrel_name_res::{ResolveBuiltin, ResolveTypePath, TypeResolution, resolve_type_path_chain};
use kestrel_semantics::{
    CopyRequirement, CopySemantics, NominalCopySemantics, StaticRequirement,
    TypeParamCopyRequirement, TypeParamStaticRequirement,
};
use kestrel_span::Span;

use crate::resolve::{WhereClause, WhereSubject};

/// Query: resolved where clauses attached to `entity`, with all names looked
/// up in `entity`'s own scope.
///
/// Returns an empty vec if the entity has no where clauses or none resolve.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct WhereClausesOf {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for WhereClausesOf {
    type Output = Vec<WhereClause>;

    fn describe(&self) -> String {
        format!("WhereClausesOf({:?})", self.entity)
    }

    fn execute(&self, ctx: &QueryContext<'_>) -> Vec<WhereClause> {
        resolve_where_clauses(ctx, self.entity, self.root)
    }
}

/// Query: the clauses **written** on `entity` (no implicit `Copyable` /
/// `Static` bounds), plus every equality clause that could not be resolved.
///
/// `WhereClausesOf` is this query's `clauses` plus the implicit bounds. The
/// `dropped` half exists so a clause that resolution throws away is reported
/// at the clause (G29) by an analyzer — the one place a declaration-level
/// diagnostic is emitted exactly once — and never inside this memoized,
/// many-caller query.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ExplicitWhereClauses {
    pub entity: Entity,
    pub root: Entity,
}

/// Output of [`ExplicitWhereClauses`].
#[derive(Clone, Debug, Default, Hash)]
pub struct ExplicitWhereResolution {
    pub clauses: Vec<WhereClause>,
    pub dropped: Vec<DroppedEquality>,
}

/// An equality clause `param.segment = RHS` whose `segment` names no single
/// associated type. Recorded, not reported: `GenericsAnalyzer` turns it into
/// E479 (ambiguous) or E440 (not found).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DroppedEquality {
    /// The clause's left side, `Item.Out`.
    pub lhs_span: Span,
    /// The clause's root, `Item`.
    pub param: Entity,
    /// The segment that did not resolve, `Out`.
    pub segment: String,
    /// The associated-type requirements `segment` could name among the
    /// protocols this holder bounds `param` with. Two or more: ambiguous.
    /// None: no bound declares it.
    pub candidates: Vec<Entity>,
}

impl QueryFn for ExplicitWhereClauses {
    type Output = ExplicitWhereResolution;

    fn describe(&self) -> String {
        format!("ExplicitWhereClauses({:?})", self.entity)
    }

    fn execute(&self, ctx: &QueryContext<'_>) -> ExplicitWhereResolution {
        resolve_explicit_where_clauses(ctx, self.entity, self.root)
    }
}

/// The where clauses written on the decl that **declares** `param`: its
/// owning function, type, extension or protocol. A `TypeParameter` entity
/// never carries a where clause or `TypeParams` of its own, so
/// `WhereClausesOf { entity: param }` is always empty. This is the one place
/// that makes the hop to the parent (G15 / A16).
pub fn param_owner_where_clauses(
    ctx: &QueryContext<'_>,
    param: Entity,
    root: Entity,
) -> Vec<WhereClause> {
    ctx.parent_of(param)
        .map(|owner| {
            ctx.query(WhereClausesOf {
                entity: owner,
                root,
            })
        })
        .unwrap_or_default()
}

/// Free-function implementation. Takes `entity` (which is also the resolution
/// context). Separated from the query impl so it can be called directly by
/// other queries without going through the memoization layer when that
/// wouldn't help.
pub fn resolve_where_clauses(
    ctx: &QueryContext<'_>,
    entity: Entity,
    root: Entity,
) -> Vec<WhereClause> {
    let mut result = ctx.query(ExplicitWhereClauses { entity, root }).clauses;

    // Inject the implicit `T: Copyable` / `Cloneable` bound for every generic
    // param that is not declared `: not Copyable`. Emitting it as a Bound lets
    // the standard conformance machinery reject `not Copyable` arguments at the
    // call site. Runs even when the entity has no explicit where clause
    // (unconstrained params still get the implicit bound).
    inject_implicit_copyable_bounds(ctx, entity, root, &mut result);

    // Likewise the implicit `T: Static` containment bound (references 2a):
    // generic code may store/return/capture its params, so a param accepts
    // reference-bearing arguments only when relaxed with `where T: not
    // Static` (or when its owner is itself `not Static`).
    inject_implicit_static_bounds(ctx, entity, root, &mut result);

    result
}

/// The clauses written on `entity`, in source order, and the equalities that
/// did not resolve. See [`ExplicitWhereClauses`].
fn resolve_explicit_where_clauses(
    ctx: &QueryContext<'_>,
    entity: Entity,
    root: Entity,
) -> ExplicitWhereResolution {
    let mut out = ExplicitWhereResolution::default();
    let result = &mut out.clauses;
    if let Some(ast_wc) = ctx.get::<AstWhereClause>(entity) {
        // Pass 1: every `Bound` in this holder. An equality's assoc may be
        // reachable only through one of them (`where Item: Addable,
        // Item.Output = Item` — `Output` is `Addable`'s, and name-res follows
        // only `Item`'s *declared* bounds), so pass 2 needs them all first.
        let bounds: Vec<Option<ResolvedBound>> = ast_wc
            .0
            .iter()
            .map(|c| match c {
                WhereConstraint::Bound {
                    subject, protocols, ..
                } => resolve_bound(ctx, subject, protocols, entity, root),
                _ => None,
            })
            .collect();
        // Pass 2: emit in source order — body emitters are order-sensitive.
        for (constraint, bound) in ast_wc.0.iter().zip(&bounds) {
            match constraint {
                WhereConstraint::Bound { .. } => {
                    let Some(bound) = bound else { continue };
                    for (protocol, protocol_type_args) in &bound.protocols {
                        result.push(WhereClause::Bound {
                            subject: bound.subject.clone(),
                            protocol: *protocol,
                            protocol_type_args: protocol_type_args.clone(),
                        });
                    }
                },
                WhereConstraint::Equality { lhs, rhs, .. } => {
                    // Stage 2d: an equality RHS may itself be a ref
                    // (`where I.Item = &Int64` — generic algorithms over
                    // ref-Item iterators pin the Item this way). Nested
                    // non-aggregate refs still reject; protocol-bound type
                    // args (below) stay Strict.
                    let rhs_hir = kestrel_hir_lower::reject_ref_types_allowing_top_ref(
                        ctx,
                        kestrel_hir_lower::lower_ast_type(ctx, entity, root, rhs),
                    );
                    match resolve_equality_subject(ctx, lhs, &bounds, entity, root) {
                        Ok(subject) => result.push(WhereClause::Equality {
                            subject,
                            rhs: rhs_hir,
                        }),
                        // Reported at the clause by `GenericsAnalyzer`. No
                        // recovery clause: the body emitters re-key an
                        // equality by name (`find_assoc_type_in_bounds`), so
                        // pinning each candidate to the error type cannot
                        // reach the body's use; the follow-on E100 stays
                        // until those consumers key on the entity (G29).
                        Err(Some(dropped)) => out.dropped.push(dropped),
                        // A shape this resolver does not model — dropped
                        // unrecorded, as it always has been.
                        Err(None) => {},
                    }
                },
                WhereConstraint::NegativeBound { .. } => {
                    // Negative bounds are not modeled in inference where clauses.
                },
            }
        }
    }
    out
}

/// Push an implicit `T: Copyable` (or `Cloneable`) `WhereClause::Bound` for each
/// generic param of `entity` whose copy requirement is `RequiresCopyable` /
/// `RequiresCloneable`. Params declared `: not Copyable` (`MayBeNonCopyable`)
/// get nothing — they accept any argument. Skips params that already carry an
/// explicit Copyable/Cloneable bound to avoid duplicate constraints.
fn inject_implicit_copyable_bounds(
    ctx: &QueryContext<'_>,
    entity: Entity,
    root: Entity,
    result: &mut Vec<WhereClause>,
) {
    // Extensions don't have callers passing type args — injecting a Copyable
    // requirement on their params is meaningless (and would pollute the
    // where-clause the conditional-conformance evaluator reads back).
    if ctx.get::<NodeKind>(entity) == Some(&NodeKind::Extension) {
        return;
    }
    // Compiler intrinsics (`lang.ptr_read`, `lang.cast_ptr`, `lang.sizeof`, …)
    // operate on their type params at the ABI level — reinterpreting addresses,
    // measuring layout, moving bytes — without ever requiring the param to be
    // bit-copyable. Injecting `T: Copyable` here wrongly rejects non-Copyable
    // pointees (e.g. `Pointer[T].isNull` casting `ptr[T]` to `ptr[i8]`). Any
    // genuine copy an intrinsic performs is enforced downstream by OSSA verify.
    if ctx.get::<Intrinsic>(entity).is_some() {
        return;
    }
    // A type that opts out of Copyable (`struct X: not Copyable`) never
    // bit-copies its params, so it accepts any argument — no implicit bound.
    // Per-instantiation Copyable for such a type is granted conditionally via
    // `extend X: Copyable where T: Copyable` and evaluated in the solver.
    if ctx.query(NominalCopySemantics { entity, root }).semantics == CopySemantics::NotCopyable {
        return;
    }
    let Some(type_params) = ctx.get::<TypeParams>(entity) else {
        return;
    };
    let Some(copyable) = ctx.query(ResolveBuiltin {
        builtin: Builtin::Copyable,
        root,
    }) else {
        return;
    };
    let cloneable = ctx.query(ResolveBuiltin {
        builtin: Builtin::Cloneable,
        root,
    });

    for &param in &type_params.0 {
        let protocol = match ctx.query(TypeParamCopyRequirement {
            param,
            context: entity,
            root,
        }) {
            CopyRequirement::RequiresCopyable => copyable,
            CopyRequirement::RequiresCloneable => cloneable.unwrap_or(copyable),
            CopyRequirement::MayBeNonCopyable => continue,
        };
        // Only a bare-param bound counts as "already bound": a projection
        // subject can never be equal to `Param(param)`, so the `Eq` derive
        // preserves the pre-D7 variant split with no extra branch.
        let already_bound = result.iter().any(|wc| {
            matches!(wc,
                WhereClause::Bound { subject: WhereSubject::Param(p), protocol: pr, .. }
                if *p == param && (*pr == copyable || Some(*pr) == cloneable))
        });
        if !already_bound {
            result.push(WhereClause::Bound {
                subject: WhereSubject::Param(param),
                protocol,
                protocol_type_args: Vec::new(),
            });
        }
    }
}

/// Push an implicit `T: Static` `WhereClause::Bound` for each generic param
/// of `entity` whose requirement is `RequiresStatic`. Relaxed params
/// (`where T: not Static`, or a `not Static` owner — both folded into
/// `TypeParamStaticRequirement`) get nothing: they accept any argument.
fn inject_implicit_static_bounds(
    ctx: &QueryContext<'_>,
    entity: Entity,
    root: Entity,
    result: &mut Vec<WhereClause>,
) {
    // Same exclusions as the Copyable injection: extensions have no callers
    // passing type args, and intrinsics (`lang.ptr_read`, `lang.cast_ptr`,
    // `lang.sizeof`, …) operate at the ABI level — `lang.ptr` itself must
    // not require `T: Static` or `Pointer[T]`'s unsafe escape hatch closes.
    if ctx.get::<NodeKind>(entity) == Some(&NodeKind::Extension) {
        return;
    }
    if ctx.get::<Intrinsic>(entity).is_some() {
        return;
    }
    let Some(type_params) = ctx.get::<TypeParams>(entity) else {
        return;
    };
    let Some(static_proto) = ctx.query(ResolveBuiltin {
        builtin: Builtin::Static,
        root,
    }) else {
        return;
    };

    for &param in &type_params.0 {
        match ctx.query(TypeParamStaticRequirement {
            param,
            context: entity,
            root,
        }) {
            StaticRequirement::RequiresStatic => {},
            StaticRequirement::MayBeNonStatic => continue,
        }
        // See the Copyable injection: bare-param subjects only.
        let already_bound = result.iter().any(|wc| {
            matches!(wc,
                WhereClause::Bound { subject: WhereSubject::Param(p), protocol: pr, .. }
                if *p == param && *pr == static_proto)
        });
        if !already_bound {
            result.push(WhereClause::Bound {
                subject: WhereSubject::Param(param),
                protocol: static_proto,
                protocol_type_args: Vec::new(),
            });
        }
    }
}

/// Resolve a where-clause bound subject (`T`, `Self`, `T.Assoc`, …) to a
/// `WhereSubject`, preserving the receiver where the current resolver can see
/// it. `None` means unresolvable — the clause is dropped, as before.
///
/// A projection subject (`T.Assoc: P`) must keep its base: `resolve_type_entity`
/// resolves the whole dotted path in one shot and keeps only the last entity,
/// collapsing `T.Assoc` to `Assoc` and losing the receiver. So the projection
/// shape is tried first and only falls back to the collapsing path.
fn resolve_bound_subject(
    ctx: &QueryContext<'_>,
    ast_ty: &AstType,
    entity: Entity,
    root: Entity,
) -> Option<WhereSubject> {
    if let Some(projection) = resolve_projection_subject(ctx, ast_ty, entity, root) {
        return Some(projection);
    }
    // A bare `Self` subject is the *conformer*, resolved per-conformance — not
    // the enclosing entity. Collapsing it to `Param(<enclosing>)` is receiver
    // loss one level up, so it gets its own payload-free variant (D8 /
    // G17 stage 3a, `docs/fragility/G14-G17/decisions.md`). Readers that need a
    // concrete type for it have one — the receiver they are judging.
    if is_bare_self(ast_ty) {
        return Some(WhereSubject::SelfType);
    }
    resolve_type_entity(ctx, ast_ty, entity, root).map(WhereSubject::Param)
}

/// The subject is the single segment `Self`. Spelled against the AST rather
/// than against a `TypeResolution::SelfType` return so the answer is the same
/// whether or not `Self` happens to resolve in this scope.
fn is_bare_self(ast_ty: &AstType) -> bool {
    matches!(ast_ty, AstType::Named { segments, .. }
        if segments.len() == 1 && segments[0].name == "Self")
}

/// If `ast_ty` is a dotted path rooted at a type parameter (`T.Assoc`,
/// `C.Iter.Item`, …), return it as a nested `WhereSubject::Projection` that
/// keeps every receiver. Returns `None` for a plain type/param subject
/// (handled by `resolve_type_entity`) or any shape whose root is not a type
/// parameter — those still take the collapsing path.
fn resolve_projection_subject(
    ctx: &QueryContext<'_>,
    ast_ty: &AstType,
    entity: Entity,
    root: Entity,
) -> Option<WhereSubject> {
    let AstType::Named { segments, .. } = ast_ty else {
        return None;
    };
    if segments.len() < 2 {
        return None;
    }
    let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    let chain = resolve_type_path_chain(ctx, &seg_names, entity, root);
    if !matches!(chain.resolution, TypeResolution::Found(_)) {
        return None;
    }
    let mut steps = chain.steps.into_iter();
    let root_subject = if chain.self_rooted {
        // `Self.Item` — `steps[0]` is whatever `Self` resolved *through* (a
        // synthetic `Self` param, or the enclosing extension's target). That
        // entity is not what the clause is about: `Self` is the conformer, so
        // the root is the position, not the thing it resolved through.
        steps.next()?;
        WhereSubject::SelfType
    } else {
        let base = steps.next()?;
        // Only type-parameter roots project; a path rooted at a concrete type
        // or a module (`std.collections.Array`) is a plain type, not a
        // projection.
        if ctx.get::<NodeKind>(base) != Some(&NodeKind::TypeParameter) {
            return None;
        }
        WhereSubject::Param(base)
    };
    Some(
        steps.fold(root_subject, |base, assoc| WhereSubject::Projection {
            base: Box::new(base),
            assoc,
        }),
    )
}

/// Resolve a where-clause name in **`entity`'s own scope**. `pub(crate)` so the
/// G17 S1 `audit-scope` probe can ask "what would per-holder resolution have
/// said?" using the very function the query uses, rather than a second copy
/// that could drift from it.
pub(crate) fn resolve_type_entity(
    ctx: &QueryContext<'_>,
    ast_ty: &AstType,
    entity: Entity,
    root: Entity,
) -> Option<Entity> {
    let AstType::Named { segments, .. } = ast_ty else {
        return None;
    };
    let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    match ctx.query(ResolveTypePath {
        segments: seg_names,
        context: entity,
        root,
    }) {
        TypeResolution::Found(e) => Some(e),
        TypeResolution::SelfType => resolve_self_entity(ctx, entity, root),
        _ => None,
    }
}

/// Walk up from `start` to find the enclosing type entity that `Self` refers to.
fn resolve_self_entity(ctx: &QueryContext<'_>, start: Entity, root: Entity) -> Option<Entity> {
    let mut current = Some(start);
    while let Some(e) = current {
        match ctx.get::<NodeKind>(e) {
            Some(NodeKind::Extension) => {
                return ctx.query(kestrel_name_res::ExtensionTargetEntity { extension: e, root });
            },
            Some(NodeKind::Struct) | Some(NodeKind::Enum) | Some(NodeKind::Protocol) => {
                return Some(e);
            },
            _ => {},
        }
        current = ctx.parent_of(e);
    }
    None
}

/// `V = RHS` — resolve a bare type-param or associated-type LHS in `entity`'s
/// scope. Returns the resolved TypeParameter/TypeAlias entity.
fn resolve_type_param_or_assoc(
    ctx: &QueryContext<'_>,
    ast_ty: &AstType,
    entity: Entity,
    root: Entity,
) -> Option<Entity> {
    let AstType::Named { segments, .. } = ast_ty else {
        return None;
    };
    let all_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    match ctx.query(ResolveTypePath {
        segments: all_names,
        context: entity,
        root,
    }) {
        TypeResolution::Found(e)
            if matches!(
                ctx.get::<NodeKind>(e),
                Some(&NodeKind::TypeParameter) | Some(&NodeKind::TypeAlias)
            ) =>
        {
            Some(e)
        },
        _ => None,
    }
}

/// `T.AssocName = RHS` — extract `(param_entity, assoc_name)`. `param_entity`
/// is resolved in `entity`'s scope (must be a type param or type alias).
fn extract_associated_type_path(
    ctx: &QueryContext<'_>,
    ast_ty: &AstType,
    entity: Entity,
    root: Entity,
) -> Option<(Entity, String)> {
    let AstType::Named { segments, .. } = ast_ty else {
        return None;
    };
    if segments.len() != 2 {
        return None;
    }
    let param_name = &segments[0].name;
    let assoc_name = &segments[1].name;
    match ctx.query(ResolveTypePath {
        segments: vec![param_name.clone()],
        context: entity,
        root,
    }) {
        TypeResolution::Found(e) => Some((e, assoc_name.clone())),
        _ => None,
    }
}

/// One resolved `Bound` constraint: its subject and each protocol it names.
struct ResolvedBound {
    subject: WhereSubject,
    protocols: Vec<(Entity, Vec<HirTy>)>,
}

fn resolve_bound(
    ctx: &QueryContext<'_>,
    subject: &AstType,
    protocols: &[AstType],
    entity: Entity,
    root: Entity,
) -> Option<ResolvedBound> {
    let subject = resolve_bound_subject(ctx, subject, entity, root)?;
    let protocols = protocols
        .iter()
        .filter_map(|protocol_ty| {
            let protocol = resolve_type_entity(ctx, protocol_ty, entity, root)?;
            Some((
                protocol,
                extract_protocol_type_args(ctx, entity, root, protocol_ty),
            ))
        })
        .collect();
    Some(ResolvedBound { subject, protocols })
}

/// An equality's left side as a subject. `T.Item = X` is
/// `Projection { Param(T), Item }` with `Item` resolved to its **entity**;
/// `V = X` is `Param(V)`.
///
/// `Err` drops the clause. `Err(Some(_))` is a `T.Seg` whose `Seg` names no
/// single associated type — recorded so it is reported at the clause (G29).
/// `Err(None)` is any other unresolvable shape, dropped unrecorded.
fn resolve_equality_subject(
    ctx: &QueryContext<'_>,
    lhs: &AstType,
    bounds: &[Option<ResolvedBound>],
    entity: Entity,
    root: Entity,
) -> Result<WhereSubject, Option<DroppedEquality>> {
    let Some((param, assoc_name)) = extract_associated_type_path(ctx, lhs, entity, root) else {
        return resolve_type_param_or_assoc(ctx, lhs, entity, root)
            .map(WhereSubject::Param)
            .ok_or(None);
    };
    let assoc = match resolve_assoc_by_path(ctx, lhs, param, entity, root) {
        Some(assoc) => assoc,
        None => resolve_assoc_via_holder_bounds(ctx, param, &assoc_name, bounds, root).map_err(
            |candidates| {
                let AstType::Named { span, .. } = lhs else {
                    return None;
                };
                Some(DroppedEquality {
                    lhs_span: span.clone(),
                    param,
                    segment: assoc_name.clone(),
                    candidates,
                })
            },
        )?,
    };
    Ok(WhereSubject::Projection {
        base: Box::new(WhereSubject::Param(param)),
        assoc,
    })
}

/// The assoc entity by the same segment walk the bound-subject path uses
/// (`resolve_type_path_chain`), in `entity`'s scope. The root may be a type
/// parameter or an associated-type alias (`TargetIterator.Item`), which
/// `Param` already denotes for bare-assoc subjects.
fn resolve_assoc_by_path(
    ctx: &QueryContext<'_>,
    ast_ty: &AstType,
    param: Entity,
    entity: Entity,
    root: Entity,
) -> Option<Entity> {
    let AstType::Named { segments, .. } = ast_ty else {
        return None;
    };
    let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    let chain = resolve_type_path_chain(ctx, &seg_names, entity, root);
    match chain.steps.as_slice() {
        [base, assoc] if *base == param => Some(*assoc),
        _ => None,
    }
}

/// Pass-2 fallback: `assoc_name` among the associated types of the protocols
/// **this holder's** own bounds place on `param` (refinement parents included,
/// via `ProtocolAssociatedTypes`). Scoped to the holder, like S2: another
/// holder's bounds never widen what a name means here.
///
/// Only the declaring protocol's requirement counts (parent is a `Protocol`),
/// so an extension default of the same name is not a second candidate. Two or
/// more distinct requirements is a genuine ambiguity. `Err` carries the
/// candidates (empty when none declares it); the clause is dropped and
/// reported at the clause by `GenericsAnalyzer`.
fn resolve_assoc_via_holder_bounds(
    ctx: &QueryContext<'_>,
    param: Entity,
    assoc_name: &str,
    bounds: &[Option<ResolvedBound>],
    root: Entity,
) -> Result<Entity, Vec<Entity>> {
    let subject = WhereSubject::Param(param);
    let mut found: Vec<Entity> = Vec::new();
    for bound in bounds.iter().flatten().filter(|b| b.subject == subject) {
        for &(protocol, _) in &bound.protocols {
            let members = ctx.query(kestrel_name_res::ProtocolAssociatedTypes { protocol, root });
            for m in members {
                let named = ctx
                    .get::<kestrel_ast_builder::Name>(m.entity)
                    .is_some_and(|n| n.0 == assoc_name);
                let is_requirement = ctx
                    .parent_of(m.entity)
                    .is_some_and(|p| ctx.get::<NodeKind>(p) == Some(&NodeKind::Protocol));
                if named && is_requirement && !found.contains(&m.entity) {
                    found.push(m.entity);
                }
            }
        }
    }
    match found.as_slice() {
        [one] => Ok(*one),
        [] => {
            kestrel_debug::ktrace!(
                "where-eq",
                "UNRESOLVED param={param:?} assoc={assoc_name}: no bound in this holder declares it"
            );
            Err(found)
        },
        many => {
            kestrel_debug::ktrace!(
                "where-eq",
                "AMBIGUOUS param={param:?} assoc={assoc_name} candidates={many:?}: clause dropped"
            );
            Err(found)
        },
    }
}

fn extract_protocol_type_args(
    ctx: &QueryContext<'_>,
    entity: Entity,
    root: Entity,
    protocol_ty: &AstType,
) -> Vec<HirTy> {
    match protocol_ty {
        AstType::Named { segments, .. } => segments
            .last()
            .map(|seg| {
                seg.type_args
                    .iter()
                    .map(|a| {
                        // Protocol bound type args are Strict ref territory
                        // (pre-2b this site had no reject walk).
                        kestrel_hir_lower::reject_ref_types(
                            ctx,
                            kestrel_hir_lower::lower_ast_type(ctx, entity, root, a),
                            kestrel_hir_lower::RefPosition::GenericArg,
                            kestrel_hir_lower::RefPolicy::Strict,
                        )
                    })
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}
