//! Clone elaboration pass — rewrites `Rvalue::Copy` of Clone types into
//! explicit `Callee::Witness` clone calls.
//!
//! Scope:
//! - Assignment copies (`%dest = copy %src`)
//! - Closure captures (`apply_partial(captures: [copy %x])`)
//!
//! Call args, construct fields, tuple elements, etc. are left as-is —
//! those positions use Copy as a lightweight alias whose ownership is
//! managed by the callee or container.
//!
//! Must run **before** drop elaboration: drop elab's `copied_from` tracking
//! assumes Copy destinations alias the source. Clone destinations are
//! independent owned values (from a Call result), and drop elab needs to see
//! the Call + Move pattern to insert correct drops.

use std::collections::HashSet;

use crate::item::{CopyBehavior, FunctionKind};
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

/// Check if copying this place requires a clone call. Returns the type
/// if so. Only rewrites bare locals of Named types with Cloneable conformance.
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

/// Collect the set of locals that receive an assignment anywhere in the body.
/// Used to distinguish genuinely-alive locals from stale captures leaked
/// by sibling closures in find_captures.
fn assigned_locals(body: &MirBody) -> HashSet<LocalId> {
    let mut set = HashSet::new();
    for block in &body.blocks {
        for stmt in &block.stmts {
            match &stmt.kind {
                StatementKind::Assign { dest, .. } => {
                    if let Some(id) = dest.root_local() {
                        set.insert(id);
                    }
                },
                StatementKind::Call { dest: Some(dest), .. } => {
                    if let Some(id) = dest.root_local() {
                        set.insert(id);
                    }
                },
                _ => {},
            }
        }
    }
    set
}

fn elaborate_function(
    body: &mut MirBody,
    params: &HashSet<LocalId>,
    info: &CloneInfo,
) {
    let assigned = assigned_locals(body);
    let num_blocks = body.blocks.len();
    for bi in 0..num_blocks {
        elaborate_block(body, params, info, &assigned, bi);
    }
}

fn elaborate_block(
    body: &mut MirBody,
    params: &HashSet<LocalId>,
    info: &CloneInfo,
    assigned: &HashSet<LocalId>,
    bi: usize,
) {
    let mut si = body.blocks[bi].stmts.len();
    while si > 0 {
        si -= 1;

        // Rewrite Rvalue::Copy in assignments
        let should_rewrite_assign = match &body.blocks[bi].stmts[si].kind {
            StatementKind::Assign { rvalue: Rvalue::Copy(place), .. } => {
                needs_clone(body, params, info, place).is_some()
            },
            _ => false,
        };
        if should_rewrite_assign {
            rewrite_assign_copy(body, params, info, bi, si);
            continue;
        }

        // Rewrite Value::Copy in ApplyPartial captures
        let capture_rewrites = collect_capture_rewrites(body, params, info, assigned, bi, si);
        if !capture_rewrites.is_empty() {
            apply_capture_rewrites(body, info, bi, si, &capture_rewrites);
            si += capture_rewrites.len();
        }
    }

    elaborate_terminator(body, params, info, bi);
}

/// Rewrite `%dest = copy %place` → `%tmp = call clone(&place); %dest = move %tmp`
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

/// Collect capture positions in ApplyPartial that need clone elaboration.
/// Only clones captures whose source local is assigned somewhere in the
/// function — this filters out stale locals from sibling closures that
/// leaked through find_captures.
fn collect_capture_rewrites(
    body: &MirBody,
    params: &HashSet<LocalId>,
    info: &CloneInfo,
    assigned: &HashSet<LocalId>,
    bi: usize,
    si: usize,
) -> Vec<(usize, Place, MirTy)> {
    let mut rewrites = Vec::new();
    let stmt = &body.blocks[bi].stmts[si];
    if let StatementKind::Assign { rvalue: Rvalue::ApplyPartial { captures, .. }, .. } = &stmt.kind {
        for (i, val) in captures.iter().enumerate() {
            if let Value::Copy(place) = val {
                if let Some(local_id) = place.root_local() {
                    // Only clone if this local was assigned in the parent body
                    if assigned.contains(&local_id) {
                        if let Some(ty) = needs_clone(body, params, info, place) {
                            rewrites.push((i, place.clone(), ty));
                        }
                    }
                }
            }
        }
    }
    rewrites
}

/// Insert clone calls before the ApplyPartial and patch capture values.
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

/// Handle Value::Copy in terminators (primarily Return).
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
