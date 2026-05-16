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

        // Drop/DropIf are no-ops at the cranelift level today. Drop-elab
        // emits them as structural markers; explicit `deinit` method calls
        // are already lowered as regular Call statements.
        //
        // A future codegen pass will turn `Drop` into a structural destructor
        // call sequence and `DropIf` into a branch on its flag local.
        StatementKind::Drop { .. } | StatementKind::DropIf { .. } => Ok(()),

        // Deinit/DeinitIf: no-op at codegen — the deinit pass has already
        // expanded these into explicit calls
        StatementKind::Deinit { .. } | StatementKind::DeinitIf { .. } => Ok(()),

        // SetDeinitFlag: store a bool value into the flag local
        StatementKind::SetDeinitFlag { flag, value } => {
            let val = builder
                .ins()
                .iconst(cranelift_codegen::ir::types::I8, *value as i64);
            let var = state.local_vars[flag.index()];
            builder.def_var(var, val);
            Ok(())
        },

        // ScopeLive: no-op at codegen — consumed by drop elaboration pass
        StatementKind::ScopeLive(_) => Ok(()),
    }
}
