//! Clone elaboration pass — rewrites `Rvalue::Copy` of Clone types into
//! explicit `Callee::Witness` clone calls.
//!
//! Scope (all ownership-creating positions):
//! - Assignment copies (`%dest = copy %src`)
//! - Closure captures (`apply_partial(captures: [copy %x])`)
//! - Composite rvalue fields (`construct { f: copy %x }`, `tuple(copy %x)`,
//!   `enum Variant(copy %x)`, `array[T] [copy %x]`)
//! - Return values (`return copy %x`)
//!
//! Call args are NOT rewritten — the lowering applies correct param modes
//! (Ref/Move/RefMut) via `apply_callee_param_modes` / `apply_witness_param_modes`,
//! so Clone-type call args never appear as Copy.
//!
//! Must run **before** drop elaboration: drop elab's `copied_from` tracking
//! assumes Copy destinations alias the source. Clone destinations are
//! independent owned values (from a Call result), and drop elab needs to see
//! the Call + Move pattern to insert correct drops.
//!
//! ## Decision rule
//!
//! Clone is needed when a Copy creates a new owner alongside the source.
//! Two analyses determine this:
//!
//! - **Structural ownership** (for assignments, composites, returns): a
//!   non-temp, non-param, bare local of a Clone type is assumed to be an
//!   owner that drop elaboration will drop. Temps (`_t*`, `$`, `_clone*`)
//!   are excluded — drop elaboration's `find_owned_locals` excludes them
//!   via `copied_from`/`call_result_targets`, so they won't be dropped.
//!
//! - **Liveness** (for closure captures): backward read-liveness replaces
//!   the old `assigned_locals` heuristic that guarded against stale
//!   captures leaked by `find_captures`. A dead capture local has no
//!   future reads and won't be dropped, so cloning is unnecessary.

use std::collections::HashSet;

use crate::item::{CopyBehavior, FunctionKind};
use crate::passes::liveness::Liveness;
use crate::statement::{Callee, Rvalue, Statement, StatementKind};
use crate::terminator::TerminatorKind;
use crate::value::Value;
use crate::{LocalDef, LocalId, MirBody, MirModule, MirTy, Place, WitnessMethodKey};
use kestrel_hecs::Entity;

struct CloneInfo {
    cloneable: Entity,
    clone_entities: HashSet<Entity>,
}

pub fn run_clone_elaboration(module: &mut MirModule) {
    let Some(cloneable) = find_cloneable_protocol(module) else {
        return;
    };
    let clone_entities = collect_clone_entities(module);
    let info = CloneInfo { cloneable, clone_entities };

    for func in &mut module.functions {
        if should_skip_function(func) {
            continue;
        }
        let params: HashSet<LocalId> = func.params.iter().map(|p| p.local).collect();
        let body = match func.body.as_mut() {
            Some(b) => b,
            None => continue,
        };
        elaborate_function(body, &params, &info);
    }
}

fn find_cloneable_protocol(module: &MirModule) -> Option<Entity> {
    module.protocols.iter()
        .find(|p| p.name.ends_with("Cloneable"))
        .map(|p| p.entity)
}

fn collect_clone_entities(module: &MirModule) -> HashSet<Entity> {
    let mut set = HashSet::new();
    for s in &module.structs {
        if matches!(s.copy_behavior, CopyBehavior::Clone(_)) {
            set.insert(s.entity);
        }
    }
    for e in &module.enums {
        if matches!(e.copy_behavior, CopyBehavior::Clone(_)) {
            set.insert(e.entity);
        }
    }
    set
}

fn should_skip_function(func: &crate::FunctionDef) -> bool {
    if func.name.ends_with(".clone") {
        return true;
    }
    if matches!(func.kind, FunctionKind::Deinit { .. }) {
        return true;
    }
    func.body.is_none()
}

/// Structural ownership check: clone if this is a non-temp, non-param,
/// bare local of a Clone type. Aligned with drop elaboration's
/// `find_owned_locals` exclusion rules.
fn needs_clone(
    body: &MirBody,
    params: &HashSet<LocalId>,
    info: &CloneInfo,
    place: &Place,
) -> Option<MirTy> {
    if !place.is_local() {
        return None;
    }
    let local_id = place.root_local()?;
    if params.contains(&local_id) {
        return None;
    }
    let local = &body.locals[local_id.index()];
    if local.name.starts_with("_t")
        || local.name.starts_with("$")
        || local.name.starts_with("_clone")
    {
        return None;
    }
    let MirTy::Named { entity, .. } = &local.ty else {
        return None;
    };
    if info.clone_entities.contains(entity) {
        Some(local.ty.clone())
    } else {
        None
    }
}

/// Liveness-based clone check for closure captures. A capture needs
/// cloning only if the source local is live (has future reads). Dead
/// locals won't be read or dropped, so the bitwise copy is safe.
fn needs_clone_capture(
    body: &MirBody,
    info: &CloneInfo,
    live_after: &crate::passes::liveness::BitVec,
    place: &Place,
) -> Option<MirTy> {
    let local_id = place.root_local()?;
    if !Liveness::is_live_in(live_after, local_id) {
        return None;
    }
    let local = &body.locals[local_id.index()];
    let MirTy::Named { entity, .. } = &local.ty else {
        return None;
    };
    if info.clone_entities.contains(entity) {
        Some(local.ty.clone())
    } else {
        None
    }
}

fn elaborate_function(body: &mut MirBody, params: &HashSet<LocalId>, info: &CloneInfo) {
    let liveness = Liveness::compute(body);
    let num_blocks = body.blocks.len();
    for bi in 0..num_blocks {
        elaborate_block(body, params, info, &liveness, bi);
    }
}

fn elaborate_block(
    body: &mut MirBody,
    params: &HashSet<LocalId>,
    info: &CloneInfo,
    liveness: &Liveness,
    bi: usize,
) {
    // Precompute per-statement liveness for capture decisions.
    let live_after = liveness.block_liveness_after(body, bi);

    let mut si = body.blocks[bi].stmts.len();
    // `offset` tracks insertions so we can map back to original indices
    // for the precomputed `live_after` array.
    let mut offset: usize = 0;
    while si > 0 {
        si -= 1;

        // Rewrite Rvalue::Copy in assignments (structural check)
        let should_rewrite_assign = match &body.blocks[bi].stmts[si].kind {
            StatementKind::Assign { rvalue: Rvalue::Copy(place), .. } => {
                needs_clone(body, params, info, place).is_some()
            },
            _ => false,
        };
        if should_rewrite_assign {
            rewrite_assign_copy(body, params, info, bi, si);
            // Inserts 1 stmt AFTER si — doesn't shift earlier indices.
            offset += 1;
            continue;
        }

        // Rewrite Value::Copy in ApplyPartial captures (liveness check)
        let orig_si = si.saturating_sub(offset);
        let la = live_after.get(orig_si).unwrap_or_else(|| &live_after[0]);
        let capture_rewrites = collect_capture_rewrites(body, info, la, bi, si);
        if !capture_rewrites.is_empty() {
            let n = capture_rewrites.len();
            apply_capture_rewrites(body, info, bi, si, &capture_rewrites);
            offset += n;
            si += n;
            continue;
        }

        // Rewrite Value::Copy in composite rvalues (structural check)
        let composite_rewrites = collect_composite_rewrites(body, params, info, bi, si);
        if !composite_rewrites.is_empty() {
            let n = composite_rewrites.len();
            apply_composite_rewrites(body, info, bi, si, &composite_rewrites);
            offset += n;
            si += n;
        }
    }

    elaborate_terminator(body, params, info, bi);
}

// ---- Rewrite helpers ----

fn rewrite_assign_copy(
    body: &mut MirBody,
    params: &HashSet<LocalId>,
    info: &CloneInfo,
    bi: usize,
    si: usize,
) {
    let (dest, place) = match &body.blocks[bi].stmts[si].kind {
        StatementKind::Assign { dest, rvalue: Rvalue::Copy(place) } => {
            (dest.clone(), place.clone())
        },
        _ => return,
    };
    let ty = match needs_clone(body, params, info, &place) {
        Some(ty) => ty,
        None => return,
    };
    let tmp = body.add_local(LocalDef::new("_clone", ty.clone()));
    let clone_call = make_clone_call(info.cloneable, &ty, Place::local(tmp), &place);
    let move_stmt = Statement::new(StatementKind::Assign {
        dest,
        rvalue: Rvalue::Move(Place::local(tmp)),
    });
    body.blocks[bi].stmts[si] = clone_call;
    body.blocks[bi].stmts.insert(si + 1, move_stmt);
}

fn collect_capture_rewrites(
    body: &MirBody,
    info: &CloneInfo,
    live_after: &crate::passes::liveness::BitVec,
    bi: usize,
    si: usize,
) -> Vec<(usize, Place, MirTy)> {
    let mut rewrites = Vec::new();
    let stmt = &body.blocks[bi].stmts[si];
    if let StatementKind::Assign { rvalue: Rvalue::ApplyPartial { captures, .. }, .. } = &stmt.kind {
        for (i, val) in captures.iter().enumerate() {
            if let Value::Copy(place) = val {
                if let Some(ty) = needs_clone_capture(body, info, live_after, place) {
                    rewrites.push((i, place.clone(), ty));
                }
            }
        }
    }
    rewrites
}

fn apply_capture_rewrites(
    body: &mut MirBody,
    info: &CloneInfo,
    bi: usize,
    si: usize,
    rewrites: &[(usize, Place, MirTy)],
) {
    let mut tmp_locals = Vec::new();
    for (idx, (_cap_idx, place, ty)) in rewrites.iter().enumerate() {
        let tmp = body.add_local(LocalDef::new("_clone", ty.clone()));
        let clone_call = make_clone_call(info.cloneable, ty, Place::local(tmp), place);
        body.blocks[bi].stmts.insert(si + idx, clone_call);
        tmp_locals.push(tmp);
    }

    let stmt_idx = si + rewrites.len();
    if let StatementKind::Assign { rvalue: Rvalue::ApplyPartial { captures, .. }, .. }
        = &mut body.blocks[bi].stmts[stmt_idx].kind
    {
        for (i, (cap_idx, _place, _ty)) in rewrites.iter().enumerate() {
            captures[*cap_idx] = Value::Move(Place::local(tmp_locals[i]));
        }
    }
}

fn collect_composite_rewrites(
    body: &MirBody,
    params: &HashSet<LocalId>,
    info: &CloneInfo,
    bi: usize,
    si: usize,
) -> Vec<(usize, Place, MirTy)> {
    let stmt = &body.blocks[bi].stmts[si];
    let values: &[Value] = match &stmt.kind {
        StatementKind::Assign { rvalue: Rvalue::Construct { fields, .. }, .. } => {
            return collect_construct_rewrites(body, params, info, fields);
        }
        StatementKind::Assign { rvalue: Rvalue::Tuple(values), .. } => values,
        StatementKind::Assign { rvalue: Rvalue::EnumVariant { payload, .. }, .. } => payload,
        StatementKind::Assign { rvalue: Rvalue::ArrayLiteral { values, .. }, .. } => values,
        _ => return Vec::new(),
    };
    let mut rewrites = Vec::new();
    for (i, val) in values.iter().enumerate() {
        if let Value::Copy(place) = val {
            if let Some(ty) = needs_clone(body, params, info, place) {
                rewrites.push((i, place.clone(), ty));
            }
        }
    }
    rewrites
}

fn collect_construct_rewrites(
    body: &MirBody,
    params: &HashSet<LocalId>,
    info: &CloneInfo,
    fields: &[(String, Value)],
) -> Vec<(usize, Place, MirTy)> {
    let mut rewrites = Vec::new();
    for (i, (_name, val)) in fields.iter().enumerate() {
        if let Value::Copy(place) = val {
            if let Some(ty) = needs_clone(body, params, info, place) {
                rewrites.push((i, place.clone(), ty));
            }
        }
    }
    rewrites
}

fn apply_composite_rewrites(
    body: &mut MirBody,
    info: &CloneInfo,
    bi: usize,
    si: usize,
    rewrites: &[(usize, Place, MirTy)],
) {
    let mut tmp_locals = Vec::new();
    for (idx, (_val_idx, place, ty)) in rewrites.iter().enumerate() {
        let tmp = body.add_local(LocalDef::new("_clone", ty.clone()));
        let clone_call = make_clone_call(info.cloneable, ty, Place::local(tmp), place);
        body.blocks[bi].stmts.insert(si + idx, clone_call);
        tmp_locals.push(tmp);
    }

    let stmt_idx = si + rewrites.len();
    match &mut body.blocks[bi].stmts[stmt_idx].kind {
        StatementKind::Assign { rvalue: Rvalue::Construct { fields, .. }, .. } => {
            for (i, (val_idx, _, _)) in rewrites.iter().enumerate() {
                fields[*val_idx].1 = Value::Move(Place::local(tmp_locals[i]));
            }
        }
        StatementKind::Assign { rvalue: Rvalue::Tuple(values), .. }
        | StatementKind::Assign { rvalue: Rvalue::ArrayLiteral { values, .. }, .. } => {
            for (i, (val_idx, _, _)) in rewrites.iter().enumerate() {
                values[*val_idx] = Value::Move(Place::local(tmp_locals[i]));
            }
        }
        StatementKind::Assign { rvalue: Rvalue::EnumVariant { payload, .. }, .. } => {
            for (i, (val_idx, _, _)) in rewrites.iter().enumerate() {
                payload[*val_idx] = Value::Move(Place::local(tmp_locals[i]));
            }
        }
        _ => {}
    }
}

fn elaborate_terminator(
    body: &mut MirBody,
    params: &HashSet<LocalId>,
    info: &CloneInfo,
    bi: usize,
) {
    let needs_rewrite = match &body.blocks[bi].terminator.kind {
        TerminatorKind::Return(Value::Copy(place)) => {
            needs_clone(body, params, info, place).is_some()
        },
        _ => false,
    };
    if !needs_rewrite {
        return;
    }

    let (place, ty) = match &body.blocks[bi].terminator.kind {
        TerminatorKind::Return(Value::Copy(place)) => {
            let ty = needs_clone(body, params, info, place).unwrap();
            (place.clone(), ty)
        },
        _ => unreachable!(),
    };

    let tmp = body.add_local(LocalDef::new("_clone", ty.clone()));
    let clone_call = make_clone_call(info.cloneable, &ty, Place::local(tmp), &place);
    body.blocks[bi].stmts.push(clone_call);
    body.blocks[bi].terminator.kind = TerminatorKind::Return(Value::Move(Place::local(tmp)));
}

fn make_clone_call(
    cloneable: Entity,
    self_type: &MirTy,
    dest: Place,
    source: &Place,
) -> Statement {
    let callee = Callee::witness(
        cloneable,
        WitnessMethodKey::bare("clone"),
        self_type.clone(),
        vec![],
    );
    Statement::new(StatementKind::Call {
        dest: Some(dest),
        callee,
        args: vec![Value::Ref(source.clone())],
    })
}
