//! Drop expansion — translate `Drop` / `DropIf` statements into the
//! actual `Call(__drop$T, [Move(p)])` shim invocations. Trivial drops
//! disappear entirely.
//!
//! ## Pipeline position
//!
//! 1. `drop_elab` placed `Drop` / `DropIf` statements at the right
//!    program points.
//! 2. `drop_shim` synthesized one `__drop$T(self: T)` function per
//!    non-trivial nominal type. It returned the `nominal → shim`
//!    entity map this pass consumes.
//! 3. This pass walks every Drop/DropIf and replaces it with a call
//!    to the appropriate shim — passing `Move(p)` so the dataflow
//!    correctly kills the path, matching Rust's drop-glue convention.
//!
//! After this pass, the only `Drop` / `DropIf` statements that remain
//! are those whose type didn't get a shim (trivial: no user deinit,
//! no non-trivial fields) — those are removed, not replaced. The
//! post-elaboration verifier should see zero of either kind.
//!
//! ## Shapes
//!
//! - `Drop(p)` where `p.ty` is a non-trivial nominal:
//!
//!   ```text
//!   Call(__drop$<entity_of_p_ty>, type_args=p.ty.type_args, [Move(p)])
//!   ```
//!
//! - `Drop(p)` where `p.ty` is a tuple: inline-decompose. Tuples have
//!   no nominal entity to attach a shim to, so we emit
//!   `Drop(p.0), Drop(p.1), ...` and recurse. Each non-trivial element
//!   gets its own shim call (or trivial drop = removed).
//!
//! - `Drop(p)` where `p.ty` is trivial (primitive, all-Bitwise nominal,
//!   reference, etc.): removed.
//!
//! - `DropIf(p, flag)` where `p.ty` is non-trivial: block-split with
//!   a runtime branch on the flag. Two new blocks are inserted between
//!   the current block's pre-DropIf statements and its post-DropIf
//!   statements; the `then` block contains the shim Call and jumps to
//!   the continuation; the `else` block jumps straight to the
//!   continuation.

use std::collections::HashMap;

use kestrel_hecs::Entity;
use kestrel_mir::passes::place_type;
use kestrel_mir::{
    BasicBlock, BlockId, Callee, FunctionDef, LocalDef, LocalId, MirBody, MirModule, MirTy, Place,
    Rvalue, Statement, StatementKind, Terminator, Value,
};

use crate::drop_shim::ShimMap;

pub fn run(module: &mut MirModule, shim_map: &ShimMap) {
    // Snapshot the module for read-only `place_type` lookups while we
    // mutate function bodies in place. Cheap enough at this stage —
    // module isn't huge.
    let module_snapshot = module.clone();

    for (i, func) in module.functions.iter_mut().enumerate() {
        let Some(body) = func.body.as_mut() else {
            continue;
        };
        let func_snapshot = &module_snapshot.functions[i];
        expand_body(body, func_snapshot, &module_snapshot, shim_map);
    }
}

fn expand_body(
    body: &mut MirBody,
    func_snapshot: &FunctionDef,
    module_snapshot: &MirModule,
    shim_map: &ShimMap,
) {
    // Two-phase walk because DropIf needs CFG surgery: we can't mutate
    // body.blocks while iterating it. Collect the per-block expansions
    // first, then apply them.
    //
    // Each entry is `(block_index, new_stmts, new_terminator)` for
    // blocks that change. Block-splitting (DropIf) appends new blocks
    // and rewrites the original's terminator.
    let snapshot_body = func_snapshot
        .body
        .as_ref()
        .expect("snapshot must have body");

    let block_count = body.blocks.len();
    for bi in 0..block_count {
        expand_block(bi, body, snapshot_body, func_snapshot, module_snapshot, shim_map);
    }
}

fn expand_block(
    bi: usize,
    body: &mut MirBody,
    snapshot_body: &MirBody,
    func_snapshot: &FunctionDef,
    module_snapshot: &MirModule,
    shim_map: &ShimMap,
) {
    // Walk this block's statements. For each Drop, replace inline.
    // For each DropIf, split the block: stash statements after the
    // DropIf into a continuation block, emit a branch terminator on
    // the flag, and a `then` block that does the drop.
    let old_stmts = std::mem::take(&mut body.blocks[bi].stmts);
    let mut new_stmts: Vec<Statement> = Vec::with_capacity(old_stmts.len());

    let mut iter = old_stmts.into_iter().enumerate();
    while let Some((_idx, stmt)) = iter.next() {
        match &stmt.kind {
            StatementKind::Drop { place } => {
                let place = place.clone();
                let Some(ty) = place_type(
                    module_snapshot,
                    snapshot_body,
                    func_snapshot,
                    &place,
                ) else {
                    // Can't resolve type — skip (verifier elsewhere
                    // would have flagged the real issue).
                    continue;
                };
                emit_drop(&mut new_stmts, body, &place, &ty, module_snapshot, shim_map);
            },
            StatementKind::DropIf { place, flag } => {
                // CFG surgery: end this block here with a branch on
                // the flag, route to a then-block that drops, route
                // the else to the continuation. Pack remaining
                // statements + original terminator into the
                // continuation block.
                let place = place.clone();
                let flag = *flag;
                let Some(ty) = place_type(
                    module_snapshot,
                    snapshot_body,
                    func_snapshot,
                    &place,
                ) else {
                    continue;
                };
                // Stash everything after this point into a continuation
                // block; we'll wire the branch to it below.
                let after_stmts: Vec<Statement> = iter.by_ref().map(|(_, s)| s).collect();
                let old_term = std::mem::replace(
                    &mut body.blocks[bi].terminator,
                    Terminator::unreachable(),
                );

                // bb_cont takes the remaining stmts + original term.
                let mut cont_block = BasicBlock::new();
                cont_block.stmts = after_stmts;
                cont_block.terminator = old_term;
                let cont_id = body.add_block(cont_block);

                // bb_then: emit the drop sequence, then jump to cont.
                let mut then_block = BasicBlock::new();
                let mut then_stmts: Vec<Statement> = Vec::new();
                emit_drop(&mut then_stmts, body, &place, &ty, module_snapshot, shim_map);
                then_block.stmts = then_stmts;
                then_block.terminator = Terminator::jump(cont_id);
                let then_id = body.add_block(then_block);

                // Original block: take the stmts accumulated so far,
                // append the branch on flag.
                body.blocks[bi].stmts = std::mem::take(&mut new_stmts);
                body.blocks[bi].terminator = Terminator::branch(
                    Value::Copy(Place::local(flag)),
                    then_id,
                    cont_id,
                );
                // No more processing for this block; the rest of the
                // statements are in cont_block, which will be visited
                // by the outer block_count loop only if its index is
                // < block_count at start (it isn't — newly added).
                // That's intentional: cont_block contains the
                // already-expanded continuation. If it contains more
                // Drop/DropIf statements, recurse on it.
                expand_block(
                    cont_id.index(),
                    body,
                    snapshot_body,
                    func_snapshot,
                    module_snapshot,
                    shim_map,
                );
                return;
            },
            _ => new_stmts.push(stmt),
        }
    }
    body.blocks[bi].stmts = new_stmts;
}

/// Emit the drop sequence for `place: ty` into `out`. Allocates new
/// locals in `body` as needed (for projection-move temps).
fn emit_drop(
    out: &mut Vec<Statement>,
    body: &mut MirBody,
    place: &Place,
    ty: &MirTy,
    module: &MirModule,
    shim_map: &ShimMap,
) {
    match ty {
        MirTy::Named { entity, type_args } => {
            // Nominal: call the shim if one exists. Trivial nominals
            // (not in shim_map) drop out silently.
            let Some(shim_entity) = shim_map.get(entity).copied() else {
                return;
            };
            // Project-move into a temp so the recursive shim takes
            // ownership cleanly. The temp's path is killed when moved
            // into the call, matching what `Drop(p)` semantically
            // means — `p` is consumed.
            let temp = body.add_local(LocalDef::new("_drop_arg", ty.clone()));
            out.push(Statement::new(StatementKind::Assign {
                dest: Place::local(temp),
                rvalue: Rvalue::Move(place.clone()),
            }));
            let callee = Callee::direct_generic(shim_entity, type_args.clone());
            out.push(Statement::new(StatementKind::Call {
                dest: None,
                callee,
                args: vec![Value::Move(Place::local(temp))],
            }));
        },
        MirTy::Tuple(elems) => {
            // Tuples have no nominal entity. Decompose by index and
            // emit a drop for each non-trivial element.
            for (i, elem_ty) in elems.iter().enumerate() {
                if !ty_might_need_drop(elem_ty, shim_map) {
                    continue;
                }
                let elem_place = Place::Index {
                    parent: Box::new(place.clone()),
                    index: i,
                };
                emit_drop(out, body, &elem_place, elem_ty, module, shim_map);
            }
        },
        // Trivial / Bitwise / Cloneable / unresolved — no drop work.
        _ => {},
    }
}

/// Quick predicate: would emitting a drop for `ty` produce any
/// statements? `Named` types check the shim map; tuples recurse.
/// Used to skip allocating temps for trivial drops.
fn ty_might_need_drop(ty: &MirTy, shim_map: &ShimMap) -> bool {
    match ty {
        MirTy::Named { entity, .. } => shim_map.contains_key(entity),
        MirTy::Tuple(elems) => elems.iter().any(|t| ty_might_need_drop(t, shim_map)),
        _ => false,
    }
}
