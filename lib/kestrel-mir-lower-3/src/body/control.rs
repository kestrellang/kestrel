//! Control flow lowering — if/else, loop, break, continue.
//!
//! OSSA approach: instead of writing to shared result locals, each arm
//! threads its result through block parameters on the merge block. All
//! @owned values live before the branch must be destroyed in each arm
//! (or threaded through block args). Match lowering lives in `pattern.rs`.

use kestrel_hir::body::{HirBlock, HirExprId};
use kestrel_mir_3::value::Ownership;
use kestrel_mir_3::{BlockId, Immediate, TyId, ValueId};

use super::{LoopInfo, OssaBodyCtx};

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

        let (then_block, then_params) = self.new_block_with_params(&descs);
        let (else_block, else_params) = self.new_block_with_params(&descs);
        let mut merge_descs: Vec<(TyId, Ownership)> = vec![(result_ty, result_ownership)];
        merge_descs.extend(&descs);
        let (merge_block, merge_param_vals) = self.new_block_with_params(&merge_descs);
        let result_param = merge_param_vals[0];

        self.emit_branch(
            cond_val,
            then_block, live_vals.clone(),
            else_block, live_vals.clone(),
        );

        let snapshot = self.snapshot_scope();

        // -- Then arm --
        self.switch_to(then_block);
        self.rebind_scope_values(&live_vals, &then_params);
        self.push_scope();
        let then_val = self.lower_hir_block(then_body);
        if !self.is_terminated() {
            let tracker_vals = self.tracker.values();
            let mut keep = vec![then_val];
            keep.extend(&tracker_vals);
            self.destroy_scope_except(&keep);
            let mut args = vec![then_val];
            args.extend(tracker_vals);
            self.emit_jump(merge_block, args);
        }
        self.pop_scope();

        // -- Else arm --
        self.restore_scope(&snapshot);
        self.switch_to(else_block);
        self.rebind_scope_values(&live_vals, &else_params);
        self.push_scope();
        if let Some(else_body) = else_body {
            let else_val = self.lower_hir_block(else_body);
            if !self.is_terminated() {
                let tracker_vals = self.tracker.values();
                let mut keep = vec![else_val];
                keep.extend(&tracker_vals);
                self.destroy_scope_except(&keep);
                let mut args = vec![else_val];
                args.extend(tracker_vals);
                self.emit_jump(merge_block, args);
            }
        } else {
            let unit = self.emit_literal(Immediate::unit());
            let mut args = vec![unit];
            args.extend(self.tracker.values());
            self.emit_jump(merge_block, args);
        }
        self.pop_scope();

        // -- Merge --
        self.restore_scope(&snapshot);
        self.switch_to(merge_block);
        let merge_live = &merge_param_vals[1..];
        self.rebind_scope_values(&live_vals, merge_live);
        if result_ownership == Ownership::Owned {
            self.track_owned(result_param);
        }

        // Restore outer tracker with current values propagated
        self.tracker = saved_tracker;
        self.tracker.rebind(&live_vals, merge_live);
        result_param
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
                self.consume(v);
            }
            self.emit_jump(header_block, initial_args.clone());
        }

        self.switch_to(header_block);
        self.rebind_scope_values(&initial_args, &header_params);
        // initial_args were consumed before the jump, so rebind won't
        // find them in scope. Track the header params fresh.
        for (i, &param) in header_params.iter().enumerate() {
            if descs[i].1 == Ownership::Owned {
                self.track_owned(param);
            } else {
                self.track_none(param);
            }
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
                self.consume(v);
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
            Some(label) => self.loop_stack.iter().rev()
                .find(|l| l.label.as_deref() == Some(label)),
            None => self.loop_stack.last(),
        }
    }

    fn collect_current_for_values(&self, expected: &[ValueId]) -> Vec<ValueId> {
        let all_tracked = self.all_live_tracked();
        if all_tracked.len() >= expected.len() {
            all_tracked[..expected.len()].iter().map(|&(v, _, _)| v).collect()
        } else {
            let mut vals: Vec<_> = all_tracked.iter().map(|&(v, _, _)| v).collect();
            while vals.len() < expected.len() {
                vals.push(expected[vals.len()]);
            }
            vals
        }
    }

    fn header_param_values(&self, header: BlockId) -> Vec<ValueId> {
        self.body.block(header).params.iter().map(|p| p.value).collect()
    }
}
