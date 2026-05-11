//! Move-check pass — runs the init/maybe-init dataflow over MIR move paths.
//!
//! ## Stage 4 (current)
//!
//! Silent infrastructure. For every function body, build the move-path set,
//! run the forward dataflow, and (eventually) walk reads against the
//! resulting state. Diagnostics are NOT emitted at Stage 4 — the existing
//! HIR move tracker in `kestrel-analyze::body::move_tracking` still covers
//! the user-facing E500/E501. The Stage 7 cleanup deletes that tracker and
//! flips this pass to be the sole emitter, with the same diagnostic
//! wording.
//!
//! Running the dataflow silently on every test still validates correctness
//! by construction: any panic / infinite loop / unsoundness here is
//! exercised by the existing memory_model suite.
//!
//! ## What's tracked
//!
//! - Per-function: a [`MovePathSet`] of every non-`Copy` local. Parameters
//!   are included with entry state = `DefinitelyInit`.
//! - Per program point: an [`InitState`] (sets of `MovePathId`).
//! - Stage 4 granularity: root locals only. Projections (`s.f.0`) fold to
//!   their root; Stage 7 extends to field-level tracking.

use crate::Diagnostics;
use crate::dataflow;
use crate::move_path::MovePathSet;
use kestrel_mir::MirModule;

pub fn run(module: &mut MirModule, _diags: &mut Diagnostics) {
    for func in &module.functions {
        let Some(body) = &func.body else { continue };
        let paths = MovePathSet::build(body, module);
        if paths.is_empty() {
            // No non-Copy locals — nothing for the dataflow to track.
            continue;
        }
        // Run the dataflow for its side-effects (catching infinite loops,
        // panics, etc.) but discard the result. Stage 7 will walk reads
        // against this result to emit E500/E501.
        let _result = dataflow::run(body, &paths);
    }
}
