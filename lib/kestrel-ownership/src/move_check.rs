//! Move-check pass.
//!
//! Stage 1: stub. Real implementation lands at Stage 4 (move-paths +
//! init/maybe-init forward dataflow, E500/E501/E502).

use crate::Diagnostics;
use kestrel_mir::MirModule;

pub fn run(_module: &mut MirModule, _diags: &mut Diagnostics) {
    // Intentionally empty for Stage 1. The HIR-level move tracker in
    // `kestrel-analyze::body::move_tracking` still emits the
    // use-after-move diagnostics until Stage 7.
}
