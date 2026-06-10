//! Staticness: the structural "transitively contains no reference" predicate
//! (references stage 2a).
//!
//! A type is **Static** iff no reference (`&T`) is reachable through its
//! stored fields / enum payloads / tuple elements / type arguments. Static is
//! a FRONT-END containment bound only — it never changes MIR lowering,
//! drop/copy expansion, or codegen — so unlike copy semantics (5 layers via
//! `kestrel-copy-fold`) it needs just the HIR layer here plus thin TyVar /
//! ResolvedTy mirrors in the solver and analyzer.
//!
//! DELIBERATE divergence from `kestrel_copy_fold::instance_semantics`: the
//! staticness fold is a 2-state AND whose gating positions are *structural*
//! (which type params appear in stored positions — computed below, not
//! declared via `extend … where`) and are legal on ANY non-`not Static`
//! base. Do not "converge" this into the copy kernel: copy's invariant is
//! "gating positions non-empty only when the base is NotCopyable", and its
//! tri-state Cloneable arm has no Static analog.
//!
//! Single source of truth: `instance_is_static` is THE per-instantiation
//! rule; every layer (HIR here, solver TyVar, analyze ResolvedTy) routes
//! nominal instantiations through it via a `StaticLayer` impl.

use std::cell::RefCell;
use std::collections::HashSet;

use kestrel_ast_builder::{TypeParams, WhereClause as AstWhereClause, WhereConstraint};
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir::{Builtin, HirTy};
use kestrel_name_res::ResolveBuiltin;

use crate::{ExplicitlyNegatesProtocol, collect_child_types, resolve_type_entity};

// ===== Vocabulary =====

/// Per-nominal classification: unconditionally Static, unconditionally not,
/// or Static iff the type args at the given positions are Static.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Staticness {
    Static,
    NotStatic,
    /// Positions are indices into the nominal's type-param list, sorted,
    /// deduped. Unlike `ConditionalCopyableParams`, a conditional base is
    /// the NORMAL state for any generic container that stores its param.
    ConditionalOn(Vec<usize>),
}

/// Bound state of a type param: every param requires Static by default;
/// `where T: not Static` (or a `not Static` owner) relaxes it — the param
/// then ACCEPTS both Static and non-Static arguments (need-not, not
/// must-not — the `T: not Copyable` convention).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StaticRequirement {
    RequiresStatic,
    MayBeNonStatic,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StaticnessReason {
    Default,
    DeclaredNotStatic,
    /// The stored child (field or enum-case payload) whose type is
    /// non-Static. Drives the "field 'x' is non-Static" diagnostic detail.
    NonStaticChild(Entity),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StaticnessInfo {
    pub staticness: Staticness,
    pub reason: StaticnessReason,
}

// ===== The shared instance rule =====

/// Layer hooks for `instance_is_static`. Implementors: the HIR walker below,
/// the solver's TyVar layer, the analyzer's ResolvedTy layer.
pub trait StaticLayer {
    type Ty;
    /// The nominal's classification (HIR layer: `NominalStaticness` through
    /// the cycle guard).
    fn nominal_staticness(&self, entity: Entity) -> Staticness;
    /// Is this member type Static in the layer's vocabulary?
    fn member_is_static(&self, ty: &Self::Ty) -> bool;
}

/// THE per-instantiation staticness rule, single source of truth for every
/// layer: a Static base wins; a NotStatic base loses; a conditional base
/// ANDs the args at the gating positions. A missing arg at a gating
/// position is unprovable ⇒ false. Recursing ONLY into gating args is
/// load-bearing: `Pointer[T]` / `lang.ptr[T]` store no `T` (raw address
/// payload), so they classify Static for any `T` — the unsafe escape hatch.
pub fn instance_is_static<L: StaticLayer>(layer: &L, entity: Entity, args: &[L::Ty]) -> bool {
    match layer.nominal_staticness(entity) {
        Staticness::Static => true,
        Staticness::NotStatic => false,
        Staticness::ConditionalOn(positions) => positions
            .iter()
            .all(|&i| args.get(i).is_some_and(|arg| layer.member_is_static(arg))),
    }
}

// ===== Type-param requirement =====

/// Does `param` carry the (implicit) `Static` bound, or was it relaxed?
/// Relaxation routes: `where param: not Static` anywhere in the context
/// chain, or the param's owning nominal itself declares `: not Static`
/// (a reference-bearing type's params are relaxed wholesale — this also
/// makes the implicit-bound injection skip fall out without a second
/// owner check at the injection site).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TypeParamStaticRequirement {
    pub param: Entity,
    pub context: Entity,
    pub root: Entity,
}

impl QueryFn for TypeParamStaticRequirement {
    type Output = StaticRequirement;

    fn execute(&self, ctx: &QueryContext<'_>) -> StaticRequirement {
        // No builtin registered (stdlib-less fixture) ⇒ nothing can spell a
        // relaxation and nothing emits Static constraints — default holds.
        let Some(static_proto) = ctx.query(ResolveBuiltin {
            builtin: Builtin::Static,
            root: self.root,
        }) else {
            return StaticRequirement::RequiresStatic;
        };

        let owner = ctx.parent_of(self.param);
        if let Some(owner) = owner
            && ctx.query(ExplicitlyNegatesProtocol {
                entity: owner,
                protocol: static_proto,
                root: self.root,
            })
        {
            return StaticRequirement::MayBeNonStatic;
        }

        // Same context-chain walk as TypeParamCopyRequirement: the param's
        // parent plus the ancestry of the asking context (covers methods
        // consulting their container's clauses).
        let mut checked = HashSet::new();
        let mut contexts = Vec::new();
        if let Some(owner) = owner {
            contexts.push(owner);
        }
        let mut current = Some(self.context);
        while let Some(entity) = current {
            contexts.push(entity);
            current = ctx.parent_of(entity);
        }

        for context in contexts {
            if !checked.insert(context) {
                continue;
            }
            let Some(wc) = ctx.get::<AstWhereClause>(context) else {
                continue;
            };
            for constraint in &wc.0 {
                let WhereConstraint::NegativeBound {
                    subject, protocol, ..
                } = constraint
                else {
                    continue;
                };
                if resolve_type_entity(ctx, subject, context, self.root) == Some(self.param)
                    && resolve_type_entity(ctx, protocol, context, self.root) == Some(static_proto)
                {
                    return StaticRequirement::MayBeNonStatic;
                }
            }
        }

        StaticRequirement::RequiresStatic
    }

    fn describe(&self) -> String {
        format!("TypeParamStaticRequirement({:?})", self.param)
    }
}

// ===== Nominal classification =====

/// Classify a struct/enum: declared `: not Static` ⇒ NotStatic; else fold
/// the stored child types, recording which of the nominal's own (relaxed)
/// type params gate the answer.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NominalStaticness {
    pub entity: Entity,
    pub root: Entity,
}

// WARNING: side-channel state invisible to the query framework's dependency
// tracker — memoized results that consulted it are not invalidated when it
// changes. It exists only because the framework panics on re-entrant queries
// (recursive types). Same caveat as COMPUTING_COPY_SEMANTICS in lib.rs.
thread_local! {
    static COMPUTING_STATICNESS: RefCell<Vec<(Entity, Entity)>> = const { RefCell::new(Vec::new()) };
}

impl QueryFn for NominalStaticness {
    type Output = StaticnessInfo;

    fn execute(&self, ctx: &QueryContext<'_>) -> StaticnessInfo {
        let key = (self.entity, self.root);
        COMPUTING_STATICNESS.with(|stack| stack.borrow_mut().push(key));
        let result = nominal_staticness_impl(ctx, self.entity, self.root);
        COMPUTING_STATICNESS.with(|stack| {
            stack.borrow_mut().retain(|entry| *entry != key);
        });
        result
    }

    fn describe(&self) -> String {
        format!("NominalStaticness({:?})", self.entity)
    }
}

fn computing_contains(entity: Entity, root: Entity) -> bool {
    COMPUTING_STATICNESS.with(|stack| stack.borrow().contains(&(entity, root)))
}

/// Cycle-guarded nominal lookup: a self-reference (direct or transitive,
/// e.g. an indirect enum) falls back to Static — a cycle cannot introduce a
/// reference on its own; any actual ref is found on its own branch.
fn query_nominal_staticness(ctx: &QueryContext<'_>, entity: Entity, root: Entity) -> Staticness {
    if computing_contains(entity, root) {
        Staticness::Static
    } else {
        ctx.query(NominalStaticness { entity, root }).staticness
    }
}

fn nominal_staticness_impl(ctx: &QueryContext<'_>, entity: Entity, root: Entity) -> StaticnessInfo {
    let static_proto = ctx.query(ResolveBuiltin {
        builtin: Builtin::Static,
        root,
    });

    // Declared `: not Static` ⇒ reference-bearing by fiat. No string-match
    // fallback for unresolved builtins (unlike Copyable's legacy carve):
    // stdlib-less fixtures must inline `@builtin(.Static) protocol Static {}`.
    if let Some(static_proto) = static_proto
        && ctx.query(ExplicitlyNegatesProtocol {
            entity,
            protocol: static_proto,
            root,
        })
    {
        return StaticnessInfo {
            staticness: Staticness::NotStatic,
            reason: StaticnessReason::DeclaredNotStatic,
        };
    }

    let own_params: Vec<Entity> = ctx
        .get::<TypeParams>(entity)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();

    let walker = HirStaticWalk {
        ctx,
        context: entity,
        root,
        owner_params: Some(own_params),
        deps: RefCell::new(Vec::new()),
    };

    for (child, ty) in collect_child_types(ctx, entity, root) {
        if !walker.member_is_static(&ty) {
            return StaticnessInfo {
                staticness: Staticness::NotStatic,
                reason: StaticnessReason::NonStaticChild(child),
            };
        }
    }

    let mut deps = walker.deps.into_inner();
    if deps.is_empty() {
        return StaticnessInfo {
            staticness: Staticness::Static,
            reason: StaticnessReason::Default,
        };
    }
    deps.sort_unstable();
    deps.dedup();
    StaticnessInfo {
        staticness: Staticness::ConditionalOn(deps),
        reason: StaticnessReason::Default,
    }
}

// ===== The HIR walker =====

/// `StaticLayer` over `HirTy`. Two modes:
/// - boolean face (`owner_params: None`): a relaxed param is unprovable ⇒
///   false (this is what makes a `not Static`-relaxed `T` fail Static
///   bounds inside the generic body — the containment doing its job);
/// - nominal-classification mode (`owner_params: Some`): a relaxed param of
///   the owner is recorded as a gating position and treated as
///   conditionally-true.
struct HirStaticWalk<'a, 'q> {
    ctx: &'a QueryContext<'q>,
    context: Entity,
    root: Entity,
    owner_params: Option<Vec<Entity>>,
    deps: RefCell<Vec<usize>>,
}

impl StaticLayer for HirStaticWalk<'_, '_> {
    type Ty = HirTy;

    fn nominal_staticness(&self, entity: Entity) -> Staticness {
        query_nominal_staticness(self.ctx, entity, self.root)
    }

    fn member_is_static(&self, ty: &HirTy) -> bool {
        match ty {
            // The axiom. Unreachable from stored positions in 2a (E483/E484/
            // E485 reject refs there first) — 2b's ref payloads land here.
            HirTy::Ref { .. } => false,
            HirTy::Struct { entity, args, .. } | HirTy::Enum { entity, args, .. } => {
                instance_is_static(self, *entity, args)
            },
            HirTy::Tuple(elems, _) => elems.iter().all(|e| self.member_is_static(e)),
            HirTy::Param(entity, _) => {
                match self.ctx.query(TypeParamStaticRequirement {
                    param: *entity,
                    context: self.context,
                    root: self.root,
                }) {
                    StaticRequirement::RequiresStatic => true,
                    StaticRequirement::MayBeNonStatic => {
                        let Some(params) = &self.owner_params else {
                            return false;
                        };
                        match params.iter().position(|p| p == entity) {
                            Some(pos) => {
                                self.deps.borrow_mut().push(pos);
                                true
                            },
                            // A relaxed param from an enclosing scope can't
                            // gate THIS nominal's instantiations ⇒ unprovable.
                            None => false,
                        }
                    },
                }
            },
            // Base-only treatment for Self/alias uses (mirrors the copy
            // layer): a conditional base is treated permissively — Self
            // inside its own definition is covered by the cycle guard.
            HirTy::SelfType(entity, _) | HirTy::AliasUse { entity, .. } => {
                !matches!(self.nominal_staticness(*entity), Staticness::NotStatic)
            },
            // Function types are Static in 2a — the capture-derived Static
            // bit on function types lands with ref-capturing closures.
            // TODO(static-2c): thread the fn-type Static bit here.
            HirTy::Function { .. } => true,
            // Conservative never-block leaves, mirroring the copy layer:
            // refs cannot reach these positions in 2a (revisit in 2b).
            HirTy::Protocol { .. }
            | HirTy::Opaque { .. }
            | HirTy::AssocProjection { .. }
            | HirTy::Never(_)
            | HirTy::Infer(_)
            | HirTy::Error(_) => true,
        }
    }
}

/// The boolean face: is this HIR type provably Static? `context` scopes
/// type-param bound lookups (the entity whose where-clauses govern params
/// mentioned by `ty`).
pub fn hir_type_is_static(
    ctx: &QueryContext<'_>,
    ty: &HirTy,
    context: Entity,
    root: Entity,
) -> bool {
    HirStaticWalk {
        ctx,
        context,
        root,
        owner_params: None,
        deps: RefCell::new(Vec::new()),
    }
    .member_is_static(ty)
}
