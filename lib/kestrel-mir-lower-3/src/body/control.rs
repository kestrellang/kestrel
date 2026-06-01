//! Control flow lowering — if/else, loop, break, continue.
//!
//! OSSA approach: instead of writing to shared result locals, each arm
//! threads its result through block parameters on the merge block. All
//! @owned values live before the branch must be destroyed in each arm
//! (or threaded through block args). Match lowering lives in `pattern.rs`.

use kestrel_hir::body::{HirBlock, HirExprId};
use kestrel_mir_3::value::Ownership;
use kestrel_mir_3::{BlockId, Immediate, TyId, ValueId};

use super::{ArmExit, LoopInfo, OssaBodyCtx};

impl OssaBodyCtx<'_, '_> {
    // ================================================================
    // If / Else
    // ================================================================

    pub fn lower_if(
        &mut self,
        expr_id: HirExprId,
        condition: HirExprId,
        then_body: &HirBlock,
        else_body: Option<&HirBlock>,
    ) -> ValueId {
        let cond_val = self.lower_expr(condition);
        let result_ty = self.resolve_expr_type(expr_id);
        let result_ownership = self.ownership_for(result_ty);

        // Save outer tracker, set new one for this region
        let saved_tracker = self.tracker.clone();
        self.tracker = super::LiveTracker::from_live(&self.all_live_tracked());
        let live_vals = self.tracker.values();
        let descs = self.tracker.descs();
        let n = live_vals.len();

        let (then_block, then_params) = self.new_block_with_params(&descs);
        let (else_block, else_params) = self.new_block_with_params(&descs);

        self.emit_branch(
            cond_val,
            then_block,
            live_vals.clone(),
            else_block,
            live_vals.clone(),
        );

        let snapshot = self.snapshot_scope();

        // -- Then arm: lower into its block, capture exit, defer the jump --
        self.switch_to(then_block);
        self.rebind_scope_values(&live_vals, &then_params);
        self.push_scope();
        let then_val = self.lower_hir_block(then_body);
        let then_exit = self.capture_arm_exit(then_val);
        self.pop_scope();

        // -- Else arm --
        self.restore_scope(&snapshot);
        self.switch_to(else_block);
        self.rebind_scope_values(&live_vals, &else_params);
        self.push_scope();
        let else_exit = if let Some(else_body) = else_body {
            let else_val = self.lower_hir_block(else_body);
            self.capture_arm_exit(else_val)
        } else {
            let unit = self.emit_literal(Immediate::unit());
            self.capture_arm_exit(unit)
        };
        self.pop_scope();

        // -- Reconcile divergent liveness: a value is live at the merge only if
        // it survived on every reaching edge. Values moved on some edge are dead
        // at the merge and are dropped on the edges where they survived. --
        let reaching: Vec<&ArmExit> = [&then_exit, &else_exit].into_iter().flatten().collect();
        let mut merge_mask = vec![true; n];
        for exit in &reaching {
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

        for exit in &reaching {
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

        // -- Merge --
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

        // Reconcile conditional moves of `var` slots: join each var's per-arm
        // init-state over the reaching edges and apply it at the merge. The
        // in-memory drop flag is already correct per-path (set in the took arm).
        self.fold_var_inits(&reaching);

        result_param
    }

    /// Join per-arm `var` init-states over the reaching edges (lattice via
    /// `VarInit::join`) and write the result onto the (restored) merge entries.
    /// A var taken on some-but-not-all edges becomes `MaybeUninit`.
    pub(crate) fn fold_var_inits(&mut self, reaching: &[&ArmExit]) {
        for (local, _) in self.scope_var_inits() {
            let mut joined: Option<super::VarInit> = None;
            for exit in reaching {
                if let Some((_, ai)) = exit.var_inits.iter().find(|(l, _)| *l == local) {
                    joined = Some(match joined {
                        None => *ai,
                        Some(j) => j.join(*ai),
                    });
                }
            }
            if let Some(j) = joined {
                self.set_var_init(local, j);
            }
        }
    }

    // ================================================================
    // Loop
    // ================================================================

    pub fn lower_loop(&mut self, body: &HirBlock, label: Option<&str>) -> ValueId {
        let saved_tracker = self.tracker.clone();
        let saved_snapshot = self.snapshot_scope();
        self.tracker = super::LiveTracker::from_live(&self.all_live_tracked());
        let initial_args = self.tracker.values();
        let descs = self.tracker.descs();

        let (header_block, header_params) = self.new_block_with_params(&descs);
        // Exit block gets the same params as the header so that values
        // live before the loop are properly threaded through break sites.
        let (exit_block, exit_params) = self.new_block_with_params(&descs);

        if !self.is_terminated() {
            for &v in &initial_args {
                self.pop_owned_from_scope(v);
            }
            self.emit_jump(header_block, initial_args.clone());
        }

        self.switch_to(header_block);
        self.rebind_scope_values(&initial_args, &header_params);
        // initial_args were consumed before the jump, so rebind won't
        // find them in scope. Track the header params fresh.
        for &param in header_params.iter() {
            self.track_owned(param);
        }

        let scope_depth = self.scope_stack.len();
        self.loop_stack.push(LoopInfo {
            header_block,
            exit_block,
            label: label.map(|s| s.to_string()),
            scope_depth,
            tracker_len: self.tracker.len(),
        });

        self.push_scope();
        let _ = self.lower_hir_block(body);

        if !self.is_terminated() {
            let back_edge_vals = self.tracker.values();
            self.destroy_scope_except(&back_edge_vals);
            self.pop_scope();
            for &v in &back_edge_vals {
                self.pop_owned_from_scope(v);
            }
            self.emit_jump(header_block, back_edge_vals);
        } else {
            self.pop_scope();
        }

        self.loop_stack.pop();
        self.switch_to(exit_block);
        // Restore scope/local_map to the pre-loop state so that
        // rebind_scope_values can find the initial_args values.
        self.restore_scope(&saved_snapshot);
        self.rebind_scope_values(&initial_args, &exit_params);
        self.tracker = saved_tracker;
        self.tracker.rebind(&initial_args, &exit_params);
        self.emit_literal(Immediate::unit())
    }

    // ================================================================
    // Break
    // ================================================================

    pub fn lower_break(&mut self, label: Option<&str>) -> ValueId {
        if let Some(info) = self.find_loop(label) {
            let exit = info.exit_block;
            let depth = info.scope_depth;
            let tracker_len = info.tracker_len;
            // The active tracker (possibly the loop's or a nested if's)
            // starts with the loop's tracked values in its first N slots.
            // These are the values that need to be threaded to the exit block.
            let all_vals = self.tracker.values();
            let exit_vals: Vec<ValueId> = all_vals[..tracker_len.min(all_vals.len())].to_vec();
            // Destroy inner scopes (loop body + any nested ones),
            // keeping the values we're threading to the exit block.
            self.destroy_scopes_to_depth(depth, &exit_vals);
            self.emit_jump(exit, exit_vals);
        }
        self.emit_literal(Immediate::unit())
    }

    // ================================================================
    // Continue
    // ================================================================

    pub fn lower_continue(&mut self, label: Option<&str>) -> ValueId {
        if let Some(info) = self.find_loop(label) {
            let header = info.header_block;
            let depth = info.scope_depth;
            let header_param_vals = self.header_param_values(header);
            self.destroy_scopes_to_depth(depth, &header_param_vals);
            let current_vals = self.collect_current_for_values(&header_param_vals);
            self.emit_jump(header, current_vals);
        }
        self.emit_literal(Immediate::unit())
    }

    // ================================================================
    // Helpers
    // ================================================================

    fn find_loop(&self, label: Option<&str>) -> Option<&LoopInfo> {
        match label {
            Some(label) => self
                .loop_stack
                .iter()
                .rev()
                .find(|l| l.label.as_deref() == Some(label)),
            None => self.loop_stack.last(),
        }
    }

    fn collect_current_for_values(&self, expected: &[ValueId]) -> Vec<ValueId> {
        let all_tracked = self.all_live_tracked();
        if all_tracked.len() >= expected.len() {
            all_tracked[..expected.len()]
                .iter()
                .map(|&(v, _, _)| v)
                .collect()
        } else {
            let mut vals: Vec<_> = all_tracked.iter().map(|&(v, _, _)| v).collect();
            while vals.len() < expected.len() {
                vals.push(expected[vals.len()]);
            }
            vals
        }
    }

    fn header_param_values(&self, header: BlockId) -> Vec<ValueId> {
        self.body
            .block(header)
            .params
            .iter()
            .map(|p| p.value)
            .collect()
    }
}
