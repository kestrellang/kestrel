//! Drop elaboration — the only pass that emits `Drop` / `DropIf` statements.
//!
//! ## Stage 7 (current)
//!
//! Builds the [`MovePathSet`] and runs the init/maybe-init [`dataflow`] over
//! each function body, then walks the result to emit destructor calls:
//!
//! - Non-parameter locals whose type's [`CopyBehavior`] is `None` are
//!   "drop-needing" — they participate in elaboration.
//! - Before every `Return` terminator, drops are emitted in *reverse
//!   declaration order* over the drop-needing locals.
//! - Locals that are never moved out get an unconditional [`Drop`].
//! - Locals that may be moved out get an [`DropIf`] guarded by a per-local
//!   `_init_<name>: Bool` flag local that DropElab itself maintains:
//!   - Flag is initialized to `false` at function entry (locals are uninit).
//!   - After every gen of that path (`Assign` whose dest reaches the local,
//!     or `Call` whose dest reaches the local) the flag is set `true`.
//!   - After every kill of that path (`Rvalue::Move(...)` or `Value::Move(...)`
//!     reaching the local) the flag is set `false`.
//!
//! Field-level partial moves and scope-tree drops are deferred — the move-path
//! infrastructure is still root-local-granular at Stage 7. The architectural
//! hook for per-projection paths is in place via [`MovePathSet::lookup_place`].
//!
//! ## Codegen contract
//!
//! `Drop` / `DropIf` are no-ops at the cranelift level today (see
//! `kestrel-codegen-cranelift::block`). The MIR shape they produce is
//! still important: it is the only signal future codegen has for where a
//! destructor call belongs, and the MIR text snapshots use it as the visible
//! representation of the new memory model.

use std::collections::{HashMap, HashSet};

use kestrel_mir::{
    CopyBehavior, Immediate, LocalDef, LocalId, MirBody, MirModule, MirTy, Place, Rvalue,
    Statement, StatementKind, TerminatorKind, Value,
};

use crate::dataflow;
use crate::move_path::MovePathSet;

/// Run drop elaboration on every function body in the module.
pub fn run(module: &mut MirModule) {
    // Pre-compute per-function inputs that need read access to the module
    // (struct/enum copy behavior, move-path set). After this loop nothing
    // else needs `&module`, so the mutation pass below can take `&mut`.
    let mut pre: Vec<Option<Prepared>> = Vec::with_capacity(module.functions.len());
    for func in &module.functions {
        pre.push(func.body.as_ref().map(|body| prepare(body, module)));
    }

    for (i, func) in module.functions.iter_mut().enumerate() {
        let Some(body) = &mut func.body else { continue };
        let Some(prepared) = pre[i].take() else { continue };
        elaborate(body, prepared);
    }
}

struct Prepared {
    drop_locals: Vec<LocalId>,
    paths: MovePathSet,
    df: dataflow::DataflowResult,
    moved: HashSet<LocalId>,
}

fn prepare(body: &MirBody, module: &MirModule) -> Prepared {
    let drop_locals: Vec<LocalId> = body
        .locals
        .iter()
        .enumerate()
        .skip(body.param_count)
        .filter(|(_, l)| l.ty.copy_behavior(module) == CopyBehavior::None)
        .map(|(i, _)| LocalId::new(i))
        .collect();
    let paths = MovePathSet::build(body, module);
    let df = dataflow::run(body, &paths);
    let moved = scan_moved_locals(body, &drop_locals);
    Prepared {
        drop_locals,
        paths,
        df,
        moved,
    }
}

fn elaborate(body: &mut MirBody, prepared: Prepared) {
    let Prepared {
        drop_locals,
        paths,
        df,
        moved,
    } = prepared;

    if drop_locals.is_empty() {
        return;
    }

    // Allocate a flag local for each moved local. New locals are appended to
    // the body's local list, which doesn't shift any existing `LocalId`.
    let mut flags: HashMap<LocalId, LocalId> = HashMap::new();
    let mut ordered_moved: Vec<LocalId> = moved.iter().copied().collect();
    ordered_moved.sort_by_key(|l| l.index());
    for local in ordered_moved {
        let name = format!("_init_{}", body.locals[local.index()].name);
        let flag = body.add_local(LocalDef::new(name, MirTy::Bool));
        flags.insert(local, flag);
    }

    // Inject flag updates around each gen/kill of a flagged path.
    inject_flag_updates(body, &paths, &flags);

    // Initialize every allocated flag to `false` at function entry so reads
    // in `DropIf` are well-defined on paths that never assigned the local.
    init_flags_at_entry(body, &flags);

    // Drops before each Return, in reverse declaration order.
    emit_return_drops(body, &paths, &df, &drop_locals, &flags);
}

fn scan_moved_locals(body: &MirBody, drop_locals: &[LocalId]) -> HashSet<LocalId> {
    let candidates: HashSet<LocalId> = drop_locals.iter().copied().collect();
    let mut moved = HashSet::new();
    for block in &body.blocks {
        for stmt in &block.stmts {
            match &stmt.kind {
                StatementKind::Assign { rvalue, .. } => {
                    collect_rvalue_moves(rvalue, &candidates, &mut moved);
                },
                StatementKind::Call { args, .. } => {
                    for arg in args {
                        collect_value_move(arg, &candidates, &mut moved);
                    }
                },
                _ => {},
            }
        }
        match &block.terminator.kind {
            TerminatorKind::Return(v) | TerminatorKind::Branch { condition: v, .. } => {
                collect_value_move(v, &candidates, &mut moved);
            },
            _ => {},
        }
    }
    moved
}

fn collect_rvalue_moves(rv: &Rvalue, candidates: &HashSet<LocalId>, out: &mut HashSet<LocalId>) {
    match rv {
        Rvalue::Move(place) => {
            if let Some(l) = place.root_local()
                && candidates.contains(&l)
            {
                out.insert(l);
            }
        },
        Rvalue::Op1 { arg, .. } => collect_value_move(arg, candidates, out),
        Rvalue::Op2 { lhs, rhs, .. } => {
            collect_value_move(lhs, candidates, out);
            collect_value_move(rhs, candidates, out);
        },
        Rvalue::Op3 { a, b, c, .. } => {
            collect_value_move(a, candidates, out);
            collect_value_move(b, candidates, out);
            collect_value_move(c, candidates, out);
        },
        Rvalue::Construct { fields, .. } => {
            for (_, v) in fields {
                collect_value_move(v, candidates, out);
            }
        },
        Rvalue::Tuple(vs) | Rvalue::ArrayLiteral { values: vs, .. } => {
            for v in vs {
                collect_value_move(v, candidates, out);
            }
        },
        Rvalue::EnumVariant { payload, .. } => {
            for v in payload {
                collect_value_move(v, candidates, out);
            }
        },
        Rvalue::ApplyPartial { captures, .. } => {
            for v in captures {
                collect_value_move(v, candidates, out);
            }
        },
        Rvalue::Copy(_) | Rvalue::Ref(_) | Rvalue::RefMut(_) | Rvalue::Const(_) => {},
    }
}

fn collect_value_move(v: &Value, candidates: &HashSet<LocalId>, out: &mut HashSet<LocalId>) {
    if let Value::Move(place) = v
        && let Some(l) = place.root_local()
        && candidates.contains(&l)
    {
        out.insert(l);
    }
}

fn inject_flag_updates(body: &mut MirBody, paths: &MovePathSet, flags: &HashMap<LocalId, LocalId>) {
    if flags.is_empty() {
        return;
    }
    for block_idx in 0..body.blocks.len() {
        let mut insertions: Vec<(usize, Statement)> = Vec::new();
        for (stmt_idx, stmt) in body.blocks[block_idx].stmts.iter().enumerate() {
            collect_flag_inserts_for_stmt(stmt, stmt_idx, paths, flags, &mut insertions);
        }
        insertions.sort_by_key(|(pos, _)| std::cmp::Reverse(*pos));
        for (pos, ins) in insertions {
            body.blocks[block_idx].stmts.insert(pos, ins);
        }
    }
}

fn collect_flag_inserts_for_stmt(
    stmt: &Statement,
    idx: usize,
    paths: &MovePathSet,
    flags: &HashMap<LocalId, LocalId>,
    out: &mut Vec<(usize, Statement)>,
) {
    let after = idx + 1;
    match &stmt.kind {
        StatementKind::Assign { dest, rvalue } => {
            for_each_rvalue_move(rvalue, paths, flags, &mut |flag| {
                out.push((after, set_flag(flag, false)));
            });
            if let Some(local) = dest.root_local()
                && let Some(&flag) = flags.get(&local)
            {
                out.push((after, set_flag(flag, true)));
            }
        },
        StatementKind::Call { dest, args, .. } => {
            for arg in args {
                for_each_value_move(arg, paths, flags, &mut |flag| {
                    out.push((after, set_flag(flag, false)));
                });
            }
            if let Some(dest_place) = dest
                && let Some(local) = dest_place.root_local()
                && let Some(&flag) = flags.get(&local)
            {
                out.push((after, set_flag(flag, true)));
            }
        },
        _ => {},
    }
}

fn for_each_rvalue_move(
    rv: &Rvalue,
    paths: &MovePathSet,
    flags: &HashMap<LocalId, LocalId>,
    f: &mut impl FnMut(LocalId),
) {
    match rv {
        Rvalue::Move(place) => {
            if let Some(local) = place.root_local()
                && let Some(&flag) = flags.get(&local)
                && paths.lookup_place(place).is_some()
            {
                f(flag);
            }
        },
        Rvalue::Op1 { arg, .. } => for_each_value_move(arg, paths, flags, f),
        Rvalue::Op2 { lhs, rhs, .. } => {
            for_each_value_move(lhs, paths, flags, f);
            for_each_value_move(rhs, paths, flags, f);
        },
        Rvalue::Op3 { a, b, c, .. } => {
            for_each_value_move(a, paths, flags, f);
            for_each_value_move(b, paths, flags, f);
            for_each_value_move(c, paths, flags, f);
        },
        Rvalue::Construct { fields, .. } => {
            for (_, v) in fields {
                for_each_value_move(v, paths, flags, f);
            }
        },
        Rvalue::Tuple(vs) | Rvalue::ArrayLiteral { values: vs, .. } => {
            for v in vs {
                for_each_value_move(v, paths, flags, f);
            }
        },
        Rvalue::EnumVariant { payload, .. } => {
            for v in payload {
                for_each_value_move(v, paths, flags, f);
            }
        },
        Rvalue::ApplyPartial { captures, .. } => {
            for v in captures {
                for_each_value_move(v, paths, flags, f);
            }
        },
        Rvalue::Copy(_) | Rvalue::Ref(_) | Rvalue::RefMut(_) | Rvalue::Const(_) => {},
    }
}

fn for_each_value_move(
    v: &Value,
    _paths: &MovePathSet,
    flags: &HashMap<LocalId, LocalId>,
    f: &mut impl FnMut(LocalId),
) {
    if let Value::Move(place) = v
        && let Some(local) = place.root_local()
        && let Some(&flag) = flags.get(&local)
    {
        f(flag);
    }
}

fn set_flag(flag: LocalId, value: bool) -> Statement {
    Statement::new(StatementKind::Assign {
        dest: Place::local(flag),
        rvalue: Rvalue::Const(Immediate::bool(value)),
    })
}

fn init_flags_at_entry(body: &mut MirBody, flags: &HashMap<LocalId, LocalId>) {
    if flags.is_empty() {
        return;
    }
    let mut sorted: Vec<(LocalId, LocalId)> = flags.iter().map(|(&k, &v)| (k, v)).collect();
    sorted.sort_by_key(|(local, _)| local.index());
    let entry = body.entry;
    let block = &mut body.blocks[entry.index()];
    let mut inserts: Vec<Statement> = sorted
        .into_iter()
        .map(|(_, flag)| set_flag(flag, false))
        .collect();
    inserts.append(&mut block.stmts);
    block.stmts = inserts;
}

fn emit_return_drops(
    body: &mut MirBody,
    paths: &MovePathSet,
    result: &dataflow::DataflowResult,
    drop_locals: &[LocalId],
    flags: &HashMap<LocalId, LocalId>,
) {
    for block_idx in 0..body.blocks.len() {
        if !matches!(body.blocks[block_idx].terminator.kind, TerminatorKind::Return(_)) {
            continue;
        }
        let exit_state = compute_exit_state(body, paths, result, block_idx);
        let mut to_append: Vec<Statement> = Vec::new();
        for &local in drop_locals.iter().rev() {
            let place = Place::local(local);
            let path = paths.lookup_local(local);
            // If the dataflow proves this path is entirely uninit at the
            // terminator, skip emitting a drop. This trims dead drops on
            // locals declared in branches that all moved them out.
            let touched = path.map(|p| exit_state.may_init.contains(&p)).unwrap_or(true);
            if !touched {
                continue;
            }
            match flags.get(&local) {
                Some(&flag) => {
                    to_append.push(Statement::new(StatementKind::DropIf { place, flag }));
                },
                None => {
                    to_append.push(Statement::new(StatementKind::Drop { place }));
                },
            }
        }
        body.blocks[block_idx].stmts.extend(to_append);
    }
}

fn compute_exit_state(
    body: &MirBody,
    paths: &MovePathSet,
    result: &dataflow::DataflowResult,
    block_idx: usize,
) -> dataflow::InitState {
    let mut state = result.blocks[block_idx].entry.clone();
    for stmt in &body.blocks[block_idx].stmts {
        apply_stmt_for_view(&mut state, stmt, paths);
    }
    state
}

fn apply_stmt_for_view(state: &mut dataflow::InitState, stmt: &Statement, paths: &MovePathSet) {
    match &stmt.kind {
        StatementKind::Assign { dest, rvalue } => {
            kill_rvalue_for_view(state, rvalue, paths);
            if let Some(p) = paths.lookup_place(dest) {
                state.mark_init(p);
            }
        },
        StatementKind::Call { dest, args, .. } => {
            for arg in args {
                kill_value_for_view(state, arg, paths);
            }
            if let Some(dest_place) = dest
                && let Some(p) = paths.lookup_place(dest_place)
            {
                state.mark_init(p);
            }
        },
        StatementKind::Drop { .. } | StatementKind::DropIf { .. } => {},
    }
}

fn kill_rvalue_for_view(state: &mut dataflow::InitState, rv: &Rvalue, paths: &MovePathSet) {
    match rv {
        Rvalue::Move(p) => {
            if let Some(path) = paths.lookup_place(p) {
                state.kill(path);
            }
        },
        Rvalue::Op1 { arg, .. } => kill_value_for_view(state, arg, paths),
        Rvalue::Op2 { lhs, rhs, .. } => {
            kill_value_for_view(state, lhs, paths);
            kill_value_for_view(state, rhs, paths);
        },
        Rvalue::Op3 { a, b, c, .. } => {
            kill_value_for_view(state, a, paths);
            kill_value_for_view(state, b, paths);
            kill_value_for_view(state, c, paths);
        },
        Rvalue::Construct { fields, .. } => {
            for (_, v) in fields {
                kill_value_for_view(state, v, paths);
            }
        },
        Rvalue::Tuple(vs) | Rvalue::ArrayLiteral { values: vs, .. } => {
            for v in vs {
                kill_value_for_view(state, v, paths);
            }
        },
        Rvalue::EnumVariant { payload, .. } => {
            for v in payload {
                kill_value_for_view(state, v, paths);
            }
        },
        Rvalue::ApplyPartial { captures, .. } => {
            for v in captures {
                kill_value_for_view(state, v, paths);
            }
        },
        Rvalue::Copy(_) | Rvalue::Ref(_) | Rvalue::RefMut(_) | Rvalue::Const(_) => {},
    }
}

fn kill_value_for_view(state: &mut dataflow::InitState, v: &Value, paths: &MovePathSet) {
    if let Value::Move(p) = v
        && let Some(path) = paths.lookup_place(p)
    {
        state.kill(path);
    }
}
