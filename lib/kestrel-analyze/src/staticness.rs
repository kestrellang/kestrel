//! Analyze-side staticness: the `ResolvedTy` mirror of the structural
//! contains-ref predicate (references 2a). Routes nominal instantiations
//! through `kestrel_semantics::instance_is_static` — the single source of
//! truth shared with the semantics (HirTy) and solver (TyVar) layers.

use kestrel_hecs::{Entity, QueryContext};
use std::borrow::Cow;

use kestrel_semantics::{
    NominalStaticness, StaticLayer, StaticRequirement, StaticShape, Staticness,
    TypeParamStaticRequirement, ty_is_static,
};
use kestrel_type_infer::result::ResolvedTy;

struct ResolvedStaticLayer<'a, 'q> {
    query: &'a QueryContext<'q>,
    /// Body owner — scopes type-param bound lookups (the entity whose
    /// where-clauses govern params mentioned by the type).
    context: Entity,
    root: Entity,
}

impl StaticLayer for ResolvedStaticLayer<'_, '_> {
    type Ty = ResolvedTy;

    fn raw_existential_is_static(&self, bounds: &[Entity]) -> bool {
        kestrel_semantics::raw_existential_is_static(self.query, bounds.iter().copied(), self.root)
    }

    fn nominal_staticness(&self, entity: Entity) -> Staticness {
        self.query
            .query(NominalStaticness {
                entity,
                root: self.root,
            })
            .staticness
    }

    fn shape<'t>(&self, ty: &'t ResolvedTy) -> StaticShape<'t, ResolvedTy> {
        match ty {
            ResolvedTy::Ref { .. } => StaticShape::Ref,
            ResolvedTy::Named { entity, args } => StaticShape::Nominal {
                entity: *entity,
                args: Cow::Borrowed(args),
            },
            ResolvedTy::SelfType { entity } => StaticShape::BaseOnly(*entity),
            ResolvedTy::Param { entity } => StaticShape::Param(*entity),
            ResolvedTy::Tuple(elems) => StaticShape::Tuple(Cow::Borrowed(elems)),
            ResolvedTy::RawExistential { bounds, .. } => {
                StaticShape::RawExistential(bounds.iter().map(|(protocol, _)| *protocol).collect())
            },
            ResolvedTy::Function { kind, .. } => StaticShape::Function {
                is_view: kind.is_view(),
            },
            ResolvedTy::AssocProjection { .. }
            | ResolvedTy::Opaque { .. }
            | ResolvedTy::Never
            | ResolvedTy::Error => StaticShape::Leaf,
        }
    }

    fn param_is_static(&self, param: Entity) -> bool {
        self.query.query(TypeParamStaticRequirement {
            param,
            context: self.context,
            root: self.root,
        }) == StaticRequirement::RequiresStatic
    }
}

/// Is this resolved type provably Static? `context` is the body owner.
pub fn resolved_ty_is_static(
    query: &QueryContext<'_>,
    ty: &ResolvedTy,
    context: Entity,
    root: Entity,
) -> bool {
    let layer = ResolvedStaticLayer {
        query,
        context,
        root,
    };
    ty_is_static(&layer, ty)
}
