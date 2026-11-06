//! Staticness: the structural "transitively contains no reference" predicate
//! (references stage 2a).
//!
//! A type is **Static** iff no reference (`&T`) is reachable through its
//! stored fields / enum payloads / tuple elements / type arguments. Static is
//! a containment bound only — it never changes MIR lowering, drop/copy
//! expansion, or codegen.
//!
//! The shape and the rule (`StaticShape`, `ty_is_static`) live in the leaf
//! `kestrel-structural` crate, shared with mono, which judges a witness's
//! `where T: Static` clause on concrete types. This module holds the
//! front-end parts: the per-nominal classification (`NominalStaticness`,
//! carried into MIR on `StructDef` / `EnumDef`), the type-param requirement,
//! and the HIR layer. The solver and analyzer map their own types onto the
//! same shape.

use std::borrow::Cow;
use std::cell::RefCell;

use kestrel_ast_builder::{TypeParams, WhereClause as AstWhereClause, WhereConstraint};
use kestrel_hecs::{CycleMode, Entity, QueryContext, QueryFn};
use kestrel_hir::{Builtin, HirTy};
use kestrel_name_res::ResolveBuiltin;

use crate::{
    ExplicitlyNegatesProtocol, collect_child_types, param_bound_contexts, resolve_type_entity,
};

// ===== Vocabulary =====

// The shape and rule live in the leaf `kestrel-structural` crate, shared
// with mono.
pub use kestrel_structural::{
    StaticLayer, StaticShape, Staticness, instance_is_static, ty_is_static,
};

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

impl QueryFn for NominalStaticness {
    type Output = StaticnessInfo;

    fn execute(&self, ctx: &QueryContext<'_>) -> StaticnessInfo {
        nominal_staticness_impl(ctx, self.entity, self.root)
    }

    /// Recursive types ask for their own staticness. Start at Static — a
    /// cycle cannot introduce a reference on its own; any actual ref is
    /// found on its own branch — and descend Static → ConditionalOn
    /// (positions only grow) → NotStatic to the fixpoint.
    fn cycle_mode(&self) -> CycleMode<StaticnessInfo> {
        CycleMode::Fixpoint {
            initial: StaticnessInfo {
                staticness: Staticness::Static,
                reason: StaticnessReason::Default,
            },
        }
    }

    fn describe(&self) -> String {
        format!("NominalStaticness({:?})", self.entity)
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

    fn raw_existential_is_static(&self, bounds: &[Entity]) -> bool {
        crate::raw_existential_is_static(self.ctx, bounds.iter().copied(), self.root)
    }

    fn nominal_staticness(&self, entity: Entity) -> Staticness {
        self.ctx
            .query(NominalStaticness {
                entity,
                root: self.root,
            })
            .staticness
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
