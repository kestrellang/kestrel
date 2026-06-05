use std::collections::HashMap;

use cranelift_codegen::ir::{self, InstBuilder, Value};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::{FuncId, Module};

use kestrel_mir::ValueId;
use kestrel_mir::body::OssaBody;
use kestrel_mir::mono::MonoFunction;

use crate::abi::{self, PassMode, ReturnMode};
use crate::context::CodegenCtx;
use crate::error::CodegenError;
use crate::ty::TypeRepr;
use crate::{inst, terminator};

pub struct FuncCompiler<'a, 'm> {
    pub ctx: &'a mut CodegenCtx<'m>,
    pub func: &'m MonoFunction,
    pub body: &'m OssaBody,
    pub block_map: Vec<ir::Block>,
    pub value_map: HashMap<ValueId, Value>,
    pub sret_ptr: Option<Value>,
}

impl<'a, 'm> FuncCompiler<'a, 'm> {
    pub fn get_value(&self, _builder: &mut FunctionBuilder, id: ValueId) -> Value {
        *self.value_map.get(&id).unwrap_or_else(|| {
            let info = if id.index() < self.body.values.len() {
                let vd = &self.body.values[id.index()];
                format!("ty={:?} own={:?}", vd.ty, vd.ownership)
            } else {
                "OOB".into()
            };
            panic!(
                "ICE: ValueId {:?} ({}) not in value_map (func={})",
                id, info, self.func.name
            )
        })
    }

    /// Get the scalar value for a MIR ValueId. If the value is @guaranteed
    /// and the type is a scalar, loads from the ByRef pointer first.
    pub fn resolve_scalar(&mut self, builder: &mut FunctionBuilder, id: ValueId) -> Value {
        let val = self.get_value(builder, id);
        let vd = &self.body.values[id.index()];
        if vd.ownership == kestrel_mir::value::Ownership::Guaranteed {
            let repr = self
                .ctx
                .tc
                .repr(vd.ty, &self.ctx.module.ty_arena, self.ctx.module);
            if let crate::ty::TypeRepr::Scalar(t) = repr {
                return builder.ins().load(
                    t,
                    ir::MemFlags::new(),
                    val,
                    ir::immediates::Offset32::new(0),
                );
            }
        }
        val
    }

    pub fn map_value(&mut self, builder: &mut FunctionBuilder, id: ValueId, val: Value) {
        if cfg!(debug_assertions) {
            self.verify_value_repr(builder, id, val);
        }
        self.value_map.insert(id, val);
    }

    /// Debug-only check: the Cranelift value type must match the MIR ownership.
    ///
    /// @guaranteed scalars are pointers (ptr_ty). @owned scalars are the scalar
    /// type itself. A mismatch means a pointer leaked through without deref
    /// (or a deref happened where a pointer was expected).
    fn verify_value_repr(&self, builder: &FunctionBuilder, id: ValueId, val: Value) {
        let vd = &self.body.values[id.index()];
        let repr = self.ctx.tc.cached_repr(vd.ty);
        let Some(TypeRepr::Scalar(expected_scalar)) = repr else {
            return;
        };

        let cl_ty = builder.func.dfg.value_type(val);
        let ptr_ty = self.ctx.ptr_ty;

        match vd.ownership {
            kestrel_mir::value::Ownership::Owned => {
                if cl_ty == ptr_ty && expected_scalar != ptr_ty {
                    eprintln!(
                        "VERIFY: @owned scalar {} mapped to ptr_ty (expected {:?}) in {}",
                        id.index(),
                        expected_scalar,
                        self.func.name,
                    );
                }
            },
            kestrel_mir::value::Ownership::Guaranteed => {
                if cl_ty != ptr_ty {
                    eprintln!(
                        "VERIFY: @guaranteed scalar {} mapped to {:?} (expected ptr_ty) in {}",
                        id.index(),
                        cl_ty,
                        self.func.name,
                    );
                }
            },
        }
    }
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

    let call_conv = ctx.isa.default_call_conv();

    let sig = if func.extern_info.is_some() {
        crate::abi::build_extern_signature(
            func,
            &mut ctx.tc,
            &ctx.module.ty_arena,
            ctx.module,
            call_conv,
        )
    } else {
        crate::abi::build_signature(
            func,
            &mut ctx.tc,
            &ctx.module.ty_arena,
            ctx.module,
            call_conv,
        )
    };

    let ptr_ty = ctx.ptr_ty;

    let mut cl_func =
        ir::Function::with_name_signature(ir::UserFuncName::user(0, func_id.as_u32()), sig.clone());

    let mut fbc = std::mem::take(&mut ctx.func_builder_ctx);
    let mut builder = FunctionBuilder::new(&mut cl_func, &mut fbc);

    // Create Cranelift blocks
    let mut block_map = Vec::with_capacity(body.blocks.len());
    for _ in &body.blocks {
        block_map.push(builder.create_block());
    }

    let mut value_map: HashMap<ValueId, Value> = HashMap::new();

    // Set up block params for non-entry blocks
    for (i, mir_block) in body.blocks.iter().enumerate() {
        if i == body.entry.index() {
            continue;
        }
        let cl_block = block_map[i];
        for param in &mir_block.params {
            let cl_ty = if param.ownership == kestrel_mir::value::Ownership::Guaranteed {
                ptr_ty
            } else {
                let repr = ctx.tc.repr(param.ty, &ctx.module.ty_arena, ctx.module);
                match repr {
                    TypeRepr::Scalar(t) => t,
                    TypeRepr::Aggregate { .. } | TypeRepr::Zst => ptr_ty,
                }
            };
            let cl_val = builder.append_block_param(cl_block, cl_ty);
            value_map.insert(param.value, cl_val);
        }
    }

    // Entry block
    let entry = block_map[body.entry.index()];
    builder.append_block_params_for_function_params(entry);
    builder.switch_to_block(entry);

    let ret_repr = ctx.tc.repr(func.ret, &ctx.module.ty_arena, ctx.module);
    let ret_mode = abi::return_mode(ret_repr);

    let block_params_entry = builder.block_params(entry).to_vec();
    let mut param_idx = 0;

    let sret_ptr = if matches!(ret_mode, ReturnMode::Sret) {
        let ptr = block_params_entry[param_idx];
        param_idx += 1;
        Some(ptr)
    } else {
        None
    };

    // Map function parameters
    for i in 0..body.param_count.min(func.params.len()) {
        let value_id = ValueId::new(i);
        let param_ty = body.values[i].ty;
        let repr = ctx.tc.repr(param_ty, &ctx.module.ty_arena, ctx.module);
        let pass = abi::param_pass_mode(func.params[i].convention, repr, ptr_ty);
        match pass {
            PassMode::ByVal(_) => {
                value_map.insert(value_id, block_params_entry[param_idx]);
                param_idx += 1;
            },
            PassMode::ByRef => {
                let ptr_val = block_params_entry[param_idx];
                param_idx += 1;
                // ByRef params are always pointers — struct_extract and
                // other instructions handle loading from the address.
                value_map.insert(value_id, ptr_val);
            },
            PassMode::Zst => {
                let zero = builder.ins().iconst(ptr_ty, 0);
                value_map.insert(value_id, zero);
            },
        }
    }

    let mut fc = FuncCompiler {
        ctx,
        func,
        body,
        block_map,
        value_map,
        sret_ptr,
    };

    // Compile each block
    for i in 0..body.blocks.len() {
        let cl_block = fc.block_map[i];
        if i != body.entry.index() {
            builder.switch_to_block(cl_block);
        }

        let block = &fc.body.blocks[i];
        let mut diverged = false;
        for instruction in &block.insts {
            if inst::compile_inst(&mut fc, &mut builder, &instruction.kind)? {
                diverged = true;
                break;
            }
        }

        if !diverged {
            terminator::compile_terminator(&mut fc, &mut builder, &block.terminator)?;
        }
    }

    // Seal all blocks
    for &cl_block in &fc.block_map {
        builder.seal_block(cl_block);
    }

    builder.finalize();

    ctx.func_builder_ctx = fbc;

    if ctx.options.emit_clif {
        let clif_text = format!("{}", cl_func);
        ctx.clif_outputs.push((func.name.clone(), clif_text));
    }

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
            source: Box::new(e),
        })?;

    Ok(())
}
