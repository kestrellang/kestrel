//! Expand `Deinit` and `DeinitIf` statements into actual destructor calls.
//!
//! Runs after the deinit pass (which inserts Deinit/DeinitIf nodes) and before
//! codegen. Rewrites each node into a Call to the type's deinit function, or
//! removes it if the type has no destructor.
//!
//! `DeinitIf` expands into a conditional branch: if the flag is false (value is
//! live), call deinit; otherwise skip.

use std::collections::HashMap;

use crate::MirModule;
use crate::body::{BasicBlock, MirBody};
use crate::id::{BlockId, LocalId};
use crate::item::FunctionKind;
use crate::place::Place;
use crate::statement::{CallArg, Callee, Statement, StatementKind};
use crate::terminator::{Terminator, TerminatorKind};
use crate::ty::MirTy;
use crate::value::Value;
use kestrel_hecs::Entity;

/// Expand `DeinitIf` on init-failure paths into concrete destructor calls.
/// Expand `DeinitIf` on init-failure paths into concrete destructor calls.
///
/// General deinit expansion (all functions) is deferred — expanding Deinit
/// for stdlib locals causes crashes due to deiniting temporary copies that
/// share refcounts without proper copy-constructor calls. This requires the
/// deinit pass to be smarter about which locals actually need cleanup.
pub fn run_expand_deinit_pass(module: &mut MirModule) {
    let deinit_funcs: HashMap<Entity, Entity> = module
        .functions
        .iter()
        .filter_map(|f| match &f.kind {
            FunctionKind::Deinit { parent } => Some((*parent, f.entity)),
            _ => None,
        })
        .collect();

    for func_idx in 0..module.functions.len() {
        let Some(body) = &module.functions[func_idx].body else {
            continue;
        };

        if body.failure_return_blocks.is_empty() {
            continue;
        }

        let expansions = collect_expansions(body, &deinit_funcs, module);
        if expansions.is_empty() {
            continue;
        }

        let body = module.functions[func_idx].body.as_mut().unwrap();
        apply_expansions(body, expansions);
    }
}

/// What kind of expansion a statement needs.
enum Expansion {
    /// Replace Deinit with a Call (or remove if no deinit function).
    ReplaceDeinit {
        block: usize,
        stmt: usize,
        deinit_entity: Option<Entity>,
        place: Place,
        place_ty: MirTy,
    },
    /// Expand DeinitIf into a branch + conditional call.
    ExpandDeinitIf {
        block: usize,
        stmt: usize,
        deinit_entity: Option<Entity>,
        place: Place,
        place_ty: MirTy,
        flag: LocalId,
    },
}

fn collect_expansions(
    body: &MirBody,
    deinit_funcs: &HashMap<Entity, Entity>,
    module: &MirModule,
) -> Vec<Expansion> {
    let mut expansions = Vec::new();

    // Only expand self-field DeinitIf in failure-return blocks (init partial-drop)
    for block_idx in body.failure_return_blocks.iter().map(|b| b.index()) {
        if block_idx >= body.blocks.len() {
            continue;
        }
        for (stmt_idx, stmt) in body.blocks[block_idx].stmts.iter().enumerate() {
            if let StatementKind::DeinitIf { place, flag } = &stmt.kind {
                let is_self_field = matches!(
                    place,
                    Place::Field { parent, .. } if parent.root_local() == Some(LocalId::new(0))
                );
                if !is_self_field {
                    continue;
                }
                let place_ty = resolve_place_type(place, body, module);
                let deinit_entity = place_ty
                    .as_ref()
                    .and_then(|ty| struct_entity(ty))
                    .and_then(|e| deinit_funcs.get(&e).copied());
                if deinit_entity.is_some() {
                    expansions.push(Expansion::ExpandDeinitIf {
                        block: block_idx,
                        stmt: stmt_idx,
                        deinit_entity,
                        place: place.clone(),
                        place_ty: place_ty.unwrap_or(MirTy::Error),
                        flag: *flag,
                    });
                }
            }
        }
    }

    expansions
}

fn apply_expansions(body: &mut MirBody, expansions: Vec<Expansion>) {
    // Process in reverse order so indices stay valid
    for expansion in expansions.into_iter().rev() {
        match expansion {
            Expansion::ReplaceDeinit {
                block,
                stmt,
                deinit_entity,
                place,
                place_ty,
                ..
            } => {
                if let Some(deinit_func) = deinit_entity {
                    // Replace with a Call to the deinit function
                    let callee = deinit_callee(deinit_func, place_ty);
                    body.blocks[block].stmts[stmt] = Statement::new(StatementKind::Call {
                        dest: None,
                        callee,
                        args: vec![CallArg::mutating(Value::Place(place))],
                    });
                } else {
                    // No deinit function — remove the statement
                    body.blocks[block].stmts.remove(stmt);
                }
            }
            Expansion::ExpandDeinitIf {
                block,
                stmt,
                deinit_entity,
                place,
                place_ty,
                flag,
            } => {
                // No deinit function → just remove the DeinitIf
                let Some(deinit_func) = deinit_entity else {
                    body.blocks[block].stmts.remove(stmt);
                    continue;
                };

                // Split: remove the DeinitIf, create deinit + skip blocks,
                // branch on flag.
                //
                // Before:
                //   block: [...stmts, DeinitIf, more_stmts..., terminator]
                //
                // After:
                //   block:       [...stmts, Branch(flag, skip_block, deinit_block)]
                //   deinit_block: [Call(deinit), Jump(cont_block)]
                //   cont_block:  [more_stmts..., original_terminator]
                //
                // DeinitIf semantics: flag=false → live, needs deinit.
                // So branch: if flag (true=moved), skip; else deinit.

                let remaining_stmts = body.blocks[block].stmts.split_off(stmt + 1);
                body.blocks[block].stmts.remove(stmt); // remove the DeinitIf
                let original_terminator =
                    std::mem::replace(&mut body.blocks[block].terminator, Terminator::unreachable());

                // Create continuation block with remaining stmts + original terminator
                let cont_block_id = BlockId::new(body.blocks.len());
                let mut cont_block = BasicBlock::new();
                cont_block.stmts = remaining_stmts;
                cont_block.terminator = original_terminator;
                body.blocks.push(cont_block);

                // Create deinit block: call deinit, jump to continuation
                let deinit_block_id = BlockId::new(body.blocks.len());
                let mut deinit_block = BasicBlock::new();
                let callee = deinit_callee(deinit_func, place_ty);
                deinit_block
                    .stmts
                    .push(Statement::new(StatementKind::Call {
                        dest: None,
                        callee,
                        args: vec![CallArg::mutating(Value::Place(place))],
                    }));
                deinit_block.terminator = Terminator::jump(cont_block_id);
                body.blocks.push(deinit_block);

                // Original block: branch on flag (true → skip, false → deinit)
                body.blocks[block].terminator = Terminator {
                    kind: TerminatorKind::Branch {
                        condition: Value::Place(Place::local(flag)),
                        then_block: cont_block_id,  // flag=true → skip deinit
                        else_block: deinit_block_id, // flag=false → call deinit
                    },
                    span: None,
                };
            }
        }
    }
}

/// Resolve the MIR type of a Place.
fn resolve_place_type(place: &Place, body: &MirBody, module: &MirModule) -> Option<MirTy> {
    match place {
        Place::Local(id) => Some(body.locals[id.index()].ty.clone()),
        Place::Field { parent, name } => {
            let parent_ty = resolve_place_type(parent, body, module)?;
            // Unwrap references (init self is RefMut(Named{...}))
            let inner_ty = unwrap_ref(&parent_ty);
            let entity = struct_entity(inner_ty)?;
            let struct_def = module.structs.iter().find(|s| s.entity == entity)?;
            let field_id = struct_def.field_by_name(name)?;
            Some(struct_def.fields[field_id.index()].ty.clone())
        }
        Place::Deref(inner) => {
            let inner_ty = resolve_place_type(inner, body, module)?;
            match inner_ty {
                MirTy::Ref(pointee) | MirTy::RefMut(pointee) | MirTy::Pointer(pointee) => {
                    Some(*pointee)
                }
                _ => None,
            }
        }
        Place::Index { parent, index } => {
            let parent_ty = resolve_place_type(parent, body, module)?;
            match parent_ty {
                MirTy::Tuple(elems) => elems.get(*index).cloned(),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Build a Callee for a deinit method, extracting type args from the place type.
fn deinit_callee(deinit_func: Entity, place_ty: MirTy) -> Callee {
    let type_args = match &place_ty {
        MirTy::Named { type_args, .. } => type_args.clone(),
        _ => Vec::new(),
    };
    Callee::method(deinit_func, type_args, place_ty)
}

fn unwrap_ref(ty: &MirTy) -> &MirTy {
    match ty {
        MirTy::Ref(inner) | MirTy::RefMut(inner) => inner,
        other => other,
    }
}

fn struct_entity(ty: &MirTy) -> Option<Entity> {
    match ty {
        MirTy::Named { entity, .. } => Some(*entity),
        _ => None,
    }
}
