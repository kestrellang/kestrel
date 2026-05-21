use crate::body::{LocalDef, MirBody};
use crate::operand::Operand;
use crate::place::Place;
use crate::statement::{Statement, StatementKind};
use crate::terminator::TerminatorKind;
use crate::ty_query::needs_drop;
use crate::{BlockId, LocalId, MirModule};

use super::dataflow;
use super::init_state::{InitAnalysis, InitState};

/// Run drop elaboration on all functions in the module.
/// Drop shims must already be synthesized (see drop_shim::synthesize_drop_shims).
pub fn run_drop_elaboration(module: &mut MirModule) {
    for fi in 0..module.functions.len() {
        if module.functions[fi].body.is_none() {
            continue;
        }
        elaborate_function(module, fi);
    }
}

fn elaborate_function(module: &mut MirModule, func_idx: usize) {
    // Phase 1: Analysis (read-only)
    let body = module.functions[func_idx].body.as_ref().unwrap();
    let cfg = dataflow::compute_cfg_info(body);
    let analysis = InitAnalysis::compute_with_cfg(body, &cfg);

    let droppable: Vec<LocalId> = body
        .locals
        .iter()
        .enumerate()
        .filter(|(_, local)| needs_drop(&module.ty_arena, module, local.ty))
        .map(|(i, _)| LocalId::new(i))
        .collect();

    if droppable.is_empty() {
        return;
    }

    let return_blocks: Vec<BlockId> = body
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, bb)| matches!(bb.terminator.kind, TerminatorKind::Return(_)))
        .map(|(i, _)| BlockId::new(i))
        .collect();

    // Determine which locals need flags
    let mut needs_flag: Vec<bool> = vec![false; body.locals.len()];
    for &local in &droppable {
        needs_flag[local.index()] = return_blocks.iter().any(|&rb| {
            compute_state_before_terminator(&analysis, body, rb, local) == InitState::Maybe
        });
    }

    // Compute return drop actions per block
    let mut return_drops: Vec<(BlockId, Vec<Statement>)> = Vec::new();
    for &rb in &return_blocks {
        let mut drops = Vec::new();
        let is_returned_local = if let TerminatorKind::Return(Operand::Place(p)) =
            &body.block(rb).terminator.kind
        {
            p.root_local()
        } else {
            None
        };

        for &local in droppable.iter().rev() {
            if Some(local) == is_returned_local {
                continue;
            }
            let state = compute_state_before_terminator(&analysis, body, rb, local);
            match state {
                InitState::Live => {
                    drops.push(Statement::new(StatementKind::Drop {
                        place: Place::local(local),
                    }));
                }
                InitState::Maybe => {
                    // flag will be allocated below
                    drops.push(Statement::new(StatementKind::DropIf {
                        place: Place::local(local),
                        flag: LocalId::new(0), // placeholder, patched below
                    }));
                }
                InitState::Dead => {}
            }
        }
        return_drops.push((rb, drops));
    }

    // Compute overwrite-drop locations
    let num_blocks = body.blocks.len();
    let mut overwrite_drops: Vec<(BlockId, usize, Statement)> = Vec::new();
    for bi in 0..num_blocks {
        let block = BlockId::new(bi);
        let bb = body.block(block);
        let current_states: Vec<InitState> = droppable
            .iter()
            .map(|&l| analysis.state_at_entry(block, l))
            .collect();
        let mut states = current_states;

        for si in 0..bb.stmts.len() {
            let dest_local = match &bb.stmts[si].kind {
                StatementKind::Assign { dest, .. } => dest.root_local(),
                StatementKind::Call { dest: Some(d), .. } => d.root_local(),
                _ => None,
            };
            if let Some(local) = dest_local
                && let Some(di) = droppable.iter().position(|&d| d == local)
            {
                if states[di] == InitState::Live {
                    overwrite_drops.push((
                        block,
                        si,
                        Statement::new(StatementKind::Drop {
                            place: Place::local(local),
                        }),
                    ));
                }
                states[di] = InitState::Live;
            }
        }
    }

    // Compute flag update locations
    let mut flag_updates: Vec<(BlockId, usize, bool, LocalId)> = Vec::new();
    for bi in 0..num_blocks {
        let block = BlockId::new(bi);
        let bb = body.block(block);
        for si in 0..bb.stmts.len() {
            match &bb.stmts[si].kind {
                StatementKind::Assign { dest, rvalue } => {
                    if let Some(local) = dest.root_local()
                        && needs_flag[local.index()]
                    {
                        flag_updates.push((block, si + 1, true, local));
                    }
                    for (op, mode) in rvalue.operands_with_mode() {
                        if mode == Some(crate::UseMode::Move)
                            && let Operand::Place(p) = op
                            && let Some(local) = p.root_local()
                            && needs_flag[local.index()]
                        {
                            flag_updates.push((block, si + 1, false, local));
                        }
                    }
                }
                StatementKind::Call { dest, args, .. } => {
                    if let Some(local) = dest.as_ref().and_then(|d| d.root_local())
                        && needs_flag[local.index()]
                    {
                        flag_updates.push((block, si + 1, true, local));
                    }
                    for (operand, mode) in args {
                        if *mode == crate::ArgMode::Move
                            && let Operand::Place(p) = operand
                            && let Some(local) = p.root_local()
                            && needs_flag[local.index()]
                        {
                            flag_updates.push((block, si + 1, false, local));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Phase 2: Mutation
    let bool_ty = module.ty_arena.bool();
    let body = module.functions[func_idx].body.as_mut().unwrap();

    // Allocate flag locals
    let mut flag_map: Vec<Option<LocalId>> = vec![None; needs_flag.len()];
    for (i, &nf) in needs_flag.iter().enumerate() {
        if nf {
            let flag = body.add_local(LocalDef::new(
                format!("_drop_flag_{}", i),
                bool_ty,
            ));
            flag_map[i] = Some(flag);
        }
    }

    // Insert SetDropFlag(false) at function entry
    let entry = body.entry;
    let mut entry_flags: Vec<Statement> = Vec::new();
    for &flag in flag_map.iter().flatten() {
        entry_flags.push(Statement::new(StatementKind::SetDropFlag {
            flag,
            value: false,
        }));
    }
    let entry_stmts = &mut body.block_mut(entry).stmts;
    let mut existing = std::mem::take(entry_stmts);
    entry_flags.append(&mut existing);
    *entry_stmts = entry_flags;

    // Insert return drops (patch DropIf placeholders with actual flags)
    for (rb, mut drops) in return_drops {
        for stmt in &mut drops {
            if let StatementKind::DropIf { place, flag } = &mut stmt.kind
                && let Some(local) = place.root_local()
                && let Some(real_flag) = flag_map[local.index()]
            {
                *flag = real_flag;
            }
        }
        body.block_mut(rb).stmts.extend(drops);
    }

    // Insert overwrite-drops (reverse order to preserve indices)
    overwrite_drops.sort_by(|a, b| b.0.index().cmp(&a.0.index()).then(b.1.cmp(&a.1)));
    for (block, pos, stmt) in overwrite_drops {
        body.block_mut(block).stmts.insert(pos, stmt);
    }

    // Insert flag updates (reverse order to preserve indices)
    flag_updates.sort_by(|a, b| b.0.index().cmp(&a.0.index()).then(b.1.cmp(&a.1)));
    for (block, pos, value, local) in flag_updates {
        if let Some(flag) = flag_map[local.index()] {
            body.block_mut(block).stmts.insert(
                pos,
                Statement::new(StatementKind::SetDropFlag { flag, value }),
            );
        }
    }
}

/// Compute the init state of a local just before the terminator of a block.
fn compute_state_before_terminator(
    analysis: &InitAnalysis,
    body: &MirBody,
    block: BlockId,
    local: LocalId,
) -> InitState {
    let num_stmts = body.block(block).stmts.len();
    if num_stmts == 0 {
        return analysis.state_at_entry(block, local);
    }
    analysis.state_after(body, block, num_stmts - 1, local)
}


#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_hecs::Entity;

    use crate::builder::ModuleBuilder;
    use crate::immediate::Immediate;
    use crate::item::struct_def::{FieldDef, StructDef};
    use crate::item::{CopyBehavior, DropBehavior, TypeInfo};
    use crate::FieldIdx;

    fn setup_droppable_module() -> (ModuleBuilder, Entity, crate::TyId) {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let s_entity = m.fresh_entity();
        m.register_name(s_entity, "MyStr");
        let s_ty = m.named(s_entity, vec![]);
        let mut def = StructDef::new(s_entity, "MyStr");
        def.add_field(FieldDef::new("data", i64_ty));
        def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            drop: DropBehavior::StructDrop {
                deinit: None,
                fields: vec![],
            },
            layout: None,
        };
        m.add_struct(def);
        (m, s_entity, s_ty)
    }

    fn count_drops(body: &MirBody) -> usize {
        body.blocks
            .iter()
            .flat_map(|bb| &bb.stmts)
            .filter(|s| matches!(s.kind, StatementKind::Drop { .. }))
            .count()
    }

    fn count_drop_ifs(body: &MirBody) -> usize {
        body.blocks
            .iter()
            .flat_map(|bb| &bb.stmts)
            .filter(|s| matches!(s.kind, StatementKind::DropIf { .. }))
            .count()
    }

    fn count_set_flags(body: &MirBody) -> usize {
        body.blocks
            .iter()
            .flat_map(|bb| &bb.stmts)
            .filter(|s| matches!(s.kind, StatementKind::SetDropFlag { .. }))
            .count()
    }

    // ---- Live local at Return → Drop inserted ----

    #[test]
    fn live_local_dropped_at_return() {
        let (mut m, _, s_ty) = setup_droppable_module();
        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", s_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.ret_unit();
        }
        let mut module = m.finish();
        run_drop_elaboration(&mut module);

        let body = module.functions.last().unwrap().body.as_ref().unwrap();
        assert_eq!(count_drops(body), 1);
    }

    // ---- Dead local at Return → no Drop ----

    #[test]
    fn dead_local_not_dropped() {
        let (mut m, _, s_ty) = setup_droppable_module();
        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", s_ty);
        let y = f.local("y", s_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            // Move x to y — x becomes Dead
            b.use_move(Place::local(y), Place::local(x));
            b.ret_unit();
        }
        let mut module = m.finish();
        run_drop_elaboration(&mut module);

        let body = module.functions.last().unwrap().body.as_ref().unwrap();
        // y is Live (needs drop), x is Dead (no drop)
        assert_eq!(count_drops(body), 1);
        // The drop should be for y, not x
        let drop_stmt = body.blocks[0]
            .stmts
            .iter()
            .find(|s| matches!(s.kind, StatementKind::Drop { .. }))
            .unwrap();
        match &drop_stmt.kind {
            StatementKind::Drop { place } => {
                assert_eq!(place.root_local(), Some(y));
            }
            _ => unreachable!(),
        }
    }

    // ---- Maybe local (diamond) → DropIf with flag ----

    #[test]
    fn maybe_local_gets_drop_if() {
        let (mut m, _, s_ty) = setup_droppable_module();
        let unit_ty = m.unit();
        let bool_ty = m.bool();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", s_ty);
        let y = f.local("y", s_ty);
        let cond = f.local("cond", bool_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        let bb2 = f.block_id();
        let bb3 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.assign_const(Place::local(cond), Immediate::bool(true));
            b.branch(Operand::Place(Place::local(cond)), bb1, bb2);
        }
        {
            let mut b = f.block_at(bb1);
            // Move x → y: x becomes Dead on this path
            b.use_move(Place::local(y), Place::local(x));
            b.jump(bb3);
        }
        {
            // x stays Live on this path
            f.block_at(bb2).jump(bb3);
        }
        {
            // At bb3: x is Maybe (Live on bb2 path, Dead on bb1 path)
            f.block_at(bb3).ret_unit();
        }
        let mut module = m.finish();
        run_drop_elaboration(&mut module);

        let body = module.functions.last().unwrap().body.as_ref().unwrap();
        // x should have a DropIf (Maybe), y should have a DropIf or Drop
        assert!(count_drop_ifs(body) >= 1);
        // Drop flags should exist
        assert!(count_set_flags(body) >= 1);
    }

    // ---- Return kills, not drops ----

    #[test]
    fn returned_local_not_dropped() {
        let (mut m, _, s_ty) = setup_droppable_module();
        let mut f = m.function("f", s_ty);
        let x = f.local("x", s_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.ret(Operand::Place(Place::local(x)));
        }
        let mut module = m.finish();
        run_drop_elaboration(&mut module);

        let body = module.functions.last().unwrap().body.as_ref().unwrap();
        // x is returned (moved to caller) — should NOT be dropped
        assert_eq!(count_drops(body), 0);
        assert_eq!(count_drop_ifs(body), 0);
    }

    // ---- Panic gets no drops ----

    #[test]
    fn panic_no_drops() {
        let (mut m, _, s_ty) = setup_droppable_module();
        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", s_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.panic("abort");
        }
        let mut module = m.finish();
        run_drop_elaboration(&mut module);

        let body = module.functions.last().unwrap().body.as_ref().unwrap();
        // Panic = abort, no cleanup
        assert_eq!(count_drops(body), 0);
        assert_eq!(count_drop_ifs(body), 0);
    }

    // ---- Overwrite-drop: assign to Live droppable ----

    #[test]
    fn overwrite_inserts_drop_before_assign() {
        let (mut m, _, s_ty) = setup_droppable_module();
        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", s_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0)); // x = Live
            b.assign_const(Place::local(x), Immediate::i64(1)); // overwrite: drop old first
            b.ret_unit();
        }
        let mut module = m.finish();
        run_drop_elaboration(&mut module);

        let body = module.functions.last().unwrap().body.as_ref().unwrap();
        // 2 drops: one for overwrite, one at Return
        assert_eq!(count_drops(body), 2);
    }

    // ---- No droppable locals → no-op ----

    #[test]
    fn no_droppable_locals_noop() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let mut f = m.function("f", i64_ty);
        let x = f.local("x", i64_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.ret(Operand::Place(Place::local(x)));
        }
        let mut module = m.finish();
        let stmts_before = module.functions[0].body.as_ref().unwrap().blocks[0].stmts.len();
        run_drop_elaboration(&mut module);
        let stmts_after = module.functions[0].body.as_ref().unwrap().blocks[0].stmts.len();
        assert_eq!(stmts_before, stmts_after);
    }

    // ---- Drop flag initialized to false at entry ----

    #[test]
    fn drop_flag_initialized_false() {
        let (mut m, _, s_ty) = setup_droppable_module();
        let unit_ty = m.unit();
        let bool_ty = m.bool();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", s_ty);
        let cond = f.local("cond", bool_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        let bb2 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(cond), Immediate::bool(true));
            b.branch(Operand::Place(Place::local(cond)), bb1, bb2);
        }
        {
            let mut b = f.block_at(bb1);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.jump(bb2);
        }
        {
            // x is Maybe at bb2 (Live from bb1, Dead from bb0)
            f.block_at(bb2).ret_unit();
        }
        let mut module = m.finish();
        run_drop_elaboration(&mut module);

        let body = module.functions.last().unwrap().body.as_ref().unwrap();
        // Entry block (bb0) should start with SetDropFlag(false)
        let first_stmt = &body.blocks[0].stmts[0];
        match &first_stmt.kind {
            StatementKind::SetDropFlag { value, .. } => {
                assert!(!value);
            }
            other => panic!("expected SetDropFlag(false) at entry, got {other:?}"),
        }
    }
}
