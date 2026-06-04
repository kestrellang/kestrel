//! Per-function lowering. Faithful port of the Cranelift backend's `func.rs`.
//!
//! Value model (the central invariant, identical to the Cranelift backend):
//!   - @owned scalar      -> the LLVM SSA value IS the scalar (i/f value)
//!   - @guaranteed scalar -> the value is an i64 ADDRESS of the scalar
//!                           (`resolve_scalar` loads through it)
//!   - aggregate (any)    -> the value is an i64 ADDRESS of the memory
//!   - ZST                -> a placeholder i64 `0`
//!
//! MIR block params become LLVM phi nodes (`block_phis`); the `Builder` is
//! threaded as a separate `&Builder` argument (never stored in the context) so
//! it never aliases the `&mut CodegenCtx` borrow.

use std::collections::HashMap;

use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::values::{AnyValue, BasicValueEnum, FunctionValue, IntValue, PhiValue};

use kestrel_mir::body::OssaBody;
use kestrel_mir::mono::{MonoFunction, MonoModule};
use kestrel_mir::value::Ownership;
use kestrel_mir::ValueId;

use crate::abi::{self, PassMode, ReturnMode};
use crate::context::CodegenCtx;
use crate::error::CodegenError;
use crate::mem;
use crate::ty::TypeRepr;
use crate::{inst, terminator};

pub struct FuncCompiler<'a, 'ctx> {
    pub ctx: &'a mut CodegenCtx<'ctx>,
    pub func: &'ctx MonoFunction,
    pub body: &'ctx OssaBody,
    pub fn_value: FunctionValue<'ctx>,
    pub entry_block: BasicBlock<'ctx>,
    pub block_map: Vec<BasicBlock<'ctx>>,
    pub block_phis: Vec<Vec<PhiValue<'ctx>>>,
    pub value_map: HashMap<ValueId, BasicValueEnum<'ctx>>,
    pub is_main: bool,
    /// sret destination address (i64), set when the return mode is `Sret`.
    pub sret_ptr: Option<IntValue<'ctx>>,
}

impl<'a, 'ctx> FuncCompiler<'a, 'ctx> {
    pub fn get_value(&self, id: ValueId) -> BasicValueEnum<'ctx> {
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

    /// Resolve a MIR value to its scalar form. A @guaranteed scalar is held as
    /// an i64 address, so it must be loaded; everything else is returned as-is.
    pub fn resolve_scalar(&mut self, builder: &Builder<'ctx>, id: ValueId) -> BasicValueEnum<'ctx> {
        let val = self.get_value(id);
        let ownership = self.body.values[id.index()].ownership;
        let ty = self.body.values[id.index()].ty;
        if ownership == Ownership::Guaranteed {
            let repr = self.ctx.tc.repr(ty, &self.ctx.module.ty_arena, self.ctx.module);
            if let TypeRepr::Scalar(t) = repr {
                let cx = self.ctx.cx;
                let p = mem::int_to_ptr(cx, builder, val.into_int_value());
                return builder.build_load(t.llvm(cx), p, "g").unwrap();
            }
        }
        val
    }

    pub fn map_value(&mut self, id: ValueId, val: BasicValueEnum<'ctx>) {
        self.value_map.insert(id, val);
    }

    /// Allocate a stack slot of `size` bytes aligned to `align`, returning its
    /// address as an i64. The alloca is hoisted to the entry block (matching
    /// Cranelift's fixed stack-slot semantics — not re-allocated per loop iter).
    pub fn alloca(&self, size: u64, align: u64) -> IntValue<'ctx> {
        let cx = self.ctx.cx;
        let ptr_size = self.ctx.ptr_size;
        let tmp = cx.create_builder();
        match self.entry_block.get_first_instruction() {
            Some(instr) => tmp.position_before(&instr),
            None => tmp.position_at_end(self.entry_block),
        }
        let arr_ty = cx.i8_type().array_type(size.max(1) as u32);
        let slot = tmp.build_alloca(arr_ty, "slot").unwrap();
        if let Some(instr) = slot.as_instruction() {
            let _ = instr.set_alignment(align.max(1) as u32);
        }
        mem::ptr_to_int(cx, &tmp, slot, ptr_size)
    }
}

pub fn compile_function<'ctx>(
    ctx: &mut CodegenCtx<'ctx>,
    func_idx: usize,
    fn_value: FunctionValue<'ctx>,
) -> Result<(), CodegenError> {
    let module: &'ctx MonoModule = ctx.module;
    let func = &module.functions[func_idx];
    let body = match &func.body {
        Some(b) => b,
        None => return Ok(()),
    };

    let is_main = ctx.is_main_function(func);
    let cx = ctx.cx;
    let ptr_scalar = ctx.tc.ptr_scalar;
    let ptr_size = ctx.ptr_size;

    let builder = cx.create_builder();

    // Create one LLVM block per MIR block.
    let mut block_map = Vec::with_capacity(body.blocks.len());
    for i in 0..body.blocks.len() {
        block_map.push(cx.append_basic_block(fn_value, &format!("bb{i}")));
    }
    let entry = block_map[body.entry.index()];

    let mut value_map: HashMap<ValueId, BasicValueEnum> = HashMap::new();
    let mut block_phis: Vec<Vec<PhiValue>> = (0..body.blocks.len()).map(|_| Vec::new()).collect();

    // Phi nodes for non-entry block params (entry params are function params).
    for (i, mir_block) in body.blocks.iter().enumerate() {
        if i == body.entry.index() {
            continue;
        }
        builder.position_at_end(block_map[i]);
        for param in &mir_block.params {
            let llty = if param.ownership == Ownership::Guaranteed {
                ptr_scalar.llvm(cx)
            } else {
                match ctx.tc.repr(param.ty, &module.ty_arena, module) {
                    TypeRepr::Scalar(t) => t.llvm(cx),
                    TypeRepr::Aggregate { .. } | TypeRepr::Zst => ptr_scalar.llvm(cx),
                }
            };
            let phi = builder.build_phi(llty, "phi").unwrap();
            value_map.insert(param.value, phi.as_basic_value());
            block_phis[i].push(phi);
        }
    }

    // Map function parameters in the entry block.
    builder.position_at_end(entry);
    let ret_repr = ctx.tc.repr(func.ret, &module.ty_arena, module);
    let ret_mode = abi::return_mode(ret_repr, is_main);

    let mut param_idx = 0u32;
    let sret_ptr = if matches!(ret_mode, ReturnMode::Sret) {
        let p = fn_value.get_nth_param(param_idx).unwrap().into_int_value();
        param_idx += 1;
        Some(p)
    } else {
        None
    };

    for i in 0..body.param_count.min(func.params.len()) {
        let value_id = ValueId::new(i);
        let param_ty = body.values[i].ty;
        let repr = ctx.tc.repr(param_ty, &module.ty_arena, module);
        match abi::param_pass_mode(func.params[i].convention, repr) {
            // ByVal: the scalar itself. ByRef: the i64 address. Both are the
            // nth LLVM param value as declared by `build_signature`.
            PassMode::ByVal(_) | PassMode::ByRef => {
                value_map.insert(value_id, fn_value.get_nth_param(param_idx).unwrap());
                param_idx += 1;
            },
            PassMode::Zst => {
                value_map.insert(value_id, mem::ptr_const(cx, ptr_size, 0).into());
            },
        }
    }

    let mut fc = FuncCompiler {
        ctx,
        func,
        body,
        fn_value,
        entry_block: entry,
        block_map,
        block_phis,
        value_map,
        is_main,
        sret_ptr,
    };

    // Lower each block.
    for i in 0..fc.body.blocks.len() {
        builder.position_at_end(fc.block_map[i]);
        let block = &fc.body.blocks[i];
        let mut diverged = false;
        for instruction in &block.insts {
            if inst::compile_inst(&mut fc, &builder, &instruction.kind)? {
                diverged = true;
                break;
            }
        }
        if !diverged {
            terminator::compile_terminator(&mut fc, &builder, &block.terminator)?;
        }
    }

    if fc.ctx.options.emit_ir {
        let text = fc.fn_value.print_to_string().to_string();
        fc.ctx.ir_outputs.push((func.name.clone(), text));
    }

    Ok(())
}
