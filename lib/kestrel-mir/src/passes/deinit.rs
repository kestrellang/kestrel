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
use crate::item::FunctionKind;
use crate::place::Place;
use crate::statement::{Rvalue, Statement, StatementKind};
use crate::terminator::TerminatorKind;
use crate::ty::MirTy;
use crate::value::Value;
use kestrel_hecs::Entity;

/// Insert destructor calls for non-copyable locals with move tracking.
///
/// Locals that are never moved get unconditional `Deinit`. Locals that are
/// moved somewhere get `DeinitIf` with a flag that tracks whether the move
/// happened. This prevents double-free of moved values.
pub fn run_deinit_pass(module: &mut MirModule) {
    // Build a set of type entities that have a deinit function defined.
    let types_with_deinit: HashSet<Entity> = module
        .functions
        .iter()
        .filter_map(|f| match &f.kind {
            FunctionKind::Deinit { parent } => Some(*parent),
            _ => None,
        })
        .collect();

    // Structs without explicit deinit but with fields that have direct deinit.
    // These need scope-exit cleanup so their sub-fields are properly destroyed.
    let structs_with_droppable_fields: HashSet<Entity> = module
        .structs
        .iter()
        .filter(|s| {
            !types_with_deinit.contains(&s.entity)
                && s.fields.iter().any(|f| match &f.ty {
                    MirTy::Named { entity, .. } => types_with_deinit.contains(entity),
                    _ => false,
                })
        })
        .map(|s| s.entity)
        .collect();

    // Types needing drop: explicit deinit + structs with droppable fields +
    // enums wrapping either category.
    let types_needing_drop: HashSet<Entity> = {
        let mut set = types_with_deinit.clone();
        set.extend(&structs_with_droppable_fields);
        for enum_def in &module.enums {
            let has_droppable_payload = enum_def.cases.iter().any(|case| {
                let payload_struct = &module.structs[case.payload_struct.index()];
                payload_struct.fields.iter().any(|f| match &f.ty {
                    MirTy::Named { entity, .. } => set.contains(entity),
                    _ => false,
                })
            });
            if has_droppable_payload {
                set.insert(enum_def.entity);
            }
        }
        set
    };

    for func in &mut module.functions {
        let Some(body) = &mut func.body else {
            continue;
        };

        // Find locals that own a constructed value of a type needing drop.
        let constructed_locals = find_constructed_locals(
            body, &types_needing_drop, &structs_with_droppable_fields,
        );
        let deinit_locals: Vec<LocalId> = constructed_locals
            .into_iter()
            .filter(|id| id.index() >= body.param_count)
            .collect();

        // Phase 2: Effectful init partial-drop — insert field-level DeinitIf
        // before failure-return terminators so partially-initialized fields
        // get cleaned up when the init fails. Runs before the early-exit below
        // so it's not skipped when there are no local-level deinits.
        if matches!(func.kind, FunctionKind::Initializer { .. })
            && !body.failure_return_blocks.is_empty()
        {
            insert_init_field_deinits(body);
        }

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

        // Insert SetDeinitFlag(flag, true) after each move or construct-consumption
        for block_idx in 0..body.blocks.len() {
            let mut insertions = Vec::new();
            for (stmt_idx, stmt) in body.blocks[block_idx].stmts.iter().enumerate() {
                match &stmt.kind {
                    StatementKind::Assign {
                        rvalue: Rvalue::Move(place),
                        ..
                    } => {
                        if let Some(local_id) = place.root_local() {
                            if let Some(&flag_id) = flag_locals.get(&local_id) {
                                insertions.push((
                                    stmt_idx + 1,
                                    Statement::new(StatementKind::SetDeinitFlag {
                                        flag: flag_id,
                                        value: true,
                                    }),
                                ));
                            }
                        }
                    }
                    StatementKind::Assign {
                        rvalue: Rvalue::Construct { fields, .. },
                        ..
                    } => {
                        for (_, value) in fields {
                            if let Value::Place(place) = value {
                                if let Some(local_id) = place.root_local() {
                                    if let Some(&flag_id) = flag_locals.get(&local_id) {
                                        insertions.push((
                                            stmt_idx + 1,
                                            Statement::new(StatementKind::SetDeinitFlag {
                                                flag: flag_id,
                                                value: true,
                                            }),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
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

/// Insert DeinitIf for each tracked init field before failure-return blocks.
///
/// Flag locals are created by MIR lowering with name `_init_<fieldname>`.
/// Flag semantics: true = uninitialized (skip), false = initialized (deinit).
fn insert_init_field_deinits(body: &mut MirBody) {
    // Find init field flags: locals named "_init_*" with Bool type
    let init_flags: Vec<(String, LocalId)> = body
        .locals
        .iter()
        .enumerate()
        .filter(|(_, l)| l.name.starts_with("_init_") && l.ty == MirTy::Bool)
        .map(|(i, l)| {
            let field_name = l.name.strip_prefix("_init_").unwrap().to_string();
            (field_name, LocalId::new(i))
        })
        .collect();

    if init_flags.is_empty() {
        return;
    }

    let failure_blocks: Vec<usize> = body
        .failure_return_blocks
        .iter()
        .map(|b| b.index())
        .collect();

    for block_idx in failure_blocks {
        if !matches!(
            body.blocks[block_idx].terminator.kind,
            TerminatorKind::Return(_)
        ) {
            continue;
        }

        // Insert DeinitIf for each tracked field (reverse order for proper cleanup)
        let deinit_stmts: Vec<Statement> = init_flags
            .iter()
            .rev()
            .map(|(field_name, flag)| {
                // self is local 0; project into field
                let place = Place::local(LocalId::new(0)).field(field_name);
                Statement::new(StatementKind::DeinitIf {
                    place,
                    flag: *flag,
                })
            })
            .collect();

        body.blocks[block_idx].stmts.extend(deinit_stmts);
    }
}

/// Find all deinit-eligible locals that are moved anywhere in the function body.
/// Also treats Construct field consumption as a move — the value is consumed
/// into the struct and the source local must not be independently deinited.
fn find_moved_locals(body: &MirBody, deinit_locals: &[LocalId]) -> HashSet<LocalId> {
    let deinit_set: HashSet<LocalId> = deinit_locals.iter().copied().collect();
    let mut moved = HashSet::new();

    for block in &body.blocks {
        for stmt in &block.stmts {
            match &stmt.kind {
                StatementKind::Assign {
                    rvalue: Rvalue::Move(place),
                    ..
                } => {
                    if let Some(local_id) = place.root_local() {
                        if deinit_set.contains(&local_id) {
                            moved.insert(local_id);
                        }
                    }
                }
                // Construct consumes its field values — the source is moved
                StatementKind::Assign {
                    rvalue: Rvalue::Construct { fields, .. },
                    ..
                } => {
                    for (_, value) in fields {
                        if let Value::Place(place) = value {
                            if let Some(local_id) = place.root_local() {
                                if deinit_set.contains(&local_id) {
                                    moved.insert(local_id);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    moved
}

/// Find locals that own a constructed value of a type with deinit.
///
/// Traces the Construct→Copy chain: if a temp is constructed and then
/// immediately copied to another local, the copy target is the owner
/// (not the temp). This matches MIR lowering's pattern where Construct
/// always targets a temp, which is then Copy'd to the user local.
fn find_constructed_locals(
    body: &MirBody,
    types_needing_drop: &HashSet<Entity>,
    structs_with_droppable_fields: &HashSet<Entity>,
) -> Vec<LocalId> {
    // Step 1: find which locals are Construct/EnumVariant targets for droppable types
    let mut construct_targets: HashMap<LocalId, Entity> = HashMap::new();
    for block in &body.blocks {
        for stmt in &block.stmts {
            let (dest, entity) = match &stmt.kind {
                StatementKind::Assign {
                    dest,
                    rvalue: Rvalue::Construct { ty, .. },
                } => {
                    let e = match ty {
                        MirTy::Named { entity, .. } => Some(*entity),
                        _ => None,
                    };
                    (dest, e)
                }
                StatementKind::Assign {
                    dest,
                    rvalue: Rvalue::EnumVariant { enum_ty, .. },
                } => {
                    // For enums, check if any type_arg is a type needing drop.
                    // This catches Optional[Container] where Container has deinit.
                    let e = match enum_ty {
                        MirTy::Named { entity, type_args } => {
                            let has_droppable_arg = type_args.iter().any(|arg| match arg {
                                MirTy::Named { entity, .. } => {
                                    types_needing_drop.contains(entity)
                                        || structs_with_droppable_fields.contains(entity)
                                }
                                _ => false,
                            });
                            if types_needing_drop.contains(entity) || has_droppable_arg {
                                Some(*entity)
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };
                    (dest, e)
                }
                _ => continue,
            };
            if let Some(e) = entity {
                if let Some(local_id) = dest.root_local() {
                    construct_targets.insert(local_id, e);
                }
            }
        }
    }

    // Step 2: find Copy chains from construct targets to final owners.
    // If local B = copy A, and A was constructed, then B is the owner.
    let mut owners: HashMap<LocalId, Entity> = HashMap::new();
    let mut copied_from: HashSet<LocalId> = HashSet::new();
    for block in &body.blocks {
        for stmt in &block.stmts {
            if let StatementKind::Assign {
                dest,
                rvalue: Rvalue::Copy(src),
            } = &stmt.kind
            {
                if let (Some(dest_id), Some(src_id)) = (dest.root_local(), src.root_local()) {
                    if let Some(&entity) = construct_targets.get(&src_id) {
                        owners.insert(dest_id, entity);
                        copied_from.insert(src_id);
                    }
                }
            }
        }
    }

    // The owner is the copy target if it exists, otherwise the construct target.
    // Sorted by LocalId to preserve declaration order (deinit pass reverses).
    let mut result_set = HashSet::new();
    for (local_id, _) in &owners {
        result_set.insert(*local_id);
    }
    for (local_id, _) in &construct_targets {
        if !copied_from.contains(local_id) {
            result_set.insert(*local_id);
        }
    }
    let mut result: Vec<LocalId> = result_set.into_iter().collect();
    result.sort_by_key(|id| id.index());
    result
}
