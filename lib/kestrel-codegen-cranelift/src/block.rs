//! Basic block compilation.

use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::monomorphize::Substitution;
use crate::place::compile_place_read;
use crate::rvalue::{compile_call, compile_rvalue};
use crate::terminator::compile_terminator;

use kestrel_execution_graph::{Block, FunctionDef, Id, Local, Place, StatementKind};

use cranelift_codegen::ir::{InstBuilder, Value as CraneliftValue};
use cranelift_frontend::{FunctionBuilder, Variable};

use std::collections::HashMap;

/// Compile a single basic block.
pub fn compile_block(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    block_id: Id<Block>,
    builder: &mut FunctionBuilder<'_>,
    block_map: &HashMap<Id<Block>, cranelift_codegen::ir::Block>,
    local_map: &HashMap<Id<Local>, Variable>,
    is_main: bool,
    sret_ptr: Option<CraneliftValue>,
) -> Result<(), CodegenError> {
    let block = ctx.mir.block(block_id);

    // Compile each statement
    for &stmt_id in &block.statements {
        let stmt = ctx.mir.statement(stmt_id);
        compile_statement(ctx, func_def, subst, &stmt.kind, builder, local_map)?;
    }

    // Compile the terminator
    if let Some(ref terminator) = block.terminator {
        compile_terminator(
            ctx,
            func_def,
            subst,
            terminator,
            builder,
            block_map,
            local_map,
            is_main,
            sret_ptr,
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
    subst: &Substitution,
    stmt: &StatementKind,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<(), CodegenError> {
    match stmt {
        StatementKind::Assign { dest, rvalue } => {
            let value = compile_rvalue(ctx, func_def, subst, rvalue, builder, local_map)?;
            crate::place::compile_place_write(ctx, dest, value, builder, local_map, subst)?;
        }

        StatementKind::Call { callee, args } => {
            // Call without using the result - we just discard the return value
            let _ = compile_call(ctx, func_def, subst, callee, args, builder, local_map)?;
        }

        StatementKind::Deinit { place: _ } => {
            // NOTE: During MIR lowering, deinit is expanded into explicit:
            // 1. Calls to the type's deinit method (if it has one)
            // 2. Recursive deinit for struct fields / enum payloads
            //
            // So by the time we reach codegen, StatementKind::Deinit is only emitted
            // for types that don't need any actual cleanup (no deinit method, no fields
            // that need deinit). In that case, it's a no-op.
            //
            // If we need to support "raw" Deinit statements in the future (e.g., for
            // types where we don't have semantic info), we would need to look up the
            // deinit method from the MIR type and emit a call here.
        }

        StatementKind::DeinitIf { place: _, flag } => {
            // Conditional deinit: check flag, call destructor if true.
            //
            // Similar to Deinit above, the MIR lowering has already expanded any
            // deinit method calls. The DeinitIf statement itself is emitted when a
            // value might have been moved in one branch but not another.
            //
            // Since the lowering already expands deinit method calls (with proper
            // conditionals using branch blocks), this statement is effectively a no-op
            // in the current design.
            //
            // We still need to "use" the flag to avoid unused variable warnings in the
            // generated code, but since we don't actually emit anything for deinit,
            // we can just read the flag value without acting on it.
            let _flag_value =
                compile_place_read(ctx, &Place::local(*flag), builder, local_map, subst)?;
        }

        StatementKind::SetDeinitFlag { flag, value } => {
            // Set a deinit flag to true (needs deinit) or false (was moved, no deinit needed).
            //
            // Deinit flags are Bool locals. Bool is represented as i8 (0 = false, 1 = true).
            let bool_value = builder
                .ins()
                .iconst(cranelift_codegen::ir::types::I8, if *value { 1 } else { 0 });

            let var = local_map.get(flag).ok_or_else(|| {
                CodegenError::Unsupported("unknown deinit flag local".to_string())
            })?;
            builder.def_var(*var, bool_value);
        }
    }

    Ok(())
}
