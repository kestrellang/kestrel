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
        stmt::compile_statement(fc, builder, &stmt.kind)?;
    }

    terminator::compile_terminator(fc, builder, &block.terminator)?;

    Ok(())
}
