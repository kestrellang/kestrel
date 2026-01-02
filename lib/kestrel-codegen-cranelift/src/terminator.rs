//! Terminator compilation.

use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::place::compile_place_read;
use crate::rvalue::compile_value;

use kestrel_execution_graph::{Block, FunctionDef, Id, Local, Terminator, TerminatorKind, Value};

use cranelift_codegen::ir::types as cl_types;
use cranelift_codegen::ir::InstBuilder;
use cranelift_frontend::{FunctionBuilder, Variable};

use std::collections::HashMap;

/// Compile a block terminator.
pub fn compile_terminator(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    terminator: &Terminator,
    builder: &mut FunctionBuilder<'_>,
    block_map: &HashMap<Id<Block>, cranelift_codegen::ir::Block>,
    local_map: &HashMap<Id<Local>, Variable>,
    is_main: bool,
) -> Result<(), CodegenError> {
    match &terminator.kind {
        TerminatorKind::Return(value) => {
            let ret_ty = ctx.mir.ty(func_def.ret);
            if matches!(ret_ty, kestrel_execution_graph::MirTy::Unit) {
                if is_main {
                    // main() must return 0 for success exit code
                    let zero = builder.ins().iconst(cl_types::I64, 0);
                    builder.ins().return_(&[zero]);
                } else {
                    builder.ins().return_(&[]);
                }
            } else {
                // Check if we're trying to return unit in a non-unit function
                // This happens in unreachable code paths (e.g., after a loop with only returns)
                let is_unit_value = matches!(
                    value,
                    Value::Immediate(kestrel_execution_graph::Immediate {
                        kind: kestrel_execution_graph::ImmediateKind::Unit,
                        ..
                    })
                );
                if is_unit_value {
                    // This is dead code - emit trap
                    builder
                        .ins()
                        .trap(cranelift_codegen::ir::TrapCode::unwrap_user(1));
                } else {
                    let val = compile_value(ctx, func_def, value, builder, local_map)?;
                    builder.ins().return_(&[val]);
                }
            }
        }

        TerminatorKind::Jump(target) => {
            let cl_block = block_map
                .get(target)
                .ok_or_else(|| CodegenError::Unsupported("unknown jump target".to_string()))?;
            builder.ins().jump(*cl_block, &[]);
        }

        TerminatorKind::Branch {
            condition,
            then_block,
            else_block,
        } => {
            let cond = compile_value(ctx, func_def, condition, builder, local_map)?;
            let then_cl = block_map
                .get(then_block)
                .ok_or_else(|| CodegenError::Unsupported("unknown then block".to_string()))?;
            let else_cl = block_map
                .get(else_block)
                .ok_or_else(|| CodegenError::Unsupported("unknown else block".to_string()))?;
            builder.ins().brif(cond, *then_cl, &[], *else_cl, &[]);
        }

        TerminatorKind::Switch {
            discriminant,
            cases,
        } => {
            // TODO: Implement switch
            Err(CodegenError::Unsupported("switch terminator".to_string()))?
        }

        TerminatorKind::Panic(_msg) => {
            // TODO: Call panic handler
            builder
                .ins()
                .trap(cranelift_codegen::ir::TrapCode::unwrap_user(0));
        }

        TerminatorKind::Unreachable => {
            builder
                .ins()
                .trap(cranelift_codegen::ir::TrapCode::unwrap_user(1));
        }
    }

    Ok(())
}
