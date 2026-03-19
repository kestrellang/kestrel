//! Deinit pass — insert destructor calls based on liveness analysis.
//!
//! Analyzes each function body to find where non-copyable locals go out of
//! scope or are last used, and inserts `Deinit` / `DeinitIf` statements.
//!
//! Strategy:
//! 1. For each local that may need dropping (non-primitive, non-Copy):
//!    - Track where it's moved (Rvalue::Move) — no deinit needed after move
//!    - At scope exit (return/jump out), insert Deinit for live locals
//! 2. At branch merge points where a value may be moved in one branch but
//!    not another, insert DeinitIf with a flag local
//!
//! This is a simplified version — a full implementation would do proper
//! dataflow analysis across the CFG. For now, we insert Deinit before
//! every Return terminator for all locals that aren't primitives.

use crate::id::LocalId;
use crate::statement::{Statement, StatementKind};
use crate::terminator::TerminatorKind;
use crate::ty::MirTy;
use crate::place::Place;
use crate::MirModule;

/// Insert destructor calls for non-copyable locals.
///
/// Current implementation: before every Return terminator, insert Deinit
/// for all non-primitive, non-param locals. This is conservative (may
/// deinit values that have already been moved) but correct for types
/// where deinit is idempotent.
pub fn run_deinit_pass(module: &mut MirModule) {
    for func in &mut module.functions {
        let Some(body) = &mut func.body else { continue };

        // Collect locals that might need deinit (non-primitive types)
        let deinit_locals: Vec<LocalId> = body
            .locals
            .iter()
            .enumerate()
            .skip(body.param_count) // don't deinit params — caller owns them
            .filter(|(_, local)| needs_deinit(&local.ty))
            .map(|(i, _)| LocalId::new(i))
            .collect();

        if deinit_locals.is_empty() {
            continue;
        }

        // Find blocks with Return terminators and insert Deinit before them
        for block_idx in 0..body.blocks.len() {
            let is_return = matches!(
                body.blocks[block_idx].terminator.kind,
                TerminatorKind::Return(_)
            );

            if is_return {
                // Insert Deinit statements for each tracked local (reverse order)
                let mut deinit_stmts: Vec<Statement> = deinit_locals
                    .iter()
                    .rev()
                    .map(|&local| {
                        Statement::new(StatementKind::Deinit {
                            place: Place::local(local),
                        })
                    })
                    .collect();

                // Insert before the existing statements' end (before the return)
                let block = &mut body.blocks[block_idx];
                block.stmts.append(&mut deinit_stmts);
            }
        }
    }
}

/// Check if a type needs deinit (is non-trivially destructible).
/// Primitives, references, and pointers don't need deinit.
fn needs_deinit(ty: &MirTy) -> bool {
    match ty {
        // Primitives — no deinit
        MirTy::Bool | MirTy::I8 | MirTy::I16 | MirTy::I32 | MirTy::I64 => false,
        MirTy::F16 | MirTy::F32 | MirTy::F64 => false,
        MirTy::Unit | MirTy::Never => false,

        // References and pointers — no ownership, no deinit
        MirTy::Ref(_) | MirTy::RefMut(_) | MirTy::Pointer(_) => false,

        // Function pointers — no deinit
        MirTy::FuncThin { .. } => false,

        // Error/unknown — skip
        MirTy::Error => false,

        // Thick callables might own an env — need deinit
        MirTy::FuncThick { .. } => true,

        // Named types, tuples, strings — may need deinit
        MirTy::Named { .. } | MirTy::Tuple(_) | MirTy::Str => true,

        // Generic types — conservatively assume they need deinit
        MirTy::TypeParam(_) | MirTy::SelfType | MirTy::AssociatedProjection { .. } => true,
    }
}
