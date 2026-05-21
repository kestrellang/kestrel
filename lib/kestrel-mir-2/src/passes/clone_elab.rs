use kestrel_hecs::Entity;

use crate::body::{LocalDef, MirBody};
use crate::item::function::{FunctionKind, WhereClause};
use crate::operand::{ArgMode, Operand, UseMode};
use crate::place::Place;
use crate::statement::{Callee, Rvalue, Statement, StatementKind, WitnessMethodKey};
use crate::terminator::TerminatorKind;
use crate::ty::TyArena;
use crate::ty_query::{copy_behavior, find_cloneable_protocol};
use crate::{BlockId, CopyBehavior, LocalId, MirModule, TyId};

use super::liveness::Liveness;

// ---- Pass ----

pub fn run_clone_elaboration(module: &mut MirModule) {
    let Some(cloneable) = find_cloneable_protocol(module) else {
        return;
    };

    for fi in 0..module.functions.len() {
        if should_skip(&module.functions[fi]) {
            continue;
        }
        if module.functions[fi].body.is_none() {
            continue;
        }
        elaborate_function(module, fi, cloneable);
    }
}

fn should_skip(func: &crate::item::function::FunctionDef) -> bool {
    matches!(func.kind, FunctionKind::Deinit { .. })
}

fn elaborate_function(module: &mut MirModule, func_idx: usize, cloneable: Entity) {
    let body = module.functions[func_idx].body.as_ref().unwrap();
    let liveness = Liveness::compute(body);
    let num_blocks = body.blocks.len();

    for bi in 0..num_blocks {
        elaborate_block(module, func_idx, BlockId::new(bi), &liveness, cloneable);
    }
}

fn elaborate_block(
    module: &mut MirModule,
    func_idx: usize,
    block: BlockId,
    liveness: &Liveness,
    cloneable: Entity,
) {
    let body = module.functions[func_idx].body.as_ref().unwrap();
    let live_after = liveness.block_liveness_after(body, block);

    // Walk statements, collecting rewrites. We process from the end to avoid
    // invalidating indices, and insert clone calls before the statement.
    let where_clause = module.functions[func_idx].where_clause.as_ref();

    // Collect all rewrites first to avoid borrow issues
    let mut stmt_rewrites: Vec<StmtRewrite> = Vec::new();

    for (si, live) in live_after.iter().enumerate() {
        let stmt = &body.block(block).stmts[si];

        match &stmt.kind {
            StatementKind::Assign { dest: _, rvalue } => {
                collect_rvalue_rewrites(
                    &module.ty_arena,
                    module,
                    body,
                    where_clause,
                    rvalue,
                    live,
                    si,
                    &mut stmt_rewrites,
                );
            }
            StatementKind::Call { args, .. } => {
                for (ai, (operand, mode)) in args.iter().enumerate() {
                    if *mode != ArgMode::Copy {
                        continue;
                    }
                    if let Operand::Place(place) = operand
                        && let Some(local) = place.root_local()
                    {
                        let ty = body.local(local).ty;
                        if is_clone_type(&module.ty_arena, module, ty, where_clause) {
                            if !place.projections.is_empty()
                                && crate::ty_query::needs_drop(&module.ty_arena, module, ty)
                            {
                                continue;
                            }
                            let is_live = live.get(local.index());
                            stmt_rewrites.push(StmtRewrite::CallArg {
                                stmt_index: si,
                                arg_index: ai,
                                source: place.clone(),
                                ty,
                                needs_clone: is_live,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Check terminator
    let body = module.functions[func_idx].body.as_ref().unwrap();
    let mut term_rewrite: Option<TermRewrite> = None;
    if let TerminatorKind::Return(Operand::Place(place)) = &body.block(block).terminator.kind
        && let Some(local) = place.root_local()
    {
        let ty = body.local(local).ty;
        if is_clone_type(&module.ty_arena, module, ty, where_clause) {
            // Return is a terminator — nothing follows in this block. The exit
            // state of a Return block is empty, so the local is dead after the
            // return. Always rewrite to Move, no clone needed.
            term_rewrite = Some(TermRewrite {
                source: place.clone(),
                ty,
                needs_clone: false,
            });
        }
    }

    // Apply rewrites (reverse order to preserve indices)
    let body = module.functions[func_idx].body.as_mut().unwrap();

    // Apply terminator rewrite
    if let Some(rewrite) = term_rewrite {
        if rewrite.needs_clone {
            let tmp = body.add_local(LocalDef::new("_clone", rewrite.ty));
            let clone_call =
                make_clone_call(cloneable, rewrite.ty, tmp, &rewrite.source);
            body.block_mut(block).stmts.push(clone_call);
            body.block_mut(block).terminator.kind =
                TerminatorKind::Return(Operand::Place(Place::local(tmp)));
        } else {
            // Dead after return — just move, no clone needed
            // Return already implicitly moves, but if the IR had UseMode on Return
            // we'd change it. Since Return takes an Operand (not Operand+UseMode),
            // no change needed — it's already a move.
        }
    }

    // Apply statement rewrites in reverse order
    let mut insertion_offset = 0;
    let mut last_si = usize::MAX;
    for rewrite in stmt_rewrites.iter().rev() {
        match rewrite {
            StmtRewrite::RvalueOperand {
                stmt_index,
                operand_index,
                source,
                ty,
                needs_clone,
            } => {
                if *stmt_index != last_si {
                    insertion_offset = 0;
                    last_si = *stmt_index;
                }
                let body = module.functions[func_idx].body.as_mut().unwrap();
                if *needs_clone {
                    let tmp = body.add_local(LocalDef::new("_clone", *ty));
                    let clone_call =
                        make_clone_call(cloneable, *ty, tmp, source);
                    body.block_mut(block)
                        .stmts
                        .insert(*stmt_index + insertion_offset, clone_call);
                    insertion_offset += 1;
                    rewrite_rvalue_operand(
                        body,
                        block,
                        *stmt_index + insertion_offset,
                        *operand_index,
                        Place::local(tmp),
                    );
                } else {
                    rewrite_rvalue_operand_to_move(
                        body,
                        block,
                        *stmt_index + insertion_offset,
                        *operand_index,
                    );
                }
            }
            StmtRewrite::CallArg {
                stmt_index,
                arg_index,
                source,
                ty,
                needs_clone,
            } => {
                if *stmt_index != last_si {
                    insertion_offset = 0;
                    last_si = *stmt_index;
                }
                let body = module.functions[func_idx].body.as_mut().unwrap();
                if *needs_clone {
                    let tmp = body.add_local(LocalDef::new("_clone", *ty));
                    let clone_call =
                        make_clone_call(cloneable, *ty, tmp, source);
                    body.block_mut(block)
                        .stmts
                        .insert(*stmt_index + insertion_offset, clone_call);
                    insertion_offset += 1;
                    rewrite_call_arg(
                        body,
                        block,
                        *stmt_index + insertion_offset,
                        *arg_index,
                        Place::local(tmp),
                    );
                } else {
                    rewrite_call_arg_to_move(
                        body,
                        block,
                        *stmt_index + insertion_offset,
                        *arg_index,
                    );
                }
            }
        }
    }
}

// ---- Rewrite descriptors ----

enum StmtRewrite {
    RvalueOperand {
        stmt_index: usize,
        operand_index: usize,
        source: Place,
        ty: TyId,
        needs_clone: bool,
    },
    CallArg {
        stmt_index: usize,
        arg_index: usize,
        source: Place,
        ty: TyId,
        needs_clone: bool,
    },
}

struct TermRewrite {
    source: Place,
    ty: TyId,
    needs_clone: bool,
}

// ---- Helpers ----

fn is_clone_type(
    arena: &TyArena,
    module: &MirModule,
    ty: TyId,
    where_clause: Option<&WhereClause>,
) -> bool {
    matches!(
        copy_behavior(arena, module, ty, where_clause),
        CopyBehavior::Clone(_)
    )
}

fn collect_rvalue_rewrites(
    arena: &TyArena,
    module: &MirModule,
    body: &MirBody,
    where_clause: Option<&WhereClause>,
    rvalue: &Rvalue,
    live: &super::liveness::BitVec,
    stmt_index: usize,
    out: &mut Vec<StmtRewrite>,
) {
    let operands: Vec<_> = rvalue.operands_with_mode().collect();
    for (oi, (operand, mode)) in operands.iter().enumerate() {
        let Some(UseMode::Copy) = mode else { continue };
        let Operand::Place(place) = operand else { continue };
        let Some(local) = place.root_local() else {
            continue;
        };
        let ty = body.local(local).ty;
        if is_clone_type(arena, module, ty, where_clause) {
            // TODO: projected copies from droppable aggregates where the field
            // type is bitwise-copyable should stay as Copy — rewriting to Move
            // would create a projected move that the verifier rejects. Long-term,
            // per-field init tracking would handle this properly.
            if !place.projections.is_empty()
                && crate::ty_query::needs_drop(arena, module, ty)
            {
                continue;
            }
            let is_live = live.get(local.index());
            out.push(StmtRewrite::RvalueOperand {
                stmt_index,
                operand_index: oi,
                source: (*place).clone(),
                ty,
                needs_clone: is_live,
            });
        }
    }
}

fn make_clone_call(
    cloneable: Entity,
    self_type: TyId,
    dest: LocalId,
    source: &Place,
) -> Statement {
    Statement::new(StatementKind::Call {
        dest: Some(Place::local(dest)),
        callee: Callee::Witness {
            protocol: cloneable,
            method: WitnessMethodKey::simple("clone"),
            self_type,
            method_type_args: vec![],
        },
        args: vec![(Operand::Place(source.clone()), ArgMode::Ref)],
    })
}

fn rewrite_rvalue_operand(
    body: &mut MirBody,
    block: BlockId,
    stmt_index: usize,
    operand_index: usize,
    new_source: Place,
) {
    let stmt = &mut body.block_mut(block).stmts[stmt_index];
    if let StatementKind::Assign { rvalue, .. } = &mut stmt.kind {
        let mut ops: Vec<_> = rvalue.operands_mut().collect();
        if operand_index < ops.len() {
            *ops[operand_index] = Operand::Place(new_source);
        }
        // Also change mode to Move
        set_rvalue_mode(rvalue, operand_index, UseMode::Move);
    }
}

fn rewrite_rvalue_operand_to_move(
    body: &mut MirBody,
    block: BlockId,
    stmt_index: usize,
    operand_index: usize,
) {
    let stmt = &mut body.block_mut(block).stmts[stmt_index];
    if let StatementKind::Assign { rvalue, .. } = &mut stmt.kind {
        set_rvalue_mode(rvalue, operand_index, UseMode::Move);
    }
}

fn set_rvalue_mode(rvalue: &mut Rvalue, operand_index: usize, mode: UseMode) {
    match rvalue {
        Rvalue::Use(_, m) if operand_index == 0 => *m = mode,
        Rvalue::Construct { fields, .. } => {
            if operand_index < fields.len() {
                fields[operand_index].2 = mode;
            }
        }
        Rvalue::Tuple(elems) => {
            if operand_index < elems.len() {
                elems[operand_index].1 = mode;
            }
        }
        Rvalue::EnumVariant { payload, .. } => {
            if operand_index < payload.len() {
                payload[operand_index].1 = mode;
            }
        }
        Rvalue::ArrayLiteral { values, .. } => {
            if operand_index < values.len() {
                values[operand_index].1 = mode;
            }
        }
        Rvalue::ApplyPartial { captures, .. } => {
            if operand_index < captures.len() {
                captures[operand_index].1 = mode;
            }
        }
        _ => {}
    }
}

fn rewrite_call_arg(
    body: &mut MirBody,
    block: BlockId,
    stmt_index: usize,
    arg_index: usize,
    new_source: Place,
) {
    let stmt = &mut body.block_mut(block).stmts[stmt_index];
    if let StatementKind::Call { args, .. } = &mut stmt.kind
        && arg_index < args.len()
    {
        args[arg_index] = (Operand::Place(new_source), ArgMode::Move);
    }
}

fn rewrite_call_arg_to_move(
    body: &mut MirBody,
    block: BlockId,
    stmt_index: usize,
    arg_index: usize,
) {
    let stmt = &mut body.block_mut(block).stmts[stmt_index];
    if let StatementKind::Call { args, .. } = &mut stmt.kind
        && arg_index < args.len()
    {
        args[arg_index].1 = ArgMode::Move;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::ModuleBuilder;
    use crate::immediate::Immediate;
    use crate::item::protocol::ProtocolDef;
    use crate::item::struct_def::{FieldDef, StructDef};
    use crate::item::{CopyBehavior, TypeInfo};
    use crate::ty::ParamConvention;
    use crate::FieldIdx;

    // ---- Test scaffolding ----

    /// Build a module with a Cloneable protocol and a Clone-typed struct "MyStr".
    fn setup_clone_module() -> (ModuleBuilder, Entity, TyId, Entity) {
        let mut m = ModuleBuilder::new("test");

        // Cloneable protocol
        let cloneable = m.fresh_entity();
        m.register_name(cloneable, "std.Cloneable");
        let proto = ProtocolDef::new(cloneable, "std.Cloneable");
        m.add_protocol(proto);

        // Clone-typed struct
        let mystr_entity = m.fresh_entity();
        m.register_name(mystr_entity, "MyStr");
        let i64_ty = m.i64();
        let mystr_ty = m.named(mystr_entity, vec![]);
        let mut def = StructDef::new(mystr_entity, "MyStr");
        def.add_field(FieldDef::new("data", i64_ty));
        def.type_info = TypeInfo {
            copy: CopyBehavior::Clone(cloneable),
            ..TypeInfo::default()
        };
        m.add_struct(def);

        // A callee for testing call args
        let callee_entity = m.fresh_entity();
        m.register_name(callee_entity, "consume");

        (m, cloneable, mystr_ty, callee_entity)
    }

    // ---- Clone elaboration: bitwise copies left alone ----

    #[test]
    fn bitwise_copy_unchanged() {
        let (mut m, _, _, _) = setup_clone_module();
        let i64_ty = m.i64();
        let mut f = m.function("f", i64_ty);
        let x = f.param("x", i64_ty, ParamConvention::Consuming);
        let y = f.local("y", i64_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            // Bitwise copy — should NOT be rewritten
            b.assign(
                Place::local(y),
                Rvalue::Use(Operand::Place(Place::local(x)), UseMode::Copy),
            );
            b.ret(Operand::Place(Place::local(y)));
        }
        let mut module = m.finish();
        let stmt_count_before = module.functions.last().unwrap().body.as_ref().unwrap().blocks[0]
            .stmts
            .len();
        run_clone_elaboration(&mut module);
        let stmt_count_after = module.functions.last().unwrap().body.as_ref().unwrap().blocks[0]
            .stmts
            .len();
        assert_eq!(stmt_count_before, stmt_count_after);
    }

    // ---- Clone-typed copy, source live → clone call + move ----

    #[test]
    fn clone_copy_live_source_inserts_clone() {
        let (mut m, cloneable, mystr_ty, _) = setup_clone_module();
        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", mystr_ty);
        let y = f.local("y", mystr_ty);
        let z = f.local("z", mystr_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            // y = copy x (x is live after — used again in next stmt)
            b.assign(
                Place::local(y),
                Rvalue::Use(Operand::Place(Place::local(x)), UseMode::Copy),
            );
            // z = copy x (x is dead after — last use)
            b.assign(
                Place::local(z),
                Rvalue::Use(Operand::Place(Place::local(x)), UseMode::Copy),
            );
            b.ret_unit();
        }
        let mut module = m.finish();
        run_clone_elaboration(&mut module);

        let body = module.functions.last().unwrap().body.as_ref().unwrap();
        // First copy (x live): should insert a clone call before, then move clone temp
        // Second copy (x dead): should just rewrite to move, no clone
        // So we expect: clone_call, y = move _clone, z = move x
        assert_eq!(body.blocks[0].stmts.len(), 3);

        // First stmt: clone call
        assert!(matches!(
            &body.blocks[0].stmts[0].kind,
            StatementKind::Call {
                callee: Callee::Witness { protocol, .. },
                ..
            } if *protocol == cloneable
        ));

        // Second stmt: y = move _clone_tmp
        match &body.blocks[0].stmts[1].kind {
            StatementKind::Assign {
                rvalue: Rvalue::Use(_, UseMode::Move),
                ..
            } => {}
            other => panic!("expected Use+Move, got {other:?}"),
        }

        // Third stmt: z = move x (last use, no clone)
        match &body.blocks[0].stmts[2].kind {
            StatementKind::Assign {
                rvalue: Rvalue::Use(Operand::Place(p), UseMode::Move),
                ..
            } => {
                assert_eq!(p.root_local(), Some(x));
            }
            other => panic!("expected Use(x, Move), got {other:?}"),
        }
    }

    // ---- Clone-typed copy, source dead → just move ----

    #[test]
    fn clone_copy_dead_source_rewrites_to_move() {
        let (mut m, _, mystr_ty, _) = setup_clone_module();
        let mut f = m.function("f", mystr_ty);
        let x = f.local("x", mystr_ty);
        let y = f.local("y", mystr_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            // y = copy x — x is dead after (only use)
            b.assign(
                Place::local(y),
                Rvalue::Use(Operand::Place(Place::local(x)), UseMode::Copy),
            );
            b.ret(Operand::Place(Place::local(y)));
        }
        let mut module = m.finish();
        run_clone_elaboration(&mut module);

        let body = module.functions.last().unwrap().body.as_ref().unwrap();
        // No clone inserted — just rewritten to Move
        assert_eq!(body.blocks[0].stmts.len(), 1);
        match &body.blocks[0].stmts[0].kind {
            StatementKind::Assign {
                rvalue: Rvalue::Use(Operand::Place(p), UseMode::Move),
                ..
            } => {
                assert_eq!(p.root_local(), Some(x));
            }
            other => panic!("expected Use(x, Move), got {other:?}"),
        }
    }

    // ---- Call arg Copy on clone type ----

    #[test]
    fn call_arg_copy_clone_type_live() {
        let (mut m, cloneable, mystr_ty, callee_entity) = setup_clone_module();
        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", mystr_ty);
        let y = f.local("y", mystr_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            // call consume(copy x) — x is live (used again)
            b.call(
                None,
                Callee::direct(callee_entity),
                vec![(Operand::Place(Place::local(x)), ArgMode::Copy)],
            );
            // y = move x — last use
            b.assign(
                Place::local(y),
                Rvalue::Use(Operand::Place(Place::local(x)), UseMode::Move),
            );
            b.ret_unit();
        }
        let mut module = m.finish();
        run_clone_elaboration(&mut module);

        let body = module.functions.last().unwrap().body.as_ref().unwrap();
        // Should insert clone before the call, rewrite arg to move _clone
        assert_eq!(body.blocks[0].stmts.len(), 3); // clone_call, call(move _clone), y = move x

        // First: clone call
        assert!(matches!(
            &body.blocks[0].stmts[0].kind,
            StatementKind::Call {
                callee: Callee::Witness { protocol, .. },
                ..
            } if *protocol == cloneable
        ));

        // Second: original call with arg rewritten to Move
        match &body.blocks[0].stmts[1].kind {
            StatementKind::Call { args, .. } => {
                assert_eq!(args[0].1, ArgMode::Move);
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn call_arg_copy_clone_type_dead() {
        let (mut m, _, mystr_ty, callee_entity) = setup_clone_module();
        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", mystr_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            // call consume(copy x) — x is dead after (last use)
            b.call(
                None,
                Callee::direct(callee_entity),
                vec![(Operand::Place(Place::local(x)), ArgMode::Copy)],
            );
            b.ret_unit();
        }
        let mut module = m.finish();
        run_clone_elaboration(&mut module);

        let body = module.functions.last().unwrap().body.as_ref().unwrap();
        // No clone — just rewrite to Move
        assert_eq!(body.blocks[0].stmts.len(), 1);
        match &body.blocks[0].stmts[0].kind {
            StatementKind::Call { args, .. } => {
                assert_eq!(args[0].1, ArgMode::Move);
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    // ---- Compound rvalue with clone-typed operand ----

    #[test]
    fn construct_clone_operand() {
        let (mut m, cloneable, mystr_ty, _) = setup_clone_module();
        let wrapper_entity = m.fresh_entity();
        let wrapper_ty = m.named(wrapper_entity, vec![]);
        let mut f = m.function("f", wrapper_ty);
        let x = f.local("x", mystr_ty);
        let w = f.local("w", wrapper_ty);
        let y = f.local("y", mystr_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            // w = construct Wrapper { .0: copy x } — x live (used again)
            b.assign_construct(
                Place::local(w),
                wrapper_ty,
                vec![(FieldIdx::new(0), Operand::Place(Place::local(x)), UseMode::Copy)],
            );
            // y = move x
            b.assign(
                Place::local(y),
                Rvalue::Use(Operand::Place(Place::local(x)), UseMode::Move),
            );
            b.ret(Operand::Place(Place::local(w)));
        }
        let mut module = m.finish();
        run_clone_elaboration(&mut module);

        let body = module.functions.last().unwrap().body.as_ref().unwrap();
        // clone_call, construct(move _clone), y = move x, (return w in terminator)
        assert_eq!(body.blocks[0].stmts.len(), 3);
        assert!(matches!(
            &body.blocks[0].stmts[0].kind,
            StatementKind::Call {
                callee: Callee::Witness { protocol, .. },
                ..
            } if *protocol == cloneable
        ));
    }

    // ---- Affine type (Move only) left alone ----

    #[test]
    fn affine_type_move_unchanged() {
        let (mut m, _, _, _) = setup_clone_module();
        let affine_entity = m.fresh_entity();
        let affine_ty = m.named(affine_entity, vec![]);
        let mut def = StructDef::new(affine_entity, "FileHandle");
        def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            ..TypeInfo::default()
        };
        m.add_struct(def);

        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", affine_ty);
        let y = f.local("y", affine_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            // y = move x — affine, already Move, should not be touched
            b.assign(
                Place::local(y),
                Rvalue::Use(Operand::Place(Place::local(x)), UseMode::Move),
            );
            b.ret_unit();
        }
        let mut module = m.finish();
        let stmts_before = module.functions.last().unwrap().body.as_ref().unwrap().blocks[0]
            .stmts.len();
        run_clone_elaboration(&mut module);
        let stmts_after = module.functions.last().unwrap().body.as_ref().unwrap().blocks[0]
            .stmts.len();
        assert_eq!(stmts_before, stmts_after);

        // Verify it's still Move, not rewritten
        match &module.functions.last().unwrap().body.as_ref().unwrap().blocks[0].stmts[0].kind {
            StatementKind::Assign {
                rvalue: Rvalue::Use(Operand::Place(p), UseMode::Move),
                ..
            } => {
                assert_eq!(p.root_local(), Some(x));
            }
            other => panic!("expected Use(x, Move), got {other:?}"),
        }
    }

    // ---- No Cloneable protocol → pass is a no-op ----

    #[test]
    fn no_cloneable_protocol_is_noop() {
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
        // No protocols registered — pass should return immediately
        run_clone_elaboration(&mut module);
        let body = module.functions[0].body.as_ref().unwrap();
        assert_eq!(body.blocks[0].stmts.len(), 1);
    }
}
