//! Pattern matching / match lowering.
//!
//! Compiles match expressions to MIR via kestrel-pattern-matching's
//! decision tree, then emits the tree as basic blocks with switch/branch
//! terminators.

use kestrel_hecs::Entity;
use kestrel_hir::body::{HirExprId, HirMatchArm};
use kestrel_mir_2::{
    ArgMode, Callee, FieldIdx, Immediate, MirTy, Operand, Place, Rvalue, SwitchCase,
    TyId, UseMode, VariantIdx, WitnessMethodKey,
};
use kestrel_pattern_matching::constructor::Constructor;
use kestrel_pattern_matching::decision_tree::{Binding, DecisionTree, PathElement};
use kestrel_type_infer::result::ResolvedTy;

use super::BodyCtx;
use crate::ty::lower_resolved_ty;

impl BodyCtx<'_, '_> {
    pub fn lower_match(
        &mut self,
        expr_id: HirExprId,
        scrutinee_expr: HirExprId,
        arms: &[HirMatchArm],
    ) -> Operand {
        let result_ty = self.resolve_expr_type(expr_id);
        let scrutinee_ty = self.resolve_expr_resolved_ty(scrutinee_expr);

        // Lower scrutinee to a place
        let scrutinee_val = self.lower_expr(scrutinee_expr);
        let scrutinee_mir_ty = self.resolve_expr_type(scrutinee_expr);
        let scrutinee_place = self.operand_to_place(scrutinee_val, scrutinee_mir_ty);

        let result_local = self.fresh_temp(result_ty);
        let join_block = self.new_block();

        let tree = kestrel_pattern_matching::compile_decision_tree(
            &self.hir,
            &self.ctx.query,
            self.ctx.root,
            &scrutinee_ty,
            arms,
        );

        self.emit_decision_tree(&tree, &scrutinee_place, arms, result_local, result_ty, join_block);

        self.switch_to(join_block);
        Operand::Place(Place::local(result_local))
    }

    fn emit_decision_tree(
        &mut self,
        tree: &DecisionTree,
        scrutinee: &Place,
        arms: &[HirMatchArm],
        result_local: kestrel_mir_2::LocalId,
        result_ty: TyId,
        join_block: kestrel_mir_2::BlockId,
    ) {
        match tree {
            DecisionTree::Switch {
                path,
                ty,
                cases,
                default,
            } => {
                let test_place = apply_access_path(self, scrutinee.clone(), path);

                // Single case — no switch needed
                if cases.len() == 1 && default.is_none() {
                    let (_, subtree) = &cases[0];
                    self.emit_decision_tree(subtree, scrutinee, arms, result_local, result_ty, join_block);
                    return;
                }

                // Boolean branch optimization
                if cases.len() == 2
                    && matches!(&cases[0].0, Constructor::True)
                    && matches!(&cases[1].0, Constructor::False)
                {
                    let true_block = self.new_block();
                    let false_block = self.new_block();
                    self.emit_branch(Operand::Place(test_place), true_block, false_block);

                    self.switch_to(true_block);
                    self.emit_decision_tree(&cases[0].1, scrutinee, arms, result_local, result_ty, join_block);

                    self.switch_to(false_block);
                    self.emit_decision_tree(&cases[1].1, scrutinee, arms, result_local, result_ty, join_block);
                    return;
                }

                // String literal chain — no native string switch
                if cases
                    .iter()
                    .any(|(c, _)| matches!(c, Constructor::StringLiteral(_)))
                {
                    let test_ty = lower_resolved_ty(self.ctx, ty);
                    for (ctor, subtree) in cases.iter() {
                        let Constructor::StringLiteral(lit) = ctor else {
                            continue;
                        };
                        let cmp = self.emit_string_match_test(&test_place, test_ty, lit);
                        let hit_block = self.new_block();
                        let miss_block = self.new_block();
                        self.emit_branch(cmp, hit_block, miss_block);

                        self.switch_to(hit_block);
                        self.emit_decision_tree(subtree, scrutinee, arms, result_local, result_ty, join_block);

                        self.switch_to(miss_block);
                    }
                    if let Some(def_tree) = default {
                        self.emit_decision_tree(def_tree, scrutinee, arms, result_local, result_ty, join_block);
                    } else {
                        self.emit_panic("match failure: non-exhaustive string patterns");
                    }
                    return;
                }

                // General switch
                let mut case_blocks: Vec<(SwitchCase, kestrel_mir_2::BlockId)> =
                    Vec::with_capacity(cases.len());
                for (ctor, _) in cases.iter() {
                    let case = constructor_to_switch_case(self, ctor);
                    let block = self.new_block();
                    case_blocks.push((case, block));
                }

                let default_block = default.as_ref().map(|_| self.new_block());

                let mut switch_cases = case_blocks.clone();
                if let Some(def_block) = default_block {
                    switch_cases.push((SwitchCase::Wildcard, def_block));
                }

                self.emit_switch(test_place, switch_cases);

                for ((_, subtree), (_, block_id)) in cases.iter().zip(case_blocks.iter()) {
                    self.switch_to(*block_id);
                    self.emit_decision_tree(subtree, scrutinee, arms, result_local, result_ty, join_block);
                }

                if let (Some(def_tree), Some(def_block)) = (default, default_block) {
                    self.switch_to(def_block);
                    self.emit_decision_tree(def_tree, scrutinee, arms, result_local, result_ty, join_block);
                }
            }

            DecisionTree::Success {
                arm_index,
                bindings,
            } => {
                self.emit_bindings(bindings, scrutinee);
                if let Some(arm) = arms.get(*arm_index) {
                    let body_val = self.lower_expr(arm.body);
                    if !self.is_terminated() {
                        self.emit_value_transfer(
                            Place::local(result_local),
                            body_val,
                            result_ty,
                        );
                        self.emit_jump(join_block);
                    }
                }
            }

            DecisionTree::Guard {
                arm_index,
                bindings,
                success,
                failure,
            } => {
                self.emit_bindings(bindings, scrutinee);
                if let Some(arm) = arms.get(*arm_index) {
                    if let Some(guard_expr) = arm.guard {
                        let guard_val = self.lower_expr(guard_expr);
                        let success_block = self.new_block();
                        let failure_block = self.new_block();
                        self.emit_branch(guard_val, success_block, failure_block);

                        self.switch_to(success_block);
                        self.emit_decision_tree(success, scrutinee, arms, result_local, result_ty, join_block);

                        self.switch_to(failure_block);
                        self.emit_decision_tree(failure, scrutinee, arms, result_local, result_ty, join_block);
                    } else {
                        self.emit_decision_tree(success, scrutinee, arms, result_local, result_ty, join_block);
                    }
                }
            }

            DecisionTree::Failure => {
                self.emit_panic("match failure: non-exhaustive patterns");
            }
        }
    }

    fn emit_bindings(&mut self, bindings: &[Binding], scrutinee: &Place) {
        // Check if scrutinee is owned (not a reference parameter)
        let scrutinee_is_owned = scrutinee
            .root_local()
            .map(|root| {
                let idx = root.index();
                if idx < self.body.param_count {
                    let func = &self.ctx.module.functions[self.func_idx];
                    func.params
                        .get(idx)
                        .map(|p| !matches!(
                            p.convention,
                            kestrel_mir_2::ParamConvention::Borrow
                                | kestrel_mir_2::ParamConvention::MutBorrow
                        ))
                        .unwrap_or(true)
                } else {
                    true
                }
            })
            .unwrap_or(false);

        for binding in bindings {
            let mir_local = self.map_local(binding.local_id);
            let source = apply_access_path(self, scrutinee.clone(), &binding.path);
            let local_ty = self.body.local(mir_local).ty;

            // Move from owned scrutinee for non-copyable payloads
            let use_move = !self.is_copy_type(local_ty)
                && !binding.path.is_empty()
                && scrutinee_is_owned;

            let mode = if use_move { UseMode::Move } else { UseMode::Copy };
            self.emit_assign(
                Place::local(mir_local),
                Rvalue::Use(Operand::Place(source), mode),
            );
        }
    }

    fn resolve_expr_resolved_ty(&self, expr_id: HirExprId) -> ResolvedTy {
        if let Some(typed) = self.typed.as_ref()
            && let Some(resolved) = typed.expr_types.get(&expr_id)
        {
            return resolved.clone();
        }
        ResolvedTy::Error
    }

    /// Emit a `Matchable.matches(other:)` call for string matching.
    fn emit_string_match_test(
        &mut self,
        scrutinee_place: &Place,
        string_ty: TyId,
        literal: &str,
    ) -> Operand {
        // Materialize the string literal
        let lit_op = if let MirTy::Named { entity, .. } = self.ctx.module.ty_arena.get(string_ty) {
            let entity = *entity;
            if let Some(init) = self.find_string_literal_init(entity) {
                let ptr = Operand::Const(Immediate::string_pointer(literal.to_string()));
                let len = Operand::Const(Immediate::i64(literal.len() as i128));
                self.ctx.register_name(init);
                let callee = Callee::direct_with_args(init, vec![], None);
                self.emit_init_literal_call(
                    callee,
                    vec![(ptr, ArgMode::Copy), (len, ArgMode::Copy)],
                    string_ty,
                )
            } else {
                Operand::Const(Immediate::string(literal.to_string()))
            }
        } else {
            Operand::Const(Immediate::string(literal.to_string()))
        };

        // Call Matchable.matches(other:)
        let proto_entity = self.ctx.query.query(kestrel_name_res::ResolveBuiltin {
            builtin: kestrel_hir::Builtin::Matchable,
            root: self.ctx.root,
        });
        let Some(proto) = proto_entity else {
            return Operand::Const(Immediate::bool(false));
        };
        self.ctx.register_name(proto);

        let bool_ty = self.ctx.module.ty_arena.bool();
        let method_key = WitnessMethodKey::new("matches", vec![None]);
        let callee = Callee::Witness {
            protocol: proto,
            method: method_key,
            self_type: string_ty,
            method_type_args: vec![],
        };

        let dest = self.fresh_temp(bool_ty);
        self.emit_call(
            Some(Place::local(dest)),
            callee,
            vec![
                (Operand::Place(scrutinee_place.clone()), ArgMode::Ref),
                (lit_op, ArgMode::Ref),
            ],
        );
        Operand::Place(Place::local(dest))
    }
}

/// Apply an access path to a place (field, index, downcast projections).
fn apply_access_path(bctx: &BodyCtx, mut place: Place, path: &[PathElement]) -> Place {
    for elem in path {
        place = match elem {
            PathElement::Field(name) => {
                // Resolve field name to FieldIdx via struct entity
                let struct_entity = find_place_struct_entity(bctx, &place);
                if let Some(se) = struct_entity {
                    if let Some(idx) = bctx.ctx.resolve_field_idx(se, name) {
                        place.field(idx)
                    } else {
                        place.field(FieldIdx::new(0))
                    }
                } else {
                    place.field(FieldIdx::new(0))
                }
            }
            PathElement::Index(i) => place.tuple_index(*i as u32),
            PathElement::Downcast(variant_name) => {
                // Resolve variant name to VariantIdx
                let enum_entity = find_place_enum_entity(bctx, &place);
                if let Some(ee) = enum_entity {
                    if let Some(idx) = bctx.ctx.resolve_variant_idx(ee, variant_name) {
                        place.downcast(idx)
                    } else {
                        place.downcast(VariantIdx::new(0))
                    }
                } else {
                    place.downcast(VariantIdx::new(0))
                }
            }
            PathElement::IndexFromEnd(_) | PathElement::RestSlice { .. } => place,
        };
    }
    place
}

/// Try to find the struct entity for a place's type (for FieldIdx resolution).
fn find_place_struct_entity(bctx: &BodyCtx, place: &Place) -> Option<Entity> {
    // Only works for bare locals — projected places would need type tracking
    let local = place.as_local()?;
    let ty = bctx.body.local(local).ty;
    match bctx.ctx.module.ty_arena.get(ty) {
        MirTy::Named { entity, .. } => Some(*entity),
        _ => None,
    }
}

/// Try to find the enum entity for a place's type (for VariantIdx resolution).
fn find_place_enum_entity(bctx: &BodyCtx, place: &Place) -> Option<Entity> {
    let local = place.as_local()?;
    let ty = bctx.body.local(local).ty;
    match bctx.ctx.module.ty_arena.get(ty) {
        MirTy::Named { entity, .. } => Some(*entity),
        _ => None,
    }
}

/// Map a decision-tree Constructor to a MIR SwitchCase.
fn constructor_to_switch_case(bctx: &mut BodyCtx, ctor: &Constructor) -> SwitchCase {
    match ctor {
        Constructor::True => SwitchCase::Bool(true),
        Constructor::False => SwitchCase::Bool(false),
        Constructor::Variant { entity, .. } => {
            bctx.ctx.register_name(*entity);
            let case_name = bctx
                .ctx
                .world
                .get::<kestrel_ast_builder::Name>(*entity)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| panic!("ICE: enum case {:?} has no Name", entity));
            let enum_entity = bctx.ctx.world.parent_of(*entity)
                .unwrap_or_else(|| panic!("ICE: enum case {:?} has no parent", entity));
            let idx = bctx.ctx.resolve_variant_idx(enum_entity, &case_name)
                .unwrap_or_else(|| panic!(
                    "ICE: variant '{}' not found in enum {:?}", case_name, enum_entity
                ));
            SwitchCase::Variant(idx)
        }
        Constructor::IntLiteral(v) => SwitchCase::IntLiteral(*v),
        Constructor::IntRange { start, end } => {
            SwitchCase::IntRange {
                start: start.unwrap_or(i64::MIN),
                end: end.unwrap_or(i64::MAX),
            }
        }
        Constructor::CharLiteral(c) => SwitchCase::CharLiteral(*c as u32),
        Constructor::CharRange { start, end } => {
            SwitchCase::CharRange {
                start: start.map(|c| c as u32).unwrap_or(0),
                end: end.map(|c| c as u32).unwrap_or(u32::MAX),
            }
        }
        Constructor::Wildcard
        | Constructor::Tuple { .. }
        | Constructor::Struct { .. }
        | Constructor::Unit
        | Constructor::Array { .. }
        | Constructor::NonExhaustive
        | Constructor::Missing
        | Constructor::StringLiteral(_) => SwitchCase::Wildcard,
    }
}
