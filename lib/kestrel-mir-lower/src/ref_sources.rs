//! MIR-type view for `kestrel_semantics::ref_source_mask`: which call
//! arguments a ref-carrying result may borrow from (the E494 root join).

use kestrel_mir::{MirModule, MirTy, SubstMap, TyId, substitute};
use kestrel_semantics::{RefShape, RefSourceLayer};

/// Interns substituted field types as it walks, hence `&mut`.
pub(crate) struct MirRefSources<'a>(pub &'a mut MirModule);

impl RefSourceLayer for MirRefSources<'_> {
    type Ty = TyId;

    fn shape(&mut self, ty: &TyId) -> RefShape<TyId> {
        match self.0.ty_arena.get(*ty).clone() {
            MirTy::Ref { pointee, .. } => RefShape::Ref(pointee),
            MirTy::Pointer(p) => RefShape::Pointer(p),
            MirTy::Tuple(elems) => RefShape::Tuple(elems),
            MirTy::Named { entity, type_args } => RefShape::Nominal {
                entity,
                fields: self.fields(entity, &type_args),
                args: type_args,
            },
            MirTy::TypeParam(_)
            | MirTy::AssociatedProjection { .. }
            | MirTy::RawExistential { .. }
            | MirTy::FuncThick { .. }
            | MirTy::Error => RefShape::Opaque,
            _ => RefShape::Leaf,
        }
    }

    fn same(&self, a: &TyId, b: &TyId) -> bool {
        a == b
    }
}

impl MirRefSources<'_> {
    /// Stored field / payload types of a nominal instance, type args
    /// substituted. `None` when the module has no definition for it.
    fn fields(&mut self, entity: kestrel_hecs::Entity, type_args: &[TyId]) -> Option<Vec<TyId>> {
        let module = &mut *self.0;
        let (params, tys): (Vec<_>, Vec<TyId>) = if let Some(def) = module.structs.get(&entity) {
            (
                def.type_params.iter().map(|p| p.entity).collect(),
                def.fields.iter().map(|f| f.ty).collect(),
            )
        } else if let Some(def) = module.enums.get(&entity) {
            (
                def.type_params.iter().map(|p| p.entity).collect(),
                def.cases
                    .iter()
                    .flat_map(|c| c.payload_fields.iter().map(|f| f.ty))
                    .collect(),
            )
        } else {
            return None;
        };
        let mut subst = SubstMap::new();
        subst.type_params = params.into_iter().zip(type_args.iter().copied()).collect();
        Some(
            tys.into_iter()
                .map(|t| substitute(&mut module.ty_arena, t, &subst))
                .collect(),
        )
    }
}
