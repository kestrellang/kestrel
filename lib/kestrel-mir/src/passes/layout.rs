use kestrel_hecs::Entity;

use crate::item::{Layout, TargetConfig};
use crate::layout::{EnumLayout, StructLayout};
use crate::ty::{MirTy, TyArena};
use crate::{IntBits, MirModule, TyId};

/// Compute layouts for all non-generic structs and enums.
/// Generic types (with TypeParam in fields) get `layout: None`.
pub fn run_layout_pass(module: &mut MirModule, target: &TargetConfig) {
    // Multi-pass fixed-point: each pass resolves types whose deps are ready.
    loop {
        let mut progress = false;

        let struct_entities: Vec<Entity> = module.structs.keys().copied().collect();
        for entity in struct_entities {
            let s = &module.structs[&entity];
            if s.type_info.layout.is_some() {
                continue;
            }
            let field_tys: Vec<TyId> = s.fields.iter().map(|f| f.ty).collect();
            if has_type_params(&module.ty_arena, &field_tys) {
                continue;
            }
            if let Some(layout) =
                compute_struct_layout(&module.ty_arena, module, &field_tys, target)
            {
                module.structs.get_mut(&entity).unwrap().type_info.layout =
                    Some(Layout::Struct(layout));
                progress = true;
            }
        }

        let enum_entities: Vec<Entity> = module.enums.keys().copied().collect();
        for entity in enum_entities {
            let e = &module.enums[&entity];
            if e.type_info.layout.is_some() {
                continue;
            }
            let case_fields: Vec<Vec<TyId>> = e
                .cases
                .iter()
                .map(|c| c.payload_fields.iter().map(|f| f.ty).collect())
                .collect();
            if case_fields
                .iter()
                .any(|fields| has_type_params(&module.ty_arena, fields))
            {
                continue;
            }
            if let Some(layout) =
                compute_enum_layout(&module.ty_arena, module, &case_fields, target)
            {
                module.enums.get_mut(&entity).unwrap().type_info.layout =
                    Some(Layout::Enum(layout));
                progress = true;
            }
        }

        if !progress {
            break;
        }
    }
}

fn has_type_params(arena: &TyArena, field_tys: &[TyId]) -> bool {
    field_tys.iter().any(|&ty| ty_contains_param(arena, ty))
}

fn ty_contains_param(arena: &TyArena, ty: TyId) -> bool {
    match arena.get(ty) {
        MirTy::TypeParam(_) | MirTy::AssociatedProjection { .. } => true,
        MirTy::Pointer(inner) => ty_contains_param(arena, *inner),
        MirTy::Tuple(elems) => {
            let elems = elems.clone();
            elems.iter().any(|&e| ty_contains_param(arena, e))
        },
        MirTy::Named { type_args, .. } => {
            let args = type_args.clone();
            args.iter().any(|&a| ty_contains_param(arena, a))
        },
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            let ret = *ret;
            let params = params.clone();
            params.iter().any(|&(t, _)| ty_contains_param(arena, t))
                || ty_contains_param(arena, ret)
        },
        _ => false,
    }
}

/// Size and alignment of a primitive MirTy (scalars, pointers, function types).
/// Returns None for composite types (Named, Tuple, TypeParam, etc.).
pub fn primitive_size_and_align(ty: &MirTy, target: &TargetConfig) -> Option<(u64, u64)> {
    match ty {
        MirTy::Bool => Some((1, 1)),
        MirTy::I8 => Some((1, 1)),
        MirTy::I16 => Some((2, 2)),
        MirTy::I32 => Some((4, 4)),
        MirTy::I64 => Some((8, 8)),
        MirTy::F16 => Some((2, 2)),
        MirTy::F32 => Some((4, 4)),
        MirTy::F64 => Some((8, 8)),
        MirTy::Never => Some((0, 1)),
        MirTy::Str => Some((target.pointer_width * 2, target.pointer_width)),
        MirTy::Pointer(_) => Some((target.pointer_width, target.pointer_width)),
        MirTy::FuncThin { .. } => Some((target.pointer_width, target.pointer_width)),
        MirTy::FuncThick { .. } => Some((target.pointer_width * 2, target.pointer_width)),
        MirTy::Error => Some((0, 1)),
        _ => None,
    }
}

/// Get size and alignment of a type, if resolvable.
pub fn size_and_align_of(
    arena: &TyArena,
    module: &MirModule,
    ty: TyId,
    target: &TargetConfig,
) -> Option<(u64, u64)> {
    if let Some(sa) = primitive_size_and_align(arena.get(ty), target) {
        return Some(sa);
    }
    match arena.get(ty) {
        MirTy::Tuple(elems) => {
            let elems = elems.clone();
            if elems.is_empty() {
                return Some((0, 1));
            }
            let mut layout = StructLayout::new();
            for elem in &elems {
                let (size, align) = size_and_align_of(arena, module, *elem, target)?;
                layout.append_field(StructLayout::scalar(size, align));
            }
            layout.pad_to_align();
            Some((layout.size, layout.align))
        },

        MirTy::Named { entity, type_args } => {
            if !type_args.is_empty() {
                return None;
            }
            let entity = *entity;
            if let Some(s) = module.structs.get(&entity) {
                if let Some(Layout::Struct(sl)) = &s.type_info.layout {
                    return Some((sl.size, sl.align));
                }
                return None;
            }
            if let Some(e) = module.enums.get(&entity) {
                if let Some(Layout::Enum(el)) = &e.type_info.layout {
                    return Some((el.size, el.align));
                }
                return None;
            }
            None
        },

        _ => None,
    }
}

fn compute_struct_layout(
    arena: &TyArena,
    module: &MirModule,
    field_tys: &[TyId],
    target: &TargetConfig,
) -> Option<StructLayout> {
    let mut layout = StructLayout::new();
    for &ty in field_tys {
        let (size, align) = size_and_align_of(arena, module, ty, target)?;
        layout.append_field(StructLayout::scalar(size, align));
    }
    layout.pad_to_align();
    Some(layout)
}

fn compute_enum_layout(
    arena: &TyArena,
    module: &MirModule,
    case_fields: &[Vec<TyId>],
    target: &TargetConfig,
) -> Option<EnumLayout> {
    let mut variant_layouts = Vec::with_capacity(case_fields.len());
    for fields in case_fields {
        let mut vl = StructLayout::new();
        for &ty in fields {
            let (size, align) = size_and_align_of(arena, module, ty, target)?;
            vl.append_field(StructLayout::scalar(size, align));
        }
        vl.pad_to_align();
        variant_layouts.push(vl);
    }
    Some(build_enum_layout(&variant_layouts, case_fields.len()))
}

/// Build an EnumLayout from pre-computed variant layouts.
pub fn build_enum_layout(variant_layouts: &[StructLayout], num_variants: usize) -> EnumLayout {
    let disc_width = discriminant_width(num_variants);
    let disc_size = disc_width.byte_width();
    let disc_align = disc_size;

    let mut max_payload_size: u64 = 0;
    let mut max_payload_align: u64 = 1;
    for vl in variant_layouts {
        max_payload_size = max_payload_size.max(vl.size);
        max_payload_align = max_payload_align.max(vl.align);
    }

    let overall_align = disc_align.max(max_payload_align);
    let payload_offset = if max_payload_align == 0 || disc_size.is_multiple_of(max_payload_align) {
        disc_size
    } else {
        disc_size + (overall_align - disc_size % overall_align) % overall_align
    };
    let total_size = payload_offset + max_payload_size;
    let padding = if overall_align == 0 {
        0
    } else {
        (overall_align - total_size % overall_align) % overall_align
    };

    EnumLayout {
        size: total_size + padding,
        align: overall_align,
        discriminant_width: disc_width,
        payload_offset,
        variant_layouts: variant_layouts.to_vec(),
    }
}

pub fn discriminant_width(num_variants: usize) -> IntBits {
    if num_variants <= 256 {
        IntBits::I8
    } else if num_variants <= 65536 {
        IntBits::I16
    } else {
        IntBits::I32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::enum_def::{EnumCaseDef, EnumDef};
    use crate::item::struct_def::{FieldDef, StructDef};
    use crate::ty::MirTy;

    fn target() -> TargetConfig {
        TargetConfig::host_64()
    }

    #[test]
    fn struct_two_i64_fields() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();
        let s_entity = Entity::from_raw(1);
        let _s_ty = module.ty_arena.named(s_entity, vec![]);
        let mut def = StructDef::new(s_entity, "Point");
        def.add_field(FieldDef::new("x", i64_ty));
        def.add_field(FieldDef::new("y", i64_ty));
        module.add_struct(def);
        run_layout_pass(&mut module, &target());

        match get_struct_layout(&module, s_entity) {
            Layout::Struct(sl) => {
                assert_eq!(sl.size, 16);
                assert_eq!(sl.align, 8);
                assert_eq!(sl.field_offsets, vec![0, 8]);
            },
            _ => panic!("expected Struct layout"),
        }
    }

    #[test]
    fn struct_mixed_fields_padding() {
        let mut module = MirModule::new("test");
        let i8_ty = module.ty_arena.i8();
        let i64_ty = module.ty_arena.i64();
        let s_entity = Entity::from_raw(1);
        let _s_ty = module.ty_arena.named(s_entity, vec![]);
        let mut def = StructDef::new(s_entity, "Padded");
        def.add_field(FieldDef::new("a", i8_ty));
        def.add_field(FieldDef::new("b", i64_ty));
        module.add_struct(def);
        run_layout_pass(&mut module, &target());

        match get_struct_layout(&module, s_entity) {
            Layout::Struct(sl) => {
                assert_eq!(sl.field_offsets, vec![0, 8]);
                assert_eq!(sl.size, 16);
                assert_eq!(sl.align, 8);
            },
            _ => panic!("expected Struct layout"),
        }
    }

    #[test]
    fn struct_empty() {
        let mut module = MirModule::new("test");
        let s_entity = Entity::from_raw(1);
        let _s_ty = module.ty_arena.named(s_entity, vec![]);
        let def = StructDef::new(s_entity, "Empty");
        module.add_struct(def);
        run_layout_pass(&mut module, &target());

        match get_struct_layout(&module, s_entity) {
            Layout::Struct(sl) => {
                assert_eq!(sl.size, 0);
                assert_eq!(sl.align, 1);
            },
            _ => panic!("expected Struct layout"),
        }
    }

    #[test]
    fn nested_struct_fixpoint() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();

        let inner_entity = Entity::from_raw(1);
        let inner_ty = module.ty_arena.named(inner_entity, vec![]);
        let mut inner_def = StructDef::new(inner_entity, "Inner");
        inner_def.add_field(FieldDef::new("val", i64_ty));
        module.add_struct(inner_def);

        let outer_entity = Entity::from_raw(2);
        let _outer_ty = module.ty_arena.named(outer_entity, vec![]);
        let mut outer_def = StructDef::new(outer_entity, "Outer");
        outer_def.add_field(FieldDef::new("inner", inner_ty));
        outer_def.add_field(FieldDef::new("extra", i64_ty));
        module.add_struct(outer_def);

        run_layout_pass(&mut module, &target());

        assert!(module.structs[&inner_entity].type_info.layout.is_some());
        match get_struct_layout(&module, outer_entity) {
            Layout::Struct(sl) => {
                assert_eq!(sl.size, 16);
                assert_eq!(sl.field_offsets, vec![0, 8]);
            },
            _ => panic!("expected Struct layout"),
        }
    }

    #[test]
    fn generic_struct_skipped() {
        let mut module = MirModule::new("test");
        let t_entity = Entity::from_raw(1);
        let t_ty = module.ty_arena.intern(MirTy::TypeParam(t_entity));
        let s_entity = Entity::from_raw(2);
        let _s_ty = module.ty_arena.named(s_entity, vec![]);
        let mut def = StructDef::new(s_entity, "Box");
        def.add_field(FieldDef::new("value", t_ty));
        module.add_struct(def);
        run_layout_pass(&mut module, &target());

        assert!(module.structs[&s_entity].type_info.layout.is_none());
    }

    #[test]
    fn enum_simple_two_variants() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();
        let e_entity = Entity::from_raw(1);
        let _e_ty = module.ty_arena.named(e_entity, vec![]);
        let mut def = EnumDef::new(e_entity, "Optional");
        def.add_case(EnumCaseDef::new("None", 0));
        def.add_case(EnumCaseDef::with_payload(
            "Some",
            1,
            vec![FieldDef::new("0", i64_ty)],
        ));
        module.add_enum(def);
        run_layout_pass(&mut module, &target());

        match get_enum_layout(&module, e_entity) {
            Layout::Enum(el) => {
                assert_eq!(el.discriminant_width, IntBits::I8);
                assert!(el.size > 0);
                assert!(el.payload_offset >= 1);
                assert_eq!(el.variant_layouts.len(), 2);
            },
            _ => panic!("expected Enum layout"),
        }
    }

    #[test]
    fn enum_no_payload() {
        let mut module = MirModule::new("test");
        let e_entity = Entity::from_raw(1);
        let _e_ty = module.ty_arena.named(e_entity, vec![]);
        let mut def = EnumDef::new(e_entity, "Color");
        def.add_case(EnumCaseDef::new("Red", 0));
        def.add_case(EnumCaseDef::new("Green", 1));
        def.add_case(EnumCaseDef::new("Blue", 2));
        module.add_enum(def);
        run_layout_pass(&mut module, &target());

        match get_enum_layout(&module, e_entity) {
            Layout::Enum(el) => {
                assert_eq!(el.discriminant_width, IntBits::I8);
                assert_eq!(el.size, 1);
            },
            _ => panic!("expected Enum layout"),
        }
    }

    #[test]
    fn pointer_field_uses_target_width() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();
        let ptr_ty = module.ty_arena.pointer(i64_ty);
        let s_entity = Entity::from_raw(1);
        let _s_ty = module.ty_arena.named(s_entity, vec![]);
        let mut def = StructDef::new(s_entity, "PtrHolder");
        def.add_field(FieldDef::new("ptr", ptr_ty));
        module.add_struct(def);
        run_layout_pass(&mut module, &target());

        match get_struct_layout(&module, s_entity) {
            Layout::Struct(sl) => {
                assert_eq!(sl.size, 8);
                assert_eq!(sl.align, 8);
            },
            _ => panic!("expected Struct layout"),
        }
    }

    #[test]
    fn primitive_sizes() {
        let mut module = MirModule::new("test");
        let bool_ty = module.ty_arena.bool();
        let i32_ty = module.ty_arena.i32();
        let i64_ty = module.ty_arena.i64();
        let f64_ty = module.ty_arena.f64();
        let unit_ty = module.ty_arena.unit();
        let t = target();
        assert_eq!(
            size_and_align_of(&module.ty_arena, &module, bool_ty, &t),
            Some((1, 1))
        );
        assert_eq!(
            size_and_align_of(&module.ty_arena, &module, i32_ty, &t),
            Some((4, 4))
        );
        assert_eq!(
            size_and_align_of(&module.ty_arena, &module, i64_ty, &t),
            Some((8, 8))
        );
        assert_eq!(
            size_and_align_of(&module.ty_arena, &module, f64_ty, &t),
            Some((8, 8))
        );
        assert_eq!(
            size_and_align_of(&module.ty_arena, &module, unit_ty, &t),
            Some((0, 1))
        );
    }

    #[test]
    fn discriminant_width_thresholds() {
        assert_eq!(discriminant_width(1), IntBits::I8);
        assert_eq!(discriminant_width(256), IntBits::I8);
        assert_eq!(discriminant_width(257), IntBits::I16);
        assert_eq!(discriminant_width(65536), IntBits::I16);
        assert_eq!(discriminant_width(65537), IntBits::I32);
    }

    use kestrel_hecs::Entity;

    fn get_struct_layout(module: &MirModule, entity: Entity) -> &Layout {
        module.structs[&entity].type_info.layout.as_ref().unwrap()
    }
    fn get_enum_layout(module: &MirModule, entity: Entity) -> &Layout {
        module.enums[&entity].type_info.layout.as_ref().unwrap()
    }
}
