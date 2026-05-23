use cranelift_frontend::FunctionBuilder;

use kestrel_mir_2::BlockId;

use crate::error::CodegenError;
use crate::func::FuncCompiler;
use crate::stmt;
use crate::terminator;

pub fn compile_block(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    block_id: BlockId,
) -> Result<(), CodegenError> {
    let block = &fc.body.blocks[block_id.index()];

    for stmt in &block.stmts {
        if stmt::compile_statement(fc, builder, &stmt.kind)? {
            // Statement diverged (e.g. call to a `!`-returning function).
            // A trap was already emitted; skip remaining statements and terminator.
            return Ok(());
        }
    }

    terminator::compile_terminator(fc, builder, &block.terminator)?;

    Ok(())
}
