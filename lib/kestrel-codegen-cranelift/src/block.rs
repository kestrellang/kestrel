//! Basic block compilation.

use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::place::compile_place_read;
use crate::rvalue::{compile_call, compile_rvalue};
use crate::terminator::compile_terminator;

use kestrel_execution_graph::{Block, FunctionDef, Id, Local, StatementKind};

use cranelift_codegen::ir::InstBuilder;
use cranelift_frontend::{FunctionBuilder, Variable};

use std::collections::HashMap;

/// Compile a single basic block.
pub fn compile_block(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    block_id: Id<Block>,
    builder: &mut FunctionBuilder<'_>,
    block_map: &HashMap<Id<Block>, cranelift_codegen::ir::Block>,
    local_map: &HashMap<Id<Local>, Variable>,
    is_main: bool,
) -> Result<(), CodegenError> {
    let block = ctx.mir.block(block_id);

    // Compile each statement
    for &stmt_id in &block.statements {
        let stmt = ctx.mir.statement(stmt_id);
        compile_statement(ctx, func_def, &stmt.kind, builder, local_map)?;
    }

    // Compile the terminator
    if let Some(ref terminator) = block.terminator {
        compile_terminator(
            ctx, func_def, terminator, builder, block_map, local_map, is_main,
        )?;
    } else {
        // Block has no terminator - this is dead code (unreachable)
        // Emit a trap to satisfy Cranelift's requirement that all blocks have terminators
        builder
            .ins()
            .trap(cranelift_codegen::ir::TrapCode::unwrap_user(1));
    }

    Ok(())
}

/// Compile a single statement.
fn compile_statement(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    stmt: &StatementKind,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<(), CodegenError> {
    match stmt {
        StatementKind::Assign { dest, rvalue } => {
            let value = compile_rvalue(ctx, func_def, rvalue, builder, local_map)?;
            crate::place::compile_place_write(ctx, dest, value, builder, local_map)?;
        }

        StatementKind::Call { callee, args } => {
            // Call without using the result - we just discard the return value
            let _ = compile_call(ctx, func_def, callee, args, builder, local_map)?;
        }

        StatementKind::Deinit { place } => {
            // TODO: Implement destructor calls
        }

        StatementKind::DeinitIf { place, flag } => {
            // TODO: Implement conditional destructor calls
        }

        StatementKind::SetDeinitFlag { flag, value } => {
            // TODO: Implement deinit flag setting
        }
    }

    Ok(())
}
