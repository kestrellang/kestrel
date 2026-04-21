//! Shared helpers used across codegen modules.
//!
//! Eliminates the duplication found in lib1 where `is_aggregate`,
//! `copy_aggregate_value`, `get_place_type`, and `get_field_info` were
//! each defined 2-3 times across different files.

use crate::error::CodegenError;
use crate::types;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, Value as CrValue};
use cranelift_frontend::FunctionBuilder;
use kestrel_codegen2::{Layout, LayoutCache, TargetConfig, substitute_type};
use kestrel_hecs::Entity;
use kestrel_mir::{
    BasicBlock, BlockId, EnumId, LocalId, MirBody, MirModule, MirTy, Place, StructId,
};
use std::collections::HashMap;

/// Apply `substitute_type` to every element of a type-argument list.
///
/// Replaces the repeated pattern
/// `type_args.iter().map(|a| substitute_type(a, subst)).collect()`
/// that appeared at 8+ call sites across `place.rs`, `rvalue/construct.rs`,
/// `rvalue/call.rs`, and `monomorphize/collect.rs`.
pub fn substitute_type_args(type_args: &[MirTy], subst: &HashMap<Entity, MirTy>) -> Vec<MirTy> {
    type_args
        .iter()
        .map(|a| substitute_type(a, subst))
        .collect()
}

/// Extract the final segment of a fully-qualified MIR name.
///
/// Entities like `AssociatedTypeDef` and `EnumCaseDef` store a short name
/// (e.g., `"Item"`, `"Less"`), but the ECS `resolve_name(entity)` returns the
/// fully-qualified path (e.g., `"std.core.Iterator.Item"`). This helper strips
/// the prefix so the short name can be used with `ProtocolDef::associated_type_by_name`
/// or `EnumDef::case_by_name`.
pub fn short_name(qualified: &str) -> &str {
    qualified.rsplit('.').next().unwrap_or(qualified)
}

/// Check if a MirTy is an aggregate type (passed by pointer, not in registers).
///
/// Treats all `Named` types as aggregate (passed by pointer). This is
/// conservative but consistent — every codegen site uses the same rule so
/// callers and callees agree on ABI. `layouts` is threaded through for
/// future layout-aware refinement; it is currently unused by this predicate.
pub fn is_aggregate(ty: &MirTy, _layouts: &mut LayoutCache) -> bool {
    matches!(
        ty,
        MirTy::Tuple(_) | MirTy::Named { .. } | MirTy::Str | MirTy::FuncThick { .. }
    )
}

/// Check if a type contains any unresolved TypeParam or Error references.
/// Used to detect cases where type substitution was incomplete, which
/// can cause wrong layout computations.
pub fn type_has_unresolved_params(ty: &MirTy) -> bool {
    match ty {
        MirTy::TypeParam(_) | MirTy::Error => true,
        MirTy::Pointer(inner) | MirTy::Ref(inner) | MirTy::RefMut(inner) => {
            type_has_unresolved_params(inner)
        },
        MirTy::Tuple(elems) => elems.iter().any(type_has_unresolved_params),
        MirTy::Named { type_args, .. } => type_args.iter().any(type_has_unresolved_params),
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            params.iter().any(type_has_unresolved_params) || type_has_unresolved_params(ret)
        },
        _ => false,
    }
}

/// Check if a function return type requires sret (struct-return) ABI.
pub fn needs_sret(ret: &MirTy, layouts: &mut LayoutCache) -> bool {
    !matches!(ret, MirTy::Unit | MirTy::Never) && is_aggregate(ret, layouts)
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

/// Copy an aggregate value between two pointers using 8/4/2/1-byte strides.
///
/// Mirrors the word-stride strategy in `zero_memory` so a struct copy emits a
/// handful of loads/stores instead of one per byte.
pub fn copy_aggregate(
    builder: &mut FunctionBuilder,
    layouts: &mut LayoutCache,
    ty: &MirTy,
    dest: CrValue,
    src: CrValue,
) {
    let layout = layouts.layout_of(ty);
    let size = layout.size;
    if size == 0 {
        return;
    }

    let mut offset = 0u64;

    while offset + 8 <= size {
        let v = builder.ins().load(
            ir::types::I64,
            MemFlags::new(),
            src,
            Offset32::new(offset as i32),
        );
        builder
            .ins()
            .store(MemFlags::new(), v, dest, Offset32::new(offset as i32));
        offset += 8;
    }
    if offset + 4 <= size {
        let v = builder.ins().load(
            ir::types::I32,
            MemFlags::new(),
            src,
            Offset32::new(offset as i32),
        );
        builder
            .ins()
            .store(MemFlags::new(), v, dest, Offset32::new(offset as i32));
        offset += 4;
    }
    if offset + 2 <= size {
        let v = builder.ins().load(
            ir::types::I16,
            MemFlags::new(),
            src,
            Offset32::new(offset as i32),
        );
        builder
            .ins()
            .store(MemFlags::new(), v, dest, Offset32::new(offset as i32));
        offset += 2;
    }
    if offset < size {
        let v = builder.ins().load(
            ir::types::I8,
            MemFlags::new(),
            src,
            Offset32::new(offset as i32),
        );
        builder
            .ins()
            .store(MemFlags::new(), v, dest, Offset32::new(offset as i32));
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
        },

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
        },

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
                        },
                        _ => Err(CodegenError::Unsupported(format!(
                            "field access on non-struct Named type: {name}"
                        ))),
                    }
                },
                _ => Err(CodegenError::Unsupported(format!(
                    "field access on non-Named type: {name}"
                ))),
            }
        },

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
                },
                // Index on a Named type is a struct field by index
                MirTy::Named { entity, type_args } => match layouts.resolve_named(*entity) {
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
                    },
                    _ => Err(CodegenError::Unsupported(format!(
                        "index on non-struct Named type: {index}"
                    ))),
                },
                _ => Err(CodegenError::Unsupported(format!(
                    "index on non-tuple/struct type: {index}"
                ))),
            }
        },

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
                        },
                        _ => Err(CodegenError::Unsupported(format!(
                            "downcast on non-enum type: {variant}"
                        ))),
                    }
                },
                _ => Err(CodegenError::Unsupported(format!(
                    "downcast on non-Named type: {variant}"
                ))),
            }
        },

        Place::Deref(inner) => {
            let inner_ty = get_place_type(module, body, inner, subst, layouts)?;
            match inner_ty {
                MirTy::Pointer(pointee) | MirTy::Ref(pointee) | MirTy::RefMut(pointee) => {
                    Ok(*pointee)
                },
                _ => Err(CodegenError::Unsupported(
                    "deref of non-pointer type".into(),
                )),
            }
        },
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
    let _ = module;
    layouts.enum_payload_offset(enum_id, type_args)
}

/// Translate a MirTy to its Cranelift type representation.
/// Re-export for convenience.
pub fn translate_type(ty: &MirTy, target: &TargetConfig) -> ir::Type {
    types::translate_type(ty, target)
}
