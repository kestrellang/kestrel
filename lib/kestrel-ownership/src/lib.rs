//! Move-check and drop elaboration on MIR.
//!
//! This crate owns the greenfield memory model. It consumes a `MirModule`
//! after lowering and runs two passes:
//!
//! 1. [`move_check`] — interns move paths, runs an init/maybe-init forward
//!    dataflow, emits use-after-move diagnostics (E500/E501/E502).
//! 2. [`drop_elab`] — the *only* pass that emits `Drop` / `DropIf`
//!    statements. Drops are placed at scope-exit edges in reverse declaration
//!    order; user `deinit` runs before structural field drops.
//!
//! Lowering must never emit `Drop` / `DropIf`. The MIR verifier
//! (Stage 6+) enforces this.
//!
//! ## Stage 1 (current)
//!
//! Stage 1 is the "tracer bullet": the entry point exists and is wired into
//! the compiler driver, but it does the minimum possible work — it rewrites
//! existing legacy `Deinit` / `DeinitIf` statements to `Drop` / `DropIf` 1:1
//! so the new statement types are visible end-to-end. Real move-checking and
//! drop-elaboration land at Stages 4 and 7.

pub mod drop_elab;
pub mod move_check;

use kestrel_mir::MirModule;

/// Diagnostics produced by the ownership passes.
///
/// Stage 1 emits no diagnostics. The shape of this type will mirror
/// `kestrel-reporting`'s diagnostic surface once Stage 4 (MoveCheck) lands.
#[derive(Debug, Clone, Default)]
pub struct Diagnostics {
    pub messages: Vec<String>,
}

/// Run all ownership passes on the module. Single entry point.
///
/// Stage 1: rewrites legacy `Deinit`/`DeinitIf`/`SetDeinitFlag` to
/// `Drop`/`DropIf`/(unchanged). No diagnostics emitted.
pub fn run(module: &mut MirModule) -> Diagnostics {
    let mut diags = Diagnostics::default();
    move_check::run(module, &mut diags);
    drop_elab::run(module);
    diags
}
