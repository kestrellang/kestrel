//! String operation compilation.
//!
//! Only StrPtr and StrLen are actually emitted by lib2's MIR lowering.
//! String equality goes through the `.equals()` method protocol.
//! IntToString doesn't exist in lib2's MIR.

use crate::common;
use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::function::FunctionState;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, Value as CrValue};
use cranelift_frontend::FunctionBuilder;
use kestrel_mir::Op;

/// Compile string unary ops (StrPtr, StrLen).
pub fn compile_string_op1(
    ctx: &mut CodegenContext,
    _state: &FunctionState,
    builder: &mut FunctionBuilder,
    op: &Op,
    arg: CrValue,
) -> Result<CrValue, CodegenError> {
    let ptr_ty = common::ptr_type(ctx.target);
    let ptr_size = ctx.target.pointer_size() as i32;

    match op {
        // String is a fat pointer: (ptr, len)
        Op::StrPtr => Ok(builder.ins().load(ptr_ty, MemFlags::new(), arg, Offset32::new(0))),
        Op::StrLen => Ok(builder.ins().load(
            ir::types::I64,
            MemFlags::new(),
            arg,
            Offset32::new(ptr_size),
        )),
        _ => Err(CodegenError::Unsupported(format!("string op1: {op:?}"))),
    }
}
