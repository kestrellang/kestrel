use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, AbiParam, InstBuilder, MemFlags, Value};
use cranelift_frontend::FunctionBuilder;

use cranelift_module::Module;
use kestrel_mir_2::{ArgMode, Callee, MirTy, MonoFuncId, Operand, Place};

use crate::abi::{self, PassMode, ReturnMode};
use crate::error::CodegenError;
use crate::func::FuncCompiler;
use crate::{mem, place, rvalue};

pub fn compile_call(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    callee: &Callee,
    args: &[(Operand, ArgMode)],
    dest: Option<&Place>,
) -> Result<(), CodegenError> {
    match callee {
        Callee::Resolved(mono_id) => compile_resolved_call(fc, builder, *mono_id, args, dest),
        Callee::Thin(place) => compile_thin_call(fc, builder, place, args, dest),
        Callee::Thick(place) => compile_thick_call(fc, builder, place, args, dest),
        Callee::Direct { .. } | Callee::Witness { .. } => {
            debug_assert!(false, "unresolved callee in codegen: {callee:?}");
            Err(CodegenError::Unsupported(
                "unresolved callee post-mono".into(),
            ))
        }
    }
}

fn compile_resolved_call(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    mono_id: MonoFuncId,
    args: &[(Operand, ArgMode)],
    dest: Option<&Place>,
) -> Result<(), CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;
    let target_func = &fc.ctx.module.functions[mono_id.index()];

    let func_id = fc.ctx.func_ids[mono_id.index()].ok_or_else(|| {
        CodegenError::Unsupported(format!(
            "resolved function {} not declared",
            target_func.name
        ))
    })?;
    let func_ref = fc.ctx.cl_module.declare_func_in_func(func_id, builder.func);

    let ret_repr = fc
        .ctx
        .tc
        .repr(target_func.ret, &fc.ctx.module.ty_arena, fc.ctx.module);
    let is_main = fc.ctx.is_main_function(target_func);
    let ret_mode = abi::return_mode(ret_repr, is_main);

    let mut call_args: Vec<Value> = Vec::new();

    // Sret: allocate return slot, pass as first arg
    let sret_slot = if matches!(ret_mode, ReturnMode::Sret) {
        let slot = mem::alloc_stack_slot(builder, ret_repr.size(), ret_repr.align(), ptr_ty);
        call_args.push(slot);
        Some(slot)
    } else {
        None
    };

    // Build argument values
    for (i, (operand, arg_mode)) in args.iter().enumerate() {
        if i >= target_func.params.len() {
            break;
        }
        let param = &target_func.params[i];
        let repr = fc
            .ctx
            .tc
            .repr(param.ty, &fc.ctx.module.ty_arena, fc.ctx.module);
        let pass = abi::param_pass_mode(param.convention, repr, ptr_ty);

        match pass {
            PassMode::ByVal(_) => {
                let val = match arg_mode {
                    ArgMode::Ref | ArgMode::RefMut => {
                        // Caller wants to pass by ref but ABI is by-val — shouldn't happen
                        // for Consuming params. Compile as operand value.
                        rvalue::compile_operand(fc, builder, operand)?
                    }
                    ArgMode::Copy | ArgMode::Move => {
                        rvalue::compile_operand(fc, builder, operand)?
                    }
                };
                call_args.push(val);
            }
            PassMode::ByRef => {
                let addr = match arg_mode {
                    ArgMode::Ref | ArgMode::RefMut => {
                        if let Operand::Place(p) = operand {
                            place::place_addr(fc, builder, p)?
                        } else {
                            // Constant passed by ref: spill to stack
                            let val = rvalue::compile_operand(fc, builder, operand)?;
                            let slot =
                                mem::alloc_stack_slot(builder, repr.size(), repr.align(), ptr_ty);
                            mem::store_to_repr(builder, repr, slot, val);
                            slot
                        }
                    }
                    ArgMode::Copy | ArgMode::Move => {
                        // Consuming aggregate: operand is already a pointer (place_read returns ptr for aggregates)
                        let val = rvalue::compile_operand(fc, builder, operand)?;
                        if repr.is_aggregate() {
                            // Value is already a pointer
                            val
                        } else {
                            // Scalar that needs to be passed by ref (e.g., borrow convention)
                            let slot = mem::alloc_stack_slot(
                                builder,
                                repr.size(),
                                repr.align(),
                                ptr_ty,
                            );
                            mem::store_to_repr(builder, repr, slot, val);
                            slot
                        }
                    }
                };
                call_args.push(addr);
            }
            PassMode::Zst => {}
        }
    }

    let inst = builder.ins().call(func_ref, &call_args);

    // Handle return value
    write_call_result(fc, builder, inst, ret_mode, sret_slot, dest)
}

fn compile_thin_call(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    func_place: &Place,
    args: &[(Operand, ArgMode)],
    dest: Option<&Place>,
) -> Result<(), CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;
    let func_ptr = place::place_read(fc, builder, func_place)?;

    // Infer return type from the place's FuncThin type
    let func_ty = place::place_type(func_place, fc.body, &fc.ctx.module.ty_arena, fc.ctx.module, &fc.ctx.tc);
    let arena = &fc.ctx.module.ty_arena;

    let (param_tys, ret_ty) = if let MirTy::FuncThin { params, ret } = arena.get(func_ty) {
        (params.clone(), *ret)
    } else {
        return Err(CodegenError::Unsupported("thin call on non-FuncThin".into()));
    };

    let ret_repr = fc.ctx.tc.repr(ret_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
    let ret_mode = abi::return_mode(ret_repr, false);

    // Build signature
    let call_conv = fc.ctx.isa.default_call_conv();
    let mut sig = ir::Signature::new(call_conv);

    if matches!(ret_mode, ReturnMode::Sret) {
        sig.params.push(AbiParam::new(ptr_ty));
    }
    for (ty, convention) in &param_tys {
        let repr = fc.ctx.tc.repr(*ty, arena, fc.ctx.module);
        match abi::param_pass_mode(*convention, repr, ptr_ty) {
            PassMode::ByVal(t) => sig.params.push(AbiParam::new(t)),
            PassMode::ByRef => sig.params.push(AbiParam::new(ptr_ty)),
            PassMode::Zst => {}
        }
    }
    match ret_mode {
        ReturnMode::Direct(t) => sig.returns.push(AbiParam::new(t)),
        _ => {}
    }

    let sig_ref = builder.import_signature(sig);

    let mut call_args: Vec<Value> = Vec::new();
    let sret_slot = if matches!(ret_mode, ReturnMode::Sret) {
        let slot = mem::alloc_stack_slot(builder, ret_repr.size(), ret_repr.align(), ptr_ty);
        call_args.push(slot);
        Some(slot)
    } else {
        None
    };

    for (i, (operand, arg_mode)) in args.iter().enumerate() {
        if i >= param_tys.len() {
            break;
        }
        let (ty, convention) = &param_tys[i];
        let repr = fc.ctx.tc.repr(*ty, arena, fc.ctx.module);
        let pass = abi::param_pass_mode(*convention, repr, ptr_ty);
        let val = build_call_arg(fc, builder, operand, *arg_mode, pass, repr, ptr_ty)?;
        if let Some(v) = val {
            call_args.push(v);
        }
    }

    let inst = builder.ins().call_indirect(sig_ref, func_ptr, &call_args);

    write_call_result(fc, builder, inst, ret_mode, sret_slot, dest)
}

fn compile_thick_call(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    closure_place: &Place,
    args: &[(Operand, ArgMode)],
    dest: Option<&Place>,
) -> Result<(), CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;
    let ptr_size = fc.ctx.ptr_size;

    let closure_ptr = place::place_read(fc, builder, closure_place)?;

    // Load func_ptr and env_ptr from thick closure {func_ptr, env_ptr}
    let func_ptr = builder
        .ins()
        .load(ptr_ty, MemFlags::new(), closure_ptr, Offset32::new(0));
    let env_ptr = builder.ins().load(
        ptr_ty,
        MemFlags::new(),
        closure_ptr,
        Offset32::new(ptr_size as i32),
    );

    // Infer types from FuncThick
    let func_ty = place::place_type(closure_place, fc.body, &fc.ctx.module.ty_arena, fc.ctx.module, &fc.ctx.tc);
    let arena = &fc.ctx.module.ty_arena;

    let (param_tys, ret_ty) = if let MirTy::FuncThick { params, ret } = arena.get(func_ty) {
        (params.clone(), *ret)
    } else {
        return Err(CodegenError::Unsupported(
            "thick call on non-FuncThick".into(),
        ));
    };

    let ret_repr = fc.ctx.tc.repr(ret_ty, arena, fc.ctx.module);
    let ret_mode = abi::return_mode(ret_repr, false);

    // Build signature: env_ptr as first arg, then params
    let call_conv = fc.ctx.isa.default_call_conv();
    let mut sig = ir::Signature::new(call_conv);

    if matches!(ret_mode, ReturnMode::Sret) {
        sig.params.push(AbiParam::new(ptr_ty));
    }
    sig.params.push(AbiParam::new(ptr_ty)); // env_ptr
    for (ty, convention) in &param_tys {
        let repr = fc.ctx.tc.repr(*ty, arena, fc.ctx.module);
        match abi::param_pass_mode(*convention, repr, ptr_ty) {
            PassMode::ByVal(t) => sig.params.push(AbiParam::new(t)),
            PassMode::ByRef => sig.params.push(AbiParam::new(ptr_ty)),
            PassMode::Zst => {}
        }
    }
    match ret_mode {
        ReturnMode::Direct(t) => sig.returns.push(AbiParam::new(t)),
        _ => {}
    }

    let sig_ref = builder.import_signature(sig);

    let mut call_args: Vec<Value> = Vec::new();
    let sret_slot = if matches!(ret_mode, ReturnMode::Sret) {
        let slot = mem::alloc_stack_slot(builder, ret_repr.size(), ret_repr.align(), ptr_ty);
        call_args.push(slot);
        Some(slot)
    } else {
        None
    };

    call_args.push(env_ptr);
    for (i, (operand, arg_mode)) in args.iter().enumerate() {
        if i >= param_tys.len() {
            break;
        }
        let (ty, convention) = &param_tys[i];
        let repr = fc.ctx.tc.repr(*ty, arena, fc.ctx.module);
        let pass = abi::param_pass_mode(*convention, repr, ptr_ty);
        let val = build_call_arg(fc, builder, operand, *arg_mode, pass, repr, ptr_ty)?;
        if let Some(v) = val {
            call_args.push(v);
        }
    }

    let inst = builder.ins().call_indirect(sig_ref, func_ptr, &call_args);

    write_call_result(fc, builder, inst, ret_mode, sret_slot, dest)
}

/// Build a single call argument, respecting the callee's PassMode.
/// Returns None for Zst (no ABI slot).
fn build_call_arg(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    operand: &Operand,
    arg_mode: ArgMode,
    pass: PassMode,
    repr: crate::ty::TypeRepr,
    ptr_ty: ir::Type,
) -> Result<Option<Value>, CodegenError> {
    match pass {
        PassMode::ByVal(_) => {
            let val = rvalue::compile_operand(fc, builder, operand)?;
            Ok(Some(val))
        }
        PassMode::ByRef => {
            let addr = match arg_mode {
                ArgMode::Ref | ArgMode::RefMut => {
                    if let Operand::Place(p) = operand {
                        place::place_addr(fc, builder, p)?
                    } else {
                        let val = rvalue::compile_operand(fc, builder, operand)?;
                        let slot = mem::alloc_stack_slot(builder, repr.size(), repr.align(), ptr_ty);
                        mem::store_to_repr(builder, repr, slot, val);
                        slot
                    }
                }
                ArgMode::Copy | ArgMode::Move => {
                    let val = rvalue::compile_operand(fc, builder, operand)?;
                    if repr.is_aggregate() {
                        val
                    } else {
                        let slot = mem::alloc_stack_slot(builder, repr.size(), repr.align(), ptr_ty);
                        mem::store_to_repr(builder, repr, slot, val);
                        slot
                    }
                }
            };
            Ok(Some(addr))
        }
        PassMode::Zst => Ok(None),
    }
}

/// Write the call result to the destination place.
fn write_call_result(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    inst: ir::Inst,
    ret_mode: ReturnMode,
    sret_slot: Option<Value>,
    dest: Option<&Place>,
) -> Result<(), CodegenError> {
    let Some(dest) = dest else {
        return Ok(());
    };

    match ret_mode {
        ReturnMode::Direct(_) => {
            let result = builder.inst_results(inst)[0];
            place::place_write(fc, builder, dest, result)?;
        }
        ReturnMode::Sret => {
            let slot = sret_slot.expect("sret slot must exist");
            place::place_write(fc, builder, dest, slot)?;
        }
        ReturnMode::Void => {}
    }

    Ok(())
}
