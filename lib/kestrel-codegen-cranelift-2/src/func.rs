use std::collections::HashSet;

use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, Value};
use cranelift_frontend::{FunctionBuilder, Variable};
use cranelift_module::{FuncId, Module};

use kestrel_mir_2::{
    ArgMode, BlockId, LocalId, MirBody, MonoFunction, Operand, PlaceBase, Rvalue, StatementKind,
};

use crate::abi::{self, PassMode, ReturnMode};
use crate::block;
use crate::context::CodegenCtx;
use crate::error::CodegenError;
use crate::mem;
use crate::ty::TypeRepr;

pub struct FuncCompiler<'a, 'm> {
    pub ctx: &'a mut CodegenCtx<'m>,
    pub func: &'m MonoFunction,
    pub body: &'m MirBody,
    pub block_map: Vec<ir::Block>,
    pub local_vars: Vec<Variable>,
    pub stack_locals: HashSet<LocalId>,
    pub is_main: bool,
    pub sret_ptr: Option<Value>,
}

pub fn compile_function(
    ctx: &mut CodegenCtx<'_>,
    func_idx: usize,
    func_id: FuncId,
) -> Result<(), CodegenError> {
    let func = &ctx.module.functions[func_idx];

    let body = match &func.body {
        Some(b) => b,
        None => return Ok(()),
    };

    let is_main = ctx.is_main_function(func);
    let call_conv = ctx.isa.default_call_conv();

    let sig = if func.extern_info.is_some() {
        crate::abi::build_extern_signature(func, &mut ctx.tc, &ctx.module.ty_arena, ctx.module, call_conv)
    } else {
        crate::abi::build_signature(func, is_main, &mut ctx.tc, &ctx.module.ty_arena, ctx.module, call_conv)
    };

    let ptr_ty = ctx.ptr_ty;

    // Scan for address-taken locals
    let stack_locals = collect_stack_locals(body, func, ctx);

    let mut cl_func = ir::Function::with_name_signature(
        ir::UserFuncName::user(0, func_idx as u32),
        sig.clone(),
    );

    // Take func_builder_ctx out temporarily to avoid double-borrow
    let mut fbc = std::mem::take(&mut ctx.func_builder_ctx);
    let mut builder = FunctionBuilder::new(&mut cl_func, &mut fbc);

    // Create Cranelift blocks
    let mut block_map = Vec::with_capacity(body.blocks.len());
    for _ in &body.blocks {
        block_map.push(builder.create_block());
    }

    // Set up entry block
    let entry = block_map[body.entry.index()];
    builder.append_block_params_for_function_params(entry);
    builder.switch_to_block(entry);

    // Declare variables for all locals.
    // Locals in stack_locals (address-taken, by-ref params, aggregates) hold
    // pointers; scalars not in the set hold the value directly.
    let mut local_vars = Vec::with_capacity(body.locals.len());
    for (i, local) in body.locals.iter().enumerate() {
        let repr = ctx.tc.repr(local.ty, &ctx.module.ty_arena, ctx.module);
        let cl_ty = if repr.is_scalar() && stack_locals.contains(&LocalId::new(i)) {
            ptr_ty
        } else {
            ctx.tc.cl_type(local.ty, &ctx.module.ty_arena, ctx.module)
        };
        let var = builder.declare_var(cl_ty);
        local_vars.push(var);
    }

    let ret_repr = ctx.tc.repr(func.ret, &ctx.module.ty_arena, ctx.module);
    let ret_mode = abi::return_mode(ret_repr, is_main);

    // Initialize parameters from entry block params
    let block_params = builder.block_params(entry).to_vec();
    let mut param_idx = 0;

    // Sret pointer
    let sret_ptr = if matches!(ret_mode, ReturnMode::Sret) {
        let ptr = block_params[param_idx];
        param_idx += 1;
        Some(ptr)
    } else {
        None
    };

    // Parameter locals
    for (i, param) in func.params.iter().enumerate() {
        let local_idx = i; // params occupy the first N locals
        let repr = ctx.tc.repr(param.ty, &ctx.module.ty_arena, ctx.module);
        let pass = abi::param_pass_mode(param.convention, repr, ptr_ty);

        match pass {
            PassMode::ByVal(_) => {
                let val = block_params[param_idx];
                param_idx += 1;

                if stack_locals.contains(&LocalId::new(local_idx)) {
                    // Scalar that's address-taken: allocate slot, store, def_var with addr
                    let slot =
                        mem::alloc_stack_slot(&mut builder, repr.size(), repr.align(), ptr_ty);
                    builder
                        .ins()
                        .store(MemFlags::new(), val, slot, Offset32::new(0));
                    builder.def_var(local_vars[local_idx], slot);
                } else {
                    builder.def_var(local_vars[local_idx], val);
                }
            }
            PassMode::ByRef => {
                let ptr = block_params[param_idx];
                param_idx += 1;
                // Variable holds the pointer itself
                builder.def_var(local_vars[local_idx], ptr);
            }
            PassMode::Zst => {
                let zero = builder.ins().iconst(ptr_ty, 0);
                builder.def_var(local_vars[local_idx], zero);
            }
        }
    }

    // Initialize non-parameter locals
    for i in func.params.len()..body.locals.len() {
        let local_ty = body.locals[i].ty;
        let repr = ctx.tc.repr(local_ty, &ctx.module.ty_arena, ctx.module);

        match repr {
            TypeRepr::Aggregate { size, align } => {
                let slot = mem::alloc_stack_slot(&mut builder, size, align, ptr_ty);
                mem::zero_memory(&mut builder, slot, size);
                builder.def_var(local_vars[i], slot);
            }
            TypeRepr::Scalar(_) if stack_locals.contains(&LocalId::new(i)) => {
                let slot =
                    mem::alloc_stack_slot(&mut builder, repr.size(), repr.align(), ptr_ty);
                mem::zero_memory(&mut builder, slot, repr.size());
                builder.def_var(local_vars[i], slot);
            }
            TypeRepr::Scalar(t) => {
                let zero = if t.is_int() || t == ptr_ty {
                    builder.ins().iconst(t, 0)
                } else if t == ir::types::F32 {
                    builder.ins().f32const(0.0)
                } else if t == ir::types::F64 {
                    builder.ins().f64const(0.0)
                } else {
                    builder.ins().iconst(t, 0)
                };
                builder.def_var(local_vars[i], zero);
            }
            TypeRepr::Zst => {
                let zero = builder.ins().iconst(ptr_ty, 0);
                builder.def_var(local_vars[i], zero);
            }
        }
    }

    // Build FuncCompiler and compile blocks
    let mut fc = FuncCompiler {
        ctx,
        func,
        body,
        block_map,
        local_vars,
        stack_locals,
        is_main,
        sret_ptr,
    };

    for i in 0..body.blocks.len() {
        let cl_block = fc.block_map[i];
        if i != body.entry.index() {
            builder.switch_to_block(cl_block);
        }
        block::compile_block(&mut fc, &mut builder, BlockId::new(i))?;
    }

    // Seal all blocks
    for &cl_block in &fc.block_map {
        builder.seal_block(cl_block);
    }

    builder.finalize();

    // Restore func_builder_ctx
    ctx.func_builder_ctx = fbc;

    // Optionally capture CLIF text
    if ctx.options.emit_clif {
        let clif_text = format!("{}", cl_func);
        ctx.clif_outputs.push((func.name.clone(), clif_text));
    }

    // Compile and define
    let mut comp_ctx = cranelift_codegen::Context::for_function(cl_func);
    comp_ctx
        .compile(ctx.isa.as_ref(), &mut Default::default())
        .map_err(|e| CodegenError::FunctionCompilation {
            name: func.name.clone(),
            source: Box::new(std::io::Error::other(format!("{e:?}"))),
        })?;

    ctx.cl_module
        .define_function(func_id, &mut comp_ctx)
        .map_err(|e| CodegenError::FunctionDefinition {
            name: func.name.clone(),
            source: e,
        })?;

    Ok(())
}

/// Scan the body for locals that need stack addresses
/// (address-taken via Ref/RefMut, or passed by Ref/RefMut to calls).
fn collect_stack_locals(body: &MirBody, func: &MonoFunction, ctx: &mut CodegenCtx<'_>) -> HashSet<LocalId> {
    let mut stack = HashSet::new();

    for block in &body.blocks {
        for stmt in &block.stmts {
            match &stmt.kind {
                StatementKind::Assign { rvalue, .. } => match rvalue {
                    Rvalue::Ref(p) | Rvalue::RefMut(p) => {
                        if let PlaceBase::Local(id) = &p.base {
                            stack.insert(*id);
                        }
                    }
                    _ => {}
                },
                StatementKind::Call { args, .. } => {
                    for (operand, mode) in args {
                        if matches!(mode, ArgMode::Ref | ArgMode::RefMut) {
                            if let Operand::Place(p) = operand {
                                if let PlaceBase::Local(id) = &p.base {
                                    stack.insert(*id);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Mark aggregate locals and by-ref scalar params as stack locals.
    // By-ref params hold a pointer even for scalar types.
    let ptr_ty = ctx.ptr_ty;
    for (i, local) in body.locals.iter().enumerate() {
        let repr = ctx.tc.repr(local.ty, &ctx.module.ty_arena, ctx.module);
        if matches!(repr, TypeRepr::Aggregate { .. }) {
            stack.insert(LocalId::new(i));
        } else if let Some(param) = func.params.get(i) {
            let pass = crate::abi::param_pass_mode(param.convention, repr, ptr_ty);
            if matches!(pass, crate::abi::PassMode::ByRef) {
                stack.insert(LocalId::new(i));
            }
        }
    }

    stack
}
