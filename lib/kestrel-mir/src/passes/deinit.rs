//! Deinit pass — insert destructor calls based on move tracking.
//!
//! Analyzes each function body to find where non-copyable locals go out of
//! scope or are last used, and inserts `Deinit` / `DeinitIf` statements.
//!
//! Strategy:
//! 1. Collect locals that may need deinit (non-primitive, non-param types)
//! 2. Scan all blocks for `Rvalue::Move` to find locals that are moved
//! 3. For moved locals: create Bool flag locals and insert `SetDeinitFlag(flag, true)`
//!    after each move
//! 4. Before each Return terminator:
//!    - Never-moved locals → unconditional `Deinit`
//!    - Moved locals → `DeinitIf(place, flag)` (only deinit if not moved)

use std::collections::{HashMap, HashSet};

use crate::MirModule;
use crate::body::{LocalDef, MirBody};
use crate::id::LocalId;
use crate::place::Place;
use crate::statement::{Rvalue, Statement, StatementKind};
use crate::terminator::TerminatorKind;
use crate::ty::MirTy;

/// Insert destructor calls for non-copyable locals with move tracking.
///
/// Locals that are never moved get unconditional `Deinit`. Locals that are
/// moved somewhere get `DeinitIf` with a flag that tracks whether the move
/// happened. This prevents double-free of moved values.
pub fn run_deinit_pass(module: &mut MirModule) {
    for func in &mut module.functions {
        let Some(body) = &mut func.body else {
            continue;
        };

        // Collect locals that might need deinit (non-primitive, non-param types)
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

        // Find locals that are moved anywhere in the function
        let moved_locals = find_moved_locals(body, &deinit_locals);

        // Create flag locals for moved locals: flag=false means "still live, needs deinit"
        // flag=true means "was moved, skip deinit"
        let mut flag_locals: HashMap<LocalId, LocalId> = HashMap::new();
        for &local_id in &moved_locals {
            let flag_name = format!("_moved_{}", body.locals[local_id.index()].name);
            let flag_id = body.add_local(LocalDef::new(flag_name, MirTy::Bool));
            flag_locals.insert(local_id, flag_id);
        }

        // Insert SetDeinitFlag(flag, true) after each Move of a flagged local
        for block_idx in 0..body.blocks.len() {
            let mut insertions = Vec::new();
            for (stmt_idx, stmt) in body.blocks[block_idx].stmts.iter().enumerate() {
                if let StatementKind::Assign {
                    rvalue: Rvalue::Move(place),
                    ..
                } = &stmt.kind
                    && let Some(local_id) = place.root_local()
                        && let Some(&flag_id) = flag_locals.get(&local_id) {
                            insertions.push((
                                stmt_idx + 1,
                                Statement::new(StatementKind::SetDeinitFlag {
                                    flag: flag_id,
                                    value: true,
                                }),
                            ));
                        }
            }
            // Insert in reverse order to maintain indices
            for (pos, stmt) in insertions.into_iter().rev() {
                body.blocks[block_idx].stmts.insert(pos, stmt);
            }
        }

        // Insert deinit statements before each Return terminator
        for block_idx in 0..body.blocks.len() {
            let is_return = matches!(
                body.blocks[block_idx].terminator.kind,
                TerminatorKind::Return(_)
            );

            if is_return {
                let deinit_stmts: Vec<Statement> = deinit_locals
                    .iter()
                    .rev()
                    .map(|&local| {
                        if let Some(&flag) = flag_locals.get(&local) {
                            // Moved somewhere — conditional deinit
                            Statement::new(StatementKind::DeinitIf {
                                place: Place::local(local),
                                flag,
                            })
                        } else {
                            // Never moved — unconditional deinit
                            Statement::new(StatementKind::Deinit {
                                place: Place::local(local),
                            })
                        }
                    })
                    .collect();

                body.blocks[block_idx].stmts.extend(deinit_stmts);
            }
        }
    }
}

/// Find all deinit-eligible locals that are moved anywhere in the function body.
fn find_moved_locals(body: &MirBody, deinit_locals: &[LocalId]) -> HashSet<LocalId> {
    let deinit_set: HashSet<LocalId> = deinit_locals.iter().copied().collect();
    let mut moved = HashSet::new();

    for block in &body.blocks {
        for stmt in &block.stmts {
            if let StatementKind::Assign {
                rvalue: Rvalue::Move(place),
                ..
            } = &stmt.kind
                && let Some(local_id) = place.root_local()
                    && deinit_set.contains(&local_id) {
                        moved.insert(local_id);
                    }
        }
    }

    moved
}

/// Check if a type needs deinit (is non-trivially destructible).
/// Primitives, references, and pointers don't need deinit.
fn needs_deinit(ty: &MirTy) -> bool {
    match ty {
        // Primitives — no deinit
        MirTy::Bool | MirTy::I8 | MirTy::I16 | MirTy::I32 | MirTy::I64 => false,
        MirTy::F16 | MirTy::F32 | MirTy::F64 => false,
        MirTy::Never => false,

        // References and pointers — no ownership, no deinit
        MirTy::Ref(_) | MirTy::RefMut(_) | MirTy::Pointer(_) => false,

        // Function pointers — no deinit
        MirTy::FuncThin { .. } => false,

        // Error/unknown — skip
        MirTy::Error => false,

        // Thick callables might own an env — need deinit
        MirTy::FuncThick { .. } => true,

        // Empty tuple == unit: nothing to deinit.
        MirTy::Tuple(elems) if elems.is_empty() => false,

        // Named types, tuples, strings — may need deinit
        MirTy::Named { .. } | MirTy::Tuple(_) | MirTy::Str => true,

        // Generic types — conservatively assume they need deinit
        MirTy::TypeParam(_) | MirTy::SelfType | MirTy::AssociatedProjection { .. } => true,
    }
}
