//! Pass: reject `CopyValue` / `CopyAddr` of a `not Copyable` value.
//!
//! In OSSA a copy duplicates an @owned value, which is only legal for Copyable
//! types. A copy of a `not Copyable` value reaching this point means either the
//! HIR lowering emitted an illegal duplication or the source program used a
//! value after it was moved. Until a dedicated move checker rejects such
//! programs upstream, this pass is the backstop: it turns the condition into a
//! hard error instead of a silent miscompile (a non-Copyable type has no clone
//! shim, so the duplication would corrupt at codegen / fail at monomorphization).
//!
//! Note: clone shims themselves emit `CopyValue`, but only on Copyable
//! fields — a type with a non-Copyable field is itself non-Copyable and gets no
//! shim — so they never trip this check.

use crate::body::OssaBody;
use crate::inst::InstKind;
use crate::item::function::FunctionDef;
use crate::ty_query::copy_behavior;
use crate::verify::VerifyError;
use crate::{BlockId, CopyBehavior, MirModule, TyId};

/// Scan every body for copies of non-Copyable values. Returns one error per
/// offending `CopyValue` / `CopyAddr`.
pub fn check_copies(module: &MirModule) -> Vec<VerifyError> {
    let mut errors = Vec::new();
    for func in module.functions.values() {
        let Some(body) = &func.body else { continue };
        if body.values.is_empty() || body.blocks.is_empty() {
            continue;
        }
        check_body(module, func, body, &mut errors);
    }
    errors
}

fn check_body(
    module: &MirModule,
    func: &FunctionDef,
    body: &OssaBody,
    errors: &mut Vec<VerifyError>,
) {
    let where_clause = func.where_clause.as_ref();
    for (block_idx, block) in body.blocks.iter().enumerate() {
        for (inst_idx, inst) in block.insts.iter().enumerate() {
            // The type being duplicated: the operand's type for CopyValue, the
            // pointee type for CopyAddr.
            let copied_ty: TyId = match &inst.kind {
                InstKind::CopyValue { operand, .. } => body.value(*operand).ty,
                InstKind::CopyAddr { ty, .. } => *ty,
                _ => continue,
            };

            if copy_behavior(&module.ty_arena, module, copied_ty, where_clause)
                != CopyBehavior::None
            {
                continue;
            }

            errors.push(VerifyError {
                block: BlockId::new(block_idx),
                inst: Some(inst_idx as u32),
                message: format!(
                    "copy of non-Copyable value of type `{}`",
                    crate::display::ty_to_string(copied_ty, module),
                ),
                span: inst.span.clone(),
                func_name: func.name.clone(),
                entity: func.entity,
            });
        }
    }
}
