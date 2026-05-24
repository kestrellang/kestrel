use crate::body::LocalDef;
use crate::item::enum_def::EnumDef;
use crate::item::static_def::StaticDef;
use crate::item::struct_def::StructDef;
use crate::place::{Place, PlaceBase, PlaceElem};
use crate::substitute::{substitute, SubstMap};
use crate::ty::{MirTy, TyArena};
use crate::{FieldIdx, TyId, VariantIdx};

/// Resolve the type of a MIR place after applying all projections.
///
/// This is intentionally layout-independent so it can be used by generic MIR
/// passes and by monomorphization before a MonoModule exists.
pub fn place_type(
    arena: &mut TyArena,
    structs: &[StructDef],
    enums: &[EnumDef],
    statics: &[StaticDef],
    locals: &[LocalDef],
    place: &Place,
) -> Option<TyId> {
    let mut ty = match &place.base {
        PlaceBase::Local(local) => locals.get(local.index()).map(|l| l.ty)?,
        PlaceBase::Global(entity) => statics.iter().find(|s| s.entity == *entity).map(|s| s.ty)?,
    };

    let mut current_variant: Option<VariantIdx> = None;
    for projection in &place.projections {
        match *projection {
            PlaceElem::Field(field) => {
                ty = field_type(arena, structs, enums, ty, field, current_variant)?;
                current_variant = None;
            }
            PlaceElem::TupleIndex(index) => {
                ty = tuple_or_variant_field_type(
                    arena,
                    structs,
                    enums,
                    ty,
                    index as usize,
                    current_variant,
                )?;
                current_variant = None;
            }
            PlaceElem::Downcast(variant) => {
                current_variant = Some(variant);
            }
            PlaceElem::Deref => {
                ty = match arena.get(ty) {
                    MirTy::Pointer(inner) => *inner,
                    _ => return None,
                };
                current_variant = None;
            }
        }
    }

    Some(ty)
}

fn tuple_or_variant_field_type(
    arena: &mut TyArena,
    structs: &[StructDef],
    enums: &[EnumDef],
    ty: TyId,
    index: usize,
    current_variant: Option<VariantIdx>,
) -> Option<TyId> {
    if let MirTy::Tuple(elems) = arena.get(ty) {
        return elems.get(index).copied();
    }

    let variant = current_variant?;
    field_type(
        arena,
        structs,
        enums,
        ty,
        FieldIdx::new(index),
        Some(variant),
    )
}

fn field_type(
    arena: &mut TyArena,
    structs: &[StructDef],
    enums: &[EnumDef],
    ty: TyId,
    field: FieldIdx,
    current_variant: Option<VariantIdx>,
) -> Option<TyId> {
    let (entity, type_args) = match arena.get(ty) {
        MirTy::Named { entity, type_args } => (*entity, type_args.clone()),
        _ => return None,
    };

    if let Some(variant) = current_variant {
        let enum_def = enums.iter().find(|e| e.entity == entity)?;
        let field_ty = enum_def
            .cases
            .get(variant.index())?
            .payload_fields
            .get(field.index())?
            .ty;
        return Some(substitute_nominal_field(
            arena,
            enum_def.type_params.iter().map(|tp| tp.entity),
            &type_args,
            field_ty,
        ));
    }

    let struct_def = structs.iter().find(|s| s.entity == entity)?;
    let field_ty = struct_def.fields.get(field.index())?.ty;
    Some(substitute_nominal_field(
        arena,
        struct_def.type_params.iter().map(|tp| tp.entity),
        &type_args,
        field_ty,
    ))
}

fn substitute_nominal_field(
    arena: &mut TyArena,
    type_params: impl Iterator<Item = kestrel_hecs::Entity>,
    type_args: &[TyId],
    ty: TyId,
) -> TyId {
    if type_args.is_empty() {
        return ty;
    }
    let mut subst = SubstMap::new();
    for (param, &arg) in type_params.zip(type_args.iter()) {
        subst.type_params.insert(param, arg);
    }
    substitute(arena, ty, &subst)
}
