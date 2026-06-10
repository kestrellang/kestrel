//! Analyze-side staticness: the `ResolvedTy` mirror of the structural
//! contains-ref predicate (references 2a). Routes nominal instantiations
//! through `kestrel_semantics::instance_is_static` — the single source of
//! truth shared with the semantics (HirTy) and solver (TyVar) layers.

use kestrel_hecs::{Entity, QueryContext};
use kestrel_semantics::{
    NominalStaticness, StaticLayer, StaticRequirement, Staticness, TypeParamStaticRequirement,
    instance_is_static,
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

    fn nominal_staticness(&self, entity: Entity) -> Staticness {
        self.query
            .query(NominalStaticness {
                entity,
                root: self.root,
            })
            .staticness
    }

    fn member_is_static(&self, ty: &ResolvedTy) -> bool {
        match ty {
            // The axiom: a reference is never Static.
            ResolvedTy::Ref { .. } => false,
            ResolvedTy::Named { entity, args } => instance_is_static(self, *entity, args),
            // Base-only treatment for Self (mirrors the other layers).
            ResolvedTy::SelfType { entity } => {
                !matches!(self.nominal_staticness(*entity), Staticness::NotStatic)
            },
            ResolvedTy::Param { entity } => {
                self.query.query(TypeParamStaticRequirement {
                    param: *entity,
                    context: self.context,
                    root: self.root,
                }) == StaticRequirement::RequiresStatic
            },
            ResolvedTy::Tuple(elems) => elems.iter().all(|e| self.member_is_static(e)),
            // Function types are Static in 2a (TODO(static-2c): capture-
            // derived bit); assoc projections / opaque / never / error are
            // the conservative never-block leaves, mirroring the HIR layer.
            ResolvedTy::Function { .. }
            | ResolvedTy::AssocProjection { .. }
            | ResolvedTy::Opaque { .. }
            | ResolvedTy::Never
            | ResolvedTy::Error => true,
        }
    }
}

/// Is this resolved type provably Static? `context` is the body owner.
pub fn resolved_ty_is_static(
    query: &QueryContext<'_>,
    ty: &ResolvedTy,
    context: Entity,
    root: Entity,
) -> bool {
    ResolvedStaticLayer {
        query,
        context,
        root,
    }
    .member_is_static(ty)
}
