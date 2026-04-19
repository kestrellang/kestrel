//! Place compilation — reading from and writing to memory locations.
//!
//! Uses `common::get_place_type`, `common::get_field_info`, and
//! `common::copy_aggregate` as shared helpers (eliminating lib1's duplication).

use crate::common::{self, get_enum_payload_offset, get_field_info, is_aggregate};
use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::function::FunctionState;
use crate::types;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, Value as CrValue};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use kestrel_codegen2::{substitute_type, LayoutCache, NamedKind};
use kestrel_hecs::Entity;
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
        ctx.module, state.body, place, &state.subst, &ctx.layouts,
    )?;

    // Unit/Never are zero-sized — return a zero constant, never load from memory
    if matches!(ty, MirTy::Unit | MirTy::Never) {
        return Ok(builder.ins().iconst(ptr_ty, 0));
    }

    match place {
        Place::Local(id) => Ok(builder.use_var(state.local_vars[id.index()])),
        Place::Global(entity) => read_global(ctx, builder, entity, &ty),
        Place::Field { parent, name } => read_field(ctx, state, builder, parent, name),
        Place::Index { parent, index } => read_index(ctx, state, builder, parent, *index),
        Place::Downcast { parent, variant } => {
            read_downcast(ctx, state, builder, parent, variant)
        }
        Place::Deref(inner) => read_deref(ctx, state, builder, inner, &ty),
    }
}

/// Either load a scalar value from `ptr` or return `ptr` unchanged when `ty`
/// is aggregate (aggregates are passed/held by pointer).
fn load_or_ptr(
    ctx: &mut CodegenContext,
    builder: &mut FunctionBuilder,
    ty: &MirTy,
    ptr: CrValue,
) -> CrValue {
    if is_aggregate(ty, &mut ctx.layouts) {
        ptr
    } else {
        let cl_ty = types::translate_type(ty, ctx.target);
        builder.ins().load(cl_ty, MemFlags::new(), ptr, Offset32::new(0))
    }
}

fn read_global(
    ctx: &mut CodegenContext,
    builder: &mut FunctionBuilder,
    entity: &Entity,
    ty: &MirTy,
) -> Result<CrValue, CodegenError> {
    let ptr_ty = common::ptr_type(ctx.target);
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

    Ok(load_or_ptr(ctx, builder, ty, addr))
}

fn read_field(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    parent: &Place,
    name: &str,
) -> Result<CrValue, CodegenError> {
    let parent_val = compile_place_read(ctx, state, builder, parent)?;
    let parent_ty = common::get_place_type(
        ctx.module, state.body, parent, &state.subst, &ctx.layouts,
    )?;

    let MirTy::Named { entity, type_args } = &parent_ty else {
        return Err(CodegenError::Unsupported(format!("field access on non-Named: {name}")));
    };
    let NamedKind::Struct(struct_id) = ctx.layouts.resolve_named(*entity) else {
        return Err(CodegenError::Unsupported(format!("field access on non-struct: {name}")));
    };

    let type_args = common::substitute_type_args(type_args, &state.subst);
    let (offset, field_ty) = get_field_info(
        ctx.module, &mut ctx.layouts, struct_id, &type_args, name,
    )?;
    let field_ptr = builder.ins().iadd_imm(parent_val, offset as i64);
    Ok(load_or_ptr(ctx, builder, &field_ty, field_ptr))
}

fn read_index(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    parent: &Place,
    index: usize,
) -> Result<CrValue, CodegenError> {
    let parent_val = compile_place_read(ctx, state, builder, parent)?;
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
                if i == index {
                    offset = field_offset;
                    break;
                }
                layout = new_layout;
            }
            let field_ptr = builder.ins().iadd_imm(parent_val, offset as i64);
            let elem_ty = elems[index].clone();
            Ok(load_or_ptr(ctx, builder, &elem_ty, field_ptr))
        }
        MirTy::Named { entity, type_args } => {
            let NamedKind::Struct(struct_id) = ctx.layouts.resolve_named(*entity) else {
                return Err(CodegenError::Unsupported(
                    format!("index on non-struct Named: {index}")
                ));
            };
            let type_args = common::substitute_type_args(type_args, &state.subst);
            let (offset, field_ty) = common::get_field_by_index(
                ctx.module, &mut ctx.layouts, struct_id, &type_args, index,
            )?;
            let field_ptr = builder.ins().iadd_imm(parent_val, offset as i64);
            Ok(load_or_ptr(ctx, builder, &field_ty, field_ptr))
        }
        _ => Err(CodegenError::Unsupported(
            format!("index on unsupported type: {index}")
        )),
    }
}

fn read_downcast(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    parent: &Place,
    variant: &str,
) -> Result<CrValue, CodegenError> {
    let parent_val = compile_place_read(ctx, state, builder, parent)?;
    let parent_ty = common::get_place_type(
        ctx.module, state.body, parent, &state.subst, &ctx.layouts,
    )?;

    let MirTy::Named { entity, type_args } = &parent_ty else {
        return Err(CodegenError::Unsupported(format!("downcast on non-Named: {variant}")));
    };
    let NamedKind::Enum(enum_id) = ctx.layouts.resolve_named(*entity) else {
        return Err(CodegenError::Unsupported(format!("downcast on non-enum: {variant}")));
    };

    let type_args = common::substitute_type_args(type_args, &state.subst);
    let payload_offset = get_enum_payload_offset(
        ctx.module, &mut ctx.layouts, enum_id, &type_args,
    );
    Ok(builder.ins().iadd_imm(parent_val, payload_offset as i64))
}

fn read_deref(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    inner: &Place,
    ty: &MirTy,
) -> Result<CrValue, CodegenError> {
    let ptr_val = compile_place_read(ctx, state, builder, inner)?;
    Ok(load_or_ptr(ctx, builder, ty, ptr_val))
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
            if is_aggregate(&ty, &mut ctx.layouts) || state.stack_locals.contains(id) {
                // Write to the stack slot pointed to by the variable
                let dest_ptr = builder.use_var(state.local_vars[id.index()]);
                if is_aggregate(&ty, &mut ctx.layouts) {
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
                    let type_args = common::substitute_type_args(type_args, &state.subst);

                    match ctx.layouts.resolve_named(*entity) {
                        NamedKind::Struct(struct_id) => {
                            let (offset, field_ty) = get_field_info(
                                ctx.module, &mut ctx.layouts, struct_id, &type_args, name,
                            )?;
                            let field_ptr = builder.ins().iadd_imm(parent_ptr, offset as i64);

                            if is_aggregate(&field_ty, &mut ctx.layouts) {
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
            if is_aggregate(&ty, &mut ctx.layouts) {
                common::copy_aggregate(builder, &mut ctx.layouts, &ty, ptr_val, value);
            } else {
                builder.ins().store(MemFlags::new(), value, ptr_val, Offset32::new(0));
            }
            Ok(())
        }

        // Index, Downcast, Global writes follow the same pattern
        _ => {
            let dest_ptr = compile_place_addr(ctx, state, builder, place)?;
            if is_aggregate(&ty, &mut ctx.layouts) {
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
                    let type_args = common::substitute_type_args(type_args, &state.subst);

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
                    let type_args = common::substitute_type_args(type_args, &state.subst);
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
                    let type_args = common::substitute_type_args(type_args, &state.subst);
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
