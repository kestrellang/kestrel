//! Lookups into the mono module's layouts that both backends need when
//! addressing fields, tuple elements and enum discriminants.

use kestrel_hecs::Entity;
use kestrel_mir::mono::{MonoEnum, MonoModule, MonoStruct};
use kestrel_mir::{FieldIdx, Layout, MirTy, StructLayout, TyArena, TyId};

use crate::repr::{ScalarModel, TypeCache};

pub fn find_mono_struct<'m>(
    entity: &Entity,
    type_args: &[TyId],
    module: &'m MonoModule,
) -> Option<&'m MonoStruct> {
    module.structs.get(&(*entity, type_args.to_vec()))
}

pub fn find_mono_enum<'m>(
    entity: &Entity,
    type_args: &[TyId],
    module: &'m MonoModule,
) -> Option<&'m MonoEnum> {
    module.enums.get(&(*entity, type_args.to_vec()))
}

/// The mono struct a `Named` type instantiates, if it is one.
fn named_struct<'m>(ty: TyId, arena: &TyArena, module: &'m MonoModule) -> Option<&'m MonoStruct> {
    let MirTy::Named { entity, type_args } = arena.get(ty) else {
        return None;
    };
    find_mono_struct(entity, type_args, module)
}

/// Byte offset of `field_idx` in the struct `container_ty`, from its layout.
pub fn struct_field_offset(
    container_ty: TyId,
    field_idx: FieldIdx,
    arena: &TyArena,
    module: &MonoModule,
) -> u64 {
    match named_struct(container_ty, arena, module).and_then(|s| s.type_info.layout.as_ref()) {
        Some(Layout::Struct(sl)) => sl.field_offsets[field_idx.index()],
        _ => 0,
    }
}

/// Type of `field_idx` in the struct `container_ty` (the container itself if
/// it is not a known struct).
pub fn struct_field_type(
    container_ty: TyId,
    field_idx: FieldIdx,
    arena: &TyArena,
    module: &MonoModule,
) -> TyId {
    named_struct(container_ty, arena, module)
        .map(|s| s.fields[field_idx.index()].ty)
        .unwrap_or(container_ty)
}

/// Byte offset and type of tuple element `index`, laid out as consecutive
/// fields (matching the tuple arm of `TypeCache::classify`).
pub fn tuple_elem_offset<M: ScalarModel>(
    tc: &mut TypeCache<M>,
    arena: &TyArena,
    module: &MonoModule,
    elems: &[TyId],
    index: u32,
) -> (u64, TyId) {
    let mut layout = StructLayout::new();
    for (i, &elem) in elems.iter().enumerate() {
        let repr = tc.repr(elem, arena, module);
        layout.append_field(StructLayout::scalar(repr.size(), repr.align()));
        if i == index as usize {
            return (layout.field_offsets[i], elem);
        }
    }
    (0, elems[index as usize])
}

/// The integer scalar an enum's discriminant is stored as. Falls back to the
/// `Discriminant` instruction's result type when `ty` is not a known enum.
pub fn discriminant_width<M: ScalarModel>(
    ty: TyId,
    result_ty: TyId,
    arena: &TyArena,
    module: &MonoModule,
) -> M::Scalar {
    if let MirTy::Named { entity, type_args } = arena.get(ty)
        && let Some(e) = find_mono_enum(entity, type_args, module)
    {
        return M::from_int_bits(e.discriminant_width);
    }
    let bits = match arena.get(result_ty) {
        MirTy::I8 => kestrel_mir::IntBits::I8,
        MirTy::I16 => kestrel_mir::IntBits::I16,
        MirTy::I64 => kestrel_mir::IntBits::I64,
        _ => kestrel_mir::IntBits::I32,
    };
    M::from_int_bits(bits)
}

/// Whether a function returns `Never` (the call is followed by a trap).
pub fn ret_ty_is_never(module: &MonoModule, ret: TyId) -> bool {
    matches!(module.ty_arena.get(ret), MirTy::Never)
}
