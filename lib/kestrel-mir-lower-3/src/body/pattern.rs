//! Pattern matching / match lowering (OSSA).
//!
//! Compiles match expressions to OSSA via kestrel-pattern-matching's
//! decision tree, then emits the tree as basic blocks with switch/branch
//! terminators. Unlike MIR-2 which uses Place projections to navigate
//! the scrutinee, OSSA emits SSA extraction instructions (StructExtract,
//! TupleExtract, EnumPayload, Discriminant) to destructure values.

use kestrel_hecs::Entity;
use kestrel_hir::body::{HirExprId, HirMatchArm};
use kestrel_mir_3::callee::Callee;
use kestrel_mir_3::item::witness::WitnessMethodKey;
use kestrel_mir_3::terminator::{SwitchArm, SwitchCase};
use kestrel_mir_3::{
    FieldIdx, Immediate, MirTy, Ownership, ParamConvention, TyId, ValueId, VariantIdx,
};
use kestrel_pattern_matching::constructor::Constructor;
use kestrel_pattern_matching::decision_tree::{Binding, DecisionTree, PathElement};
use kestrel_type_infer::result::ResolvedTy;

use super::OssaBodyCtx;
use crate::ty::lower_resolved_ty;

impl OssaBodyCtx<'_, '_> {
    /// Lower a match expression.
    ///
    /// The result is threaded via a block parameter on the merge block
    /// (same pattern as `lower_if`). Each arm jumps to the merge block
    /// passing its result value.
    pub fn lower_match(
        &mut self,
        expr_id: HirExprId,
        scrutinee_expr: HirExprId,
        arms: &[HirMatchArm],
    ) -> ValueId {
        let result_ty = self.resolve_expr_type(expr_id);
        let scrutinee_resolved_ty = self.resolve_expr_resolved_ty(scrutinee_expr);

        let scrutinee_val = self.lower_expr(scrutinee_expr);
        let scrutinee_ty = self.resolve_expr_type(scrutinee_expr);

        // Snapshot live @owned values after scrutinee eval
        let live_before: Vec<(ValueId, TyId)> = self.all_live_owned();
        let live_vals: Vec<ValueId> = live_before.iter().map(|&(v, _)| v).collect();
        let arm_descs: Vec<(TyId, Ownership)> = live_before
            .iter()
            .map(|&(_, ty)| (ty, Ownership::Owned))
            .collect();

        // Merge block: [result, ...live_owned]
        let ownership = self.ownership_for(result_ty);
        let mut merge_descs: Vec<(TyId, Ownership)> = vec![(result_ty, ownership)];
        merge_descs.extend(&arm_descs);
        let (join_block, join_params) = self.new_block_with_params(&merge_descs);
        let result_param = join_params[0];

        let tree = kestrel_pattern_matching::compile_decision_tree(
            &self.hir,
            &self.ctx.query,
            self.ctx.root,
            &scrutinee_resolved_ty,
            arms,
        );

        let snapshot = self.snapshot_scope();

        self.emit_decision_tree_threaded(
            &tree,
            scrutinee_val,
            scrutinee_ty,
            arms,
            result_ty,
            join_block,
            &live_vals,
            &arm_descs,
            &snapshot,
        );

        // Continue from merge block
        self.restore_scope(&snapshot);
        self.switch_to(join_block);
        let merge_live = &join_params[1..];
        self.rebind_scope_values(&live_vals, merge_live);
        if ownership == Ownership::Owned {
            self.track_owned(result_param);
        }
        result_param
    }

    /// Recursively emit a decision tree, threading live @owned values
    /// through every branch/switch as block parameters.
    fn emit_decision_tree_threaded(
        &mut self,
        tree: &DecisionTree,
        scrutinee: ValueId,
        scrutinee_ty: TyId,
        arms: &[HirMatchArm],
        result_ty: TyId,
        join_block: kestrel_mir_3::BlockId,
        live_vals: &[ValueId],
        arm_descs: &[(TyId, Ownership)],
        snapshot: &super::ScopeSnapshot,
    ) {
        match tree {
            DecisionTree::Switch {
                path,
                ty,
                cases,
                default,
            } => {
                let (test_val, _test_ty) =
                    self.apply_access_path(scrutinee, scrutinee_ty, path);

                // Single case — no switch needed, just fall through
                if cases.len() == 1 && default.is_none() {
                    let (_, subtree) = &cases[0];
                    self.emit_decision_tree_threaded(
                        subtree, scrutinee, scrutinee_ty, arms, result_ty, join_block,
                        live_vals, arm_descs, snapshot,
                    );
                    return;
                }

                // Snapshot before branching so each arm starts fresh
                let branch_snapshot = self.snapshot_scope();
                let current_live: Vec<ValueId> = self.collect_outer_live(self.scope_stack.len());
                // Compute descriptors from current live set (may differ from
                // arm_descs if extractions consumed values since match entry)
                let local_descs: Vec<(TyId, Ownership)> = current_live
                    .iter()
                    .map(|&v| (self.body.value(v).ty, Ownership::Owned))
                    .collect();

                // Boolean branch optimization
                if cases.len() == 2
                    && matches!(&cases[0].0, Constructor::True)
                    && matches!(&cases[1].0, Constructor::False)
                {
                    let (true_block, true_params) = self.new_block_with_params(&local_descs);
                    let (false_block, false_params) = self.new_block_with_params(&local_descs);
                    self.emit_branch(
                        test_val,
                        true_block, current_live.clone(),
                        false_block, current_live.clone(),
                    );

                    self.switch_to(true_block);
                    self.rebind_scope_values(&current_live, &true_params);
                    self.emit_decision_tree_threaded(
                        &cases[0].1, scrutinee, scrutinee_ty, arms, result_ty, join_block,
                        live_vals, arm_descs, snapshot,
                    );

                    self.restore_scope(&branch_snapshot);
                    self.switch_to(false_block);
                    self.rebind_scope_values(&current_live, &false_params);
                    self.emit_decision_tree_threaded(
                        &cases[1].1, scrutinee, scrutinee_ty, arms, result_ty, join_block,
                        live_vals, arm_descs, snapshot,
                    );
                    return;
                }

                // String literal chain
                if cases
                    .iter()
                    .any(|(c, _)| matches!(c, Constructor::StringLiteral(_)))
                {
                    let test_mir_ty = lower_resolved_ty(self.ctx, ty);
                    for (ctor, subtree) in cases.iter() {
                        let Constructor::StringLiteral(lit) = ctor else {
                            continue;
                        };
                        let cmp = self.emit_string_match_test(test_val, test_mir_ty, lit);
                        let str_snapshot = self.snapshot_scope();
                        let str_live: Vec<ValueId> = self.collect_outer_live(self.scope_stack.len());
                        let str_descs: Vec<(TyId, Ownership)> = str_live
                            .iter()
                            .map(|&v| (self.body.value(v).ty, Ownership::Owned))
                            .collect();
                        let (hit_block, hit_params) = self.new_block_with_params(&str_descs);
                        let (miss_block, miss_params) = self.new_block_with_params(&str_descs);
                        self.emit_branch(
                            cmp,
                            hit_block, str_live.clone(),
                            miss_block, str_live.clone(),
                        );

                        self.switch_to(hit_block);
                        self.rebind_scope_values(&str_live, &hit_params);
                        self.emit_decision_tree_threaded(
                            subtree, scrutinee, scrutinee_ty, arms, result_ty, join_block,
                            live_vals, arm_descs, snapshot,
                        );

                        self.restore_scope(&str_snapshot);
                        self.switch_to(miss_block);
                        self.rebind_scope_values(&str_live, &miss_params);
                    }
                    if let Some(def_tree) = default {
                        self.emit_decision_tree_threaded(
                            def_tree, scrutinee, scrutinee_ty, arms, result_ty, join_block,
                            live_vals, arm_descs, snapshot,
                        );
                    } else {
                        self.emit_panic("match failure: non-exhaustive string patterns");
                    }
                    return;
                }

                // General switch — create case blocks with live-value params
                let discriminant = self.emit_discriminant(test_val);

                let mut switch_arms: Vec<SwitchArm> = Vec::with_capacity(cases.len());
                let mut case_blocks_with_params = Vec::with_capacity(cases.len());
                for (ctor, _) in cases.iter() {
                    let pattern = constructor_to_switch_case(self, ctor);
                    let (block, params) = self.new_block_with_params(&local_descs);
                    switch_arms.push(SwitchArm {
                        pattern,
                        target: block,
                        args: current_live.clone(),
                    });
                    case_blocks_with_params.push((block, params));
                }

                let default_with_params = default.as_ref().map(|_| {
                    let (block, params) = self.new_block_with_params(&local_descs);
                    switch_arms.push(SwitchArm {
                        pattern: SwitchCase::Wildcard,
                        target: block,
                        args: current_live.clone(),
                    });
                    (block, params)
                });

                self.emit_switch(discriminant, switch_arms);

                // Emit each case's subtree
                for (i, ((_, subtree), (block_id, params))) in
                    cases.iter().zip(case_blocks_with_params.iter()).enumerate()
                {
                    if i > 0 {
                        self.restore_scope(&branch_snapshot);
                    }
                    self.switch_to(*block_id);
                    self.rebind_scope_values(&current_live, params);
                    self.emit_decision_tree_threaded(
                        subtree, scrutinee, scrutinee_ty, arms, result_ty, join_block,
                        live_vals, arm_descs, snapshot,
                    );
                }

                if let (Some(def_tree), Some((def_block, def_params))) =
                    (default, default_with_params)
                {
                    self.restore_scope(&branch_snapshot);
                    self.switch_to(def_block);
                    self.rebind_scope_values(&current_live, &def_params);
                    self.emit_decision_tree_threaded(
                        def_tree, scrutinee, scrutinee_ty, arms, result_ty, join_block,
                        live_vals, arm_descs, snapshot,
                    );
                }
            }

            DecisionTree::Success {
                arm_index,
                bindings,
            } => {
                self.push_scope();
                self.emit_bindings(bindings, scrutinee, scrutinee_ty);
                if let Some(arm) = arms.get(*arm_index) {
                    let body_val = self.lower_expr(arm.body);
                    if !self.is_terminated() {
                        self.destroy_scope_except(&[body_val]);
                        let mut args = vec![body_val];
                        args.extend(self.collect_outer_live(snapshot.scopes.len()));
                        self.emit_jump(join_block, args);
                    }
                }
                self.pop_scope();
            }

            DecisionTree::Guard {
                arm_index,
                bindings,
                success,
                failure,
            } => {
                self.push_scope();
                self.emit_bindings(bindings, scrutinee, scrutinee_ty);
                if let Some(arm) = arms.get(*arm_index) {
                    if let Some(guard_expr) = arm.guard {
                        let guard_val = self.lower_expr(guard_expr);
                        let guard_snapshot = self.snapshot_scope();
                        let guard_live: Vec<ValueId> = self.collect_outer_live(self.scope_stack.len());
                        let guard_descs: Vec<(TyId, Ownership)> = guard_live
                            .iter()
                            .map(|&v| (self.body.value(v).ty, Ownership::Owned))
                            .collect();
                        let (success_block, success_params) = self.new_block_with_params(&guard_descs);
                        let (failure_block, failure_params) = self.new_block_with_params(&guard_descs);
                        self.emit_branch(
                            guard_val,
                            success_block, guard_live.clone(),
                            failure_block, guard_live.clone(),
                        );

                        self.switch_to(success_block);
                        self.rebind_scope_values(&guard_live, &success_params);
                        self.emit_decision_tree_threaded(
                            success, scrutinee, scrutinee_ty, arms, result_ty, join_block,
                            live_vals, arm_descs, snapshot,
                        );

                        self.restore_scope(&guard_snapshot);
                        self.switch_to(failure_block);
                        self.rebind_scope_values(&guard_live, &failure_params);
                        self.pop_scope();
                        self.emit_decision_tree_threaded(
                            failure, scrutinee, scrutinee_ty, arms, result_ty, join_block,
                            live_vals, arm_descs, snapshot,
                        );
                        return;
                    } else {
                        self.emit_decision_tree_threaded(
                            success, scrutinee, scrutinee_ty, arms, result_ty, join_block,
                            live_vals, arm_descs, snapshot,
                        );
                    }
                }
                self.pop_scope();
            }

            DecisionTree::Failure => {
                self.emit_panic("match failure: non-exhaustive patterns");
            }
        }
    }

    /// Emit SSA bindings for a matched arm.
    ///
    /// Each binding's access path is applied to the scrutinee to extract
    /// the bound sub-value via SSA instructions. For @owned types, we
    /// copy the extracted value so the binding has its own lifetime.
    fn emit_bindings(
        &mut self,
        bindings: &[Binding],
        scrutinee: ValueId,
        scrutinee_ty: TyId,
    ) {
        for binding in bindings {
            let hir_local = binding.local_id;
            let (extracted, _) =
                self.apply_access_path(scrutinee, scrutinee_ty, &binding.path);

            // Lower the binding's type to get the MIR type
            let local_ty = lower_resolved_ty(self.ctx, &binding.ty);
            let ownership = self.ownership_for(local_ty);

            // For @owned types, copy the extracted value so the binding
            // owns its own value. For @none types, use directly.
            let bound_val = if ownership == Ownership::Owned {
                self.emit_copy_value(extracted)
            } else {
                extracted
            };

            self.local_map.insert(hir_local, bound_val);
        }
    }

    /// Resolve the `ResolvedTy` for an HIR expression.
    ///
    /// Used to get the scrutinee type needed by
    /// `kestrel_pattern_matching::compile_decision_tree`.
    fn resolve_expr_resolved_ty(&self, expr_id: HirExprId) -> ResolvedTy {
        if let Some(typed) = self.typed.as_ref()
            && let Some(resolved) = typed.expr_types.get(&expr_id)
        {
            return resolved.clone();
        }
        ResolvedTy::Error
    }

    /// Apply an access path to a value, emitting SSA extraction instructions.
    ///
    /// Walks the path elements and emits `StructExtract`, `TupleExtract`,
    /// or `EnumPayload` instructions as needed. Returns the extracted value
    /// and its type.
    ///
    /// For Downcast elements (enum variant navigation), the actual extraction
    /// happens on the subsequent Field access via `EnumPayload`. A bare
    /// Downcast without a following Field just tracks the variant context.
    fn apply_access_path(
        &mut self,
        value: ValueId,
        value_ty: TyId,
        path: &[PathElement],
    ) -> (ValueId, TyId) {
        let mut current = value;
        let mut current_ty = value_ty;
        // Track pending downcast: when we see Downcast(name), record the
        // variant so the next Field element uses EnumPayload instead of
        // StructExtract.
        let mut pending_downcast: Option<(String, VariantIdx)> = None;

        for elem in path {
            match elem {
                PathElement::Field(name) => {
                    if let Some((_, variant_idx)) = pending_downcast.take() {
                        // After a Downcast, Field accesses extract from
                        // the enum variant's payload via EnumPayload.
                        let (field_idx, field_ty) =
                            self.resolve_enum_payload_field(current_ty, variant_idx, name);
                        current = self.emit_enum_payload(
                            current, variant_idx, field_idx, field_ty,
                        );
                        current_ty = field_ty;
                    } else {
                        // Regular struct field extraction
                        let (field_idx, field_ty) =
                            self.resolve_struct_field(current_ty, name);
                        current = self.emit_struct_extract(current, field_idx, field_ty);
                        current_ty = field_ty;
                    }
                }
                PathElement::Index(i) => {
                    if let Some((_, variant_idx)) = pending_downcast.take() {
                        // After a Downcast, Index extracts from the enum
                        // variant's payload (same as Downcast + Field but
                        // positional instead of named).
                        let field_idx = FieldIdx::new(*i);
                        let field_ty = self.resolve_enum_payload_field_by_index(
                            current_ty, variant_idx, field_idx,
                        );
                        current = self.emit_enum_payload(
                            current, variant_idx, field_idx, field_ty,
                        );
                        current_ty = field_ty;
                    } else {
                        let elem_ty = self.resolve_tuple_element(current_ty, *i);
                        current = self.emit_tuple_extract(current, *i as u32, elem_ty);
                        current_ty = elem_ty;
                    }
                }
                PathElement::Downcast(variant_name) => {
                    // Record the variant for the next Field element.
                    // The scrutinee value stays the same — discriminant
                    // testing already happened in the switch.
                    let entity = self.ty_entity(current_ty);
                    let variant_idx = entity
                        .and_then(|e| self.ctx.resolve_variant_idx(e, variant_name))
                        .unwrap_or(VariantIdx::new(0));
                    pending_downcast = Some((variant_name.clone(), variant_idx));
                }
                PathElement::IndexFromEnd(_) | PathElement::RestSlice { .. } => {
                    // Array rest patterns — not yet supported in OSSA.
                    // Keep current value unchanged.
                }
            }
        }
        (current, current_ty)
    }

    /// Emit a `Matchable.matches(other:)` call for string matching.
    ///
    /// Materializes the string literal, then calls the Matchable protocol
    /// witness to compare. Returns a bool-typed ValueId.
    fn emit_string_match_test(
        &mut self,
        scrutinee: ValueId,
        string_ty: TyId,
        literal: &str,
    ) -> ValueId {
        // Materialize the string literal
        let lit_val = if let MirTy::Named { entity, .. } = self.ctx.module.ty_arena.get(string_ty) {
            let entity = *entity;
            if let Some(init) = self.find_string_literal_init(entity) {
                let ptr = self.emit_literal(Immediate::string_pointer(literal.to_string()));
                let len = self.emit_literal(Immediate::i64(literal.len() as i128));
                self.ctx.register_name(init);
                let callee = Callee::direct_with_args(init, vec![], None);
                self.emit_init_literal_call(callee, vec![ptr, len], string_ty)
            } else {
                self.emit_literal(Immediate::string(literal.to_string()))
            }
        } else {
            self.emit_literal(Immediate::string(literal.to_string()))
        };

        // Resolve Matchable protocol
        let proto_entity = self.ctx.query.query(kestrel_name_res::ResolveBuiltin {
            builtin: kestrel_hir::Builtin::Matchable,
            root: self.ctx.root,
        });
        let Some(proto) = proto_entity else {
            return self.emit_literal(Immediate::bool(false));
        };
        self.ctx.register_name(proto);

        // Call Matchable.matches(other:) via witness dispatch
        let bool_ty = self.ctx.module.ty_arena.bool();
        let method_key = WitnessMethodKey::new("matches", vec![None]);
        let callee = Callee::Witness {
            protocol: proto,
            method: method_key,
            self_type: string_ty,
            method_type_args: vec![],
        };

        let scrutinee_arg = self.prepare_call_arg(scrutinee, ParamConvention::Borrow);
        let lit_arg = self.prepare_call_arg(lit_val, ParamConvention::Borrow);
        self.emit_call_returning(callee, vec![scrutinee_arg, lit_arg], bool_ty)
    }

    /// Emit an `EnumPayload` instruction to extract a field from an enum
    /// variant's payload.
    pub fn emit_enum_payload(
        &mut self,
        operand: ValueId,
        variant: VariantIdx,
        field: FieldIdx,
        result_ty: TyId,
    ) -> ValueId {
        let ownership = self.ownership_for(result_ty);
        let result = self.alloc_value(result_ty, ownership);
        self.push_inst(kestrel_mir_3::inst::InstKind::EnumPayload {
            result,
            operand,
            variant,
            field,
        });
        // Extraction from @owned aggregate consumes it (OSSA rule)
        if self.body.value(operand).ownership == Ownership::Owned {
            self.consume(operand);
        }
        if ownership == kestrel_mir_3::Ownership::Owned {
            self.track_owned(result);
        }
        result
    }

    // ================================================================
    // Type resolution helpers for access paths
    // ================================================================

    /// Get the Named entity from a MIR type.
    fn ty_entity(&self, ty: TyId) -> Option<Entity> {
        match self.ctx.module.ty_arena.get(ty) {
            MirTy::Named { entity, .. } => Some(*entity),
            _ => None,
        }
    }

    /// Resolve a struct field name to its index and type.
    ///
    /// Looks up the struct definition and finds the field by name.
    /// Applies type substitution if the struct is generic and the
    /// current type has concrete type arguments.
    fn resolve_struct_field(
        &mut self,
        struct_ty: TyId,
        field_name: &str,
    ) -> (FieldIdx, TyId) {
        let (entity, type_args) = match self.ctx.module.ty_arena.get(struct_ty) {
            MirTy::Named { entity, type_args, .. } => (Some(*entity), type_args.clone()),
            _ => (None, vec![]),
        };

        if let Some(entity) = entity {
            if let Some(sdef) = self.ctx.module.structs.iter().find(|s| s.entity == entity) {
                if let Some(idx) = sdef.fields.iter().position(|f| f.name == field_name) {
                    let mut field_ty = sdef.fields[idx].ty;

                    // Substitute generic type params if needed
                    if !type_args.is_empty() {
                        let mut subst = kestrel_mir_3::SubstMap::new();
                        for (tp, &arg) in sdef.type_params.iter().zip(type_args.iter()) {
                            subst.type_params.insert(tp.entity, arg);
                        }
                        field_ty = kestrel_mir_3::substitute(
                            &mut self.ctx.module.ty_arena,
                            field_ty,
                            &subst,
                        );
                    }

                    return (FieldIdx::new(idx), field_ty);
                }
            }
        }

        // Fallback: unresolved field — use index 0 and the error type.
        // This can happen with generic types before monomorphization.
        (FieldIdx::new(0), self.ctx.module.ty_arena.error())
    }

    /// Resolve a tuple element index to its type.
    fn resolve_tuple_element(&self, tuple_ty: TyId, index: usize) -> TyId {
        match self.ctx.module.ty_arena.get(tuple_ty) {
            MirTy::Tuple(elements) => {
                elements.get(index).copied().unwrap_or(tuple_ty)
            }
            _ => tuple_ty,
        }
    }

    /// Resolve an enum variant payload field to its index and type.
    ///
    /// Finds the enum definition, looks up the variant by index, and
    /// resolves the field within that variant's payload. Applies type
    /// substitution for generic enums.
    fn resolve_enum_payload_field(
        &mut self,
        enum_ty: TyId,
        variant_idx: VariantIdx,
        field_name: &str,
    ) -> (FieldIdx, TyId) {
        let (entity, type_args) = match self.ctx.module.ty_arena.get(enum_ty) {
            MirTy::Named { entity, type_args, .. } => (Some(*entity), type_args.clone()),
            _ => (None, vec![]),
        };

        if let Some(entity) = entity {
            if let Some(edef) = self.ctx.module.enums.iter().find(|e| e.entity == entity) {
                if let Some(case) = edef.cases.get(variant_idx.index()) {
                    if let Some(idx) = case.payload_fields.iter().position(|f| f.name == field_name) {
                        let mut field_ty = case.payload_fields[idx].ty;

                        // Substitute generic type params if needed
                        if !type_args.is_empty() {
                            let mut subst = kestrel_mir_3::SubstMap::new();
                            for (tp, &arg) in edef.type_params.iter().zip(type_args.iter()) {
                                subst.type_params.insert(tp.entity, arg);
                            }
                            field_ty = kestrel_mir_3::substitute(
                                &mut self.ctx.module.ty_arena,
                                field_ty,
                                &subst,
                            );
                        }

                        return (FieldIdx::new(idx), field_ty);
                    }
                }
            }
        }

        // Fallback: unresolved payload field
        (FieldIdx::new(0), self.ctx.module.ty_arena.error())
    }

    /// Like `resolve_enum_payload_field` but by positional index.
    fn resolve_enum_payload_field_by_index(
        &mut self,
        enum_ty: TyId,
        variant_idx: VariantIdx,
        field_idx: FieldIdx,
    ) -> TyId {
        let (entity, type_args) = match self.ctx.module.ty_arena.get(enum_ty) {
            MirTy::Named { entity, type_args, .. } => (Some(*entity), type_args.clone()),
            _ => (None, vec![]),
        };

        if let Some(entity) = entity {
            if let Some(edef) = self.ctx.module.enums.iter().find(|e| e.entity == entity) {
                if let Some(case) = edef.cases.get(variant_idx.index()) {
                    if let Some(field) = case.payload_fields.get(field_idx.index()) {
                        let mut field_ty = field.ty;
                        if !type_args.is_empty() {
                            let mut subst = kestrel_mir_3::SubstMap::new();
                            for (tp, &arg) in edef.type_params.iter().zip(type_args.iter()) {
                                subst.type_params.insert(tp.entity, arg);
                            }
                            field_ty = kestrel_mir_3::substitute(
                                &mut self.ctx.module.ty_arena,
                                field_ty,
                                &subst,
                            );
                        }
                        return field_ty;
                    }
                }
            }
        }

        self.ctx.module.ty_arena.error()
    }
}

/// Map a decision-tree Constructor to an OSSA SwitchCase.
fn constructor_to_switch_case(bctx: &mut OssaBodyCtx, ctor: &Constructor) -> SwitchCase {
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
            let enum_entity = bctx
                .ctx
                .world
                .parent_of(*entity)
                .unwrap_or_else(|| panic!("ICE: enum case {:?} has no parent", entity));
            let idx = bctx
                .ctx
                .resolve_variant_idx(enum_entity, &case_name)
                .unwrap_or_else(|| {
                    panic!(
                        "ICE: variant '{}' not found in enum {:?}",
                        case_name, enum_entity
                    )
                });
            SwitchCase::Variant(idx)
        }
        Constructor::IntLiteral(v) => SwitchCase::IntLiteral(*v),
        Constructor::IntRange { start, end } => SwitchCase::IntRange {
            start: start.unwrap_or(i64::MIN),
            end: end.unwrap_or(i64::MAX),
        },
        Constructor::CharLiteral(c) => SwitchCase::CharLiteral(*c as u32),
        Constructor::CharRange { start, end } => SwitchCase::CharRange {
            start: start.map(|c| c as u32).unwrap_or(0),
            end: end.map(|c| c as u32).unwrap_or(u32::MAX),
        },
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
