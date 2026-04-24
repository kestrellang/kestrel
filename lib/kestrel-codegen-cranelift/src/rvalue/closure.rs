//! Closure construction — ApplyPartial compilation.
//!
//! Creates a thick callable by allocating an environment struct,
//! storing captures with the env struct's real layout, and pairing it
//! with a callable entry point (usually a generated thunk).

use crate::common;
use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::function::FunctionState;
use crate::rvalue;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{
    InstBuilder, MemFlags, StackSlotData, StackSlotKind, Value as CrValue,
};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use kestrel_codegen2::{mangle_function_with_self, substitute_type};
use kestrel_hecs::Entity;
use kestrel_mir::{FunctionKind, MirTy, Value};
use std::collections::HashMap;

/// Compile `apply partial func(captures...)` → thick callable (func_ptr, env_ptr).
pub fn compile_apply_partial(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    func: &Entity,
    captures: &[Value],
) -> Result<CrValue, CodegenError> {
    let ptr_ty = common::ptr_type(ctx.target);
    let ptr_size = ctx.target.pointer_size();
    let original_func_id = *ctx.entity_to_func.get(func).ok_or_else(|| {
        CodegenError::Unsupported(format!(
            "ApplyPartial target is not a known function: {func:?}"
        ))
    })?;
    let original_func = &ctx.module.functions[original_func_id.index()];
    let callable_func = ctx
        .module
        .functions
        .iter()
        .find(|f| matches!(f.kind, FunctionKind::Thunk { original } if original == *func))
        .unwrap_or(original_func);
    let type_args: Vec<MirTy> = original_func
        .type_params
        .iter()
        .filter_map(|tp| state.subst.get(&tp.entity).cloned())
        .collect();
    // Thunks/closures instantiated by the monomorphizer inherit the caller's
    // self_type (see `collect.rs::scan_rvalue` for ApplyPartial), so the
    // mangled reference must match by passing the same self_type through.
    let mangled = mangle_function_with_self(
        ctx.module,
        callable_func,
        &type_args,
        state.self_type.as_ref(),
    );
    let func_id = *ctx.func_ids_by_name.get(&mangled).ok_or_else(|| {
        CodegenError::Unsupported(format!(
            "ApplyPartial target callable '{mangled}' was not declared"
        ))
    })?;

    // Allocate the thick callable struct: (func_ptr, env_ptr)
    let thick_size = ptr_size * 2;
    let thick_slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        thick_size as u32,
        common::align_to_shift(ptr_size),
    ));
    let thick_addr = builder
        .ins()
        .stack_addr(ptr_ty, thick_slot, Offset32::new(0));

    // Store the callable entry point in the first word.
    let func_ref = ctx.cl_module.declare_func_in_func(func_id, builder.func);
    let func_addr = builder.ins().func_addr(ptr_ty, func_ref);
    builder
        .ins()
        .store(MemFlags::new(), func_addr, thick_addr, Offset32::new(0));
    let null = builder.ins().iconst(ptr_ty, 0);

    // Allocate environment struct with captures using the closure env's real layout.
    let env_addr = match &original_func.kind {
        FunctionKind::ClosureCall { env_struct } => {
            let env_type_args = type_args.clone();
            let env_layout = ctx.layouts.struct_layout(*env_struct, &env_type_args);
            let env_size = env_layout.layout.size.max(1);
            let env_align = env_layout.layout.align.max(1);
            let env_slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                env_size as u32,
                common::align_to_shift(env_align),
            ));
            let env_addr = builder.ins().stack_addr(ptr_ty, env_slot, Offset32::new(0));
            common::zero_memory(builder, env_addr, env_layout.layout.size, ptr_ty);

            let env_def = &ctx.module.structs[env_struct.index()];
            let env_subst: HashMap<Entity, MirTy> = env_def
                .type_params
                .iter()
                .zip(env_type_args.iter())
                .map(|(tp, arg)| (tp.entity, arg.clone()))
                .collect();

            for (i, capture) in captures.iter().enumerate() {
                if i >= env_def.fields.len() || i >= env_layout.field_offsets.len() {
                    return Err(CodegenError::Unsupported(format!(
                        "ApplyPartial capture/env mismatch for '{}'",
                        original_func.name
                    )));
                }

                let field_ty = substitute_type(&env_def.fields[i].ty, &env_subst, ctx.module);
                let field_ptr = builder
                    .ins()
                    .iadd_imm(env_addr, env_layout.field_offsets[i] as i64);
                let val = rvalue::compile_value(ctx, state, builder, capture)?;

                if common::is_aggregate(&field_ty, &mut ctx.layouts) {
                    common::copy_aggregate(builder, &mut ctx.layouts, &field_ty, field_ptr, val);
                } else {
                    builder
                        .ins()
                        .store(MemFlags::new(), val, field_ptr, Offset32::new(0));
                }
            }

            env_addr
        },
        FunctionKind::Closure => {
            if !captures.is_empty() {
                return Err(CodegenError::Unsupported(format!(
                    "non-capturing closure '{}' unexpectedly had captures",
                    original_func.name
                )));
            }
            null
        },
        _ if captures.is_empty() => null,
        _ => {
            return Err(CodegenError::Unsupported(format!(
                "ApplyPartial with captures is only supported for closure call functions, got '{}'",
                original_func.name
            )));
        },
    };

    // Store env_ptr in the second word.
    builder.ins().store(
        MemFlags::new(),
        env_addr,
        thick_addr,
        Offset32::new(ptr_size as i32),
    );

    Ok(thick_addr)
}
