//! Pattern matching / match lowering (OSSA).
//!
//! Compiles match expressions to OSSA via kestrel-pattern-matching's
//! decision tree, then emits the tree as basic blocks with switch/branch
//! terminators. To navigate the scrutinee, OSSA emits SSA extraction
//! instructions (StructExtract, TupleExtract, EnumPayload, Discriminant)
//! to destructure values.

use kestrel_hecs::Entity;
use kestrel_hir::body::{HirExpr, HirExprId, HirMatchArm, MatchSource};
use kestrel_hir::res::LocalId;
use kestrel_mir::callee::Callee;
use kestrel_mir::item::witness::WitnessMethodKey;
use kestrel_mir::terminator::{SwitchArm, SwitchCase};
use kestrel_mir::{
    FieldIdx, Immediate, MirTy, Ownership, ParamConvention, TyId, ValueId, VariantIdx,
};
use kestrel_pattern_matching::constructor::Constructor;
use kestrel_pattern_matching::decision_tree::{Binding, DecisionTree, PathElement};
use kestrel_type_infer::result::ResolvedTy;

use super::OssaBodyCtx;
use crate::ty::lower_resolved_ty;

/// One pattern binding being moved out of an aggregate: its access path
/// (relative to the value currently being destructured) and target local.
struct MoveItem {
    path: Vec<PathElement>,
    local_id: LocalId,
}

/// How an aggregate type is destructured for move-out.
enum AggKind {
    Enum,
    Struct,
    Tuple,
    Other,
}

impl OssaBodyCtx<'_, '_> {
    /// Lower a match expression.
    ///
    /// Mirrors `lower_if`'s deferred-merge reconciliation: each arm is lowered
    /// into its own block and its exit state captured (without jumping); after
    /// the whole decision tree is walked, the merge block is built from the
    /// intersection of values that stayed live on *every* reaching edge, and
    /// each edge drops the values it kept that are dead at the merge. This lets
    /// an arm consume (move out of) the scrutinee on some edges but not others.
    /// Lower a match scrutinee. A `match` takes ownership of its @owned
    /// scrutinee (each arm moves payloads out of it or drops it whole). For a
    /// *mono-dependent* scrutinee local — a conditionally-Copyable container
    /// gated on an unconstrained type param (e.g. `match self` where
    /// `self: Result[T,E]`) — the normal `lower_expr` path emits
    /// `emit_value_use` → a real `CopyValue` (pre-mono the type reports
    /// `Bitwise`) and keeps the original local tracked. Monomorphized with a
    /// non-Copyable arg, that copy becomes a bitwise alias and the surviving
    /// original double-frees when its arm/merge drop runs. Instead we **move**
    /// the local into the match (a fresh @owned result; the source is consumed).
    /// Sound because a mono-dependent value is never *guaranteed* Copyable, so
    /// it can't be used after the match — the frontend would have required a
    /// `Copyable` bound, which makes it no longer mono-dependent.
    ///
    /// Unconditionally non-Copyable scrutinees already move via
    /// `emit_copy_value`; genuinely-Copyable scrutinees keep the copy (they may
    /// be read after the match), so both route through `lower_expr` unchanged.
    fn lower_match_scrutinee(&mut self, scrutinee_expr: HirExprId) -> ValueId {
        let expr = self.hir.exprs[scrutinee_expr].clone();
        if let HirExpr::Local(hir_local, _) = &expr
            && !self.is_var_local(hir_local) {
                let val = self.map_local(*hir_local);
                let vdef = self.body.value(val);
                if vdef.ownership == Ownership::Owned
                    && self.copy_behavior_is_mono_dependent(vdef.ty)
                {
                    return self.emit_move_value(val);
                }
            }
        self.lower_expr(scrutinee_expr)
    }

    pub fn lower_match(
        &mut self,
        expr_id: HirExprId,
        scrutinee_expr: HirExprId,
        arms: &[HirMatchArm],
        source: MatchSource,
    ) -> ValueId {
        // Irrefutable single-arm destructures (let/param) don't need branching.
        // Emit bindings directly so local_map entries survive past the match.
        if matches!(
            source,
            MatchSource::LetDestructure | MatchSource::ParamDestructure
        ) && arms.len() == 1
        {
            return self.lower_irrefutable_destructure(expr_id, scrutinee_expr, &arms[0]);
        }

        let result_ty = self.resolve_expr_type(expr_id);
        let scrutinee_resolved_ty = self.resolve_expr_resolved_ty(scrutinee_expr);

        let scrutinee_val = self.lower_match_scrutinee(scrutinee_expr);
        let scrutinee_ty = self.resolve_expr_type(scrutinee_expr);

        let saved_tracker = self.tracker.clone();
        self.tracker = super::LiveTracker::from_live(&self.all_live_tracked());
        let live_vals = self.tracker.values();
        let descs = self.tracker.descs();
        let n = live_vals.len();
        let result_ownership = self.ownership_for(result_ty);

        let tree = kestrel_pattern_matching::compile_decision_tree(
            &self.hir,
            &self.ctx.query,
            self.ctx.root,
            &scrutinee_resolved_ty,
            arms,
        );

        let snapshot = self.snapshot_scope();

        // Walk the tree, collecting each reaching arm's exit (block, result,
        // per-slot liveness) instead of jumping to a pre-built merge block.
        let mut exits: Vec<super::ArmExit> = Vec::new();
        self.emit_decision_tree_threaded(&tree, scrutinee_val, scrutinee_ty, arms, &mut exits);

        // A slot is live at the merge only if it survived on every reaching edge.
        let mut merge_mask = vec![true; n];
        for exit in &exits {
            for i in 0..n {
                merge_mask[i] &= exit.slots[i].1;
            }
        }
        let merge_idx: Vec<usize> = (0..n).filter(|&i| merge_mask[i]).collect();

        let mut merge_descs: Vec<(TyId, Ownership)> = vec![(result_ty, result_ownership)];
        merge_descs.extend(merge_idx.iter().map(|&i| descs[i]));
        let (merge_block, merge_param_vals) = self.new_block_with_params(&merge_descs);
        let result_param = merge_param_vals[0];
        let merge_live_params: Vec<ValueId> = merge_param_vals[1..].to_vec();

        // If `exits` is empty (every arm diverged / pure Failure) the merge block
        // has no predecessors — an unreachable join, same as `lower_if`.
        for exit in &exits {
            self.switch_to(exit.block);
            // Drop values this edge kept live but that are dead at the merge.
            for i in 0..n {
                if exit.slots[i].1 && !merge_mask[i] {
                    self.emit_destroy_value(exit.slots[i].0);
                }
            }
            let mut args = vec![exit.result];
            args.extend(merge_idx.iter().map(|&i| exit.slots[i].0));
            self.emit_jump(merge_block, args);
        }

        // Continue from the merge block.
        self.restore_scope(&snapshot);
        self.switch_to(merge_block);
        let merge_live_orig: Vec<ValueId> = merge_idx.iter().map(|&i| live_vals[i]).collect();
        self.rebind_scope_values(&merge_live_orig, &merge_live_params);
        // Values dead at the merge were dropped on every edge — stop tracking them.
        for i in 0..n {
            if !merge_mask[i] {
                self.consume(live_vals[i]);
            }
        }
        if result_ownership == Ownership::Owned {
            self.track_owned(result_param);
        }

        // Restore outer tracker, propagating survivors and dropping the dead.
        self.tracker = saved_tracker;
        self.tracker.rebind(&merge_live_orig, &merge_live_params);
        for i in 0..n {
            if !merge_mask[i] {
                self.tracker.remove(live_vals[i]);
            }
        }

        // Reconcile conditional moves of `var` slots across the match arms.
        let reaching: Vec<&super::ArmExit> = exits.iter().collect();
        self.fold_var_inits(&reaching);

        result_param
    }

    /// Fast path for irrefutable single-arm destructures (`let (a, b) = expr`).
    ///
    /// No branching needed — emit bindings directly in the current block.
    /// This keeps local_map entries alive for code after the match.
    fn lower_irrefutable_destructure(
        &mut self,
        _expr_id: HirExprId,
        scrutinee_expr: HirExprId,
        arm: &HirMatchArm,
    ) -> ValueId {
        let scrutinee_val = self.lower_expr(scrutinee_expr);
        let scrutinee_ty = self.resolve_expr_type(scrutinee_expr);
        let scrutinee_resolved_ty = self.resolve_expr_resolved_ty(scrutinee_expr);

        let tree = kestrel_pattern_matching::compile_decision_tree(
            &self.hir,
            &self.ctx.query,
            self.ctx.root,
            &scrutinee_resolved_ty,
            &[arm.clone()],
        );

        // Extract bindings from the decision tree's Success leaf.
        let bindings = Self::extract_irrefutable_bindings(&tree);
        self.emit_bindings(&bindings, scrutinee_val, scrutinee_ty);

        // Lower the arm body (typically `()` for let destructures).
        self.lower_expr(arm.body)
    }

    /// Walk a decision tree to find the Success leaf's bindings.
    /// For irrefutable patterns the tree is a chain of single-case Switches
    /// ending in Success.
    fn extract_irrefutable_bindings(tree: &DecisionTree) -> Vec<Binding> {
        match tree {
            DecisionTree::Success { bindings, .. } => bindings.clone(),
            DecisionTree::Switch { cases, default, .. } => {
                if cases.len() == 1 {
                    Self::extract_irrefutable_bindings(&cases[0].1)
                } else if let Some(def) = default {
                    Self::extract_irrefutable_bindings(def)
                } else {
                    Vec::new()
                }
            },
            DecisionTree::Guard { success, .. } => Self::extract_irrefutable_bindings(success),
            DecisionTree::Failure => Vec::new(),
        }
    }

    /// Recursively emit a decision tree, threading live @owned values
    /// through every branch/switch as block parameters. Each reaching arm's
    /// exit (block, result, per-slot liveness) is pushed to `exits` rather than
    /// jumped to a merge block — `lower_match` builds the merge afterward.
    fn emit_decision_tree_threaded(
        &mut self,
        tree: &DecisionTree,
        scrutinee: ValueId,
        scrutinee_ty: TyId,
        arms: &[HirMatchArm],
        exits: &mut Vec<super::ArmExit>,
    ) {
        match tree {
            DecisionTree::Switch {
                path,
                ty,
                cases,
                default,
            } => {
                // A switch with no constructor cases (or one case and no
                // default) is degenerate — it always falls through. Recurse
                // without extracting/testing, which also avoids copying a
                // non-Copyable value just to compute an unused discriminant.
                if cases.is_empty() {
                    if let Some(def_tree) = default {
                        self.emit_decision_tree_threaded(
                            def_tree,
                            scrutinee,
                            scrutinee_ty,
                            arms,
                            exits,
                        );
                    } else {
                        self.emit_panic("match failure: empty switch");
                    }
                    return;
                }

                if cases.len() == 1 && default.is_none() {
                    self.emit_decision_tree_threaded(
                        &cases[0].1,
                        scrutinee,
                        scrutinee_ty,
                        arms,
                        exits,
                    );
                    return;
                }

                // Boolean branch
                if cases.len() == 2
                    && matches!(&cases[0].0, Constructor::True)
                    && matches!(&cases[1].0, Constructor::False)
                {
                    // Bool is Copyable — extract-copy is fine here.
                    let (test_val, _) = self.apply_access_path(scrutinee, scrutinee_ty, path);
                    let branch_snapshot = self.snapshot_scope();
                    let current_live: Vec<ValueId> =
                        self.all_live_tracked().iter().map(|&(v, _, _)| v).collect();
                    let local_descs: Vec<(TyId, Ownership)> = current_live
                        .iter()
                        .map(|&v| (self.body.value(v).ty, self.body.value(v).ownership))
                        .collect();

                    let (true_block, true_params) = self.new_block_with_params(&local_descs);
                    let (false_block, false_params) = self.new_block_with_params(&local_descs);
                    self.emit_branch(
                        test_val,
                        true_block,
                        current_live.clone(),
                        false_block,
                        current_live.clone(),
                    );

                    self.switch_to(true_block);
                    self.rebind_scope_values(&current_live, &true_params);
                    let rebound = rebound_value(scrutinee, &current_live, &true_params);
                    self.emit_decision_tree_threaded(
                        &cases[0].1,
                        rebound,
                        scrutinee_ty,
                        arms,
                        exits,
                    );

                    self.restore_scope(&branch_snapshot);
                    self.switch_to(false_block);
                    self.rebind_scope_values(&current_live, &false_params);
                    let rebound = rebound_value(scrutinee, &current_live, &false_params);
                    self.emit_decision_tree_threaded(
                        &cases[1].1,
                        rebound,
                        scrutinee_ty,
                        arms,
                        exits,
                    );
                    return;
                }

                // String literal chain
                if cases
                    .iter()
                    .any(|(c, _)| matches!(c, Constructor::StringLiteral(_)))
                {
                    // String is Copyable — extract-copy is fine here.
                    let (test_val, _) = self.apply_access_path(scrutinee, scrutinee_ty, path);
                    let test_mir_ty = lower_resolved_ty(self.ctx, ty);
                    let mut current_scrutinee = scrutinee;
                    // Snapshot before the chain so each iteration starts clean.
                    let chain_snapshot = self.snapshot_scope();
                    let chain_live: Vec<ValueId> =
                        self.all_live_tracked().iter().map(|&(v, _, _)| v).collect();
                    let chain_descs: Vec<(TyId, Ownership)> = chain_live
                        .iter()
                        .map(|&v| (self.body.value(v).ty, self.body.value(v).ownership))
                        .collect();
                    for (ctor, subtree) in cases.iter() {
                        let Constructor::StringLiteral(lit) = ctor else {
                            continue;
                        };
                        let cmp = self.emit_string_match_test(test_val, test_mir_ty, lit);
                        // Use the chain-level live set (not the inflated post-test set)
                        // so intermediates from emit_string_match_test are destroyed
                        // in each arm rather than forwarded and accumulated.
                        let str_live: Vec<ValueId> =
                            self.all_live_tracked().iter().map(|&(v, _, _)| v).collect();
                        let str_descs: Vec<(TyId, Ownership)> = str_live
                            .iter()
                            .map(|&v| (self.body.value(v).ty, self.body.value(v).ownership))
                            .collect();
                        let (hit_block, hit_params) = self.new_block_with_params(&str_descs);
                        let (miss_block, miss_params) = self.new_block_with_params(&chain_descs);
                        self.emit_branch(
                            cmp,
                            hit_block,
                            str_live.clone(),
                            miss_block,
                            chain_live.clone(),
                        );

                        self.switch_to(hit_block);
                        self.rebind_scope_values(&str_live, &hit_params);
                        // Destroy test intermediates — they're only needed
                        // for the branch condition, not the arm body.
                        let chain_set: std::collections::HashSet<ValueId> =
                            chain_live.iter().copied().collect();
                        for (old, new) in str_live.iter().zip(hit_params.iter()) {
                            if !chain_set.contains(old) {
                                self.emit_destroy_value(*new);
                            }
                        }
                        let rebound = rebound_value(current_scrutinee, &str_live, &hit_params);
                        self.emit_decision_tree_threaded(
                            subtree,
                            rebound,
                            scrutinee_ty,
                            arms,
                            exits,
                        );

                        self.restore_scope(&chain_snapshot);
                        self.switch_to(miss_block);
                        self.rebind_scope_values(&chain_live, &miss_params);
                        current_scrutinee =
                            rebound_value(current_scrutinee, &chain_live, &miss_params);
                    }
                    if let Some(def_tree) = default {
                        self.emit_decision_tree_threaded(
                            def_tree,
                            current_scrutinee,
                            scrutinee_ty,
                            arms,
                            exits,
                        );
                    } else {
                        self.emit_panic("match failure: non-exhaustive string patterns");
                    }
                    return;
                }

                // General switch — read the discriminant via a borrow so a
                // non-Copyable scrutinee/payload is never illegally copied just
                // to be inspected. The borrow is closed right after the read.
                let (disc_operand, borrow_to_end) = if path.is_empty() {
                    (scrutinee, None)
                } else if self.body.value(scrutinee).ownership == Ownership::Guaranteed {
                    (
                        self.apply_access_path(scrutinee, scrutinee_ty, path).0,
                        None,
                    )
                } else {
                    let borrow = self.emit_begin_borrow(scrutinee);
                    (
                        self.apply_access_path(borrow, scrutinee_ty, path).0,
                        Some(borrow),
                    )
                };
                let discriminant = self.emit_discriminant(disc_operand);
                if let Some(borrow) = borrow_to_end {
                    self.emit_end_borrow(borrow);
                }

                // Snapshot and live set after discriminant so it's included
                let branch_snapshot = self.snapshot_scope();
                let switch_live: Vec<ValueId> =
                    self.all_live_tracked().iter().map(|&(v, _, _)| v).collect();
                let switch_descs: Vec<(TyId, Ownership)> = switch_live
                    .iter()
                    .map(|&v| (self.body.value(v).ty, self.body.value(v).ownership))
                    .collect();

                let mut switch_arms: Vec<SwitchArm> = Vec::with_capacity(cases.len());
                let mut case_blocks = Vec::with_capacity(cases.len());
                for (ctor, _) in cases.iter() {
                    let pattern = constructor_to_switch_case(self, ctor);
                    let (block, params) = self.new_block_with_params(&switch_descs);
                    switch_arms.push(SwitchArm {
                        pattern,
                        target: block,
                        args: switch_live.clone(),
                    });
                    case_blocks.push((block, params));
                }
                let default_block = default.as_ref().map(|_| {
                    let (block, params) = self.new_block_with_params(&switch_descs);
                    switch_arms.push(SwitchArm {
                        pattern: SwitchCase::Wildcard,
                        target: block,
                        args: switch_live.clone(),
                    });
                    (block, params)
                });

                self.emit_switch(discriminant, switch_arms);

                // Identify values that are forwarded but not in the original tracker.
                // These (like the discriminant) must be destroyed in each arm since
                // they won't be forwarded to the merge block.
                let tracker_set: std::collections::HashSet<ValueId> =
                    self.tracker.values().into_iter().collect();
                let extra_vals: Vec<ValueId> = switch_live
                    .iter()
                    .filter(|v| !tracker_set.contains(v))
                    .copied()
                    .collect();

                for (i, ((_, subtree), (block_id, params))) in
                    cases.iter().zip(case_blocks.iter()).enumerate()
                {
                    if i > 0 {
                        self.restore_scope(&branch_snapshot);
                    }
                    self.switch_to(*block_id);
                    self.rebind_scope_values(&switch_live, params);
                    // Destroy values that aren't in the tracker (e.g., discriminant)
                    for &extra in &extra_vals {
                        if let Some(pos) = switch_live.iter().position(|&v| v == extra) {
                            self.emit_destroy_value(params[pos]);
                        }
                    }
                    let rebound = rebound_value(scrutinee, &switch_live, params);
                    self.emit_decision_tree_threaded(subtree, rebound, scrutinee_ty, arms, exits);
                }

                if let (Some(def_tree), Some((def_block, def_params))) = (default, default_block) {
                    self.restore_scope(&branch_snapshot);
                    self.switch_to(def_block);
                    self.rebind_scope_values(&switch_live, &def_params);
                    for &extra in &extra_vals {
                        if let Some(pos) = switch_live.iter().position(|&v| v == extra) {
                            self.emit_destroy_value(def_params[pos]);
                        }
                    }
                    let rebound = rebound_value(scrutinee, &switch_live, &def_params);
                    self.emit_decision_tree_threaded(def_tree, rebound, scrutinee_ty, arms, exits);
                }
            },

            DecisionTree::Success {
                arm_index,
                bindings,
            } => {
                self.emit_success_leaf(*arm_index, bindings, scrutinee, scrutinee_ty, arms, exits);
            },

            DecisionTree::Guard {
                arm_index,
                bindings,
                failure,
                ..
            } => {
                // `success` is always a `Success { bindings: [] }` leaf (the
                // bindings are hoisted onto this Guard node), so we commit the
                // arm directly in the success branch using the guard's bindings.
                let guard_expr = arms.get(*arm_index).and_then(|a| a.guard);
                let Some(guard_expr) = guard_expr else {
                    // No guard expression — behave as a plain success leaf.
                    self.emit_success_leaf(
                        *arm_index,
                        bindings,
                        scrutinee,
                        scrutinee_ty,
                        arms,
                        exits,
                    );
                    return;
                };

                // Bind non-consumingly (borrow views) so the guard can read the
                // pattern variables without consuming the scrutinee — a failing
                // guard must leave it intact for the remaining patterns.
                self.push_scope();
                self.emit_bindings_for_guard(bindings, scrutinee, scrutinee_ty);
                let guard_val = self.lower_expr(guard_expr);
                let guard_snapshot = self.snapshot_scope();
                let guard_live: Vec<ValueId> =
                    self.all_live_tracked().iter().map(|&(v, _, _)| v).collect();
                let guard_descs: Vec<(TyId, Ownership)> = guard_live
                    .iter()
                    .map(|&v| (self.body.value(v).ty, self.body.value(v).ownership))
                    .collect();
                let (success_block, success_params) = self.new_block_with_params(&guard_descs);
                let (failure_block, failure_params) = self.new_block_with_params(&guard_descs);
                self.emit_branch(
                    guard_val,
                    success_block,
                    guard_live.clone(),
                    failure_block,
                    guard_live.clone(),
                );

                // Success: commit the arm — move-out bindings, body, capture exit.
                self.switch_to(success_block);
                self.rebind_scope_values(&guard_live, &success_params);
                let rebound = rebound_value(scrutinee, &guard_live, &success_params);
                self.pop_scope();
                self.emit_success_leaf(*arm_index, bindings, rebound, scrutinee_ty, arms, exits);

                // Failure: scrutinee untouched — continue matching other patterns.
                self.restore_scope(&guard_snapshot);
                self.switch_to(failure_block);
                self.rebind_scope_values(&guard_live, &failure_params);
                let rebound = rebound_value(scrutinee, &guard_live, &failure_params);
                self.pop_scope();
                self.emit_decision_tree_threaded(failure, rebound, scrutinee_ty, arms, exits);
            },

            DecisionTree::Failure => {
                self.emit_panic("match failure: non-exhaustive patterns");
            },
        }
    }

    /// Emit one matched arm: bind its pattern variables (move-out for
    /// non-Copyable @owned payloads), lower the body, and capture the arm's
    /// exit. Shared by the plain `Success` leaf and a guard's success branch.
    fn emit_success_leaf(
        &mut self,
        arm_index: usize,
        bindings: &[Binding],
        scrutinee: ValueId,
        scrutinee_ty: TyId,
        arms: &[HirMatchArm],
        exits: &mut Vec<super::ArmExit>,
    ) {
        self.push_scope();
        self.emit_bindings(bindings, scrutinee, scrutinee_ty);
        if let Some(arm) = arms.get(arm_index) {
            let body_val = self.lower_expr(arm.body);
            if let Some(exit) = self.capture_arm_exit(body_val) {
                exits.push(exit);
            }
        }
        self.pop_scope();
    }
}

/// Find the rebound version of `val` after a rebind from `old` to `new`.
/// If `val` appears in `old`, return the corresponding `new` entry;
/// otherwise return `val` unchanged.
fn rebound_value(val: ValueId, old: &[ValueId], new: &[ValueId]) -> ValueId {
    if let Some(pos) = old.iter().position(|&v| v == val) {
        new[pos]
    } else {
        val
    }
}

impl OssaBodyCtx<'_, '_> {
    /// Emit SSA bindings for a matched arm.
    ///
    /// Each binding's access path is applied to the scrutinee. When the
    /// scrutinee is @owned and any binding extracts a non-Copyable component
    /// (which can't be copied), the whole leaf is **moved out** of the
    /// scrutinee via consuming destructure (`emit_moveout`), consuming it.
    /// Otherwise the per-binding borrow-extract-copy path runs unchanged
    /// (Copyable matches and @guaranteed/borrowed scrutinees).
    fn emit_bindings(&mut self, bindings: &[Binding], scrutinee: ValueId, scrutinee_ty: TyId) {
        let owned = self.body.value(scrutinee).ownership == Ownership::Owned;
        let needs_moveout = owned
            && bindings
                .iter()
                .any(|b| self.path_requires_moveout(scrutinee_ty, &b.path));

        if needs_moveout {
            let items: Vec<MoveItem> = bindings
                .iter()
                .map(|b| MoveItem {
                    path: b.path.clone(),
                    local_id: b.local_id,
                })
                .collect();
            self.emit_moveout(scrutinee, scrutinee_ty, items);
        } else {
            for binding in bindings {
                self.bind_path_copy(scrutinee, scrutinee_ty, &binding.path, binding.local_id);
            }
        }
    }

    /// Borrow-extract-copy a single binding from `base` and insert it into the
    /// local map. @owned results are copied (so the binding owns its value);
    /// @guaranteed results are used directly.
    fn bind_path_copy(
        &mut self,
        base: ValueId,
        base_ty: TyId,
        path: &[PathElement],
        local_id: LocalId,
    ) {
        let (extracted, _) = self.apply_access_path(base, base_ty, path);
        let bound_val = if self.body.value(extracted).ownership == Ownership::Owned {
            self.emit_copy_value(extracted)
        } else {
            extracted
        };
        self.local_map
            .insert(local_id, super::LocalBinding::Ssa(bound_val));
    }

    /// Bind pattern variables for a guard condition **without consuming** the
    /// scrutinee: borrow it once and navigate by @guaranteed views. A failing
    /// guard must leave the scrutinee intact for the remaining patterns, so we
    /// cannot move out here. The borrow is closed at the branch terminator.
    fn emit_bindings_for_guard(
        &mut self,
        bindings: &[Binding],
        scrutinee: ValueId,
        scrutinee_ty: TyId,
    ) {
        if bindings.is_empty() {
            return;
        }
        let base = if self.body.value(scrutinee).ownership == Ownership::Owned {
            self.emit_begin_borrow(scrutinee)
        } else {
            scrutinee
        };
        for binding in bindings {
            let (view, _) = self.apply_access_path(base, scrutinee_ty, &binding.path);
            self.local_map
                .insert(binding.local_id, super::LocalBinding::Ssa(view));
        }
    }

    /// True if applying `path` to a value of `root_ty` extracts a component
    /// that an @owned scrutinee must **move out** (via `emit_moveout`) rather
    /// than borrow-extract-copy. That holds when a step's type either:
    ///   - is `not Copyable` (no clone shim — a copy would be illegal), or
    ///   - has mono-dependent copy behavior (a type param / associated
    ///     projection): pre-mono it defaults to `Bitwise`, but it can
    ///     monomorphize to a `not Copyable` type, where the copy path's
    ///     bitwise alias + whole-scrutinee drop becomes a double-free. Moving
    ///     out (consuming the scrutinee) is correct for every instantiation.
    /// Mirrors `apply_access_path`'s type resolution without emitting code.
    fn path_requires_moveout(&mut self, root_ty: TyId, path: &[PathElement]) -> bool {
        let mut current_ty = root_ty;
        let mut pending_downcast: Option<VariantIdx> = None;
        for elem in path {
            match elem {
                PathElement::Downcast(name) => {
                    let variant_idx = self
                        .ty_entity(current_ty)
                        .and_then(|e| self.ctx.resolve_variant_idx(e, name))
                        .unwrap_or(VariantIdx::new(0));
                    pending_downcast = Some(variant_idx);
                },
                PathElement::Field(name) => {
                    let field_ty = if let Some(variant_idx) = pending_downcast.take() {
                        self.resolve_enum_payload_field(current_ty, variant_idx, name)
                            .1
                    } else {
                        self.resolve_struct_field(current_ty, name).1
                    };
                    if self.is_non_copyable(field_ty)
                        || self.copy_behavior_is_mono_dependent(field_ty)
                    {
                        return true;
                    }
                    current_ty = field_ty;
                },
                PathElement::Index(i) => {
                    let field_ty = if let Some(variant_idx) = pending_downcast.take() {
                        self.resolve_enum_payload_field_by_index(
                            current_ty,
                            variant_idx,
                            FieldIdx::new(*i),
                        )
                    } else {
                        self.resolve_tuple_element(current_ty, *i)
                    };
                    if self.is_non_copyable(field_ty)
                        || self.copy_behavior_is_mono_dependent(field_ty)
                    {
                        return true;
                    }
                    current_ty = field_ty;
                },
                // Array suffix/rest patterns aren't aggregate destructures.
                PathElement::IndexFromEnd(_) | PathElement::RestSlice { .. } => return false,
            }
        }
        false
    }

    /// Recursively move bindings out of an @owned aggregate `value`, consuming
    /// it. Destructures `value` into its components (one consuming op per
    /// level), binds the components named by `items` (recursing for deeper
    /// paths), and drops every component with no binding. This is the
    /// move-out path for non-Copyable payloads.
    fn emit_moveout(&mut self, value: ValueId, value_ty: TyId, items: Vec<MoveItem>) {
        // A binding that ends here owns the whole (already @owned) value.
        if let Some(item) = items.iter().find(|it| it.path.is_empty()) {
            self.local_map
                .insert(item.local_id, super::LocalBinding::Ssa(value));
            return;
        }

        let (results, comp_tys, enum_variant): (Vec<ValueId>, Vec<TyId>, Option<VariantIdx>) =
            match self.aggregate_kind(value_ty) {
                AggKind::Enum => {
                    let variant_name = items.iter().find_map(|it| match it.path.first() {
                        Some(PathElement::Downcast(n)) => Some(n.clone()),
                        _ => None,
                    });
                    let Some(vname) = variant_name else {
                        // Unexpected shape — fall back to per-item copy.
                        for it in &items {
                            self.bind_path_copy(value, value_ty, &it.path, it.local_id);
                        }
                        return;
                    };
                    let variant_idx = self
                        .ty_entity(value_ty)
                        .and_then(|e| self.ctx.resolve_variant_idx(e, &vname))
                        .unwrap_or(VariantIdx::new(0));
                    let comp_tys = self.enum_variant_payload_tys(value_ty, variant_idx);
                    let results = self.emit_destructure_enum(value, variant_idx, &comp_tys);
                    (results, comp_tys, Some(variant_idx))
                },
                AggKind::Struct => {
                    let comp_tys = self.struct_field_tys(value_ty);
                    let results = self.emit_destructure_struct(value, &comp_tys);
                    (results, comp_tys, None)
                },
                AggKind::Tuple => {
                    let comp_tys = self.tuple_elem_tys(value_ty);
                    let results = self.emit_destructure_tuple(value, &comp_tys);
                    (results, comp_tys, None)
                },
                AggKind::Other => {
                    // Not a destructurable aggregate (e.g. array pattern) — fall
                    // back to per-item copy from the original value.
                    for it in &items {
                        self.bind_path_copy(value, value_ty, &it.path, it.local_id);
                    }
                    return;
                },
            };

        // Route each item to its component, stripping the consumed prefix.
        let mut by_comp: Vec<Vec<MoveItem>> = (0..results.len()).map(|_| Vec::new()).collect();
        for it in items {
            let (comp_idx, rest) = self.route_item(value_ty, enum_variant, &it.path);
            match comp_idx {
                Some(ci) if ci < by_comp.len() => by_comp[ci].push(MoveItem {
                    path: rest,
                    local_id: it.local_id,
                }),
                _ => {
                    // Shouldn't happen for a well-formed pattern; the component
                    // is dropped below since nothing routed into it.
                },
            }
        }

        for (ci, sub) in by_comp.into_iter().enumerate() {
            if sub.is_empty() {
                self.emit_destroy_value(results[ci]);
            } else {
                self.emit_moveout(results[ci], comp_tys[ci], sub);
            }
        }
    }

    /// Map an item's leading path element to a component index of the just-
    /// destructured `value_ty`, returning the remaining (stripped) path.
    fn route_item(
        &mut self,
        value_ty: TyId,
        enum_variant: Option<VariantIdx>,
        path: &[PathElement],
    ) -> (Option<usize>, Vec<PathElement>) {
        if let Some(variant_idx) = enum_variant {
            // Enum payload paths are `[Downcast(V), accessor, ...rest]`.
            if path.len() >= 2 && matches!(path[0], PathElement::Downcast(_)) {
                let comp = match &path[1] {
                    PathElement::Index(i) => Some(*i),
                    PathElement::Field(name) => Some(
                        self.resolve_enum_payload_field(value_ty, variant_idx, name)
                            .0
                            .index(),
                    ),
                    _ => None,
                };
                return (comp, path[2..].to_vec());
            }
            return (None, path.to_vec());
        }
        // Struct/tuple paths are `[accessor, ...rest]`.
        if let Some(first) = path.first() {
            let comp = match first {
                PathElement::Index(i) => Some(*i),
                PathElement::Field(name) => {
                    Some(self.resolve_struct_field(value_ty, name).0.index())
                },
                _ => None,
            };
            return (comp, path[1..].to_vec());
        }
        (None, path.to_vec())
    }

    /// Classify an aggregate type for destructuring.
    fn aggregate_kind(&self, ty: TyId) -> AggKind {
        match self.ctx.module.ty_arena.get(ty) {
            MirTy::Named { entity, .. } => {
                let e = *entity;
                if self.ctx.module.enums.contains_key(&e) {
                    AggKind::Enum
                } else if self.ctx.module.structs.contains_key(&e) {
                    AggKind::Struct
                } else {
                    AggKind::Other
                }
            },
            MirTy::Tuple(_) => AggKind::Tuple,
            _ => AggKind::Other,
        }
    }

    /// All payload field types of an enum variant, with generic substitution.
    fn enum_variant_payload_tys(&mut self, enum_ty: TyId, variant_idx: VariantIdx) -> Vec<TyId> {
        let (entity, type_args) = match self.ctx.module.ty_arena.get(enum_ty) {
            MirTy::Named {
                entity, type_args, ..
            } => (Some(*entity), type_args.clone()),
            _ => (None, vec![]),
        };
        let Some(entity) = entity else {
            return vec![];
        };
        // Collect raw field tys + type-param entities, releasing the edef borrow
        // before substituting (which needs &mut ty_arena).
        let (raw_tys, tp_entities): (Vec<TyId>, Vec<Entity>) = match self
            .ctx
            .module
            .enums
            .get(&entity)
            .and_then(|edef| edef.cases.get(variant_idx.index()).map(|c| (c, edef)))
        {
            Some((case, edef)) => (
                case.payload_fields.iter().map(|f| f.ty).collect(),
                edef.type_params.iter().map(|tp| tp.entity).collect(),
            ),
            None => (vec![], vec![]),
        };
        self.substitute_all(raw_tys, &tp_entities, &type_args)
    }

    /// All field types of a struct, with generic substitution.
    fn struct_field_tys(&mut self, struct_ty: TyId) -> Vec<TyId> {
        let (entity, type_args) = match self.ctx.module.ty_arena.get(struct_ty) {
            MirTy::Named {
                entity, type_args, ..
            } => (Some(*entity), type_args.clone()),
            _ => (None, vec![]),
        };
        let Some(entity) = entity else {
            return vec![];
        };
        let (raw_tys, tp_entities): (Vec<TyId>, Vec<Entity>) =
            match self.ctx.module.structs.get(&entity) {
                Some(sdef) => (
                    sdef.fields.iter().map(|f| f.ty).collect(),
                    sdef.type_params.iter().map(|tp| tp.entity).collect(),
                ),
                None => (vec![], vec![]),
            };
        self.substitute_all(raw_tys, &tp_entities, &type_args)
    }

    /// All element types of a tuple type.
    fn tuple_elem_tys(&self, tuple_ty: TyId) -> Vec<TyId> {
        match self.ctx.module.ty_arena.get(tuple_ty) {
            MirTy::Tuple(elements) => elements.clone(),
            _ => vec![],
        }
    }

    /// Substitute generic type params into a batch of field/element types.
    fn substitute_all(
        &mut self,
        raw_tys: Vec<TyId>,
        tp_entities: &[Entity],
        type_args: &[TyId],
    ) -> Vec<TyId> {
        if type_args.is_empty() || tp_entities.is_empty() {
            return raw_tys;
        }
        let mut subst = kestrel_mir::SubstMap::new();
        for (&tp, &arg) in tp_entities.iter().zip(type_args.iter()) {
            subst.type_params.insert(tp, arg);
        }
        raw_tys
            .iter()
            .map(|&t| kestrel_mir::substitute(&mut self.ctx.module.ty_arena, t, &subst))
            .collect()
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
    /// All extractions use borrow-extract-copy: the operand stays alive for
    /// the tracker and further extractions.
    fn apply_access_path(
        &mut self,
        value: ValueId,
        value_ty: TyId,
        path: &[PathElement],
    ) -> (ValueId, TyId) {
        let mut current = value;
        let mut current_ty = value_ty;
        let mut pending_downcast: Option<(String, VariantIdx)> = None;

        for elem in path {
            match elem {
                PathElement::Field(name) => {
                    if let Some((_, variant_idx)) = pending_downcast.take() {
                        let (field_idx, field_ty) =
                            self.resolve_enum_payload_field(current_ty, variant_idx, name);
                        current = self.emit_enum_payload(current, variant_idx, field_idx, field_ty);
                        current_ty = field_ty;
                    } else {
                        let (field_idx, field_ty) = self.resolve_struct_field(current_ty, name);
                        current = self.emit_struct_extract(current, field_idx, field_ty);
                        current_ty = field_ty;
                    }
                },
                PathElement::Index(i) => {
                    if let Some((_, variant_idx)) = pending_downcast.take() {
                        let field_idx = FieldIdx::new(*i);
                        let field_ty = self.resolve_enum_payload_field_by_index(
                            current_ty,
                            variant_idx,
                            field_idx,
                        );
                        current = self.emit_enum_payload(current, variant_idx, field_idx, field_ty);
                        current_ty = field_ty;
                    } else {
                        let elem_ty = self.resolve_tuple_element(current_ty, *i);
                        current = self.emit_tuple_extract(current, *i as u32, elem_ty);
                        current_ty = elem_ty;
                    }
                },
                PathElement::Downcast(variant_name) => {
                    let entity = self.ty_entity(current_ty);
                    let variant_idx = entity
                        .and_then(|e| self.ctx.resolve_variant_idx(e, variant_name))
                        .unwrap_or(VariantIdx::new(0));
                    pending_downcast = Some((variant_name.clone(), variant_idx));
                },
                PathElement::IndexFromEnd(_) | PathElement::RestSlice { .. } => {},
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
        if self.body.value(operand).ownership == Ownership::Guaranteed {
            let result = self.alloc_guaranteed(result_ty, operand);
            self.push_inst(kestrel_mir::inst::InstKind::EnumPayload {
                result,
                operand,
                variant,
                field,
            });
            result
        } else {
            // Borrow → extract (@guaranteed) → copy (@owned). Operand stays alive.
            let borrow = self.emit_begin_borrow(operand);
            let field_ref = self.alloc_guaranteed(result_ty, borrow);
            self.push_inst(kestrel_mir::inst::InstKind::EnumPayload {
                result: field_ref,
                operand: borrow,
                variant,
                field,
            });
            let result = self.emit_copy_value(field_ref);
            self.emit_end_borrow(borrow);
            result
        }
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
    fn resolve_struct_field(&mut self, struct_ty: TyId, field_name: &str) -> (FieldIdx, TyId) {
        let (entity, type_args) = match self.ctx.module.ty_arena.get(struct_ty) {
            MirTy::Named {
                entity, type_args, ..
            } => (Some(*entity), type_args.clone()),
            _ => (None, vec![]),
        };

        if let Some(entity) = entity
            && let Some(sdef) = self.ctx.module.structs.get(&entity)
                && let Some(idx) = sdef.fields.iter().position(|f| f.name == field_name) {
                    let mut field_ty = sdef.fields[idx].ty;

                    // Substitute generic type params if needed
                    if !type_args.is_empty() {
                        let mut subst = kestrel_mir::SubstMap::new();
                        for (tp, &arg) in sdef.type_params.iter().zip(type_args.iter()) {
                            subst.type_params.insert(tp.entity, arg);
                        }
                        field_ty = kestrel_mir::substitute(
                            &mut self.ctx.module.ty_arena,
                            field_ty,
                            &subst,
                        );
                    }

                    return (FieldIdx::new(idx), field_ty);
                }

        // Fallback: unresolved field — use index 0 and the error type.
        // This can happen with generic types before monomorphization.
        (FieldIdx::new(0), self.ctx.module.ty_arena.error())
    }

    /// Resolve a tuple element index to its type.
    fn resolve_tuple_element(&self, tuple_ty: TyId, index: usize) -> TyId {
        match self.ctx.module.ty_arena.get(tuple_ty) {
            MirTy::Tuple(elements) => elements.get(index).copied().unwrap_or(tuple_ty),
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
            MirTy::Named {
                entity, type_args, ..
            } => (Some(*entity), type_args.clone()),
            _ => (None, vec![]),
        };

        if let Some(entity) = entity
            && let Some(edef) = self.ctx.module.enums.get(&entity)
                && let Some(case) = edef.cases.get(variant_idx.index())
                    && let Some(idx) = case
                        .payload_fields
                        .iter()
                        .position(|f| f.name == field_name)
                    {
                        let mut field_ty = case.payload_fields[idx].ty;

                        // Substitute generic type params if needed
                        if !type_args.is_empty() {
                            let mut subst = kestrel_mir::SubstMap::new();
                            for (tp, &arg) in edef.type_params.iter().zip(type_args.iter()) {
                                subst.type_params.insert(tp.entity, arg);
                            }
                            field_ty = kestrel_mir::substitute(
                                &mut self.ctx.module.ty_arena,
                                field_ty,
                                &subst,
                            );
                        }

                        return (FieldIdx::new(idx), field_ty);
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
            MirTy::Named {
                entity, type_args, ..
            } => (Some(*entity), type_args.clone()),
            _ => (None, vec![]),
        };

        if let Some(entity) = entity
            && let Some(edef) = self.ctx.module.enums.get(&entity)
                && let Some(case) = edef.cases.get(variant_idx.index())
                    && let Some(field) = case.payload_fields.get(field_idx.index()) {
                        let mut field_ty = field.ty;
                        if !type_args.is_empty() {
                            let mut subst = kestrel_mir::SubstMap::new();
                            for (tp, &arg) in edef.type_params.iter().zip(type_args.iter()) {
                                subst.type_params.insert(tp.entity, arg);
                            }
                            field_ty = kestrel_mir::substitute(
                                &mut self.ctx.module.ty_arena,
                                field_ty,
                                &subst,
                            );
                        }
                        return field_ty;
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
        },
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
