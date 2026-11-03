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

use std::borrow::Cow;
use std::cell::RefCell;

use kestrel_ast_builder::{TypeParams, WhereClause as AstWhereClause, WhereConstraint};
use kestrel_hecs::guard::OnDrop;
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir::{Builtin, HirTy};
use kestrel_name_res::ResolveBuiltin;

use crate::{
    ExplicitlyNegatesProtocol, collect_child_types, param_bound_contexts, resolve_type_entity,
};

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

/// One type form's view of a type, as far as staticness cares. Each layer
/// (HIR, solver `TyVar`, analyze `ResolvedTy`) only maps its own type onto
/// this; what each shape *means* is decided once, in [`ty_is_static`].
pub enum StaticShape<'t, T: Clone> {
    /// A reference: never Static (the axiom).
    Ref,
    /// A nominal instantiation: the per-instance rule.
    Nominal { entity: Entity, args: Cow<'t, [T]> },
    /// `Self` / an abstract associated type, judged by its base alone: a
    /// conditional base counts (Self inside its own definition is covered by
    /// the `NominalStaticness` cycle guard).
    BaseOnly(Entity),
    /// A type parameter: Static unless its bound was relaxed.
    Param(Entity),
    /// Static iff every element is.
    Tuple(Cow<'t, [T]>),
    /// Exactly the type it stands for (a bounded alias's expansion).
    Alias(&'t T),
    /// A raw existential with these protocol bounds.
    RawExistential(Vec<Entity>),
    /// A closure: a view-kind closure holds views into its defining frame,
    /// so it is not Static, exactly like a ref; owning kinds snapshot their
    /// captures (references 2c, docs/design/closure-fields.md).
    Function { is_view: bool },
    /// Conservative never-block leaves (protocols, opaques, projections,
    /// never, errors, unresolved inference variables).
    Leaf,
}

/// Layer hooks for the staticness rule. Implementors: the HIR walker below,
/// the solver's TyVar layer, the analyzer's ResolvedTy layer.
pub trait StaticLayer {
    type Ty: Clone;
    fn query(&self) -> &QueryContext<'_>;
    fn root(&self) -> Entity;
    /// The nominal's classification (HIR layer: `NominalStaticness` through
    /// the cycle guard).
    fn nominal_staticness(&self, entity: Entity) -> Staticness;
    /// This layer's type, seen as a [`StaticShape`].
    fn shape<'t>(&self, ty: &'t Self::Ty) -> StaticShape<'t, Self::Ty>;
    /// Does this type parameter carry the (implicit) Static bound? Layers
    /// differ in the lookup context and in dependency tracking.
    fn param_is_static(&self, param: Entity) -> bool;
    /// Is a component (type arg, tuple element, alias expansion) Static?
    /// Layers with recursion bookkeeping (the solver's depth) override this.
    fn component_is_static(&self, ty: &Self::Ty) -> bool
    where
        Self: Sized,
    {
        ty_is_static(self, ty)
    }
}

/// THE staticness rule over any layer's types: what each [`StaticShape`]
/// means is written here once, for every layer.
pub fn ty_is_static<L: StaticLayer>(layer: &L, ty: &L::Ty) -> bool {
    match layer.shape(ty) {
        StaticShape::Ref => false,
        StaticShape::Nominal { entity, args } => instance_is_static(layer, entity, &args),
        StaticShape::BaseOnly(entity) => {
            !matches!(layer.nominal_staticness(entity), Staticness::NotStatic)
        },
        StaticShape::Param(param) => layer.param_is_static(param),
        StaticShape::Tuple(elems) => elems.iter().all(|e| layer.component_is_static(e)),
        StaticShape::Alias(expanded) => layer.component_is_static(expanded),
        StaticShape::RawExistential(bounds) => {
            crate::raw_existential_is_static(layer.query(), bounds, layer.root())
        },
        StaticShape::Function { is_view } => !is_view,
        StaticShape::Leaf => true,
    }
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
        Staticness::ConditionalOn(positions) => positions.iter().all(|&i| {
            args.get(i)
                .is_some_and(|arg| layer.component_is_static(arg))
        }),
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

        // The shared context-chain walk (copy / static / size requirements).
        for context in param_bound_contexts(ctx, self.param, self.context) {
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
        // Unwind-safe pop — same reasoning as COMPUTING_COPY_SEMANTICS in
        // lib.rs: a leaked entry would make this nominal answer `Static`
        // for the rest of the thread's life (F22).
        let _guard = OnDrop::new(|| {
            COMPUTING_STATICNESS.with(|stack| {
                stack.borrow_mut().pop();
            });
        });
        nominal_staticness_impl(ctx, self.entity, self.root)
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
        if !ty_is_static(&walker, &ty) {
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

    fn query(&self) -> &QueryContext<'_> {
        self.ctx
    }

    fn root(&self) -> Entity {
        self.root
    }

    fn nominal_staticness(&self, entity: Entity) -> Staticness {
        query_nominal_staticness(self.ctx, entity, self.root)
    }

    fn shape<'t>(&self, ty: &'t HirTy) -> StaticShape<'t, HirTy> {
        match ty {
            HirTy::Ref { .. } => StaticShape::Ref,
            HirTy::Struct { entity, args, .. } | HirTy::Enum { entity, args, .. } => {
                StaticShape::Nominal {
                    entity: *entity,
                    args: Cow::Borrowed(args),
                }
            },
            HirTy::Tuple(elems, _) => StaticShape::Tuple(Cow::Borrowed(elems)),
            HirTy::Param(entity, _) => StaticShape::Param(*entity),
            HirTy::SelfType(entity, _) | HirTy::AbstractAssoc { entity, .. } => {
                StaticShape::BaseOnly(*entity)
            },
            HirTy::Alias { expanded, .. } => StaticShape::Alias(expanded),
            HirTy::Function { kind, .. } => StaticShape::Function {
                is_view: kind.is_view(),
            },
            HirTy::RawExistential { bounds, .. } => StaticShape::RawExistential(
                bounds
                    .iter()
                    .filter_map(|b| match b {
                        HirTy::Protocol { entity, .. } => Some(*entity),
                        _ => None,
                    })
                    .collect(),
            ),
            HirTy::Protocol { .. }
            | HirTy::Opaque { .. }
            | HirTy::AssocProjection { .. }
            | HirTy::Never(_)
            | HirTy::Infer(_)
            | HirTy::Error(_) => StaticShape::Leaf,
        }
    }

    fn param_is_static(&self, param: Entity) -> bool {
        let requirement = self.ctx.query(TypeParamStaticRequirement {
            param,
            context: self.context,
            root: self.root,
        });
        if requirement == StaticRequirement::RequiresStatic {
            return true;
        }
        // A relaxed param is unprovable, except in nominal-classification
        // mode, where the owner's own relaxed param becomes a gating
        // position. One from an enclosing scope can't gate THIS nominal's
        // instantiations ⇒ unprovable.
        let Some(pos) = self
            .owner_params
            .as_ref()
            .and_then(|params| params.iter().position(|p| *p == param))
        else {
            return false;
        };
        self.deps.borrow_mut().push(pos);
        true
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
    let walker = HirStaticWalk {
        ctx,
        context,
        root,
        owner_params: None,
        deps: RefCell::new(Vec::new()),
    };
    ty_is_static(&walker, ty)
}
