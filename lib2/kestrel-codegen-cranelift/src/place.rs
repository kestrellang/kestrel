//! Place compilation — reading from and writing to memory locations.
//!
//! Uses `common::get_place_type`, `common::get_field_info`, and
//! `common::copy_aggregate` as shared helpers (eliminating lib1's duplication).

use crate::common::{self, get_enum_payload_offset, get_field_info, is_aggregate_type};
use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::function::FunctionState;
use crate::types;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, Value as CrValue};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use kestrel_codegen2::{substitute_type, LayoutCache, NamedKind};
use kestrel_mir::{MirTy, Place};
use std::collections::HashMap;

/// Store a scalar value directly to an aggregate destination.
/// Used when a function returns a small value that fits in a register
/// but the destination type is aggregate (Named struct).
pub fn store_scalar_to_aggregate(
    builder: &mut FunctionBuilder,
    layouts: &mut LayoutCache,
    ty: &MirTy,
    dest: CrValue,
    value: CrValue,
) {
    let layout = layouts.layout_of(ty);
    if layout.size <= 8 && layout.size > 0 {
        builder.ins().store(MemFlags::new(), value, dest, Offset32::new(0));
    } else {
        // Shouldn't happen for non-sret returns, but fall back to copy
        common::copy_aggregate(builder, layouts, ty, dest, value);
    }
}

/// Read a value from a place expression.
pub fn compile_place_read(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    place: &Place,
) -> Result<CrValue, CodegenError> {
    // Guard against deep recursion from nested place projections
    stacker::maybe_grow(128 * 1024, 2 * 1024 * 1024, || {
        compile_place_read_inner(ctx, state, builder, place)
    })
}

fn compile_place_read_inner(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    place: &Place,
) -> Result<CrValue, CodegenError> {
    let ptr_ty = common::ptr_type(ctx.target);
    let ty = common::get_place_type(
        ctx.module,
        state.body,
        place,
        &state.subst,
        &ctx.layouts,
    )?;

    // Unit/Never are zero-sized — return a zero constant, never load from memory
    if matches!(ty, MirTy::Unit | MirTy::Never) {
        return Ok(builder.ins().iconst(ptr_ty, 0));
    }

    match place {
        Place::Local(id) => {
            let var = state.local_vars[id.index()];
            Ok(builder.use_var(var))
        }

        Place::Global(entity) => {
            // Find the static and its mangled name
            let static_def = ctx.module.statics.iter().find(|s| s.entity == *entity)
                .ok_or_else(|| CodegenError::Unsupported(format!("unknown global {:?}", entity)))?;

            let mut mangler = kestrel_codegen2::Mangler::new(ctx.module);
            mangler.push_prefix();
            mangler.mangle_name_path(&static_def.name);
            let mangled = mangler.finish();

            let data_id = ctx.cl_module
                .declare_data(&mangled, cranelift_module::Linkage::Import, false, false)
                .map_err(|e| CodegenError::DataSection(format!("declare global: {e}")))?;

            let gv = ctx.cl_module.declare_data_in_func(data_id, builder.func);
            let addr = builder.ins().global_value(ptr_ty, gv);

            if is_aggregate_type(&ty) {
                Ok(addr) // Return pointer for aggregates
            } else {
                let cl_ty = types::translate_type(&ty, ctx.target);
                Ok(builder.ins().load(cl_ty, MemFlags::new(), addr, Offset32::new(0)))
            }
        }

        Place::Field { parent, name } => {
            let parent_val = compile_place_read(ctx, state, builder, parent)?;
            let parent_ty = common::get_place_type(
                ctx.module, state.body, parent, &state.subst, &ctx.layouts,
            )?;

            match &parent_ty {
                MirTy::Named { entity, type_args } => {
                    let type_args: Vec<MirTy> = type_args.iter()
                        .map(|a| substitute_type(a, &state.subst))
                        .collect();

                    match ctx.layouts.resolve_named(*entity) {
                        NamedKind::Struct(struct_id) => {
                            let (offset, field_ty) = get_field_info(
                                ctx.module, &mut ctx.layouts, struct_id, &type_args, name,
                            )?;
                            let field_ptr = builder.ins().iadd_imm(parent_val, offset as i64);

                            if is_aggregate_type(&field_ty) {
                                Ok(field_ptr)
                            } else {
                                let cl_ty = types::translate_type(&field_ty, ctx.target);
                                Ok(builder.ins().load(cl_ty, MemFlags::new(), field_ptr, Offset32::new(0)))
                            }
                        }
                        _ => Err(CodegenError::Unsupported(
                            format!("field access on non-struct: {name}")
                        )),
                    }
                }
                _ => Err(CodegenError::Unsupported(
                    format!("field access on non-Named: {name}")
                )),
            }
        }

        Place::Index { parent, index } => {
            let parent_val = compile_place_read(ctx, state, builder, parent)?;
            let parent_ty = common::get_place_type(
                ctx.module, state.body, parent, &state.subst, &ctx.layouts,
            )?;

            match &parent_ty {
                MirTy::Tuple(elems) => {
                    // Compute offset to the index-th element
                    let mut offset = 0u64;
                    let mut layout = kestrel_codegen2::Layout::zero(1);
                    for (i, elem) in elems.iter().enumerate() {
                        let elem_layout = ctx.layouts.layout_of(elem);
                        let (field_offset, new_layout) = layout.append(elem_layout);
                        if i == *index {
                            offset = field_offset;
                            break;
                        }
                        layout = new_layout;
                    }
                    let field_ptr = builder.ins().iadd_imm(parent_val, offset as i64);
                    let elem_ty = &elems[*index];

                    if is_aggregate_type(elem_ty) {
                        Ok(field_ptr)
                    } else {
                        let cl_ty = types::translate_type(elem_ty, ctx.target);
                        Ok(builder.ins().load(cl_ty, MemFlags::new(), field_ptr, Offset32::new(0)))
                    }
                }
                MirTy::Named { entity, type_args } => {
                    let type_args: Vec<MirTy> = type_args.iter()
                        .map(|a| substitute_type(a, &state.subst))
                        .collect();

                    match ctx.layouts.resolve_named(*entity) {
                        NamedKind::Struct(struct_id) => {
                            let (offset, field_ty) = common::get_field_by_index(
                                ctx.module, &mut ctx.layouts, struct_id, &type_args, *index,
                            )?;
                            let field_ptr = builder.ins().iadd_imm(parent_val, offset as i64);

                            if is_aggregate_type(&field_ty) {
                                Ok(field_ptr)
                            } else {
                                let cl_ty = types::translate_type(&field_ty, ctx.target);
                                Ok(builder.ins().load(cl_ty, MemFlags::new(), field_ptr, Offset32::new(0)))
                            }
                        }
                        _ => Err(CodegenError::Unsupported(
                            format!("index on non-struct Named: {index}")
                        )),
                    }
                }
                _ => Err(CodegenError::Unsupported(
                    format!("index on unsupported type: {index}")
                )),
            }
        }

        Place::Downcast { parent, variant } => {
            let parent_val = compile_place_read(ctx, state, builder, parent)?;
            let parent_ty = common::get_place_type(
                ctx.module, state.body, parent, &state.subst, &ctx.layouts,
            )?;

            match &parent_ty {
                MirTy::Named { entity, type_args } => {
                    let type_args: Vec<MirTy> = type_args.iter()
                        .map(|a| substitute_type(a, &state.subst))
                        .collect();

                    match ctx.layouts.resolve_named(*entity) {
                        NamedKind::Enum(enum_id) => {
                            let payload_offset = get_enum_payload_offset(
                                ctx.module, &mut ctx.layouts, enum_id, &type_args,
                            );
                            // Return pointer to the payload area
                            Ok(builder.ins().iadd_imm(parent_val, payload_offset as i64))
                        }
                        _ => Err(CodegenError::Unsupported(
                            format!("downcast on non-enum: {variant}")
                        )),
                    }
                }
                _ => Err(CodegenError::Unsupported(
                    format!("downcast on non-Named: {variant}")
                )),
            }
        }

        Place::Deref(inner) => {
            let ptr_val = compile_place_read(ctx, state, builder, inner)?;

            if is_aggregate_type(&ty) {
                Ok(ptr_val) // Return the pointer directly for aggregates
            } else {
                let cl_ty = types::translate_type(&ty, ctx.target);
                Ok(builder.ins().load(cl_ty, MemFlags::new(), ptr_val, Offset32::new(0)))
            }
        }
    }
}

/// Write a value to a place expression.
pub fn compile_place_write(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    place: &Place,
    value: CrValue,
) -> Result<(), CodegenError> {
    let ty = common::get_place_type(
        ctx.module, state.body, place, &state.subst, &ctx.layouts,
    )?;

    // Unit/Never are zero-sized — nothing to write
    if matches!(ty, MirTy::Unit | MirTy::Never) {
        return Ok(());
    }

    match place {
        Place::Local(id) => {
            if is_aggregate_type(&ty) || state.stack_locals.contains(id) {
                // Write to the stack slot pointed to by the variable
                let dest_ptr = builder.use_var(state.local_vars[id.index()]);
                if is_aggregate_type(&ty) {
                    common::copy_aggregate(builder, &mut ctx.layouts, &ty, dest_ptr, value);
                } else {
                    builder.ins().store(MemFlags::new(), value, dest_ptr, Offset32::new(0));
                }
            } else {
                builder.def_var(state.local_vars[id.index()], value);
            }
            Ok(())
        }

        Place::Field { parent, name } => {
            let parent_ptr = compile_place_read(ctx, state, builder, parent)?;
            let parent_ty = common::get_place_type(
                ctx.module, state.body, parent, &state.subst, &ctx.layouts,
            )?;

            match &parent_ty {
                MirTy::Named { entity, type_args } => {
                    let type_args: Vec<MirTy> = type_args.iter()
                        .map(|a| substitute_type(a, &state.subst))
                        .collect();

                    match ctx.layouts.resolve_named(*entity) {
                        NamedKind::Struct(struct_id) => {
                            let (offset, field_ty) = get_field_info(
                                ctx.module, &mut ctx.layouts, struct_id, &type_args, name,
                            )?;
                            let field_ptr = builder.ins().iadd_imm(parent_ptr, offset as i64);

                            if is_aggregate_type(&field_ty) {
                                common::copy_aggregate(builder, &mut ctx.layouts, &field_ty, field_ptr, value);
                            } else {
                                builder.ins().store(MemFlags::new(), value, field_ptr, Offset32::new(0));
                            }
                            Ok(())
                        }
                        _ => Err(CodegenError::Unsupported(
                            format!("field write on non-struct: {name}")
                        )),
                    }
                }
                _ => Err(CodegenError::Unsupported(
                    format!("field write on non-Named: {name}")
                )),
            }
        }

        Place::Deref(inner) => {
            let ptr_val = compile_place_read(ctx, state, builder, inner)?;
            if is_aggregate_type(&ty) {
                common::copy_aggregate(builder, &mut ctx.layouts, &ty, ptr_val, value);
            } else {
                builder.ins().store(MemFlags::new(), value, ptr_val, Offset32::new(0));
            }
            Ok(())
        }

        // Index, Downcast, Global writes follow the same pattern
        _ => {
            let dest_ptr = compile_place_addr(ctx, state, builder, place)?;
            if is_aggregate_type(&ty) {
                common::copy_aggregate(builder, &mut ctx.layouts, &ty, dest_ptr, value);
            } else {
                builder.ins().store(MemFlags::new(), value, dest_ptr, Offset32::new(0));
            }
            Ok(())
        }
    }
}

/// Get the address of a place expression (for taking references).
pub fn compile_place_addr(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    place: &Place,
) -> Result<CrValue, CodegenError> {
    let ptr_ty = common::ptr_type(ctx.target);

    match place {
        Place::Local(id) => {
            // For address-taken or aggregate locals, the var already holds a pointer
            let var = state.local_vars[id.index()];
            Ok(builder.use_var(var))
        }

        Place::Field { parent, name } => {
            let parent_ptr = compile_place_addr(ctx, state, builder, parent)?;
            let parent_ty = common::get_place_type(
                ctx.module, state.body, parent, &state.subst, &ctx.layouts,
            )?;

            match &parent_ty {
                MirTy::Named { entity, type_args } => {
                    let type_args: Vec<MirTy> = type_args.iter()
                        .map(|a| substitute_type(a, &state.subst))
                        .collect();

                    match ctx.layouts.resolve_named(*entity) {
                        NamedKind::Struct(struct_id) => {
                            let (offset, _) = get_field_info(
                                ctx.module, &mut ctx.layouts, struct_id, &type_args, name,
                            )?;
                            Ok(builder.ins().iadd_imm(parent_ptr, offset as i64))
                        }
                        _ => Err(CodegenError::Unsupported(
                            format!("field addr on non-struct: {name}")
                        )),
                    }
                }
                _ => Err(CodegenError::Unsupported(
                    format!("field addr on non-Named: {name}")
                )),
            }
        }

        Place::Deref(inner) => {
            // The address of *ptr is just ptr
            compile_place_read(ctx, state, builder, inner)
        }

        Place::Index { parent, index } => {
            let parent_ptr = compile_place_addr(ctx, state, builder, parent)?;
            let parent_ty = common::get_place_type(
                ctx.module, state.body, parent, &state.subst, &ctx.layouts,
            )?;

            match &parent_ty {
                MirTy::Tuple(elems) => {
                    let mut offset = 0u64;
                    let mut layout = kestrel_codegen2::Layout::zero(1);
                    for (i, elem) in elems.iter().enumerate() {
                        let elem_layout = ctx.layouts.layout_of(elem);
                        let (field_offset, new_layout) = layout.append(elem_layout);
                        if i == *index {
                            offset = field_offset;
                            break;
                        }
                        layout = new_layout;
                    }
                    Ok(builder.ins().iadd_imm(parent_ptr, offset as i64))
                }
                MirTy::Named { entity, type_args } => {
                    let type_args: Vec<MirTy> = type_args.iter()
                        .map(|a| substitute_type(a, &state.subst))
                        .collect();
                    match ctx.layouts.resolve_named(*entity) {
                        NamedKind::Struct(struct_id) => {
                            let (offset, _) = common::get_field_by_index(
                                ctx.module, &mut ctx.layouts, struct_id, &type_args, *index,
                            )?;
                            Ok(builder.ins().iadd_imm(parent_ptr, offset as i64))
                        }
                        _ => Err(CodegenError::Unsupported(
                            format!("index addr on non-struct: {index}")
                        )),
                    }
                }
                _ => Err(CodegenError::Unsupported(
                    format!("index addr on unsupported type: {index}")
                )),
            }
        }

        Place::Downcast { parent, .. } => {
            // Address of a downcast is the payload address
            let parent_ptr = compile_place_addr(ctx, state, builder, parent)?;
            let parent_ty = common::get_place_type(
                ctx.module, state.body, parent, &state.subst, &ctx.layouts,
            )?;
            match &parent_ty {
                MirTy::Named { entity, type_args } => {
                    let type_args: Vec<MirTy> = type_args.iter()
                        .map(|a| substitute_type(a, &state.subst))
                        .collect();
                    match ctx.layouts.resolve_named(*entity) {
                        NamedKind::Enum(enum_id) => {
                            let payload_offset = common::get_enum_payload_offset(
                                ctx.module, &mut ctx.layouts, enum_id, &type_args,
                            );
                            Ok(builder.ins().iadd_imm(parent_ptr, payload_offset as i64))
                        }
                        _ => Err(CodegenError::Unsupported("downcast addr on non-enum".into())),
                    }
                }
                _ => Err(CodegenError::Unsupported("downcast addr on non-Named".into())),
            }
        }

        Place::Global(entity) => {
            let static_def = ctx.module.statics.iter().find(|s| s.entity == *entity)
                .ok_or_else(|| CodegenError::Unsupported(format!("unknown global {:?}", entity)))?;
            let mut mangler = kestrel_codegen2::Mangler::new(ctx.module);
            mangler.push_prefix();
            mangler.mangle_name_path(&static_def.name);
            let mangled = mangler.finish();
            let data_id = ctx.cl_module
                .declare_data(&mangled, cranelift_module::Linkage::Import, false, false)
                .map_err(|e| CodegenError::DataSection(format!("declare global: {e}")))?;
            let gv = ctx.cl_module.declare_data_in_func(data_id, builder.func);
            Ok(builder.ins().global_value(ptr_ty, gv))
        }
    }
}
