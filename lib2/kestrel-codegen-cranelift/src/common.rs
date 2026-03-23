//! Shared helpers used across codegen modules.
//!
//! Eliminates the duplication found in lib1 where `is_aggregate_type`,
//! `copy_aggregate_value`, `get_place_type`, and `get_field_info` were
//! each defined 2-3 times across different files.

use crate::error::CodegenError;
use crate::types;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, Value as CrValue};
use cranelift_frontend::FunctionBuilder;
use kestrel_codegen2::{substitute_type, Layout, LayoutCache, TargetConfig};
use kestrel_hecs::Entity;
use kestrel_mir::{
    BasicBlock, BlockId, EnumId, LocalId, MirBody, MirModule, MirTy, Place, StructId,
};
use std::collections::HashMap;

/// Check if a MirTy is an aggregate type (passed by pointer, not in registers).
/// Conservative: treats all Named types as aggregate. Use `is_aggregate_with_layout`
/// when a LayoutCache is available for accurate classification.
pub fn is_aggregate_type(ty: &MirTy) -> bool {
    matches!(
        ty,
        MirTy::Tuple(_) | MirTy::Named { .. } | MirTy::Str | MirTy::FuncThick { .. }
    )
}

/// Layout-aware aggregate check. Named types that fit in a register (≤ pointer size)
/// are passed by value, not by pointer. This correctly handles wrapper structs like
/// Bool, Int64, etc. that are Named in MIR but small enough for registers.
pub fn is_aggregate_with_layout(ty: &MirTy, layouts: &mut LayoutCache) -> bool {
    match ty {
        MirTy::Named { .. } => {
            let layout = layouts.layout_of(ty);
            layout.size > 8
        }
        _ => is_aggregate_type(ty),
    }
}

/// Check if a function return type requires sret (struct-return) ABI.
pub fn needs_sret(ret: &MirTy, layouts: &mut LayoutCache) -> bool {
    !matches!(ret, MirTy::Unit | MirTy::Never) && is_aggregate_with_layout(ret, layouts)
}

/// Get the Cranelift pointer type for the target.
pub fn ptr_type(target: &TargetConfig) -> ir::Type {
    if target.is_64bit() {
        ir::types::I64
    } else {
        ir::types::I32
    }
}

/// Convert alignment to Cranelift's `align_shift` (log2 of alignment).
pub fn align_to_shift(align: u64) -> u8 {
    align.trailing_zeros() as u8
}

/// Zero-initialize memory using multi-byte stores for efficiency.
pub fn zero_memory(builder: &mut FunctionBuilder, ptr: CrValue, size: u64, ptr_ty: ir::Type) {
    if size == 0 {
        return;
    }
    let mut offset = 0u64;
    let remaining = size;

    // Store 8 bytes at a time
    if remaining >= 8 {
        let zero = builder.ins().iconst(ir::types::I64, 0);
        while offset + 8 <= size {
            builder
                .ins()
                .store(MemFlags::new(), zero, ptr, Offset32::new(offset as i32));
            offset += 8;
        }
    }

    // Store 4 bytes
    if offset + 4 <= size {
        let zero = builder.ins().iconst(ir::types::I32, 0);
        builder
            .ins()
            .store(MemFlags::new(), zero, ptr, Offset32::new(offset as i32));
        offset += 4;
    }

    // Store 2 bytes
    if offset + 2 <= size {
        let zero = builder.ins().iconst(ir::types::I16, 0);
        builder
            .ins()
            .store(MemFlags::new(), zero, ptr, Offset32::new(offset as i32));
        offset += 2;
    }

    // Store remaining byte
    if offset < size {
        let zero = builder.ins().iconst(ir::types::I8, 0);
        builder
            .ins()
            .store(MemFlags::new(), zero, ptr, Offset32::new(offset as i32));
    }

    let _ = ptr_ty; // Available for future use
}

/// Copy an aggregate value byte-by-byte between two pointers.
///
/// Single definition replacing 3 copies in lib1 (rvalue.rs, place.rs, terminator.rs).
pub fn copy_aggregate(
    builder: &mut FunctionBuilder,
    layouts: &mut LayoutCache,
    ty: &MirTy,
    dest: CrValue,
    src: CrValue,
) {
    let layout = layouts.layout_of(ty);
    let size = layout.size;

    // Copy byte by byte (correctness-first; could be optimized with word copies)
    for i in 0..size {
        let byte = builder.ins().load(
            ir::types::I8,
            MemFlags::new(),
            src,
            Offset32::new(i as i32),
        );
        builder
            .ins()
            .store(MemFlags::new(), byte, dest, Offset32::new(i as i32));
    }
}

/// Get the type of a Place expression by walking the projection chain.
///
/// Single definition replacing 3 copies in lib1 (rvalue.rs, place.rs, terminator.rs).
pub fn get_place_type(
    module: &MirModule,
    body: &MirBody,
    place: &Place,
    subst: &HashMap<Entity, MirTy>,
    layouts: &LayoutCache,
) -> Result<MirTy, CodegenError> {
    match place {
        Place::Local(id) => {
            let ty = &body.locals[id.index()].ty;
            Ok(substitute_type(ty, subst))
        }

        Place::Global(entity) => {
            // Find the static by entity
            for s in &module.statics {
                if s.entity == *entity {
                    return Ok(substitute_type(&s.ty, subst));
                }
            }
            let name = module.resolve_name(*entity);
            Err(CodegenError::Unsupported(format!(
                "unknown global entity {:?} ({})",
                entity, name
            )))
        }

        Place::Field { parent, name } => {
            let parent_ty = get_place_type(module, body, parent, subst, layouts)?;
            match &parent_ty {
                MirTy::Named { entity, type_args } => {
                    let type_args = type_args
                        .iter()
                        .map(|a| substitute_type(a, subst))
                        .collect::<Vec<_>>();
                    match layouts.resolve_named(*entity) {
                        kestrel_codegen2::NamedKind::Struct(struct_id) => {
                            let struct_def = &module.structs[struct_id.index()];
                            let field_id = struct_def.field_by_name(name).ok_or_else(|| {
                                CodegenError::Unsupported(format!(
                                    "field '{name}' not found on struct '{}'",
                                    struct_def.name
                                ))
                            })?;
                            let field_ty = &struct_def.fields[field_id.index()].ty;
                            // Substitute the struct's type params into the field type
                            let field_subst: HashMap<Entity, MirTy> = struct_def
                                .type_params
                                .iter()
                                .zip(type_args.iter())
                                .map(|(tp, arg)| (tp.entity, arg.clone()))
                                .collect();
                            Ok(substitute_type(field_ty, &field_subst))
                        }
                        _ => Err(CodegenError::Unsupported(format!(
                            "field access on non-struct Named type: {name}"
                        ))),
                    }
                }
                _ => Err(CodegenError::Unsupported(format!(
                    "field access on non-Named type: {name}"
                ))),
            }
        }

        Place::Index { parent, index } => {
            let parent_ty = get_place_type(module, body, parent, subst, layouts)?;
            match &parent_ty {
                MirTy::Tuple(elems) => {
                    if *index < elems.len() {
                        Ok(elems[*index].clone())
                    } else {
                        Err(CodegenError::Unsupported(format!(
                            "tuple index {index} out of range"
                        )))
                    }
                }
                // Index on a Named type is a struct field by index
                MirTy::Named { entity, type_args } => {
                    match layouts.resolve_named(*entity) {
                        kestrel_codegen2::NamedKind::Struct(struct_id) => {
                            let struct_def = &module.structs[struct_id.index()];
                            if *index < struct_def.fields.len() {
                                let field_ty = &struct_def.fields[*index].ty;
                                let field_subst: HashMap<Entity, MirTy> = struct_def
                                    .type_params
                                    .iter()
                                    .zip(type_args.iter())
                                    .map(|(tp, arg)| (tp.entity, arg.clone()))
                                    .collect();
                                Ok(substitute_type(field_ty, &field_subst))
                            } else {
                                Err(CodegenError::Unsupported(format!(
                                    "struct index {index} out of range"
                                )))
                            }
                        }
                        _ => Err(CodegenError::Unsupported(format!(
                            "index on non-struct Named type: {index}"
                        ))),
                    }
                }
                _ => Err(CodegenError::Unsupported(format!(
                    "index on non-tuple/struct type: {index}"
                ))),
            }
        }

        Place::Downcast { parent, variant } => {
            let parent_ty = get_place_type(module, body, parent, subst, layouts)?;
            // Downcast returns the payload struct type for this variant
            match &parent_ty {
                MirTy::Named { entity, type_args } => {
                    match layouts.resolve_named(*entity) {
                        kestrel_codegen2::NamedKind::Enum(enum_id) => {
                            let enum_def = &module.enums[enum_id.index()];
                            let case = enum_def.case_by_name(variant).ok_or_else(|| {
                                CodegenError::Unsupported(format!(
                                    "variant '{variant}' not found on enum '{}'",
                                    enum_def.name
                                ))
                            })?;
                            // The payload struct's type
                            let payload = &module.structs[case.payload_struct.index()];
                            Ok(MirTy::Named {
                                entity: payload.entity,
                                type_args: type_args.clone(),
                            })
                        }
                        _ => Err(CodegenError::Unsupported(format!(
                            "downcast on non-enum type: {variant}"
                        ))),
                    }
                }
                _ => Err(CodegenError::Unsupported(format!(
                    "downcast on non-Named type: {variant}"
                ))),
            }
        }

        Place::Deref(inner) => {
            let inner_ty = get_place_type(module, body, inner, subst, layouts)?;
            match inner_ty {
                MirTy::Pointer(pointee) | MirTy::Ref(pointee) | MirTy::RefMut(pointee) => {
                    Ok(*pointee)
                }
                _ => Err(CodegenError::Unsupported(
                    "deref of non-pointer type".into(),
                )),
            }
        }
    }
}

/// Get field byte offset and type within a struct.
///
/// Single definition replacing 2 copies in lib1 (rvalue.rs, place.rs).
pub fn get_field_info(
    module: &MirModule,
    layouts: &mut LayoutCache,
    struct_id: StructId,
    type_args: &[MirTy],
    field_name: &str,
) -> Result<(u64, MirTy), CodegenError> {
    let struct_def = &module.structs[struct_id.index()];
    let field_id = struct_def.field_by_name(field_name).ok_or_else(|| {
        CodegenError::Unsupported(format!(
            "field '{field_name}' not found on struct '{}'",
            struct_def.name
        ))
    })?;

    let sl = layouts.struct_layout(struct_id, type_args);
    let offset = sl.field_offsets[field_id.index()];

    // Substitute type params into field type
    let field_ty = &struct_def.fields[field_id.index()].ty;
    let subst: HashMap<Entity, MirTy> = struct_def
        .type_params
        .iter()
        .zip(type_args.iter())
        .map(|(tp, arg)| (tp.entity, arg.clone()))
        .collect();
    let concrete_ty = substitute_type(field_ty, &subst);

    Ok((offset, concrete_ty))
}

/// Get field byte offset and type by numeric index.
pub fn get_field_by_index(
    module: &MirModule,
    layouts: &mut LayoutCache,
    struct_id: StructId,
    type_args: &[MirTy],
    index: usize,
) -> Result<(u64, MirTy), CodegenError> {
    let struct_def = &module.structs[struct_id.index()];
    if index >= struct_def.fields.len() {
        return Err(CodegenError::Unsupported(format!(
            "field index {index} out of range for struct '{}'",
            struct_def.name
        )));
    }

    let sl = layouts.struct_layout(struct_id, type_args);
    let offset = sl.field_offsets[index];

    let field_ty = &struct_def.fields[index].ty;
    let subst: HashMap<Entity, MirTy> = struct_def
        .type_params
        .iter()
        .zip(type_args.iter())
        .map(|(tp, arg)| (tp.entity, arg.clone()))
        .collect();
    let concrete_ty = substitute_type(field_ty, &subst);

    Ok((offset, concrete_ty))
}

/// Compute the byte offset of the enum payload area.
///
/// Enum layout: [discriminant (4 bytes, i32)] [padding] [payload]
/// The payload starts at an offset determined by the maximum payload alignment.
pub fn get_enum_payload_offset(
    module: &MirModule,
    layouts: &mut LayoutCache,
    enum_id: EnumId,
    type_args: &[MirTy],
) -> u64 {
    let enum_def = &module.enums[enum_id.index()];

    // Find maximum alignment across all case payloads
    let mut max_payload_align = 1u64;
    for case in &enum_def.cases {
        let payload_layout = layouts.struct_layout(case.payload_struct, type_args);
        max_payload_align = max_payload_align.max(payload_layout.layout.align);
    }

    // Discriminant is 4 bytes; payload starts at next aligned offset
    let discriminant_end = 4u64;
    let payload_offset = (discriminant_end + max_payload_align - 1) & !(max_payload_align - 1);
    payload_offset
}

/// Translate a MirTy to its Cranelift type representation.
/// Re-export for convenience.
pub fn translate_type(ty: &MirTy, target: &TargetConfig) -> ir::Type {
    types::translate_type(ty, target)
}
