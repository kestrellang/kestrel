pub mod clone_shim;
pub mod copy_check;
pub mod copy_propagation;
pub mod drop_fix;
pub mod drop_shim;
pub mod layout;
pub mod thunk;

use crate::MirModule;
use crate::item::TargetConfig;
use crate::verify::VerifyError;

/// Run the full pre-codegen OSSA pipeline:
/// drop_fix → thunk → drop_shim → layout → ossa_verify.
pub fn run_pipeline(
    module: &mut MirModule,
    target: &TargetConfig,
    next_entity: &mut u32,
) -> Vec<VerifyError> {
    drop_fix::fix_drop_behaviors(module);
    thunk::run_thunk_pass(module, next_entity);
    drop_shim::synthesize_drop_shims(module, next_entity);
    clone_shim::synthesize_clone_shims(module, next_entity);
    layout::run_layout_pass(module, target);

    let mut errors = Vec::new();
    for func in module.functions.values() {
        if let Some(body) = &func.body {
            if body.values.is_empty() || body.blocks.is_empty() {
                continue;
            }
            let func_errors = crate::verify::verify_ossa(body, module, &func.name, func.entity);
            errors.extend(func_errors);
        }
    }
    // Flags every CopyValue/CopyAddr of a non-Copyable value. WIP: the MIR-3
    // lowering still copies non-Copyable values by design in mainstream paths
    // (closures called per-element, resource handles like File passed around),
    // so this currently fails ~550 tests until the lowering's move/copy
    // decision is fixed to borrow/move those instead of copying.
    errors.extend(copy_check::check_copies(module));
    errors
}
