//! Block and statement compilation.

use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::function::FunctionState;
use crate::place;
use crate::rvalue;
use crate::terminator;
use cranelift_codegen::ir::InstBuilder;
use cranelift_frontend::FunctionBuilder;
use kestrel_mir::{BlockId, StatementKind};

/// Compile a single basic block: all statements, then the terminator.
pub fn compile_block(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    block_id: BlockId,
) -> Result<(), CodegenError> {
    let block = &state.body.blocks[block_id.index()];

    for stmt in &block.stmts {
        compile_statement(ctx, state, builder, &stmt.kind)?;
    }

    terminator::compile_terminator(ctx, state, builder, &block.terminator)?;

    Ok(())
}

/// Compile a single statement.
fn compile_statement(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    kind: &StatementKind,
) -> Result<(), CodegenError> {
    match kind {
        StatementKind::Assign { dest, rvalue } => {
            let value = rvalue::compile_rvalue(ctx, state, builder, rvalue)?;
            place::compile_place_write(ctx, state, builder, dest, value)?;
            Ok(())
        },

        StatementKind::Call { dest, callee, args } => {
            rvalue::call::compile_call(ctx, state, builder, callee, args, dest.as_ref())?;
            Ok(())
        },

        // Deinit is a no-op at the codegen level — the deinit pass has already
        // expanded this into explicit calls
        StatementKind::Deinit { .. } => Ok(()),

        // DeinitIf: check flag, call deinit if live
        // Also a no-op at codegen — the deinit pass handles this
        StatementKind::DeinitIf { .. } => Ok(()),

        // SetDeinitFlag: store a bool value into the flag local
        StatementKind::SetDeinitFlag { flag, value } => {
            let val = builder
                .ins()
                .iconst(cranelift_codegen::ir::types::I8, *value as i64);
            let var = state.local_vars[flag.index()];
            builder.def_var(var, val);
            Ok(())
        },

        // Drop/DropIf are the greenfield memory-model equivalents of
        // Deinit/DeinitIf. Stage 1 of the rewrite treats them identically
        // (no-op at codegen; lowering / drop-elab has already emitted the
        // explicit deinit calls). Stage 7 will switch this to real drop
        // dispatch.
        StatementKind::Drop { .. } | StatementKind::DropIf { .. } => Ok(()),
    }
}
