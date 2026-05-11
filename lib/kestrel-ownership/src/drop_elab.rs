//! Drop elaboration pass.
//!
//! Stage 1: rewrites legacy `Deinit { place }` → `Drop { place }` and
//! `DeinitIf { place, flag }` → `DropIf { place, flag }`. Existing
//! `SetDeinitFlag` statements are left alone — they're still the flag
//! mechanism the legacy `passes::deinit` pass produces.
//!
//! Real implementation (Stage 7) computes a scope tree on `MirBody`, runs
//! against the move-path init/maybe-init dataflow, and emits drops at
//! scope-exit edges in reverse declaration order. At Stage 7 the legacy
//! `passes::deinit` is deleted and this pass becomes the sole drop emitter.

use kestrel_mir::{MirModule, StatementKind};

pub fn run(module: &mut MirModule) {
    for func in &mut module.functions {
        let Some(body) = &mut func.body else { continue };
        for block in &mut body.blocks {
            for stmt in &mut block.stmts {
                stmt.kind = match std::mem::replace(
                    &mut stmt.kind,
                    StatementKind::SetDeinitFlag {
                        flag: kestrel_mir::LocalId::new(0),
                        value: false,
                    },
                ) {
                    StatementKind::Deinit { place } => StatementKind::Drop { place },
                    StatementKind::DeinitIf { place, flag } => {
                        StatementKind::DropIf { place, flag }
                    },
                    other => other,
                };
            }
        }
    }
}
