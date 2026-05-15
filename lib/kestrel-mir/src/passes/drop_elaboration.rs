//! Unified drop elaboration pass — single-source destructor insertion.
//!
//! Replaces the old deinit + expand_deinit two-pass system. Tracks init-state
//! per droppable local via forward dataflow, inserts cleanup at all exit points
//! (returns, break/continue, back-edges), and expands into concrete CFG + Call.

use std::collections::{HashMap, HashSet};

use crate::MirModule;
use crate::body::{BasicBlock, LocalDef, MirBody, ScopeId};
use crate::id::{BlockId, LocalId};
use crate::item::FunctionKind;
use crate::place::Place;
use crate::statement::{Callee, Rvalue, Statement, StatementKind};
use crate::terminator::{SwitchCase, Terminator, TerminatorKind};
use crate::ty::MirTy;
use crate::value::Value;
use kestrel_hecs::Entity;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn run_drop_elaboration(module: &mut MirModule) {
    let types_with_deinit = collect_types_with_deinit(module);
    let structs_with_droppable_fields = collect_structs_with_droppable_fields(module, &types_with_deinit);
    let types_needing_drop = compute_types_needing_drop(module, &types_with_deinit, &structs_with_droppable_fields);

    let deinit_funcs: HashMap<Entity, Entity> = module
        .functions
        .iter()
        .filter_map(|f| match &f.kind {
            FunctionKind::Deinit { parent } => Some((*parent, f.entity)),
            _ => None,
        })
        .collect();

    // Pre-compute set of functions with consuming receivers
    let consuming_receiver_funcs: HashSet<Entity> = module
        .functions
        .iter()
        .filter_map(|f| match &f.kind {
            FunctionKind::Method { receiver: crate::item::ReceiverConvention::Consuming, .. } => Some(f.entity),
            _ => None,
        })
        .collect();

    // Phase A: collect droppable locals for each function (immutable borrow)
    let per_func: Vec<(usize, Vec<DroppableLocal>, bool, bool)> = (0..module.functions.len())
        .filter_map(|func_idx| {
            let func = &module.functions[func_idx];
            let body = func.body.as_ref()?;
            let droppable = identify_droppable_locals(
                body, func, &types_needing_drop, &structs_with_droppable_fields,
            );
            let is_effectful_init = matches!(func.kind, FunctionKind::Initializer { .. })
                && !body.failure_return_blocks.is_empty();
            Some((func_idx, droppable, is_effectful_init, true))
        })
        .collect();

    // Phase B + C + D: mutate each function body
    // Collect return types before mutating (avoids borrow conflict)
    let per_func_ret_tys: Vec<MirTy> = per_func
        .iter()
        .map(|(idx, _, _, _)| module.functions[*idx].ret.clone())
        .collect();

    for (i, (func_idx, droppable, is_effectful_init, _)) in per_func.iter().enumerate() {
        let body = module.functions[*func_idx].body.as_mut().unwrap();

        if *is_effectful_init {
            insert_init_field_deinits(body);
        }

        if !droppable.is_empty() {
            elaborate_drops(body, droppable, &consuming_receiver_funcs, &per_func_ret_tys[i]);
        }

    }

    // Phase D: expand Deinit/DeinitIf into CFG + Call (needs &module for type resolution)
    // Expansion can create new Deinit nodes (e.g. nested enum drops), so
    // loop until no more expansions are found (max 8 iterations for safety).
    for _ in 0..8 {
        let mut any_expanded = false;
        for (func_idx, _, _, _) in &per_func {
            let expansions = {
                let body = module.functions[*func_idx].body.as_ref().unwrap();
                collect_expansions(body, &deinit_funcs, module)
            };
            if !expansions.is_empty() {
                any_expanded = true;
                let body = module.functions[*func_idx].body.as_mut().unwrap();
                apply_expansions(body, expansions);
            }
        }
        if !any_expanded { break; }
    }

    // Phase E: field cascading — inject sub-field deinit calls in deinit bodies
    inject_field_deinits(module, &deinit_funcs);
}

// ---------------------------------------------------------------------------
// Phase A: identify droppable locals
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct DroppableLocal {
    local: LocalId,
    scope: ScopeId,
    starts_live: bool,
}

fn identify_droppable_locals(
    body: &MirBody,
    func: &crate::item::FunctionDef,
    types_needing_drop: &HashSet<Entity>,
    structs_with_droppable_fields: &HashSet<Entity>,
) -> Vec<DroppableLocal> {
    // Step 1: find locals that are Construct/EnumVariant/Call targets for droppable types
    let mut construct_targets: HashMap<LocalId, Entity> = HashMap::new();
    let mut call_result_targets: HashSet<LocalId> = HashSet::new();
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
                    let e = if is_type_droppable(enum_ty, types_needing_drop, structs_with_droppable_fields) {
                        match enum_ty {
                            MirTy::Named { entity, .. } => Some(*entity),
                            _ => None,
                        }
                    } else {
                        None
                    };
                    (dest, e)
                }
                StatementKind::Call { dest: Some(dest), .. } => {
                    let local_id = dest.root_local();
                    if let Some(id) = local_id {
                        let ty = &body.locals[id.index()].ty;
                        if is_type_droppable(ty, types_needing_drop, structs_with_droppable_fields) {
                            let e = match ty {
                                MirTy::Named { entity, .. } => Some(*entity),
                                _ => None,
                            };
                            call_result_targets.insert(id);
                            (dest, e)
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    }
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

    // Step 2: follow Construct→Copy/Move chains to find final owners.
    // Move is emitted when the source is a temp (ownership transfer);
    // Copy is emitted when the source is a user local (value duplication).
    let mut owners: HashMap<LocalId, Entity> = HashMap::new();
    let mut copied_from: HashSet<LocalId> = HashSet::new();
    for block in &body.blocks {
        for stmt in &block.stmts {
            let (dest, src) = match &stmt.kind {
                StatementKind::Assign { dest, rvalue: Rvalue::Copy(src) }
                | StatementKind::Assign { dest, rvalue: Rvalue::Move(src) } => (dest, src),
                _ => continue,
            };
            if let (Some(dest_id), Some(src_id)) = (dest.root_local(), src.root_local()) {
                if let Some(&entity) = construct_targets.get(&src_id) {
                    owners.insert(dest_id, entity);
                    copied_from.insert(src_id);
                }
            }
        }
    }

    // Step 3: merge — owner is the copy target if it exists, else the construct target.
    // Call-result targets that are NOT copied to another local are excluded — they
    // are intermediary temps whose lifetime is managed by the caller, not owned here.
    let mut result_set = HashSet::new();
    for &local_id in owners.keys() {
        result_set.insert(local_id);
    }
    for &local_id in construct_targets.keys() {
        if !copied_from.contains(&local_id) {
            if call_result_targets.contains(&local_id) {
                continue;
            }
            result_set.insert(local_id);
        }
    }

    // Step 4: add consuming params (including consuming-self receivers).
    // A consuming param's type is not Ref/RefMut — ownership is baked into the type.
    let is_consuming_self_method = matches!(
        &func.kind,
        FunctionKind::Method { receiver: crate::item::ReceiverConvention::Consuming, .. }
    );
    for (pi, param) in func.params.iter().enumerate() {
        let is_consuming = !matches!(&param.ty, MirTy::Ref(_) | MirTy::RefMut(_));
        let dominated = is_consuming
            || (is_consuming_self_method && pi == 0);
        if dominated {
            let local_id = param.local;
            if is_type_droppable(&body.locals[local_id.index()].ty, types_needing_drop, structs_with_droppable_fields) {
                result_set.insert(local_id);
            }
        }
    }

    // Step 4b: add loop-scoped locals with droppable types.
    // These may be assigned via Copy (e.g. pattern bindings in for-in),
    // not Construct, so the Construct-target scan misses them.
    for (&local_id, _scope) in &body.local_scopes {
        if is_type_droppable(&body.locals[local_id.index()].ty, types_needing_drop, structs_with_droppable_fields) {
            result_set.insert(local_id);
        }
    }

    // Step 4c is disabled. A broad "scan all droppable-typed locals" catches
    // temporaries from CowBox.write() whose buffers are manually freed by
    // the caller (e.g. String.grow frees oldStorage.ptr then sets a new
    // StringStorage). The consuming-setValue convention handles the
    // checkout/checkin pattern (appendByte), but not manual-free patterns
    // (grow). Enabling this requires a way to mark locals as "already
    // cleaned up" — either via a consume annotation or explicit dead-marking.

    // Step 5: build result, excluding params that aren't consuming.
    // Step 4 already added the consuming params (including consuming-self),
    // so the filter here just keeps non-param locals and those params.
    let mut result: Vec<DroppableLocal> = result_set
        .into_iter()
        .filter(|id| {
            let is_param = id.index() < body.param_count;
            if is_param {
                // Only keep params that Step 4 explicitly added (consuming or consuming-self)
                let dominated = func.params.iter().enumerate().any(|(pi, p)| {
                    let is_consuming = !matches!(&p.ty, MirTy::Ref(_) | MirTy::RefMut(_));
                    p.local == *id && (is_consuming
                        || (is_consuming_self_method && pi == 0))
                });
                dominated
            } else {
                true
            }
        })
        .map(|local_id| {
            let is_param = local_id.index() < body.param_count;
            let scope = body.local_scopes.get(&local_id).copied().unwrap_or(ScopeId::Function);
            DroppableLocal {
                local: local_id,
                scope,
                starts_live: is_param,
            }
        })
        .collect();

    result.sort_by_key(|d| d.local.index());
    result
}

pub(crate) fn is_type_droppable(
    ty: &MirTy,
    types_needing_drop: &HashSet<Entity>,
    structs_with_droppable_fields: &HashSet<Entity>,
) -> bool {
    match ty {
        MirTy::Named { entity, type_args } => {
            if types_needing_drop.contains(entity) || structs_with_droppable_fields.contains(entity) {
                return true;
            }
            // Check type args for droppable types (e.g. Optional[Container])
            type_args.iter().any(|arg| is_type_droppable(arg, types_needing_drop, structs_with_droppable_fields))
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Phase B + C: dataflow analysis + deinit insertion
// ---------------------------------------------------------------------------

/// Init state for a single droppable local at a program point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InitState {
    Dead,
    Live,
    Maybe,
}

impl InitState {
    /// Meet operator for join points: if both paths agree, keep that;
    /// otherwise it's Maybe (needs runtime flag).
    fn meet(self, other: Self) -> Self {
        if self == other { self } else { InitState::Maybe }
    }
}

fn elaborate_drops(body: &mut MirBody, droppable: &[DroppableLocal], consuming_receiver_funcs: &HashSet<Entity>, ret_ty: &MirTy) {
    if droppable.is_empty() {
        return;
    }

    let num_locals = droppable.len();
    let local_to_idx: HashMap<LocalId, usize> = droppable.iter()
        .enumerate()
        .map(|(i, d)| (d.local, i))
        .collect();

    // Build predecessor map + identify loop structure
    let num_blocks = body.blocks.len();
    let mut predecessors: Vec<Vec<usize>> = vec![Vec::new(); num_blocks];
    let mut is_back_edge: HashSet<(usize, usize)> = HashSet::new();

    // Compute RPO for back-edge detection
    let rpo = compute_rpo(body);
    let rpo_order: Vec<usize> = {
        let mut order = vec![0usize; num_blocks];
        for (pos, &block) in rpo.iter().enumerate() {
            order[block] = pos;
        }
        order
    };

    for bi in 0..num_blocks {
        for succ in body.blocks[bi].successors() {
            let si = succ.index();
            predecessors[si].push(bi);
            if rpo_order[si] <= rpo_order[bi] {
                is_back_edge.insert((bi, si));
            }
        }
    }

    // Build loop info: which loop headers exist, which blocks are in each loop body
    let loop_headers: HashSet<usize> = is_back_edge.iter().map(|&(_, header)| header).collect();
    let loop_body_blocks = compute_loop_bodies(num_blocks, &predecessors, &is_back_edge);

    // Build a map from loop header → exit block from local_scopes
    let mut scope_loop_exits: HashMap<usize, usize> = HashMap::new();
    for d in droppable.iter() {
        if let ScopeId::Loop { header, exit } = d.scope {
            scope_loop_exits.insert(header.index(), exit.index());
        }
    }

    // Forward dataflow: compute init state at exit of each block
    let mut block_exit_states: Vec<Vec<InitState>> = vec![vec![InitState::Dead; num_locals]; num_blocks];

    // Entry block: consuming params start Live
    let entry = body.entry.index();
    for (i, d) in droppable.iter().enumerate() {
        if d.starts_live {
            block_exit_states[entry][i] = InitState::Live;
        }
    }

    // Worklist-based iteration
    let mut worklist: Vec<usize> = rpo.clone();
    let mut in_worklist: Vec<bool> = vec![true; num_blocks];
    let mut block_entry_states: Vec<Vec<InitState>> = vec![vec![InitState::Dead; num_locals]; num_blocks];
    // Entry block's entry state: params are live
    for (i, d) in droppable.iter().enumerate() {
        if d.starts_live {
            block_entry_states[entry][i] = InitState::Live;
        }
    }

    let max_iterations = num_blocks * 4 + 10;
    let mut iterations = 0;
    while let Some(bi) = worklist.pop() {
        in_worklist[bi] = false;
        iterations += 1;
        if iterations > max_iterations {
            break;
        }

        // Compute entry state from predecessors
        let preds = &predecessors[bi];
        if !preds.is_empty() && bi != entry {
            let mut merged = block_exit_states[preds[0]].clone();
            for &pred in &preds[1..] {
                for i in 0..num_locals {
                    merged[i] = merged[i].meet(block_exit_states[pred][i]);
                }
            }
            block_entry_states[bi] = merged;
        }

        // Transfer through block
        let mut state = block_entry_states[bi].clone();
        transfer_block(&body.blocks[bi], &local_to_idx, &mut state, consuming_receiver_funcs);

        // Check if exit state changed
        if state != block_exit_states[bi] {
            block_exit_states[bi] = state;
            for succ in body.blocks[bi].successors() {
                let si = succ.index();
                if !in_worklist[si] {
                    in_worklist[si] = true;
                    worklist.push(si);
                }
            }
        }
    }

    // Phase C: insert deinit statements at exit points
    // Collect all insertions first, then apply

    // Create flags for Maybe-state locals (check both entry and exit states,
    // since a Maybe at block entry may be overwritten to Live by an assignment)
    let mut flags: HashMap<LocalId, LocalId> = HashMap::new();
    let mut needs_flag: HashSet<usize> = HashSet::new();
    for bi in 0..num_blocks {
        for i in 0..num_locals {
            if block_exit_states[bi][i] == InitState::Maybe
                || block_entry_states[bi][i] == InitState::Maybe
            {
                needs_flag.insert(i);
            }
        }
    }

    for &idx in &needs_flag {
        let local = droppable[idx].local;
        if !flags.contains_key(&local) {
            let flag_name = format!("_drop_flag_{}", body.locals[local.index()].name);
            let flag_id = body.add_local(LocalDef::new(flag_name, MirTy::Bool));
            flags.insert(local, flag_id);
        }
    }

    // Insert flag initialization at entry block.
    // Flag convention (matches existing init-field flags):
    //   true  = skip deinit (dead/moved/uninit)
    //   false = needs deinit (live)
    if !flags.is_empty() {
        let entry_idx = body.entry.index();
        let mut flag_inits: Vec<Statement> = Vec::new();
        for d in droppable {
            if let Some(&flag) = flags.get(&d.local) {
                // starts_live → false (needs deinit), else → true (skip)
                flag_inits.push(Statement::new(StatementKind::SetDeinitFlag {
                    flag,
                    value: !d.starts_live,
                }));
            }
        }
        // Prepend at entry block
        let existing = std::mem::take(&mut body.blocks[entry_idx].stmts);
        body.blocks[entry_idx].stmts = flag_inits;
        body.blocks[entry_idx].stmts.extend(existing);
    }

    // Insert flag updates throughout the body
    insert_flag_updates(body, &local_to_idx, &flags, droppable, consuming_receiver_funcs);

    // Insert overwrite-drops: deinit old value before reassignment to a live local
    insert_overwrite_drops(body, droppable, &block_entry_states, &local_to_idx, consuming_receiver_funcs, &flags);

    // Now insert deinit statements at exit points
    insert_deinits_at_exits(
        body, droppable, &block_entry_states, &block_exit_states, &local_to_idx,
        &flags, &loop_headers, &loop_body_blocks, &is_back_edge,
        consuming_receiver_funcs, &scope_loop_exits, ret_ty,
    );
}

/// Apply transfer functions for one block, updating init state.
fn transfer_block(
    block: &BasicBlock,
    local_to_idx: &HashMap<LocalId, usize>,
    state: &mut Vec<InitState>,
    consuming_receiver_funcs: &HashSet<Entity>,
) {
    for stmt in &block.stmts {
        transfer_statement(&stmt.kind, local_to_idx, state, consuming_receiver_funcs);
    }
    // Return terminator consumes the returned local
    if let TerminatorKind::Return(val) = &block.terminator.kind {
        if let Some(place) = val.as_place() {
            if let Some(local) = place.root_local() {
                if let Some(&idx) = local_to_idx.get(&local) {
                    state[idx] = InitState::Dead;
                }
            }
        }
    }
}

fn transfer_statement(
    kind: &StatementKind,
    local_to_idx: &HashMap<LocalId, usize>,
    state: &mut Vec<InitState>,
    consuming_receiver_funcs: &HashSet<Entity>,
) {
    match kind {
        // Assignment to a droppable local makes it Live
        StatementKind::Assign { dest, rvalue } => {
            if let Some(local) = dest.root_local() {
                if let Some(&idx) = local_to_idx.get(&local) {
                    state[idx] = InitState::Live;
                }
            }
            // Construct consumes field values
            if let Rvalue::Construct { fields, .. } = rvalue {
                for (_, value) in fields {
                    if let Some(place) = value.as_place() {
                        if let Some(local) = place.root_local() {
                            if let Some(&idx) = local_to_idx.get(&local) {
                                state[idx] = InitState::Dead;
                            }
                        }
                    }
                }
            }
            // EnumVariant consumes payload values
            if let Rvalue::EnumVariant { payload, .. } = rvalue {
                for value in payload {
                    if let Some(place) = value.as_place() {
                        if let Some(local) = place.root_local() {
                            if let Some(&idx) = local_to_idx.get(&local) {
                                state[idx] = InitState::Dead;
                            }
                        }
                    }
                }
            }
            // ApplyPartial consumes capture values
            if let Rvalue::ApplyPartial { captures, .. } = rvalue {
                for value in captures {
                    if let Some(place) = value.as_place() {
                        if let Some(local) = place.root_local() {
                            if let Some(&idx) = local_to_idx.get(&local) {
                                state[idx] = InitState::Dead;
                            }
                        }
                    }
                }
            }
        }
        // Call with Value::Move or consuming receiver consumes the argument
        StatementKind::Call { args, callee, .. } => {
            // Check for consuming receiver: first arg is self, consumed by the method
            let has_consuming_receiver = match callee {
                Callee::Direct { func, .. } => consuming_receiver_funcs.contains(func),
                _ => false,
            };
            for (ai, arg) in args.iter().enumerate() {
                let is_consumed = matches!(arg, Value::Move(_))
                    || (ai == 0 && has_consuming_receiver);
                if is_consumed {
                    if let Some(place) = arg.as_place() {
                        if let Some(local) = place.root_local() {
                            if let Some(&idx) = local_to_idx.get(&local) {
                                state[idx] = InitState::Dead;
                            }
                        }
                    }
                }
            }
        }
        // Explicit deinit consumes the local
        StatementKind::Deinit { place } => {
            if let Some(local) = place.root_local() {
                if let Some(&idx) = local_to_idx.get(&local) {
                    state[idx] = InitState::Dead;
                }
            }
        }
        // ScopeLive resets init state (loop re-entry)
        StatementKind::ScopeLive(local) => {
            if let Some(&idx) = local_to_idx.get(local) {
                state[idx] = InitState::Dead;
            }
        }
        // DeinitIf from effectful-init partial drop — treated as consuming
        StatementKind::DeinitIf { place, .. } => {
            if let Some(local) = place.root_local() {
                if let Some(&idx) = local_to_idx.get(&local) {
                    state[idx] = InitState::Dead;
                }
            }
        }
        StatementKind::SetDeinitFlag { .. } => {}
        // Drop/DropIf are emitted by the ownership pass, not by drop elaboration.
        // They should not appear during the elaboration dataflow, but handle them
        // gracefully as no-ops for the init-state transfer.
        StatementKind::Drop { .. } | StatementKind::DropIf { .. } => {}
    }
}

/// Insert SetDeinitFlag updates wherever init-state changes for Maybe locals.
fn insert_flag_updates(
    body: &mut MirBody,
    _local_to_idx: &HashMap<LocalId, usize>,
    flags: &HashMap<LocalId, LocalId>,
    _droppable: &[DroppableLocal],
    consuming_receiver_funcs: &HashSet<Entity>,
) {
    if flags.is_empty() {
        return;
    }

    // Flag convention: true = skip deinit (dead), false = needs deinit (live)
    for bi in 0..body.blocks.len() {
        let mut insertions: Vec<(usize, Statement)> = Vec::new();
        for (si, stmt) in body.blocks[bi].stmts.iter().enumerate() {
            match &stmt.kind {
                // Assignment → local becomes live → flag = false (needs deinit)
                StatementKind::Assign { dest, rvalue } => {
                    if let Some(local) = dest.root_local() {
                        if let Some(&flag) = flags.get(&local) {
                            insertions.push((si + 1, Statement::new(
                                StatementKind::SetDeinitFlag { flag, value: false },
                            )));
                        }
                    }
                    // Construct field consumption → flag = true (skip deinit)
                    let consumed = collect_consumed_locals(rvalue);
                    for local in consumed {
                        if let Some(&flag) = flags.get(&local) {
                            insertions.push((si + 1, Statement::new(
                                StatementKind::SetDeinitFlag { flag, value: true },
                            )));
                        }
                    }
                }
                // Call with Move args or consuming receiver -> flag = true (skip deinit)
                StatementKind::Call { args, callee, .. } => {
                    let has_consuming_receiver = match callee {
                        Callee::Direct { func, .. } => consuming_receiver_funcs.contains(func),
                        _ => false,
                    };
                    for (ai, arg) in args.iter().enumerate() {
                        let is_consumed = matches!(arg, Value::Move(_))
                            || (ai == 0 && has_consuming_receiver);
                        if is_consumed {
                            if let Some(place) = arg.as_place() {
                                if let Some(local) = place.root_local() {
                                    if let Some(&flag) = flags.get(&local) {
                                        insertions.push((si + 1, Statement::new(
                                            StatementKind::SetDeinitFlag { flag, value: true },
                                        )));
                                    }
                                }
                            }
                        }
                    }
                }
                // Explicit deinit → flag = true (skip deinit, already consumed)
                StatementKind::Deinit { place } => {
                    if let Some(local) = place.root_local() {
                        if let Some(&flag) = flags.get(&local) {
                            insertions.push((si + 1, Statement::new(
                                StatementKind::SetDeinitFlag { flag, value: true },
                            )));
                        }
                    }
                }
                // ScopeLive → local is uninit → flag = true (skip deinit)
                StatementKind::ScopeLive(local) => {
                    if let Some(&flag) = flags.get(local) {
                        insertions.push((si + 1, Statement::new(
                            StatementKind::SetDeinitFlag { flag, value: true },
                        )));
                    }
                }
                _ => {}
            }
        }
        // Insert in reverse order to preserve indices
        for (pos, stmt) in insertions.into_iter().rev() {
            body.blocks[bi].stmts.insert(pos, stmt);
        }
    }
}

/// Insert overwrite-drops before reassignment to a droppable local that's already Live/Maybe.
fn insert_overwrite_drops(
    body: &mut MirBody,
    droppable: &[DroppableLocal],
    block_entry_states: &[Vec<InitState>],
    local_to_idx: &HashMap<LocalId, usize>,
    consuming_receiver_funcs: &HashSet<Entity>,
    flags: &HashMap<LocalId, LocalId>,
) {
    let num_blocks = body.blocks.len();
    for bi in 0..num_blocks {
        let mut state = block_entry_states[bi].clone();
        let mut insertions: Vec<(usize, Statement)> = Vec::new();

        for si in 0..body.blocks[bi].stmts.len() {
            let kind = &body.blocks[bi].stmts[si].kind;
            if let StatementKind::Assign { dest, .. } = kind {
                // Only trigger overwrite-drop for full local assignments,
                // not field/index/deref writes (which update part of the value)
                let is_full_local = matches!(dest, Place::Local(_));
                if is_full_local {
                if let Some(local) = dest.root_local() {
                    if let Some(&idx) = local_to_idx.get(&local) {
                        match state[idx] {
                            InitState::Live => {
                                insertions.push((si, Statement::new(StatementKind::Deinit {
                                    place: Place::local(droppable[idx].local),
                                })));
                            }
                            InitState::Maybe => {
                                if let Some(&flag) = flags.get(&droppable[idx].local) {
                                    insertions.push((si, Statement::new(StatementKind::DeinitIf {
                                        place: Place::local(droppable[idx].local),
                                        flag,
                                    })));
                                }
                            }
                            InitState::Dead => {}
                        }
                    }
                }
                }
            }
            transfer_statement(kind, local_to_idx, &mut state, consuming_receiver_funcs);
        }

        // Insert in reverse order to preserve indices
        for (pos, stmt) in insertions.into_iter().rev() {
            body.blocks[bi].stmts.insert(pos, stmt);
        }
    }
}

/// Collect locals consumed by an rvalue (Construct fields, EnumVariant payload, ApplyPartial captures).
/// Note: Rvalue::Move is handled separately in the main dataflow, not here —
/// this function is for effectful init flag tracking where Move of a temp
/// should not mark the source field as "skip deinit".
fn collect_consumed_locals(rvalue: &Rvalue) -> Vec<LocalId> {
    let mut consumed = Vec::new();
    match rvalue {
        Rvalue::Construct { fields, .. } => {
            for (_, value) in fields {
                if let Some(place) = value.as_place() {
                    if let Some(local) = place.root_local() {
                        consumed.push(local);
                    }
                }
            }
        }
        Rvalue::EnumVariant { payload, .. } => {
            for value in payload {
                if let Some(place) = value.as_place() {
                    if let Some(local) = place.root_local() {
                        consumed.push(local);
                    }
                }
            }
        }
        Rvalue::ApplyPartial { captures, .. } => {
            for value in captures {
                if let Some(place) = value.as_place() {
                    if let Some(local) = place.root_local() {
                        consumed.push(local);
                    }
                }
            }
        }
        _ => {}
    }
    consumed
}

/// Insert Deinit/DeinitIf at all exit points based on scope and init-state.
fn insert_deinits_at_exits(
    body: &mut MirBody,
    droppable: &[DroppableLocal],
    block_entry_states: &[Vec<InitState>],
    _block_exit_states: &[Vec<InitState>],
    local_to_idx: &HashMap<LocalId, usize>,
    flags: &HashMap<LocalId, LocalId>,
    _loop_headers: &HashSet<usize>,
    loop_body_blocks: &HashMap<usize, HashSet<usize>>,
    is_back_edge: &HashSet<(usize, usize)>,
    consuming_receiver_funcs: &HashSet<Entity>,
    scope_loop_exits: &HashMap<usize, usize>,
    ret_ty: &MirTy,
) {
    let num_blocks = body.blocks.len();
    for bi in 0..num_blocks {
        let term = &body.blocks[bi].terminator.kind.clone();
        match term {
            TerminatorKind::Return(ret_val) => {
                // Return: clean up ALL droppable locals except the returned one
                let returned_local = ret_val.as_place().and_then(|p| p.root_local());

                // Compute state just before the return (after all stmts, before terminator)
                let mut state = block_entry_states[bi].clone();
                transfer_block_stmts_only(&body.blocks[bi], local_to_idx, &mut state, consuming_receiver_funcs);
                // Return consumes the returned local
                if let Some(ret_local) = returned_local {
                    if let Some(&idx) = local_to_idx.get(&ret_local) {
                        state[idx] = InitState::Dead;
                    }
                }

                let deinit_stmts = build_deinit_stmts(droppable, &state, flags, None);
                if !deinit_stmts.is_empty() {
                    // If the return value is not a simple local (e.g., a global or
                    // field read), capture it into a temp before cleanup so the
                    // cleanup deinits don't modify it before it's read.
                    let needs_capture = ret_val.as_place()
                        .map(|p| p.root_local().is_none())
                        .unwrap_or(false);
                    if needs_capture {
                        let ret_temp = body.add_local(LocalDef::new("_ret_capture", ret_ty.clone()));
                        body.blocks[bi].stmts.push(Statement::new(StatementKind::Assign {
                            dest: Place::local(ret_temp),
                            rvalue: Rvalue::Copy(ret_val.as_place().unwrap().clone()),
                        }));
                        body.blocks[bi].terminator.kind =
                            TerminatorKind::Return(Value::Copy(Place::local(ret_temp)));
                    }
                }
                body.blocks[bi].stmts.extend(deinit_stmts);
            }
            TerminatorKind::Jump(target) => {
                let target_idx = target.index();
                if is_back_edge.contains(&(bi, target_idx)) {
                    // Back-edge: clean up only locals scoped to THIS loop
                    let mut state = block_entry_states[bi].clone();
                    transfer_block_stmts_only(&body.blocks[bi], local_to_idx, &mut state, consuming_receiver_funcs);
                    let mut this_loop = HashSet::new();
                    this_loop.insert(target_idx);
                    let deinit_stmts = build_deinit_stmts(droppable, &state, flags, Some(&this_loop));
                    body.blocks[bi].stmts.extend(deinit_stmts);
                } else {
                    // Check if this jump exits any loop scope using both
                    // back-edge-detected loops and dominator-based scope analysis
                    let mut loops_exited = find_loops_exited(bi, target_idx, loop_body_blocks);

                    // Also check scope exits via exit-block matching.
                    // If the target is an exit block for ANY loop scope, ALL
                    // inner loop scopes are also exited (structured control flow
                    // guarantee: break outer exits all nested loops).
                    let exits_any_loop = scope_loop_exits.values().any(|&exit| target_idx == exit);
                    if exits_any_loop {
                        for (&header, _) in scope_loop_exits {
                            loops_exited.insert(header);
                        }
                    }

                    if !loops_exited.is_empty() {
                        let mut state = block_entry_states[bi].clone();
                        transfer_block_stmts_only(&body.blocks[bi], local_to_idx, &mut state, consuming_receiver_funcs);
                        let deinit_stmts = build_deinit_stmts(droppable, &state, flags, Some(&loops_exited));
                        body.blocks[bi].stmts.extend(deinit_stmts);
                    }
                }
            }
            // Branch: each successor might be a loop exit
            TerminatorKind::Branch { .. } => {
                // We don't insert deinits at branch points — only at the
                // target blocks' own exits. Branch targets are handled when
                // they themselves are Returns or Jumps.
            }
            _ => {}
        }
    }
}

/// Transfer only statements (not the terminator) to get state before terminator.
fn transfer_block_stmts_only(
    block: &BasicBlock,
    local_to_idx: &HashMap<LocalId, usize>,
    state: &mut Vec<InitState>,
    consuming_receiver_funcs: &HashSet<Entity>,
) {
    for stmt in &block.stmts {
        transfer_statement(&stmt.kind, local_to_idx, state, consuming_receiver_funcs);
    }
}

/// Build Deinit/DeinitIf statements for locals that need cleanup.
/// If `only_loops` is Some, only clean locals scoped to those loop headers.
/// If None, clean all locals (for returns).
fn build_deinit_stmts(
    droppable: &[DroppableLocal],
    state: &[InitState],
    flags: &HashMap<LocalId, LocalId>,
    only_loops: Option<&HashSet<usize>>,
) -> Vec<Statement> {
    let mut stmts = Vec::new();
    for (i, d) in droppable.iter().enumerate().rev() {
        if let Some(loops) = only_loops {
            match d.scope {
                ScopeId::Loop { header, .. } => {
                    if !loops.contains(&header.index()) { continue; }
                }
                ScopeId::Function => continue,
            }
        }
        match state[i] {
            InitState::Dead => {}
            InitState::Live => {
                stmts.push(Statement::new(StatementKind::Deinit {
                    place: Place::local(d.local),
                }));
            }
            InitState::Maybe => {
                if let Some(&flag) = flags.get(&d.local) {
                    stmts.push(Statement::new(StatementKind::DeinitIf {
                        place: Place::local(d.local),
                        flag,
                    }));
                }
            }
        }
    }
    stmts
}

/// Find which loop headers' bodies contain `source` but not `target`.
fn find_loops_exited(
    source: usize,
    target: usize,
    loop_body_blocks: &HashMap<usize, HashSet<usize>>,
) -> HashSet<usize> {
    let mut exited = HashSet::new();
    for (&header, body) in loop_body_blocks {
        if body.contains(&source) && !body.contains(&target) {
            exited.insert(header);
        }
    }
    exited
}

// ---------------------------------------------------------------------------
// CFG analysis utilities
// ---------------------------------------------------------------------------

/// Compute reverse post-order of the CFG.
fn compute_rpo(body: &MirBody) -> Vec<usize> {
    let num_blocks = body.blocks.len();
    let mut visited = vec![false; num_blocks];
    let mut postorder = Vec::with_capacity(num_blocks);

    fn dfs(bi: usize, blocks: &[BasicBlock], visited: &mut Vec<bool>, postorder: &mut Vec<usize>) {
        if visited[bi] { return; }
        visited[bi] = true;
        for succ in blocks[bi].successors() {
            dfs(succ.index(), blocks, visited, postorder);
        }
        postorder.push(bi);
    }

    dfs(body.entry.index(), &body.blocks, &mut visited, &mut postorder);
    postorder.reverse();
    postorder
}

/// Compute loop bodies: for each loop header, the set of blocks in that loop.
fn compute_loop_bodies(
    _num_blocks: usize,
    predecessors: &[Vec<usize>],
    back_edges: &HashSet<(usize, usize)>,
) -> HashMap<usize, HashSet<usize>> {
    let mut loop_bodies: HashMap<usize, HashSet<usize>> = HashMap::new();

    for &(tail, header) in back_edges {
        let body_set = loop_bodies.entry(header).or_insert_with(|| {
            let mut set = HashSet::new();
            set.insert(header);
            set
        });

        // BFS backwards from tail to header to find all loop body blocks
        if body_set.contains(&tail) { continue; }
        let mut stack = vec![tail];
        body_set.insert(tail);
        while let Some(block) = stack.pop() {
            for &pred in &predecessors[block] {
                if !body_set.contains(&pred) {
                    body_set.insert(pred);
                    stack.push(pred);
                }
            }
        }
    }

    loop_bodies
}

// ---------------------------------------------------------------------------
// Effectful-init partial drop (unchanged from old deinit.rs)
// ---------------------------------------------------------------------------

fn insert_init_field_deinits(body: &mut MirBody) {
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
        let deinit_stmts: Vec<Statement> = init_flags
            .iter()
            .rev()
            .map(|(field_name, flag)| {
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

// ---------------------------------------------------------------------------
// Phase D: expand Deinit/DeinitIf into CFG + Call (ported from expand_deinit.rs)
// ---------------------------------------------------------------------------

enum Expansion {
    ReplaceDeinit {
        block: usize,
        stmt: usize,
        deinit_entity: Option<Entity>,
        place: Place,
        place_ty: MirTy,
        /// Extra field-level cascade for TypeParam fields resolved at the call site
        extra_field_drops: Vec<(Vec<String>, Entity, MirTy)>,
    },
    ExpandDeinitIf {
        block: usize,
        stmt: usize,
        deinit_entity: Option<Entity>,
        place: Place,
        place_ty: MirTy,
        flag: LocalId,
    },
    ExpandEnumDrop {
        block: usize,
        stmt: usize,
        place: Place,
        /// (variant_name, field_path, deinit_func_or_none, field_ty)
        /// None = emit Deinit node for further expansion (nested enum)
        variant_drops: Vec<(String, Vec<String>, Option<Entity>, MirTy)>,
        flag: Option<LocalId>,
    },
    ExpandStructFieldDrop {
        block: usize,
        stmt: usize,
        place: Place,
        field_drops: Vec<(Vec<String>, Entity, MirTy)>,
        flag: Option<LocalId>,
    },
}

fn collect_expansions(
    body: &MirBody,
    deinit_funcs: &HashMap<Entity, Entity>,
    module: &MirModule,
) -> Vec<Expansion> {
    let mut expansions = Vec::new();
    let failure_blocks: HashSet<usize> = body
        .failure_return_blocks
        .iter()
        .map(|b| b.index())
        .collect();

    for (block_idx, block) in body.blocks.iter().enumerate() {
        for (stmt_idx, stmt) in block.stmts.iter().enumerate() {
            match &stmt.kind {
                StatementKind::Deinit { place } => {
                    let place_ty = resolve_place_type(place, body, module);
                    let deinit_entity = place_ty
                        .as_ref()
                        .and_then(|ty| struct_entity(ty))
                        .and_then(|e| deinit_funcs.get(&e).copied());
                    if deinit_entity.is_none() {
                        if let Some(drops) = enum_variant_drops(place_ty.as_ref(), deinit_funcs, module) {
                            if !drops.is_empty() {
                                expansions.push(Expansion::ExpandEnumDrop {
                                    block: block_idx,
                                    stmt: stmt_idx,
                                    place: place.clone(),
                                    variant_drops: drops,
                                    flag: None,
                                });
                                continue;
                            }
                        }
                        let mut field_drops = Vec::new();
                        struct_field_drops(place_ty.as_ref(), deinit_funcs, module, &mut field_drops);
                        if !field_drops.is_empty() {
                            expansions.push(Expansion::ExpandStructFieldDrop {
                                block: block_idx,
                                stmt: stmt_idx,
                                place: place.clone(),
                                field_drops,
                                flag: None,
                            });
                            continue;
                        }
                    }
                    // For types with a deinit, also compute field cascade for TypeParam
                    // fields resolved via concrete type_args at the call site
                    let mut extra_field_drops = Vec::new();
                    if deinit_entity.is_some() {
                        if let Some(ty) = &place_ty {
                            compute_typeparam_field_drops(ty, deinit_funcs, module, &mut extra_field_drops);
                        }
                    }
                    expansions.push(Expansion::ReplaceDeinit {
                        block: block_idx,
                        stmt: stmt_idx,
                        deinit_entity,
                        place: place.clone(),
                        place_ty: place_ty.unwrap_or(MirTy::Error),
                        extra_field_drops,
                    });
                }
                StatementKind::DeinitIf { place, flag } => {
                    let is_self_field = matches!(
                        place,
                        Place::Field { parent, .. }
                            if parent.root_local() == Some(LocalId::new(0))
                    );
                    // Self-field DeinitIf only expanded in failure-return blocks
                    if is_self_field && !failure_blocks.contains(&block_idx) {
                        continue;
                    }
                    let place_ty = resolve_place_type(place, body, module);
                    let deinit_entity = place_ty
                        .as_ref()
                        .and_then(|ty| struct_entity(ty))
                        .and_then(|e| deinit_funcs.get(&e).copied());
                    if !is_self_field && deinit_entity.is_none() {
                        if let Some(drops) = enum_variant_drops(place_ty.as_ref(), deinit_funcs, module) {
                            if !drops.is_empty() {
                                expansions.push(Expansion::ExpandEnumDrop {
                                    block: block_idx,
                                    stmt: stmt_idx,
                                    place: place.clone(),
                                    variant_drops: drops,
                                    flag: Some(*flag),
                                });
                                continue;
                            }
                        }
                        let mut field_drops = Vec::new();
                        struct_field_drops(place_ty.as_ref(), deinit_funcs, module, &mut field_drops);
                        if !field_drops.is_empty() {
                            expansions.push(Expansion::ExpandStructFieldDrop {
                                block: block_idx,
                                stmt: stmt_idx,
                                place: place.clone(),
                                field_drops,
                                flag: Some(*flag),
                            });
                            continue;
                        }
                    }
                    if !is_self_field || deinit_entity.is_some() {
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
                _ => {}
            }
        }
    }
    expansions
}

fn apply_expansions(body: &mut MirBody, expansions: Vec<Expansion>) {
    for expansion in expansions.into_iter().rev() {
        match expansion {
            Expansion::ReplaceDeinit { block, stmt, deinit_entity, place, place_ty, extra_field_drops } => {
                if let Some(deinit_func) = deinit_entity {
                    let callee = deinit_callee(deinit_func, place_ty);
                    body.blocks[block].stmts[stmt] = Statement::new(StatementKind::Call {
                        dest: None,
                        callee,
                        args: vec![Value::RefMut(place.clone())],
                    });
                    // Insert extra field cascade calls after the deinit call
                    for (i, (field_path, df, ft)) in extra_field_drops.into_iter().enumerate() {
                        let mut field_place = place.clone();
                        for segment in &field_path {
                            field_place = field_place.field(segment);
                        }
                        let callee = deinit_callee(df, ft);
                        body.blocks[block].stmts.insert(stmt + 1 + i, Statement::new(StatementKind::Call {
                            dest: None,
                            callee,
                            args: vec![Value::RefMut(field_place)],
                        }));
                    }
                } else {
                    body.blocks[block].stmts.remove(stmt);
                }
            }
            Expansion::ExpandDeinitIf { block, stmt, deinit_entity, place, place_ty, flag } => {
                let Some(deinit_func) = deinit_entity else {
                    body.blocks[block].stmts.remove(stmt);
                    continue;
                };
                let remaining_stmts = body.blocks[block].stmts.split_off(stmt + 1);
                body.blocks[block].stmts.remove(stmt);
                let original_terminator =
                    std::mem::replace(&mut body.blocks[block].terminator, Terminator::unreachable());

                let cont_block_id = BlockId::new(body.blocks.len());
                let mut cont_block = BasicBlock::new();
                cont_block.stmts = remaining_stmts;
                cont_block.terminator = original_terminator;
                body.blocks.push(cont_block);

                let deinit_block_id = BlockId::new(body.blocks.len());
                let mut deinit_block = BasicBlock::new();
                let callee = deinit_callee(deinit_func, place_ty);
                deinit_block.stmts.push(Statement::new(StatementKind::Call {
                    dest: None,
                    callee,
                    args: vec![Value::RefMut(place)],
                }));
                deinit_block.terminator = Terminator::jump(cont_block_id);
                body.blocks.push(deinit_block);

                // flag=true → skip (dead/moved); flag=false → deinit (live)
                body.blocks[block].terminator = Terminator {
                    kind: TerminatorKind::Branch {
                        condition: Value::Copy(Place::local(flag)),
                        then_block: cont_block_id,    // flag=true → skip
                        else_block: deinit_block_id,  // flag=false → deinit
                    },
                    span: None,
                };
            }
            Expansion::ExpandEnumDrop { block, stmt, place, variant_drops, flag } => {
                let remaining_stmts = body.blocks[block].stmts.split_off(stmt + 1);
                body.blocks[block].stmts.remove(stmt);
                let original_terminator =
                    std::mem::replace(&mut body.blocks[block].terminator, Terminator::unreachable());

                let cont_block_id = BlockId::new(body.blocks.len());
                let mut cont_block = BasicBlock::new();
                cont_block.stmts = remaining_stmts;
                cont_block.terminator = original_terminator;
                body.blocks.push(cont_block);

                let mut switch_block = BasicBlock::new();
                let mut cases: Vec<(SwitchCase, BlockId)> = Vec::new();

                let mut variants_seen = HashSet::new();
                for (variant_name, _, _, _) in &variant_drops {
                    if !variants_seen.insert(variant_name.clone()) {
                        continue;
                    }
                    let deinit_block_id = BlockId::new(body.blocks.len());
                    let mut deinit_block = BasicBlock::new();
                    for (vn, field_path, df, ft) in &variant_drops {
                        if vn == variant_name {
                            let mut field_place = place.clone().downcast(vn);
                            for segment in field_path {
                                field_place = field_place.field(segment);
                            }
                            if let Some(deinit_func) = df {
                                let callee = deinit_callee(*deinit_func, ft.clone());
                                deinit_block.stmts.push(Statement::new(StatementKind::Call {
                                    dest: None,
                                    callee,
                                    args: vec![Value::RefMut(field_place)],
                                }));
                            } else {
                                // Nested enum: emit Deinit for iterative expansion
                                deinit_block.stmts.push(Statement::new(StatementKind::Deinit {
                                    place: field_place,
                                }));
                            }
                        }
                    }
                    deinit_block.terminator = Terminator::jump(cont_block_id);
                    body.blocks.push(deinit_block);
                    cases.push((SwitchCase::Variant(variant_name.clone()), deinit_block_id));
                }
                cases.push((SwitchCase::Wildcard, cont_block_id));

                let switch_block_id = BlockId::new(body.blocks.len());
                switch_block.terminator = Terminator {
                    kind: TerminatorKind::Switch {
                        discriminant: place.clone(),
                        cases,
                    },
                    span: None,
                };
                body.blocks.push(switch_block);

                if let Some(flag_id) = flag {
                    body.blocks[block].terminator = Terminator {
                        kind: TerminatorKind::Branch {
                            condition: Value::Copy(Place::local(flag_id)),
                            then_block: cont_block_id,    // flag=true → skip
                            else_block: switch_block_id,  // flag=false → drop
                        },
                        span: None,
                    };
                } else {
                    body.blocks[block].terminator = Terminator::jump(switch_block_id);
                }
            }
            Expansion::ExpandStructFieldDrop { block, stmt, place, field_drops, flag } => {
                if let Some(flag_id) = flag {
                    let remaining_stmts = body.blocks[block].stmts.split_off(stmt + 1);
                    body.blocks[block].stmts.remove(stmt);
                    let original_terminator =
                        std::mem::replace(&mut body.blocks[block].terminator, Terminator::unreachable());

                    let cont_block_id = BlockId::new(body.blocks.len());
                    let mut cont_block = BasicBlock::new();
                    cont_block.stmts = remaining_stmts;
                    cont_block.terminator = original_terminator;
                    body.blocks.push(cont_block);

                    let deinit_block_id = BlockId::new(body.blocks.len());
                    let mut deinit_block = BasicBlock::new();
                    for (field_path, deinit_entity, field_ty) in field_drops.iter().rev() {
                        let mut field_place = place.clone();
                        for segment in field_path {
                            field_place = field_place.field(segment);
                        }
                        let callee = deinit_callee(*deinit_entity, field_ty.clone());
                        deinit_block.stmts.push(Statement::new(StatementKind::Call {
                            dest: None,
                            callee,
                            args: vec![Value::RefMut(field_place)],
                        }));
                    }
                    deinit_block.terminator = Terminator::jump(cont_block_id);
                    body.blocks.push(deinit_block);

                    body.blocks[block].terminator = Terminator {
                        kind: TerminatorKind::Branch {
                            condition: Value::Copy(Place::local(flag_id)),
                            then_block: cont_block_id,    // flag=true → skip
                            else_block: deinit_block_id,  // flag=false → deinit
                        },
                        span: None,
                    };
                } else {
                    body.blocks[block].stmts.remove(stmt);
                    for (i, (field_path, deinit_entity, field_ty)) in field_drops.iter().rev().enumerate() {
                        let mut field_place = place.clone();
                        for segment in field_path {
                            field_place = field_place.field(segment);
                        }
                        let callee = deinit_callee(*deinit_entity, field_ty.clone());
                        body.blocks[block].stmts.insert(stmt + i, Statement::new(StatementKind::Call {
                            dest: None,
                            callee,
                            args: vec![Value::RefMut(field_place)],
                        }));
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Phase E: field cascading (ported from expand_deinit.rs)
// ---------------------------------------------------------------------------

fn inject_field_deinits(module: &mut MirModule, deinit_funcs: &HashMap<Entity, Entity>) {
    let mut injections: Vec<(usize, Vec<(Vec<String>, Entity, MirTy)>)> = Vec::new();

    for (func_idx, func) in module.functions.iter().enumerate() {
        let FunctionKind::Deinit { parent } = &func.kind else { continue };
        if func.body.is_none() { continue; }
        let Some(struct_def) = module.structs.iter().find(|s| s.entity == *parent) else { continue };

        let mut field_deinits: Vec<(Vec<String>, Entity, MirTy)> = Vec::new();
        for field in &struct_def.fields {
            collect_struct_field_drops(
                &[field.name.clone()],
                &field.ty,
                deinit_funcs,
                module,
                &mut field_deinits,
            );
        }
        if !field_deinits.is_empty() {
            injections.push((func_idx, field_deinits));
        }
    }

    for (func_idx, field_deinits) in injections {
        let body = module.functions[func_idx].body.as_mut().unwrap();
        for block_idx in 0..body.blocks.len() {
            if !matches!(body.blocks[block_idx].terminator.kind, TerminatorKind::Return(_)) {
                continue;
            }
            for (field_path, deinit_entity, field_ty) in field_deinits.iter().rev() {
                let self_local = LocalId::new(0);
                let mut place = Place::local(self_local);
                for segment in field_path {
                    place = place.field(segment);
                }
                let callee = deinit_callee(*deinit_entity, field_ty.clone());
                body.blocks[block_idx].stmts.push(Statement::new(StatementKind::Call {
                    dest: None,
                    callee,
                    args: vec![Value::RefMut(place)],
                }));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Shared helpers (ported from expand_deinit.rs)
// ---------------------------------------------------------------------------

pub(crate) fn collect_types_with_deinit(module: &MirModule) -> HashSet<Entity> {
    module.functions.iter()
        .filter_map(|f| match &f.kind {
            FunctionKind::Deinit { parent } => Some(*parent),
            _ => None,
        })
        .collect()
}

pub(crate) fn collect_structs_with_droppable_fields(
    module: &MirModule,
    types_with_deinit: &HashSet<Entity>,
) -> HashSet<Entity> {
    module.structs.iter()
        .filter(|s| {
            !types_with_deinit.contains(&s.entity)
                && s.fields.iter().any(|f| match &f.ty {
                    MirTy::Named { entity, .. } => types_with_deinit.contains(entity),
                    _ => false,
                })
        })
        .map(|s| s.entity)
        .collect()
}

pub(crate) fn compute_types_needing_drop(
    module: &MirModule,
    types_with_deinit: &HashSet<Entity>,
    structs_with_droppable_fields: &HashSet<Entity>,
) -> HashSet<Entity> {
    let mut set = types_with_deinit.clone();
    set.extend(structs_with_droppable_fields);
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
}

fn resolve_place_type(place: &Place, body: &MirBody, module: &MirModule) -> Option<MirTy> {
    match place {
        Place::Local(id) => Some(body.locals[id.index()].ty.clone()),
        Place::Downcast { parent, variant } => {
            let parent_ty = resolve_place_type(parent, body, module)?;
            let entity = struct_entity(&parent_ty)?;
            let enum_def = module.enums.iter().find(|e| e.entity == entity)?;
            let case = enum_def.cases.iter().find(|c| c.name == *variant)?;
            let payload = &module.structs[case.payload_struct.index()];
            // Substitute the enum's type params with the concrete type args
            let type_args = match &parent_ty {
                MirTy::Named { type_args, .. } => type_args.as_slice(),
                _ => &[],
            };
            let subst: Vec<(Entity, &MirTy)> = enum_def
                .type_params
                .iter()
                .zip(type_args.iter())
                .map(|(p, t)| (p.entity, t))
                .collect();
            // Return the payload struct type with substituted type params
            Some(MirTy::Named {
                entity: payload.entity,
                type_args: if subst.is_empty() {
                    Vec::new()
                } else {
                    payload.type_params.iter().map(|p| {
                        substitute_type_params(&MirTy::TypeParam(p.entity), &subst)
                    }).collect()
                },
            })
        }
        Place::Field { parent, name } => {
            let parent_ty = resolve_place_type(parent, body, module)?;
            let inner_ty = unwrap_ref(&parent_ty);
            let entity = struct_entity(inner_ty)?;
            let struct_def = module.structs.iter().find(|s| s.entity == entity)?;
            let field_id = struct_def.field_by_name(name)?;
            let raw_ty = &struct_def.fields[field_id.index()].ty;
            // Substitute type params if the parent type has type_args
            let type_args = match inner_ty {
                MirTy::Named { type_args, .. } => type_args.as_slice(),
                _ => &[],
            };
            if type_args.is_empty() {
                Some(raw_ty.clone())
            } else {
                let subst: Vec<(Entity, &MirTy)> = struct_def
                    .type_params
                    .iter()
                    .zip(type_args.iter())
                    .map(|(p, t)| (p.entity, t))
                    .collect();
                Some(substitute_type_params(raw_ty, &subst))
            }
        }
        Place::Deref(inner) => {
            let inner_ty = resolve_place_type(inner, body, module)?;
            match inner_ty {
                MirTy::Ref(pointee) | MirTy::RefMut(pointee) | MirTy::Pointer(pointee) => Some(*pointee),
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

fn struct_field_drops(
    ty: Option<&MirTy>,
    deinit_funcs: &HashMap<Entity, Entity>,
    module: &MirModule,
    drops: &mut Vec<(Vec<String>, Entity, MirTy)>,
) {
    let Some(ty) = ty else { return };
    let Some(entity) = struct_entity(ty) else { return };
    if deinit_funcs.contains_key(&entity) { return; }
    let Some(struct_def) = module.structs.iter().find(|s| s.entity == entity) else { return };
    let type_args = match ty {
        MirTy::Named { type_args, .. } => type_args.as_slice(),
        _ => &[],
    };
    let subst: Vec<(Entity, &MirTy)> = struct_def
        .type_params.iter()
        .zip(type_args.iter())
        .map(|(p, t)| (p.entity, t))
        .collect();
    for field in &struct_def.fields {
        let resolved_ty = if subst.is_empty() {
            field.ty.clone()
        } else {
            substitute_type_params(&field.ty, &subst)
        };
        collect_struct_field_drops(
            &[field.name.clone()], &resolved_ty, deinit_funcs, module, drops,
        );
    }
}

fn enum_variant_drops(
    ty: Option<&MirTy>,
    deinit_funcs: &HashMap<Entity, Entity>,
    module: &MirModule,
) -> Option<Vec<(String, Vec<String>, Option<Entity>, MirTy)>> {
    let (entity, type_args) = match ty? {
        MirTy::Named { entity, type_args } => (*entity, type_args),
        _ => return None,
    };
    let enum_def = module.enums.iter().find(|e| e.entity == entity)?;
    let subst: Vec<(Entity, &MirTy)> = enum_def
        .type_params.iter()
        .zip(type_args.iter())
        .map(|(p, t)| (p.entity, t))
        .collect();

    let mut drops = Vec::new();
    for case in &enum_def.cases {
        let payload = &module.structs[case.payload_struct.index()];
        for field in &payload.fields {
            let resolved_ty = substitute_type_params(&field.ty, &subst);
            collect_field_drops_recursive(
                &case.name, &[field.name.clone()], &resolved_ty,
                deinit_funcs, module, &mut drops,
            );
        }
    }
    Some(drops)
}

fn collect_field_drops_recursive(
    variant_name: &str,
    path: &[String],
    field_ty: &MirTy,
    deinit_funcs: &HashMap<Entity, Entity>,
    module: &MirModule,
    drops: &mut Vec<(String, Vec<String>, Option<Entity>, MirTy)>,
) {
    let Some(entity) = struct_entity(field_ty) else { return };
    if let Some(&deinit_func) = deinit_funcs.get(&entity) {
        drops.push((variant_name.to_string(), path.to_vec(), Some(deinit_func), field_ty.clone()));
        return;
    }
    // Check if this is an enum with droppable payloads — emit Deinit for further expansion
    if module.enums.iter().any(|e| e.entity == entity) {
        drops.push((variant_name.to_string(), path.to_vec(), None, field_ty.clone()));
        return;
    }
    let Some(struct_def) = module.structs.iter().find(|s| s.entity == entity) else { return };
    let type_args = match field_ty {
        MirTy::Named { type_args, .. } => type_args.as_slice(),
        _ => &[],
    };
    let subst: Vec<(Entity, &MirTy)> = struct_def
        .type_params.iter()
        .zip(type_args.iter())
        .map(|(p, t)| (p.entity, t))
        .collect();
    for sub_field in &struct_def.fields {
        let resolved_ty = if subst.is_empty() {
            sub_field.ty.clone()
        } else {
            substitute_type_params(&sub_field.ty, &subst)
        };
        if let Some(sub_entity) = struct_entity(&resolved_ty) {
            if let Some(&sub_deinit) = deinit_funcs.get(&sub_entity) {
                let mut sub_path = path.to_vec();
                sub_path.push(sub_field.name.clone());
                drops.push((variant_name.to_string(), sub_path, Some(sub_deinit), resolved_ty));
            }
        }
    }
}

fn collect_struct_field_drops(
    path: &[String],
    ty: &MirTy,
    deinit_funcs: &HashMap<Entity, Entity>,
    module: &MirModule,
    drops: &mut Vec<(Vec<String>, Entity, MirTy)>,
) {
    let Some(entity) = struct_entity(ty) else { return };
    if let Some(&deinit_func) = deinit_funcs.get(&entity) {
        drops.push((path.to_vec(), deinit_func, ty.clone()));
    } else {
        // Substitute the struct's type params with concrete args from `ty`
        // so sub-field types are fully resolved.
        let Some(struct_def) = module.structs.iter().find(|s| s.entity == entity) else { return };
        let type_args = match ty {
            MirTy::Named { type_args, .. } => type_args.as_slice(),
            _ => &[],
        };
        let subst: Vec<(Entity, &MirTy)> = struct_def
            .type_params.iter()
            .zip(type_args.iter())
            .map(|(p, t)| (p.entity, t))
            .collect();
        for sub_field in &struct_def.fields {
            let resolved_ty = if subst.is_empty() {
                sub_field.ty.clone()
            } else {
                substitute_type_params(&sub_field.ty, &subst)
            };
            if let Some(sub_entity) = struct_entity(&resolved_ty) {
                if let Some(&sub_deinit) = deinit_funcs.get(&sub_entity) {
                    let mut sub_path = path.to_vec();
                    sub_path.push(sub_field.name.clone());
                    drops.push((sub_path, sub_deinit, resolved_ty));
                } else {
                    // Recurse one more level: sub-field has no deinit but
                    // might contain sub-sub-fields that do. Substitute its
                    // type params so the types are concrete.
                    let Some(sub_def) = module.structs.iter().find(|s| s.entity == sub_entity) else { continue };
                    let sub_type_args = match &resolved_ty {
                        MirTy::Named { type_args, .. } => type_args.as_slice(),
                        _ => &[],
                    };
                    let sub_subst: Vec<(Entity, &MirTy)> = sub_def
                        .type_params.iter()
                        .zip(sub_type_args.iter())
                        .map(|(p, t)| (p.entity, t))
                        .collect();
                    for ssf in &sub_def.fields {
                        let ssf_ty = if sub_subst.is_empty() {
                            ssf.ty.clone()
                        } else {
                            substitute_type_params(&ssf.ty, &sub_subst)
                        };
                        if let Some(ssf_entity) = struct_entity(&ssf_ty) {
                            if let Some(&ssf_deinit) = deinit_funcs.get(&ssf_entity) {
                                let mut ssf_path = path.to_vec();
                                ssf_path.push(sub_field.name.clone());
                                ssf_path.push(ssf.name.clone());
                                drops.push((ssf_path, ssf_deinit, ssf_ty));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// For a concrete type like Box[Resource], find fields whose struct-definition type
/// is TypeParam (unresolved in the generic body) but resolves to a droppable type
/// after substituting the call-site's type_args. These need cascade at the call site.
fn compute_typeparam_field_drops(
    ty: &MirTy,
    deinit_funcs: &HashMap<Entity, Entity>,
    module: &MirModule,
    drops: &mut Vec<(Vec<String>, Entity, MirTy)>,
) {
    let MirTy::Named { entity, type_args } = ty else { return };
    if type_args.is_empty() { return; }
    let Some(struct_def) = module.structs.iter().find(|s| s.entity == *entity) else { return };
    if struct_def.type_params.is_empty() { return; }

    let subst: Vec<(Entity, &MirTy)> = struct_def
        .type_params
        .iter()
        .zip(type_args.iter())
        .map(|(p, t)| (p.entity, t))
        .collect();

    for field in &struct_def.fields {
        // Only process fields whose definition type is TypeParam or contains TypeParam
        let is_typeparam = matches!(&field.ty, MirTy::TypeParam(_));
        if !is_typeparam { continue; }

        let resolved_ty = substitute_type_params(&field.ty, &subst);
        // Recursively collect drops for the resolved type
        collect_struct_field_drops(
            &[field.name.clone()],
            &resolved_ty,
            deinit_funcs,
            module,
            drops,
        );
    }
}

fn substitute_type_params(ty: &MirTy, subst: &[(Entity, &MirTy)]) -> MirTy {
    match ty {
        MirTy::TypeParam(entity) => {
            for &(param_entity, concrete) in subst {
                if *entity == param_entity {
                    return concrete.clone();
                }
            }
            ty.clone()
        }
        MirTy::Named { entity, type_args } => MirTy::Named {
            entity: *entity,
            type_args: type_args.iter().map(|t| substitute_type_params(t, subst)).collect(),
        },
        MirTy::Pointer(inner) => MirTy::Pointer(Box::new(substitute_type_params(inner, subst))),
        MirTy::Ref(inner) => MirTy::Ref(Box::new(substitute_type_params(inner, subst))),
        MirTy::RefMut(inner) => MirTy::RefMut(Box::new(substitute_type_params(inner, subst))),
        MirTy::Tuple(elems) => MirTy::Tuple(
            elems.iter().map(|t| substitute_type_params(t, subst)).collect(),
        ),
        MirTy::FuncThin { params, ret } => MirTy::FuncThin {
            params: params.iter().map(|t| substitute_type_params(t, subst)).collect(),
            ret: Box::new(substitute_type_params(ret, subst)),
        },
        MirTy::FuncThick { params, ret } => MirTy::FuncThick {
            params: params.iter().map(|t| substitute_type_params(t, subst)).collect(),
            ret: Box::new(substitute_type_params(ret, subst)),
        },
        MirTy::AssociatedProjection { base, protocol, name } => MirTy::AssociatedProjection {
            base: Box::new(substitute_type_params(base, subst)),
            protocol: *protocol,
            name: name.clone(),
        },
        _ => ty.clone(),
    }
}
