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
        self.tracker = super::LiveTracker::from_live(&self.all_live_owned());
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
            self.destroy_scope_except(&[then_val]);
            let mut args = vec![then_val];
            args.extend(self.tracker.values());
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
                self.destroy_scope_except(&[else_val]);
                let mut args = vec![else_val];
                args.extend(self.tracker.values());
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
        self.tracker = super::LiveTracker::from_live(&self.all_live_owned());
        let initial_args = self.tracker.values();

        let (header_block, header_params) = self.new_block_with_params(&self.tracker.descs());
        let exit_block = self.new_block();

        if !self.is_terminated() {
            for &v in &initial_args {
                self.consume(v);
            }
            self.emit_jump(header_block, initial_args.clone());
        }

        self.switch_to(header_block);
        self.rebind_scope_values(&initial_args, &header_params);
        for &param in &header_params {
            self.track_owned(param);
        }

        let scope_depth = self.scope_stack.len();
        self.loop_stack.push(LoopInfo {
            header_block,
            exit_block,
            label: label.map(|s| s.to_string()),
            scope_depth,
        });

        self.push_scope();
        let _ = self.lower_hir_block(body);

        if !self.is_terminated() {
            self.exit_scope();
            // Tracker has the current versions of header values,
            // updated by nested if/match merges via rebind_scope_values.
            let back_edge_vals = self.tracker.values();
            for &v in &back_edge_vals {
                self.consume(v);
            }
            self.emit_jump(header_block, back_edge_vals);
        } else {
            self.pop_scope();
        }

        self.loop_stack.pop();
        self.switch_to(exit_block);
        self.tracker = saved_tracker;
        self.emit_literal(Immediate::unit())
    }

    // ================================================================
    // Break
    // ================================================================

    pub fn lower_break(&mut self, label: Option<&str>) -> ValueId {
        if let Some(info) = self.find_loop(label) {
            let exit = info.exit_block;
            let depth = info.scope_depth;
            // Destroy all scopes from current down to (and including) loop scope
            self.destroy_scopes_to_depth(depth, &[]);
            // Collect and destroy values in scopes above the loop (outer owned)
            let outer_to_destroy: Vec<ValueId> = self.scope_stack[..depth]
                .iter()
                .flat_map(|s| s.owned_values.iter().rev().copied())
                .collect();
            for value in outer_to_destroy {
                self.push_inst(kestrel_mir_3::inst::InstKind::DestroyValue { operand: value });
            }
            self.emit_jump(exit, vec![]);
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

    /// Get the current version of each value in `original`. The values
    /// may have been rebound during arm lowering (via assign).
    pub(crate) fn current_live_matching(&self, original: &[ValueId]) -> Vec<ValueId> {
        original.iter().map(|&orig| {
            // Check if the value is still tracked in any scope
            for scope in self.scope_stack.iter().rev() {
                if scope.owned_values.contains(&orig) {
                    return orig;
                }
            }
            // Value was consumed during the arm — it's no longer live.
            // The arm must have destroyed it, so we need to handle this.
            // For now, return the original — the verifier will catch
            // any inconsistency.
            orig
        }).collect()
    }

    pub(crate) fn collect_current_for(&self, header_params: &[ValueId]) -> Vec<ValueId> {
        let all_owned = self.all_live_owned();
        if all_owned.len() >= header_params.len() {
            all_owned[..header_params.len()].iter().map(|&(v, _)| v).collect()
        } else {
            let mut vals: Vec<_> = all_owned.iter().map(|&(v, _)| v).collect();
            while vals.len() < header_params.len() {
                vals.push(header_params[vals.len()]);
            }
            vals
        }
    }

    fn collect_current_for_values(&self, expected: &[ValueId]) -> Vec<ValueId> {
        let all_owned = self.all_live_owned();
        if all_owned.len() >= expected.len() {
            all_owned[..expected.len()].iter().map(|&(v, _)| v).collect()
        } else {
            let mut vals: Vec<_> = all_owned.iter().map(|&(v, _)| v).collect();
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
