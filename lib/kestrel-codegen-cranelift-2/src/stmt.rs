use cranelift_frontend::FunctionBuilder;

use kestrel_mir_2::StatementKind;

use crate::call;
use crate::error::CodegenError;
use crate::func::FuncCompiler;
use crate::place;
use crate::rvalue;

/// Returns true if the statement diverges (e.g. calls a `!`-returning
/// function), meaning the rest of the block is unreachable.
pub fn compile_statement(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    kind: &StatementKind,
) -> Result<bool, CodegenError> {
    match kind {
        StatementKind::Assign { dest, rvalue } => {
            let val = rvalue::compile_rvalue(fc, builder, rvalue)?;
            place::place_write(fc, builder, dest, val)?;
        }

        StatementKind::Call {
            dest,
            callee,
            args,
        } => {
            if call::compile_call(fc, builder, callee, args, dest.as_ref())? {
                return Ok(true);
            }
        }

        StatementKind::Uninit { dest } => {
            let ty = place::place_type(dest, fc.body, &fc.ctx.module.ty_arena, fc.ctx.module, &fc.ctx.tc);
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            if let crate::ty::TypeRepr::Aggregate { size, .. } = repr {
                let addr = place::place_addr(fc, builder, dest)?;
                crate::mem::zero_memory(builder, addr, size);
            }
        }

        // Drop/DropIf/SetDropFlag are expanded by mono into Assign/Call/Branch.
        // If they somehow survive, treat as no-ops.
        StatementKind::Drop { .. }
        | StatementKind::DropIf { .. }
        | StatementKind::SetDropFlag { .. }
        | StatementKind::ScopeLive(_) => {}
    }

    Ok(false)
}
