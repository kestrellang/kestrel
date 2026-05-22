use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, Value};
use cranelift_frontend::FunctionBuilder;

use cranelift_module::Module;
use kestrel_mir_2::{
    FieldIdx, Layout, MirTy, MonoModule, Place, PlaceBase, PlaceElem, StructLayout,
    TyArena, TyId, VariantIdx,
};

use crate::error::CodegenError;
use crate::func::FuncCompiler;
use crate::mem;
use crate::ty::{TypeCache, TypeRepr};

/// Walk a place's projections to determine its final type.
pub fn place_type(
    place: &Place,
    body: &kestrel_mir_2::MirBody,
    arena: &TyArena,
    module: &MonoModule,
    tc: &TypeCache,
) -> TyId {
    let mut ty = match &place.base {
        PlaceBase::Local(id) => body.locals[id.index()].ty,
        PlaceBase::Global(entity) => {
            module
                .statics
                .iter()
                .find(|s| s.entity == *entity)
                .map(|s| s.ty)
                .expect("global not found in statics")
        }
    };

    let mut current_variant: Option<VariantIdx> = None;

    for proj in &place.projections {
        match proj {
            PlaceElem::Field(idx) => {
                ty = field_type(ty, *idx, current_variant, arena, module, tc);
                current_variant = None;
            }
            PlaceElem::TupleIndex(i) => {
                if let MirTy::Tuple(elems) = arena.get(ty) {
                    ty = elems[*i as usize];
                } else if let Some(variant) = current_variant {
                    // After Downcast: index into the variant's payload fields
                    if let MirTy::Named { entity, type_args } = arena.get(ty) {
                        let entity = *entity;
                        let type_args = type_args.clone();
                        if let Some(e) = find_mono_enum(&entity, &type_args, module, tc) {
                            if let Some(case) = e.cases.get(variant.index()) {
                                if let Some(field) = case.payload_fields.get(*i as usize) {
                                    ty = field.ty;
                                }
                            }
                        }
                    }
                }
                current_variant = None;
            }
            PlaceElem::Downcast(variant) => {
                // Type stays the same (enum), but subsequent Field projections
                // index into this variant's payload
                current_variant = Some(*variant);
            }
            PlaceElem::Deref => {
                if let MirTy::Pointer(inner) = arena.get(ty) {
                    ty = *inner;
                }
                current_variant = None;
            }
        }
    }

    ty
}

/// Compute the address of a place. Iterative left-fold over projections.
pub fn place_addr(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    place: &Place,
) -> Result<Value, CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;
    let arena = &fc.ctx.module.ty_arena;

    // Start from the base address
    let mut addr = match &place.base {
        PlaceBase::Local(id) => {
            // For stack locals and aggregates, the variable holds a pointer
            builder.use_var(fc.local_vars[id.index()])
        }
        PlaceBase::Global(entity) => {
            let data_id = fc.ctx.static_data.get(entity).ok_or_else(|| {
                CodegenError::Unsupported(format!("global entity not found in statics"))
            })?;
            let gv = fc
                .ctx
                .cl_module
                .declare_data_in_func(*data_id, builder.func);
            builder.ins().global_value(ptr_ty, gv)
        }
    };

    let mut current_ty = match &place.base {
        PlaceBase::Local(id) => fc.body.locals[id.index()].ty,
        PlaceBase::Global(entity) => {
            fc.ctx
                .module
                .statics
                .iter()
                .find(|s| s.entity == *entity)
                .map(|s| s.ty)
                .expect("global not found")
        }
    };

    let mut current_variant: Option<VariantIdx> = None;

    for proj in &place.projections {
        match proj {
            PlaceElem::Field(idx) => {
                let offset = field_offset(
                    current_ty,
                    *idx,
                    current_variant,
                    arena,
                    fc.ctx.module,
                    &fc.ctx.tc,
                );
                if offset != 0 {
                    addr = builder.ins().iadd_imm(addr, offset as i64);
                }
                current_ty = field_type(current_ty, *idx, current_variant, arena, fc.ctx.module, &fc.ctx.tc);
                current_variant = None;
            }

            PlaceElem::TupleIndex(i) => {
                if let MirTy::Tuple(elems) = arena.get(current_ty) {
                    let elems = elems.clone();
                    let (offset, _) =
                        tuple_elem_offset(&mut fc.ctx.tc, arena, fc.ctx.module, &elems, *i);
                    if offset != 0 {
                        addr = builder.ins().iadd_imm(addr, offset as i64);
                    }
                    current_ty = elems[*i as usize];
                }
                current_variant = None;
            }

            PlaceElem::Downcast(variant) => {
                // Add payload offset for the enum
                if let MirTy::Named { entity, type_args } = arena.get(current_ty) {
                    let entity = *entity;
                    let type_args = type_args.clone();
                    let payload_off = enum_payload_offset(&entity, &type_args, fc.ctx.module, &fc.ctx.tc);
                    if payload_off != 0 {
                        addr = builder.ins().iadd_imm(addr, payload_off as i64);
                    }
                }
                current_variant = Some(*variant);
            }

            PlaceElem::Deref => {
                addr = builder
                    .ins()
                    .load(ptr_ty, MemFlags::new(), addr, Offset32::new(0));
                if let MirTy::Pointer(inner) = arena.get(current_ty) {
                    current_ty = *inner;
                }
                current_variant = None;
            }
        }
    }

    Ok(addr)
}

/// Read a value from a place.
pub fn place_read(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    place: &Place,
) -> Result<Value, CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;

    // Fast path: bare local with no projections
    if place.projections.is_empty() {
        if let PlaceBase::Local(id) = &place.base {
            let var = fc.local_vars[id.index()];
            let ty = fc.body.locals[id.index()].ty;
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);

            return match repr {
                // Scalar that's NOT on the stack: variable holds the value directly
                TypeRepr::Scalar(t) if !fc.stack_locals.contains(id) => Ok(builder.use_var(var)),
                // Scalar on the stack: variable holds a pointer, load through it
                TypeRepr::Scalar(t) => {
                    let addr = builder.use_var(var);
                    Ok(builder
                        .ins()
                        .load(t, MemFlags::new(), addr, Offset32::new(0)))
                }
                // Aggregate: variable holds a pointer, return the pointer
                TypeRepr::Aggregate { .. } => Ok(builder.use_var(var)),
                // Zst: return sentinel
                TypeRepr::Zst => Ok(builder.ins().iconst(ptr_ty, 0)),
            };
        }
    }

    // General path: compute address, load based on type
    let final_ty = place_type(place, fc.body, &fc.ctx.module.ty_arena, fc.ctx.module, &fc.ctx.tc);
    let repr = fc
        .ctx
        .tc
        .repr(final_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
    let addr = place_addr(fc, builder, place)?;

    Ok(mem::load_from_repr(builder, repr, addr, ptr_ty))
}

/// Write a value to a place.
pub fn place_write(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    place: &Place,
    value: Value,
) -> Result<(), CodegenError> {
    // Fast path: bare local with no projections
    if place.projections.is_empty() {
        if let PlaceBase::Local(id) = &place.base {
            let ty = fc.body.locals[id.index()].ty;
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);

            match repr {
                TypeRepr::Scalar(_) if !fc.stack_locals.contains(id) => {
                    builder.def_var(fc.local_vars[id.index()], value);
                    return Ok(());
                }
                TypeRepr::Scalar(_) => {
                    let addr = builder.use_var(fc.local_vars[id.index()]);
                    builder
                        .ins()
                        .store(MemFlags::new(), value, addr, Offset32::new(0));
                    return Ok(());
                }
                TypeRepr::Aggregate { size, .. } => {
                    let addr = builder.use_var(fc.local_vars[id.index()]);
                    mem::copy_aggregate(builder, size, addr, value);
                    return Ok(());
                }
                TypeRepr::Zst => return Ok(()),
            }
        }
    }

    // General path: compute address, store
    let final_ty = place_type(place, fc.body, &fc.ctx.module.ty_arena, fc.ctx.module, &fc.ctx.tc);
    let repr = fc
        .ctx
        .tc
        .repr(final_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
    let addr = place_addr(fc, builder, place)?;
    mem::store_to_repr(builder, repr, addr, value);
    Ok(())
}

/// Read a scalar value from a place, loading from aggregate if needed.
/// Used for switch discriminants and branch conditions.
pub fn place_read_scalar(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    place: &Place,
    width: ir::Type,
) -> Result<Value, CodegenError> {
    let val = place_read(fc, builder, place)?;
    let final_ty = place_type(place, fc.body, &fc.ctx.module.ty_arena, fc.ctx.module, &fc.ctx.tc);
    let repr = fc
        .ctx
        .tc
        .repr(final_ty, &fc.ctx.module.ty_arena, fc.ctx.module);

    match repr {
        TypeRepr::Scalar(t) if t == width => Ok(val),
        TypeRepr::Scalar(t) if t.bytes() < width.bytes() => {
            Ok(builder.ins().uextend(width, val))
        }
        TypeRepr::Scalar(t) if t.bytes() > width.bytes() => {
            Ok(builder.ins().ireduce(width, val))
        }
        TypeRepr::Aggregate { .. } => {
            // Load the scalar from offset 0 of the aggregate
            Ok(builder
                .ins()
                .load(width, MemFlags::new(), val, Offset32::new(0)))
        }
        _ => Ok(val),
    }
}

// -- Layout helpers --

fn field_offset(
    container_ty: TyId,
    field_idx: FieldIdx,
    variant: Option<VariantIdx>,
    arena: &TyArena,
    module: &MonoModule,
    tc: &TypeCache,
) -> u64 {
    if let MirTy::Named { entity, type_args } = arena.get(container_ty) {
        let entity = *entity;
        let type_args = type_args.clone();

        if let Some(variant_idx) = variant {
            return enum_variant_field_offset(&entity, &type_args, variant_idx, field_idx, module, tc);
        }

        return struct_field_offset(&entity, &type_args, field_idx, module, tc);
    }
    0
}

fn field_type(
    container_ty: TyId,
    field_idx: FieldIdx,
    variant: Option<VariantIdx>,
    arena: &TyArena,
    module: &MonoModule,
    tc: &TypeCache,
) -> TyId {
    if let MirTy::Named { entity, type_args } = arena.get(container_ty) {
        let entity = *entity;
        let type_args = type_args.clone();

        if let Some(variant_idx) = variant {
            if let Some(e) = find_mono_enum(&entity, &type_args, module, tc) {
                return e.cases[variant_idx.index()].payload_fields[field_idx.index()].ty;
            }
        }

        if let Some(s) = find_mono_struct(&entity, &type_args, module, tc) {
            return s.fields[field_idx.index()].ty;
        }
    }
    container_ty
}

fn struct_field_offset(
    entity: &kestrel_hecs::Entity,
    type_args: &[TyId],
    field_idx: FieldIdx,
    module: &MonoModule,
    tc: &TypeCache,
) -> u64 {
    if let Some(s) = find_mono_struct(entity, type_args, module, tc) {
        if let Some(Layout::Struct(sl)) = &s.type_info.layout {
            return sl.field_offsets[field_idx.index()];
        }
    }
    0
}

fn enum_payload_offset(
    entity: &kestrel_hecs::Entity,
    type_args: &[TyId],
    module: &MonoModule,
    tc: &TypeCache,
) -> u64 {
    if let Some(e) = find_mono_enum(entity, type_args, module, tc) {
        return e.payload_offset as u64;
    }
    0
}

fn enum_variant_field_offset(
    entity: &kestrel_hecs::Entity,
    type_args: &[TyId],
    variant: VariantIdx,
    field_idx: FieldIdx,
    module: &MonoModule,
    tc: &TypeCache,
) -> u64 {
    if let Some(e) = find_mono_enum(entity, type_args, module, tc) {
        if let Some(Layout::Enum(el)) = &e.type_info.layout {
            if let Some(vl) = el.variant_layouts.get(variant.index()) {
                return vl.field_offsets.get(field_idx.index()).copied().unwrap_or(0);
            }
        }
    }
    0
}

fn tuple_elem_offset(
    tc: &mut TypeCache,
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

pub fn find_mono_struct<'m>(
    entity: &kestrel_hecs::Entity,
    type_args: &[TyId],
    module: &'m MonoModule,
    tc: &TypeCache,
) -> Option<&'m kestrel_mir_2::MonoStruct> {
    tc.find_struct_idx(*entity, type_args)
        .map(|idx| &module.structs[idx])
}

pub fn find_mono_enum<'m>(
    entity: &kestrel_hecs::Entity,
    type_args: &[TyId],
    module: &'m MonoModule,
    tc: &TypeCache,
) -> Option<&'m kestrel_mir_2::MonoEnum> {
    tc.find_enum_idx(*entity, type_args)
        .map(|idx| &module.enums[idx])
}
