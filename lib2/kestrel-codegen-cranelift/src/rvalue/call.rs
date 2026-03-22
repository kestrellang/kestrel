//! Function call compilation.
//!
//! Handles all four Callee variants: Direct, Witness, Thin, Thick.
//! Key improvement: Direct and Witness share `compile_resolved_call` after
//! resolution, eliminating ~150 lines of duplication from lib1.

use crate::common::{self, is_aggregate_type, needs_sret};
use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::function::FunctionState;
use crate::monomorphize::witness;
use crate::place;
use crate::rvalue;
use crate::types;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, AbiParam, InstBuilder, MemFlags, Signature, StackSlotData, StackSlotKind, Value as CrValue};
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use kestrel_codegen2::{mangle_function_with_self, substitute_type, substitute_type_with_self};
use kestrel_mir::{Callee, CallArg, MirTy, PassingMode, Place, Value};

/// Compile a function call statement.
pub fn compile_call(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    callee: &Callee,
    args: &[CallArg],
    dest: Option<&Place>,
) -> Result<(), CodegenError> {
    match callee {
        Callee::Direct {
            func,
            type_args,
            self_type,
        } => {
            // Resolve the concrete type args
            let concrete_type_args: Vec<MirTy> = type_args
                .iter()
                .map(|a| substitute_type(a, &state.subst))
                .collect();
            let concrete_self = self_type
                .as_ref()
                .map(|st| substitute_type(st, &state.subst));

            let func_id_mir = ctx.entity_to_func.get(func).ok_or_else(|| {
                let name = ctx.module.resolve_name(*func);
                CodegenError::Unsupported(format!(
                    "call to unknown function entity {:?} ({})", func, name
                ))
            })?;
            let func_def = &ctx.module.functions[func_id_mir.index()];
            let mangled = mangle_function_with_self(
                ctx.module,
                func_def,
                &concrete_type_args,
                concrete_self.as_ref(),
            );

            compile_resolved_call(ctx, state, builder, &mangled, func_def, args, dest)?;
        }

        Callee::Witness {
            protocol,
            method,
            self_type,
            method_type_args,
        } => {
            // Substitute type params AND SelfType using the function's self_type
            let mut concrete_self = substitute_type_with_self(
                self_type, &state.subst, state.self_type.as_ref(),
            );
            // Resolve associated types (e.g., Iterator.Item → Int64) via witness table
            concrete_self = resolve_associated_self_type(ctx, &state, &concrete_self);
            let concrete_method_args: Vec<MirTy> = method_type_args
                .iter()
                .map(|a| substitute_type_with_self(a, &state.subst, state.self_type.as_ref()))
                .collect();

            let resolved = witness::resolve_witness_call(
                ctx.module,
                *protocol,
                method,
                &concrete_self,
                &concrete_method_args,
            )
            .map_err(|e| CodegenError::Monomorphization(format!("{e}")))?;

            let func_id_mir = ctx.entity_to_func.get(&resolved.func_entity).ok_or_else(|| {
                CodegenError::Unsupported(format!(
                    "witness resolved to unknown entity {:?}",
                    resolved.func_entity
                ))
            })?;
            let func_def = &ctx.module.functions[func_id_mir.index()];
            let mangled = mangle_function_with_self(
                ctx.module,
                func_def,
                &resolved.type_args,
                resolved.self_type.as_ref(),
            );

            compile_resolved_call(ctx, state, builder, &mangled, func_def, args, dest)?;
        }

        Callee::Thin(place) => {
            compile_indirect_call(ctx, state, builder, place, args, dest, false)?;
        }

        Callee::Thick(place) => {
            compile_indirect_call(ctx, state, builder, place, args, dest, true)?;
        }
    }

    Ok(())
}

/// Shared call emission after resolving the target function.
/// Used by both Direct and Witness callees.
fn compile_resolved_call(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    mangled: &str,
    func_def: &kestrel_mir::FunctionDef,
    args: &[CallArg],
    dest: Option<&Place>,
) -> Result<(), CodegenError> {
    let ptr_ty = common::ptr_type(ctx.target);

    let func_id = ctx.func_ids_by_name.get(mangled).ok_or_else(|| {
        CodegenError::Unsupported(format!("call to undeclared function: {mangled}"))
    })?;
    let func_ref = ctx.cl_module.declare_func_in_func(*func_id, builder.func);

    // Determine if the callee uses sret
    let ret_ty = substitute_type(&func_def.ret, &state.subst);
    let callee_sret = !ctx.is_main_function(func_def) && needs_sret(&ret_ty);

    // Build argument list
    let mut cl_args: Vec<CrValue> = Vec::new();

    // If sret, allocate a stack slot and pass as first arg
    let sret_addr = if callee_sret {
        let layout = ctx.layouts.layout_of(&ret_ty);
        let slot = builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            layout.size as u32,
            common::align_to_shift(layout.align),
        ));
        let addr = builder.ins().stack_addr(ptr_ty, slot, Offset32::new(0));
        common::zero_memory(builder, addr, layout.size, ptr_ty);
        cl_args.push(addr);
        Some(addr)
    } else {
        None
    };

    // Compile each argument
    for arg in args {
        let val = compile_call_arg(ctx, state, builder, arg)?;
        cl_args.push(val);
    }

    let inst = builder.ins().call(func_ref, &cl_args);

    // Handle return value
    if let Some(dest_place) = dest {
        if callee_sret {
            // Result is in the sret slot
            place::compile_place_write(ctx, state, builder, dest_place, sret_addr.unwrap())?;
        } else if !matches!(ret_ty, MirTy::Unit | MirTy::Never) {
            let result = builder.inst_results(inst)[0];
            place::compile_place_write(ctx, state, builder, dest_place, result)?;
        }
    }

    Ok(())
}

/// Compile an indirect call through a function pointer (thin or thick).
fn compile_indirect_call(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    callee_place: &Place,
    args: &[CallArg],
    dest: Option<&Place>,
    is_thick: bool,
) -> Result<(), CodegenError> {
    let ptr_ty = common::ptr_type(ctx.target);
    let callee_val = place::compile_place_read(ctx, state, builder, callee_place)?;

    if is_thick {
        // Thick callable: (func_ptr, env_ptr)
        let func_ptr = builder.ins().load(ptr_ty, MemFlags::new(), callee_val, Offset32::new(0));
        let env_ptr = builder.ins().load(
            ptr_ty,
            MemFlags::new(),
            callee_val,
            Offset32::new(ctx.target.pointer_size() as i32),
        );

        // Build signature: env_ptr as first arg, then regular args
        let mut sig = Signature::new(CallConv::Fast);
        sig.params.push(AbiParam::new(ptr_ty)); // env ptr
        for _arg in args {
            sig.params.push(AbiParam::new(ptr_ty)); // Simplified: all args as ptr
        }
        // TODO: determine return type properly
        sig.returns.push(AbiParam::new(ptr_ty));

        let sig_ref = builder.import_signature(sig);
        let mut cl_args = vec![env_ptr];
        for arg in args {
            cl_args.push(compile_call_arg(ctx, state, builder, arg)?);
        }

        let inst = builder.ins().call_indirect(sig_ref, func_ptr, &cl_args);

        if let Some(dest_place) = dest {
            let result = builder.inst_results(inst)[0];
            place::compile_place_write(ctx, state, builder, dest_place, result)?;
        }
    } else {
        // Thin function pointer: just the function address
        let mut sig = Signature::new(CallConv::Fast);
        for _arg in args {
            sig.params.push(AbiParam::new(ptr_ty));
        }
        sig.returns.push(AbiParam::new(ptr_ty));

        let sig_ref = builder.import_signature(sig);
        let mut cl_args = Vec::new();
        for arg in args {
            cl_args.push(compile_call_arg(ctx, state, builder, arg)?);
        }

        let inst = builder.ins().call_indirect(sig_ref, callee_val, &cl_args);

        if let Some(dest_place) = dest {
            let result = builder.inst_results(inst)[0];
            place::compile_place_write(ctx, state, builder, dest_place, result)?;
        }
    }

    Ok(())
}

/// Compile a call argument based on its passing mode.
fn compile_call_arg(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    arg: &CallArg,
) -> Result<CrValue, CodegenError> {
    match arg.mode {
        PassingMode::Ref | PassingMode::MutRef => {
            // Pass by reference: take the address
            match &arg.value {
                Value::Place(p) => place::compile_place_addr(ctx, state, builder, p),
                Value::Immediate(_) => {
                    // Need to materialize on stack to take address
                    let val = rvalue::compile_value(ctx, state, builder, &arg.value)?;
                    let ptr_ty = common::ptr_type(ctx.target);
                    let slot = builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        8,
                        3,
                    ));
                    let addr = builder.ins().stack_addr(ptr_ty, slot, Offset32::new(0));
                    builder.ins().store(MemFlags::new(), val, addr, Offset32::new(0));
                    Ok(addr)
                }
            }
        }

        PassingMode::Copy | PassingMode::Move => {
            rvalue::compile_value(ctx, state, builder, &arg.value)
        }
    }
}

/// Resolve a self_type that might be an associated type (e.g., Iterator.Item).
/// Checks if the type is a Named entity that's a TypeAlias on a protocol,
/// and if so, resolves it through the witness table using the current
/// function's self_type as the implementing type.
fn resolve_associated_self_type(
    ctx: &CodegenContext,
    state: &FunctionState,
    self_type: &MirTy,
) -> MirTy {
    // Check if self_type is Named with a TypeAlias entity (associated type)
    let entity = match self_type {
        MirTy::Named { entity, type_args } if type_args.is_empty() => *entity,
        MirTy::Named { .. } => return self_type.clone(),
        _ => return self_type.clone(),
    };

    // Check if this entity is a TypeAlias by looking at its parent
    // (TypeAliases on protocols are associated types)
    let parent_protocol = ctx.module.protocols.iter().find(|p| {
        p.associated_types.iter().any(|at| {
            // Match by name since we don't have the entity ID in ProtocolDef
            let assoc_name = ctx.module.resolve_name(entity);
            // The last component of the name is the associated type name
            let short_name = assoc_name.rsplit('.').next().unwrap_or(&assoc_name);
            at.name == short_name
        })
    });

    let Some(proto_def) = parent_protocol else {
        return self_type.clone();
    };

    // Get the associated type short name
    let assoc_name = ctx.module.resolve_name(entity);
    let short_name = assoc_name.rsplit('.').next().unwrap_or(&assoc_name).to_string();

    // Use the current function's self_type to find the concrete associated type
    let base_self = state.self_type.as_ref();
    let Some(base) = base_self else {
        return self_type.clone();
    };

    // Look up in witness table
    match witness::resolve_associated_type(ctx.module, proto_def.entity, base, &short_name) {
        Ok(resolved) => resolved,
        Err(_) => self_type.clone(),
    }
}
