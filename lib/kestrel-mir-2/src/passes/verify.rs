use crate::operand::{ArgMode, Operand, UseMode};
use crate::statement::{Rvalue, StatementKind};
use crate::terminator::TerminatorKind;
use crate::ty_query::{copy_behavior, needs_drop};
use crate::{BlockId, CopyBehavior, LocalId, MirModule};

use super::dataflow;
use super::init_state::{InitAnalysis, InitState};

// ---- Error model ----

#[derive(Debug, Clone, PartialEq)]
pub struct VerifyError {
    pub func_idx: usize,
    pub block: Option<BlockId>,
    pub stmt: Option<usize>,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct VerifyResult {
    pub errors: Vec<VerifyError>,
}

impl VerifyResult {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

// ---- Public API ----

pub fn verify(module: &MirModule) -> VerifyResult {
    let mut result = VerifyResult::default();
    verify_structure(module, &mut result);
    verify_ownership(module, &mut result);
    for err in &result.errors {
        kestrel_debug::ktrace!("verify", "ERROR in func[{}] '{}': {}",
            err.func_idx,
            module.functions[err.func_idx].name,
            err.message);
    }
    result
}

pub fn verify_structure(module: &MirModule, result: &mut VerifyResult) {
    for (fi, func) in module.functions.iter().enumerate() {
        let Some(body) = &func.body else { continue };

        // Param count matches locals
        if body.param_count > body.locals.len() {
            result.errors.push(VerifyError {
                func_idx: fi,
                block: None,
                stmt: None,
                message: format!(
                    "param_count ({}) exceeds locals count ({})",
                    body.param_count,
                    body.locals.len()
                ),
            });
        }

        // Param convention / local type consistency
        for (pi, param) in func.params.iter().enumerate() {
            if param.local.index() >= body.locals.len() {
                result.errors.push(VerifyError {
                    func_idx: fi,
                    block: None,
                    stmt: None,
                    message: format!("param {pi} references out-of-bounds local"),
                });
                continue;
            }
            // Local type must match param type — convention is metadata,
            // not encoded in the type (Option B: logical types throughout).
            let local_ty = body.locals[param.local.index()].ty;
            if local_ty != param.ty {
                result.errors.push(VerifyError {
                    func_idx: fi,
                    block: None,
                    stmt: None,
                    message: format!(
                        "param {pi} '{}': local type != param type",
                        param.name
                    ),
                });
            }
        }

        // Per-block structural checks
        for (bi, block) in body.blocks.iter().enumerate() {
            let block_id = BlockId::new(bi);

            for (si, stmt) in block.stmts.iter().enumerate() {
                verify_statement_structure(module, fi, block_id, si, &stmt.kind, body, result);
            }

            // Terminator successor blocks in bounds
            for succ in block.terminator.successors() {
                if succ.index() >= body.blocks.len() {
                    result.errors.push(VerifyError {
                        func_idx: fi,
                        block: Some(block_id),
                        stmt: None,
                        message: format!(
                            "terminator references out-of-bounds block bb{}",
                            succ.index()
                        ),
                    });
                }
            }
        }
    }
}

fn verify_statement_structure(
    _module: &MirModule,
    fi: usize,
    block: BlockId,
    si: usize,
    kind: &StatementKind,
    body: &crate::MirBody,
    result: &mut VerifyResult,
) {
    let err = |msg: String| VerifyError {
        func_idx: fi,
        block: Some(block),
        stmt: Some(si),
        message: msg,
    };

    match kind {
        StatementKind::Assign { dest, rvalue } => {
            verify_place_in_bounds(dest, body, fi, block, si, result);
            verify_rvalue_structure(rvalue, body, fi, block, si, result);
        }
        StatementKind::Call {
            dest,
            callee,
            args,
        } => {
            if let Some(d) = dest {
                verify_place_in_bounds(d, body, fi, block, si, result);
            }
            // Callee::Resolved is post-mono only
            if matches!(callee, crate::statement::Callee::Resolved(_)) {
                result.errors.push(err(
                    "Callee::Resolved in generic MIR (pre-monomorphization)".into(),
                ));
            }
            // ArgMode::Ref/RefMut must be on Place, not Const
            for (ai, (operand, mode)) in args.iter().enumerate() {
                if matches!(mode, ArgMode::Ref | ArgMode::RefMut)
                    && matches!(operand, Operand::Const(_))
                {
                    result.errors.push(err(format!(
                        "call arg {ai}: ArgMode::Ref/RefMut on Const operand"
                    )));
                }
            }
        }
        StatementKind::Uninit { dest } => {
            verify_place_in_bounds(dest, body, fi, block, si, result);
        }
        StatementKind::Drop { place } => {
            verify_place_in_bounds(place, body, fi, block, si, result);
        }
        StatementKind::DropIf { place, flag } => {
            verify_place_in_bounds(place, body, fi, block, si, result);
            if flag.index() >= body.locals.len() {
                result.errors.push(err(format!(
                    "DropIf flag references out-of-bounds local %{}",
                    flag.index()
                )));
            }
        }
        StatementKind::SetDropFlag { flag, .. } => {
            if flag.index() >= body.locals.len() {
                result.errors.push(err(format!(
                    "SetDropFlag references out-of-bounds local %{}",
                    flag.index()
                )));
            }
        }
        StatementKind::ScopeLive(local) => {
            if local.index() >= body.locals.len() {
                result.errors.push(err(format!(
                    "ScopeLive references out-of-bounds local %{}",
                    local.index()
                )));
            }
        }
    }
}

fn verify_place_in_bounds(
    place: &crate::Place,
    body: &crate::MirBody,
    fi: usize,
    block: BlockId,
    si: usize,
    result: &mut VerifyResult,
) {
    if let Some(local) = place.root_local()
        && local.index() >= body.locals.len()
    {
        result.errors.push(VerifyError {
            func_idx: fi,
            block: Some(block),
            stmt: Some(si),
            message: format!("place references out-of-bounds local %{}", local.index()),
        });
    }
}

fn verify_rvalue_structure(
    rvalue: &Rvalue,
    body: &crate::MirBody,
    fi: usize,
    block: BlockId,
    si: usize,
    result: &mut VerifyResult,
) {
    for op in rvalue.operands() {
        if let Operand::Place(p) = op {
            verify_place_in_bounds(p, body, fi, block, si, result);
        }
        // MonoFunctionRef is post-mono only
        if let Operand::Const(imm) = op
            && matches!(imm.kind, crate::ImmediateKind::MonoFunctionRef(_))
        {
            result.errors.push(VerifyError {
                func_idx: fi,
                block: Some(block),
                stmt: Some(si),
                message: "MonoFunctionRef in generic MIR (pre-monomorphization)".into(),
            });
        }
    }
    for p in rvalue.referenced_places() {
        verify_place_in_bounds(p, body, fi, block, si, result);
    }
}

// ---- Ownership checks ----

pub fn verify_ownership(module: &MirModule, result: &mut VerifyResult) {
    for (fi, func) in module.functions.iter().enumerate() {
        let Some(body) = &func.body else { continue };
        let where_clause = func.where_clause.as_ref();
        let is_drop_shim = matches!(func.kind, crate::item::function::FunctionKind::DropShim { .. });

        let cfg = dataflow::compute_cfg_info(body);
        let analysis = InitAnalysis::compute_with_cfg(body, &cfg);

        // Borrowed params and drop-shim self are not dropped by drop_elab
        let mut skip_drop_check: Vec<bool> = vec![false; body.locals.len()];
        for p in &func.params {
            if matches!(p.convention, crate::ty::ParamConvention::Borrow | crate::ty::ParamConvention::MutBorrow) {
                skip_drop_check[p.local.index()] = true;
            }
        }
        if is_drop_shim {
            if let Some(p) = func.params.first() {
                skip_drop_check[p.local.index()] = true;
            }
        }

        for (bi, block) in body.blocks.iter().enumerate() {
            let block_id = BlockId::new(bi);

            // Check statements
            for (si, stmt) in block.stmts.iter().enumerate() {
                verify_statement_ownership(
                    module, fi, block_id, si, &stmt.kind, body, &analysis, where_clause,
                    is_drop_shim, result,
                );
            }

            // At Return: every droppable local must be Dead or DropIf'd
            if let TerminatorKind::Return(ref operand) = block.terminator.kind {
                let returned_local = if let Operand::Place(p) = operand {
                    p.root_local()
                } else {
                    None
                };

                for (li, local) in body.locals.iter().enumerate() {
                    let local_id = LocalId::new(li);
                    if Some(local_id) == returned_local {
                        continue;
                    }
                    if skip_drop_check[li] {
                        continue;
                    }
                    if !needs_drop(&module.ty_arena, module, local.ty) {
                        continue;
                    }

                    let num_stmts = block.stmts.len();
                    let state = if num_stmts > 0 {
                        analysis.state_after(body, block_id, num_stmts - 1, local_id)
                    } else {
                        analysis.state_at_entry(block_id, local_id)
                    };

                    if state == InitState::Live {
                        // Check if there's a Drop for this local in this block
                        let has_drop = block.stmts.iter().any(|s| {
                            matches!(&s.kind, StatementKind::Drop { place } if place.root_local() == Some(local_id))
                                || matches!(&s.kind, StatementKind::DropIf { place, .. } if place.root_local() == Some(local_id))
                        });
                        if !has_drop {
                            result.errors.push(VerifyError {
                                func_idx: fi,
                                block: Some(block_id),
                                stmt: None,
                                message: format!(
                                    "droppable local %{li} '{}' is Live at Return but not dropped",
                                    local.name
                                ),
                            });
                        }
                    } else if state == InitState::Maybe {
                        let has_drop_if = block.stmts.iter().any(|s| {
                            matches!(&s.kind, StatementKind::DropIf { place, .. } if place.root_local() == Some(local_id))
                        });
                        if !has_drop_if {
                            result.errors.push(VerifyError {
                                func_idx: fi,
                                block: Some(block_id),
                                stmt: None,
                                message: format!(
                                    "droppable local %{li} '{}' is Maybe at Return but no DropIf",
                                    local.name
                                ),
                            });
                        }
                    }
                }
            }
        }
    }
}

fn verify_statement_ownership(
    module: &MirModule,
    fi: usize,
    block: BlockId,
    si: usize,
    kind: &StatementKind,
    body: &crate::MirBody,
    analysis: &InitAnalysis,
    where_clause: Option<&crate::item::function::WhereClause>,
    _is_drop_shim: bool,
    result: &mut VerifyResult,
) {
    let err = |msg: String| VerifyError {
        func_idx: fi,
        block: Some(block),
        stmt: Some(si),
        message: msg,
    };

    let state_before = |local: LocalId| -> InitState {
        if si > 0 {
            analysis.state_after(body, block, si - 1, local)
        } else {
            analysis.state_at_entry(block, local)
        }
    };

    match kind {
        StatementKind::Assign { rvalue, .. } => {
            for (op, mode) in rvalue.operands_with_mode() {
                let Operand::Place(p) = op else { continue };
                let Some(local) = p.root_local() else { continue };

                if mode == Some(UseMode::Move) && state_before(local) == InitState::Dead {
                    result.errors.push(err(format!(
                        "move of already-dead local %{} '{}'",
                        local.index(),
                        body.locals[local.index()].name
                    )));
                }

                if mode == Some(UseMode::Copy)
                    && place_copy_behavior(module, body, p, where_clause) == CopyBehavior::None
                {
                    result.errors.push(err(format!(
                        "copy of affine place rooted at %{} '{}'",
                        local.index(),
                        body.locals[local.index()].name
                    )));
                }
            }
        }
        StatementKind::Call { args, .. } => {
            for (ai, (operand, mode)) in args.iter().enumerate() {
                let Operand::Place(p) = operand else { continue };
                let Some(local) = p.root_local() else { continue };

                if *mode == ArgMode::Move && state_before(local) == InitState::Dead {
                    result.errors.push(err(format!(
                        "call arg {ai}: move of already-dead local %{}",
                        local.index()
                    )));
                }

                if *mode == ArgMode::Copy
                    && place_copy_behavior(module, body, p, where_clause) == CopyBehavior::None
                {
                    result.errors.push(err(format!(
                        "call arg {ai}: copy of affine place rooted at %{}",
                        local.index()
                    )));
                }
            }
        }
        StatementKind::Drop { place } | StatementKind::DropIf { place, .. } => {
            if let Some(local) = place.root_local()
                && state_before(local) == InitState::Dead
            {
                result.errors.push(err(format!(
                    "drop of already-dead local %{} '{}'",
                    local.index(),
                    body.locals[local.index()].name
                )));
            }
        }
        _ => {}
    }
}

fn place_copy_behavior(
    module: &MirModule,
    body: &crate::MirBody,
    place: &crate::Place,
    where_clause: Option<&crate::item::function::WhereClause>,
) -> CopyBehavior {
    let mut arena = module.ty_arena.clone();
    let ty = crate::place_ty::place_type(
        &mut arena,
        &module.structs,
        &module.enums,
        &module.statics,
        &body.locals,
        place,
    )
    .or_else(|| place.root_local().map(|local| body.locals[local.index()].ty))
    .unwrap_or_else(|| arena.error());
    copy_behavior(&arena, module, ty, where_clause)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::ModuleBuilder;
    use crate::immediate::Immediate;
    use crate::item::struct_def::{FieldDef, StructDef};
    use crate::item::{CopyBehavior, DropBehavior, TypeInfo};
    use crate::place::Place;
    use crate::statement::Rvalue;
    use crate::ty::ParamConvention;
    use crate::FieldIdx;

    fn setup_droppable(m: &mut ModuleBuilder) -> (kestrel_hecs::Entity, crate::TyId) {
        let i64_ty = m.i64();
        let entity = m.fresh_entity();
        m.register_name(entity, "Droppable");
        let ty = m.named(entity, vec![]);
        let mut def = StructDef::new(entity, "Droppable");
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
        (entity, ty)
    }

    // ---- Structural checks ----

    #[test]
    fn valid_function_passes() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let mut f = m.function("f", i64_ty);
        let x = f.param("x", i64_ty, ParamConvention::Consuming);
        {
            f.block().ret(Operand::Place(Place::local(x)));
        }
        let module = m.finish();
        let result = verify(&module);
        assert!(result.is_ok(), "errors: {:?}", result.errors);
    }

    #[test]
    fn param_count_exceeds_locals() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let mut f = m.function("f", i64_ty);
        {
            f.block().ret(Operand::Const(Immediate::i64(0)));
        }
        let mut module = m.finish();
        // Corrupt param_count
        module.functions[0].body.as_mut().unwrap().param_count = 99;
        let result = verify(&module);
        assert!(!result.is_ok());
        assert!(result.errors[0].message.contains("param_count"));
    }

    #[test]
    fn ref_on_const_operand() {
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let callee = m.fresh_entity();
        let mut f = m.function("f", unit_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.call(
                None,
                crate::Callee::direct(callee),
                vec![(Operand::Const(Immediate::i64(0)), ArgMode::Ref)],
            );
            b.ret_unit();
        }
        let module = m.finish();
        let result = verify(&module);
        assert!(!result.is_ok());
        assert!(result.errors[0].message.contains("Ref/RefMut on Const"));
    }

    #[test]
    fn borrow_param_type_consistency() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let _x = f.param("x", i64_ty, ParamConvention::Borrow);
        {
            f.block().ret_unit();
        }
        let module = m.finish();
        // This should pass — builder correctly wraps borrow params in Pointer
        let result = verify(&module);
        assert!(result.is_ok(), "errors: {:?}", result.errors);
    }

    // ---- Ownership: droppable local at Return ----

    #[test]
    fn live_droppable_not_dropped_is_error() {
        let mut m = ModuleBuilder::new("test");
        let (_, d_ty) = setup_droppable(&mut m);
        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", d_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            // No Drop inserted — verifier should catch this
            b.ret_unit();
        }
        let module = m.finish();
        let result = verify(&module);
        assert!(!result.is_ok());
        assert!(result.errors[0].message.contains("Live at Return but not dropped"));
    }

    #[test]
    fn live_droppable_with_drop_passes() {
        let mut m = ModuleBuilder::new("test");
        let (_, d_ty) = setup_droppable(&mut m);
        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", d_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.drop(Place::local(x));
            b.ret_unit();
        }
        let module = m.finish();
        let result = verify(&module);
        assert!(result.is_ok(), "errors: {:?}", result.errors);
    }

    #[test]
    fn returned_local_not_flagged() {
        let mut m = ModuleBuilder::new("test");
        let (_, d_ty) = setup_droppable(&mut m);
        let mut f = m.function("f", d_ty);
        let x = f.local("x", d_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.ret(Operand::Place(Place::local(x)));
        }
        let module = m.finish();
        let result = verify(&module);
        assert!(result.is_ok(), "errors: {:?}", result.errors);
    }

    // ---- Ownership: move of dead place ----

    #[test]
    fn move_of_dead_local_is_error() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", i64_ty);
        let y = f.local("y", i64_ty);
        let z = f.local("z", i64_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.use_move(Place::local(y), Place::local(x)); // x now Dead
            b.use_move(Place::local(z), Place::local(x)); // move of dead x!
            b.ret_unit();
        }
        let module = m.finish();
        let result = verify(&module);
        assert!(!result.is_ok());
        assert!(result.errors[0].message.contains("move of already-dead"));
    }

    // ---- Ownership: copy of affine type ----

    #[test]
    fn copy_of_affine_type_is_error() {
        let mut m = ModuleBuilder::new("test");
        let (_, d_ty) = setup_droppable(&mut m);
        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", d_ty);
        let y = f.local("y", d_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            // Copy of affine type — should be an error
            b.assign(
                Place::local(y),
                Rvalue::Use(Operand::Place(Place::local(x)), UseMode::Copy),
            );
            b.ret_unit();
        }
        let module = m.finish();
        let result = verify(&module);
        assert!(!result.is_ok());
        assert!(result.errors[0].message.contains("copy of affine"));
    }

    // ---- Ownership: projected copy of affine field ----

    #[test]
    fn projected_copy_of_affine_field_is_error() {
        let mut m = ModuleBuilder::new("test");
        let (_inner_entity, inner_ty) = setup_droppable(&mut m);

        let outer_entity = m.fresh_entity();
        m.register_name(outer_entity, "Outer");
        let outer_ty = m.named(outer_entity, vec![]);
        let mut outer_def = StructDef::new(outer_entity, "Outer");
        outer_def.add_field(FieldDef::new("inner", inner_ty));
        outer_def.type_info = TypeInfo {
            copy: CopyBehavior::Bitwise,
            drop: DropBehavior::None,
            layout: None,
        };
        m.add_struct(outer_def);

        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", outer_ty);
        let y = f.local("y", inner_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            // The root type is marked bitwise-copyable, but the projected
            // field type is affine.
            b.assign(
                Place::local(y),
                Rvalue::Use(
                    Operand::Place(Place::local(x).field(FieldIdx::new(0))),
                    UseMode::Copy,
                ),
            );
            b.ret_unit();
        }
        let module = m.finish();
        let result = verify(&module);
        assert!(
            result
                .errors
                .iter()
                .any(|err| err.message.contains("copy of affine")),
            "errors: {:?}",
            result.errors
        );
    }

    // ---- Panic paths: no ownership checks ----

    #[test]
    fn panic_path_not_checked() {
        let mut m = ModuleBuilder::new("test");
        let (_, d_ty) = setup_droppable(&mut m);
        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", d_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            // Panic — no drop needed, verifier shouldn't complain
            b.panic("abort");
        }
        let module = m.finish();
        let result = verify(&module);
        assert!(result.is_ok(), "errors: {:?}", result.errors);
    }

    // ---- Collects multiple errors ----

    #[test]
    fn collects_all_errors() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let unit_ty = m.unit();
        let callee = m.fresh_entity();
        let mut f = m.function("f", unit_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            // Two errors: Ref on Const
            b.call(
                None,
                crate::Callee::direct(callee),
                vec![
                    (Operand::Const(Immediate::i64(0)), ArgMode::Ref),
                    (Operand::Const(Immediate::i64(1)), ArgMode::RefMut),
                ],
            );
            b.ret_unit();
        }
        let module = m.finish();
        let result = verify(&module);
        assert_eq!(result.errors.len(), 2);
    }

    // ---- Pipeline integration: after drop_elab, verifier passes ----

    #[test]
    fn post_drop_elab_passes_verification() {
        let mut m = ModuleBuilder::new("test");
        let (_, d_ty) = setup_droppable(&mut m);
        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", d_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.ret_unit();
        }
        let mut module = m.finish();

        // Before drop elab: should fail (no drops)
        let result = verify(&module);
        assert!(!result.is_ok());

        // Run drop elab
        crate::passes::drop_elab::run_drop_elaboration(&mut module);

        // After drop elab: should pass
        let result = verify(&module);
        assert!(result.is_ok(), "errors: {:?}", result.errors);
    }
}
