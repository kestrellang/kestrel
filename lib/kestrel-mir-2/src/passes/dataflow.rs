use std::collections::{HashMap, VecDeque};

use crate::body::MirBody;
use crate::terminator::TerminatorKind;
use crate::BlockId;

pub trait Lattice: Clone + PartialEq {
    /// The bottom element — uninitialized / no information.
    fn bottom() -> Self;
    /// Merge `other` into `self`. Returns true if `self` changed.
    fn join(&mut self, other: &Self) -> bool;
}

pub trait ForwardTransfer<S> {
    fn entry_state(&self, body: &MirBody) -> S;
    fn transfer_block(&self, body: &MirBody, block: BlockId, state: &mut S);
}

pub trait BackwardTransfer<S> {
    fn exit_state(&self, body: &MirBody) -> S;
    fn transfer_block(&self, body: &MirBody, block: BlockId, state: &mut S);
}

#[derive(Debug)]
pub struct CfgInfo {
    pub rpo: Vec<BlockId>,
    pub predecessors: HashMap<BlockId, Vec<BlockId>>,
}

pub fn compute_cfg_info(body: &MirBody) -> CfgInfo {
    let num_blocks = body.blocks.len();
    if num_blocks == 0 {
        return CfgInfo {
            rpo: Vec::new(),
            predecessors: HashMap::new(),
        };
    }

    // Iterative DFS for postorder, then reverse for RPO
    let mut visited = vec![false; num_blocks];
    let mut postorder = Vec::with_capacity(num_blocks);

    // Stack holds (block_id, children_pushed). When children_pushed is false,
    // we push successors. When true, we emit the block in postorder.
    let mut stack: Vec<(BlockId, bool)> = vec![(body.entry, false)];
    visited[body.entry.index()] = true;

    while let Some((block, children_pushed)) = stack.last_mut() {
        if *children_pushed {
            postorder.push(*block);
            stack.pop();
        } else {
            *children_pushed = true;
            let succs = body.block(*block).terminator.successors();
            for &succ in succs.iter().rev() {
                if !visited[succ.index()] {
                    visited[succ.index()] = true;
                    stack.push((succ, false));
                }
            }
        }
    }

    postorder.reverse();
    let rpo = postorder;

    // Build predecessor map
    let mut predecessors: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    for (i, block) in body.blocks.iter().enumerate() {
        let block_id = BlockId::new(i);
        for succ in block.terminator.successors() {
            predecessors.entry(succ).or_default().push(block_id);
        }
    }

    CfgInfo { rpo, predecessors }
}

/// Forward dataflow fixpoint. Returns one state per block (the entry state
/// of each block after convergence).
pub fn forward_fixpoint<S: Lattice>(
    cfg: &CfgInfo,
    body: &MirBody,
    transfer: &impl ForwardTransfer<S>,
) -> Vec<S> {
    let num_blocks = body.blocks.len();
    let mut entry_states = vec![S::bottom(); num_blocks];
    if num_blocks == 0 {
        return entry_states;
    }

    entry_states[body.entry.index()] = transfer.entry_state(body);

    // RPO position for worklist ordering
    let mut rpo_pos = vec![usize::MAX; num_blocks];
    for (pos, &block) in cfg.rpo.iter().enumerate() {
        rpo_pos[block.index()] = pos;
    }

    let mut in_queue = vec![false; num_blocks];
    let mut worklist: VecDeque<BlockId> = VecDeque::new();
    worklist.push_back(body.entry);
    in_queue[body.entry.index()] = true;

    while let Some(block) = worklist.pop_front() {
        in_queue[block.index()] = false;

        let mut exit_state = entry_states[block.index()].clone();
        transfer.transfer_block(body, block, &mut exit_state);

        for succ in body.block(block).terminator.successors() {
            if entry_states[succ.index()].join(&exit_state) && !in_queue[succ.index()] {
                in_queue[succ.index()] = true;
                if worklist.is_empty()
                    || rpo_pos[succ.index()] < rpo_pos[worklist.front().unwrap().index()]
                {
                    worklist.push_front(succ);
                } else {
                    worklist.push_back(succ);
                }
            }
        }
    }

    entry_states
}

/// Backward dataflow fixpoint. Returns one state per block (the exit state
/// of each block after convergence).
pub fn backward_fixpoint<S: Lattice>(
    cfg: &CfgInfo,
    body: &MirBody,
    transfer: &impl BackwardTransfer<S>,
) -> Vec<S> {
    let num_blocks = body.blocks.len();
    let mut exit_states = vec![S::bottom(); num_blocks];
    if num_blocks == 0 {
        return exit_states;
    }

    // Seed exit blocks with the exit state
    for (i, block) in body.blocks.iter().enumerate() {
        let is_exit = matches!(
            block.terminator.kind,
            TerminatorKind::Return(_) | TerminatorKind::Panic(_) | TerminatorKind::Unreachable
        );
        if is_exit {
            exit_states[i] = transfer.exit_state(body);
        }
    }

    // RPO position for worklist ordering
    let mut rpo_pos = vec![usize::MAX; num_blocks];
    for (pos, &block) in cfg.rpo.iter().enumerate() {
        rpo_pos[block.index()] = pos;
    }

    // Seed ALL reachable blocks in reverse RPO order (backward traversal
    // needs at least one pass over every block to compute entry states)
    let mut in_queue = vec![false; num_blocks];
    let mut worklist: VecDeque<BlockId> = VecDeque::new();
    for &block in cfg.rpo.iter().rev() {
        worklist.push_back(block);
        in_queue[block.index()] = true;
    }

    while let Some(block) = worklist.pop_front() {
        in_queue[block.index()] = false;

        let mut entry_state = exit_states[block.index()].clone();
        transfer.transfer_block(body, block, &mut entry_state);

        if let Some(preds) = cfg.predecessors.get(&block) {
            for &pred in preds {
                if exit_states[pred.index()].join(&entry_state) && !in_queue[pred.index()] {
                    in_queue[pred.index()] = true;
                    if worklist.is_empty()
                        || rpo_pos[pred.index()] > rpo_pos[worklist.front().unwrap().index()]
                    {
                        worklist.push_front(pred);
                    } else {
                        worklist.push_back(pred);
                    }
                }
            }
        }
    }

    exit_states
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::ModuleBuilder;
    use crate::immediate::Immediate;
    use crate::operand::{Operand, UseMode};
    use crate::place::Place;
    use crate::statement::Rvalue;
    use crate::{IntBits, Op, Signedness};

    // ---------------------------------------------------------------
    // Helper: extract MirBody from a built module
    // ---------------------------------------------------------------
    fn body_from(module: &crate::MirModule) -> &MirBody {
        module.functions[0].body.as_ref().unwrap()
    }

    // ---------------------------------------------------------------
    // Toy lattice: BitSet (union join)
    // ---------------------------------------------------------------
    #[derive(Clone, PartialEq, Debug)]
    struct BitSet {
        bits: Vec<bool>,
        size: usize,
    }

    impl BitSet {
        fn new(size: usize) -> Self {
            Self {
                bits: vec![false; size],
                size,
            }
        }
        fn set(&mut self, i: usize) {
            self.bits[i] = true;
        }
        fn get(&self, i: usize) -> bool {
            self.bits[i]
        }
    }

    impl Lattice for BitSet {
        fn bottom() -> Self {
            // Bottom with size 0 — ForwardTransfer::entry_state sets the real size.
            // join() handles mismatched sizes by growing.
            Self::new(0)
        }
        fn join(&mut self, other: &Self) -> bool {
            if self.bits.len() < other.bits.len() {
                self.bits.resize(other.bits.len(), false);
                self.size = other.size;
            }
            let mut changed = false;
            for (a, &b) in self.bits.iter_mut().zip(other.bits.iter()) {
                if !*a && b {
                    *a = true;
                    changed = true;
                }
            }
            changed
        }
    }

    // ---------------------------------------------------------------
    // CfgInfo tests
    // ---------------------------------------------------------------

    #[test]
    fn rpo_linear() {
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        let bb2 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.jump(bb1);
        }
        {
            let mut b = f.block_at(bb1);
            b.jump(bb2);
        }
        {
            let mut b = f.block_at(bb2);
            b.ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let cfg = compute_cfg_info(body);
        assert_eq!(cfg.rpo, vec![bb0, bb1, bb2]);
    }

    #[test]
    fn rpo_diamond() {
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let bool_ty = m.bool();
        let mut f = m.function("f", unit_ty);
        let cond = f.local("cond", bool_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        let bb2 = f.block_id();
        let bb3 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(cond), Immediate::bool(true));
            b.branch(Operand::Place(Place::local(cond)), bb1, bb2);
        }
        {
            let mut b = f.block_at(bb1);
            b.jump(bb3);
        }
        {
            let mut b = f.block_at(bb2);
            b.jump(bb3);
        }
        {
            let mut b = f.block_at(bb3);
            b.ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let cfg = compute_cfg_info(body);

        let pos = |id: BlockId| cfg.rpo.iter().position(|&b| b == id).unwrap();
        assert!(pos(bb0) < pos(bb1));
        assert!(pos(bb0) < pos(bb2));
        assert!(pos(bb1) < pos(bb3));
        assert!(pos(bb2) < pos(bb3));
    }

    #[test]
    fn rpo_loop() {
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let bool_ty = m.bool();
        let mut f = m.function("f", unit_ty);
        let cond = f.local("cond", bool_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        let bb2 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.jump(bb1);
        }
        {
            let mut b = f.block_at(bb1);
            b.assign_const(Place::local(cond), Immediate::bool(true));
            b.branch(Operand::Place(Place::local(cond)), bb0, bb2);
        }
        {
            let mut b = f.block_at(bb2);
            b.ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let cfg = compute_cfg_info(body);

        let pos = |id: BlockId| cfg.rpo.iter().position(|&b| b == id).unwrap();
        assert!(pos(bb0) < pos(bb1));
        assert!(pos(bb1) < pos(bb2));
    }

    #[test]
    fn rpo_unreachable_excluded() {
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let bb0 = f.block_id();
        let _bb_orphan = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let cfg = compute_cfg_info(body);
        assert_eq!(cfg.rpo.len(), 1);
        assert_eq!(cfg.rpo[0], bb0);
    }

    #[test]
    fn predecessors_diamond() {
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let bool_ty = m.bool();
        let mut f = m.function("f", unit_ty);
        let cond = f.local("cond", bool_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        let bb2 = f.block_id();
        let bb3 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(cond), Immediate::bool(true));
            b.branch(Operand::Place(Place::local(cond)), bb1, bb2);
        }
        {
            let mut b = f.block_at(bb1);
            b.jump(bb3);
        }
        {
            let mut b = f.block_at(bb2);
            b.jump(bb3);
        }
        {
            let mut b = f.block_at(bb3);
            b.ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let cfg = compute_cfg_info(body);

        assert!(!cfg.predecessors.contains_key(&bb0));
        let preds_bb3 = cfg.predecessors.get(&bb3).unwrap();
        assert!(preds_bb3.contains(&bb1));
        assert!(preds_bb3.contains(&bb2));
        assert_eq!(preds_bb3.len(), 2);
    }

    #[test]
    fn predecessors_loop() {
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let bool_ty = m.bool();
        let mut f = m.function("f", unit_ty);
        let cond = f.local("cond", bool_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        let bb2 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.jump(bb1);
        }
        {
            let mut b = f.block_at(bb1);
            b.assign_const(Place::local(cond), Immediate::bool(true));
            b.branch(Operand::Place(Place::local(cond)), bb0, bb2);
        }
        {
            let mut b = f.block_at(bb2);
            b.ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let cfg = compute_cfg_info(body);

        let preds_bb0 = cfg.predecessors.get(&bb0).unwrap();
        assert!(preds_bb0.contains(&bb1)); // back-edge
    }

    // ---------------------------------------------------------------
    // Forward fixpoint tests
    // ---------------------------------------------------------------

    // Forward transfer: marks each block as reached
    struct MarkReached(usize);
    impl ForwardTransfer<BitSet> for MarkReached {
        fn entry_state(&self, _body: &MirBody) -> BitSet {
            let mut s = BitSet::new(self.0);
            s.set(0); // mark entry reached
            s
        }
        fn transfer_block(&self, _body: &MirBody, block: BlockId, state: &mut BitSet) {
            state.set(block.index());
        }
    }

    #[test]
    fn forward_linear() {
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        let bb2 = f.block_id();
        {
            f.block_at(bb0).jump(bb1);
        }
        {
            f.block_at(bb1).jump(bb2);
        }
        {
            f.block_at(bb2).ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let cfg = compute_cfg_info(body);

        let states = forward_fixpoint(&cfg, body, &MarkReached(3));
        // bb2's entry state should have bb0 and bb1 marked (propagated through)
        assert!(states[bb2.index()].get(0)); // entry reached
        assert!(states[bb2.index()].get(1)); // bb1 reached
    }

    #[test]
    fn forward_diamond_join() {
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let bool_ty = m.bool();
        let mut f = m.function("f", unit_ty);
        let cond = f.local("cond", bool_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        let bb2 = f.block_id();
        let bb3 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(cond), Immediate::bool(true));
            b.branch(Operand::Place(Place::local(cond)), bb1, bb2);
        }
        {
            f.block_at(bb1).jump(bb3);
        }
        {
            f.block_at(bb2).jump(bb3);
        }
        {
            f.block_at(bb3).ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let cfg = compute_cfg_info(body);

        // Transfer that sets a unique bit per block
        struct SetBlockBit;
        impl ForwardTransfer<BitSet> for SetBlockBit {
            fn entry_state(&self, _body: &MirBody) -> BitSet {
                BitSet::new(4)
            }
            fn transfer_block(&self, _body: &MirBody, block: BlockId, state: &mut BitSet) {
                state.set(block.index());
            }
        }

        let states = forward_fixpoint(&cfg, body, &SetBlockBit);
        // bb3 should have bits for bb1 AND bb2 (union of both paths)
        assert!(states[bb3.index()].get(1)); // from bb1 path
        assert!(states[bb3.index()].get(2)); // from bb2 path
    }

    #[test]
    fn forward_loop_converges() {
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let bool_ty = m.bool();
        let mut f = m.function("f", unit_ty);
        let cond = f.local("cond", bool_ty);
        // bb0 → bb1 → bb2, bb2 → bb1 (loop), bb2 → bb3 (exit)
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        let bb2 = f.block_id();
        let bb3 = f.block_id();
        {
            f.block_at(bb0).jump(bb1);
        }
        {
            f.block_at(bb1).jump(bb2);
        }
        {
            let mut b = f.block_at(bb2);
            b.assign_const(Place::local(cond), Immediate::bool(true));
            b.branch(Operand::Place(Place::local(cond)), bb1, bb3);
        }
        {
            f.block_at(bb3).ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let cfg = compute_cfg_info(body);

        let states = forward_fixpoint(&cfg, body, &MarkReached(4));
        // After convergence, bb1 entry should see bb0 AND bb2 (from back-edge)
        assert!(states[bb1.index()].get(0));
        assert!(states[bb1.index()].get(2)); // back-edge propagates bb2's mark
    }

    #[test]
    fn forward_unreachable_stays_bottom() {
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let bb0 = f.block_id();
        let _bb_orphan = f.block_id();
        {
            f.block_at(bb0).ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let cfg = compute_cfg_info(body);

        let states = forward_fixpoint(&cfg, body, &MarkReached(2));
        // Orphan block stays at bottom (size 0, never joined)
        assert_eq!(states[1], BitSet::bottom());
    }

    // ---------------------------------------------------------------
    // Backward fixpoint tests
    // ---------------------------------------------------------------

    // Backward transfer: for each statement, gen reads and kill writes
    struct LivenessTransfer {
        num_locals: usize,
    }

    impl BackwardTransfer<BitSet> for LivenessTransfer {
        fn exit_state(&self, _body: &MirBody) -> BitSet {
            BitSet::new(self.num_locals)
        }

        fn transfer_block(&self, body: &MirBody, block: BlockId, state: &mut BitSet) {
            let bb = body.block(block);

            // Process terminator: Return(Place(x)) gens x
            if let TerminatorKind::Return(Operand::Place(p)) = &bb.terminator.kind {
                if let Some(local) = p.root_local() {
                    state.set(local.index());
                }
            }

            // Process statements backward
            for stmt in bb.stmts.iter().rev() {
                match &stmt.kind {
                    crate::StatementKind::Assign { dest, rvalue } => {
                        // Kill the destination
                        if let Some(local) = dest.root_local() {
                            state.bits[local.index()] = false;
                        }
                        // Gen all operand reads
                        for op in rvalue.operands() {
                            if let Operand::Place(p) = op {
                                if let Some(local) = p.root_local() {
                                    state.set(local.index());
                                }
                            }
                        }
                        // Gen referenced places (Ref/RefMut)
                        for p in rvalue.referenced_places() {
                            if let Some(local) = p.root_local() {
                                state.set(local.index());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn backward_linear() {
        // bb0: x = 0
        // bb1: y = x + 1
        // bb2: return y
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let mut f = m.function("f", i64_ty);
        let x = f.local("x", i64_ty);
        let y = f.local("y", i64_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        let bb2 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.jump(bb1);
        }
        {
            let mut b = f.block_at(bb1);
            b.assign_op2(
                Place::local(y),
                Op::Add(IntBits::I64, Signedness::Signed),
                Operand::Place(Place::local(x)),
                Operand::Const(Immediate::i64(1)),
            );
            b.jump(bb2);
        }
        {
            f.block_at(bb2).ret(Operand::Place(Place::local(y)));
        }
        let module = m.finish();
        let body = body_from(&module);
        let cfg = compute_cfg_info(body);

        let transfer = LivenessTransfer { num_locals: body.locals.len() };
        let states = backward_fixpoint(&cfg, body, &transfer);

        // x should be live at bb0's exit (read in bb1)
        assert!(states[bb0.index()].get(x.index()));
        // y should be live at bb1's exit (read in bb2's return)
        assert!(states[bb1.index()].get(y.index()));
        // x should NOT be live at bb2's exit (not read after bb1)
        assert!(!states[bb2.index()].get(x.index()));
    }

    #[test]
    fn backward_diamond() {
        // bb0: branch → bb1, bb2
        // bb1: read x, jump bb3
        // bb2: jump bb3 (no read of x)
        // bb3: return ()
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let i64_ty = m.i64();
        let bool_ty = m.bool();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", i64_ty);
        let cond = f.local("cond", bool_ty);
        let _tmp = f.local("tmp", i64_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        let bb2 = f.block_id();
        let bb3 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(cond), Immediate::bool(true));
            b.branch(Operand::Place(Place::local(cond)), bb1, bb2);
        }
        {
            let mut b = f.block_at(bb1);
            // Read x
            b.assign(
                Place::local(_tmp),
                Rvalue::Use(Operand::Place(Place::local(x)), UseMode::Copy),
            );
            b.jump(bb3);
        }
        {
            f.block_at(bb2).jump(bb3);
        }
        {
            f.block_at(bb3).ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let cfg = compute_cfg_info(body);

        let transfer = LivenessTransfer { num_locals: body.locals.len() };
        let states = backward_fixpoint(&cfg, body, &transfer);

        // x is live on bb1 path → union means x is live at bb0's exit
        assert!(states[bb0.index()].get(x.index()));
    }

    #[test]
    fn backward_loop() {
        // bb0: jump bb1
        // bb1: tmp = x, branch → bb1 (back), bb2 (exit)
        // bb2: return ()
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let i64_ty = m.i64();
        let bool_ty = m.bool();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", i64_ty);
        let tmp = f.local("tmp", i64_ty);
        let cond = f.local("cond", bool_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        let bb2 = f.block_id();
        {
            f.block_at(bb0).jump(bb1);
        }
        {
            let mut b = f.block_at(bb1);
            b.assign(
                Place::local(tmp),
                Rvalue::Use(Operand::Place(Place::local(x)), UseMode::Copy),
            );
            b.assign_const(Place::local(cond), Immediate::bool(true));
            b.branch(Operand::Place(Place::local(cond)), bb1, bb2);
        }
        {
            f.block_at(bb2).ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let cfg = compute_cfg_info(body);

        let transfer = LivenessTransfer { num_locals: body.locals.len() };
        let states = backward_fixpoint(&cfg, body, &transfer);

        // x is read in bb1 loop body → live at bb0's exit via back-edge propagation
        assert!(states[bb0.index()].get(x.index()));
        // x is live at bb1's exit too (back-edge means it's read again)
        assert!(states[bb1.index()].get(x.index()));
    }

    #[test]
    fn backward_dead_local() {
        // bb0: x = 0
        // bb1: return ()
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let i64_ty = m.i64();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", i64_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.jump(bb1);
        }
        {
            f.block_at(bb1).ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let cfg = compute_cfg_info(body);

        let transfer = LivenessTransfer { num_locals: body.locals.len() };
        let states = backward_fixpoint(&cfg, body, &transfer);

        // x is written but never read → not live anywhere
        assert!(!states[bb0.index()].get(x.index()));
        assert!(!states[bb1.index()].get(x.index()));
    }

    // ---------------------------------------------------------------
    // Edge cases
    // ---------------------------------------------------------------

    #[test]
    fn single_block() {
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        {
            f.block().ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let cfg = compute_cfg_info(body);
        assert_eq!(cfg.rpo.len(), 1);
        assert!(cfg.predecessors.is_empty());

        let states = forward_fixpoint(&cfg, body, &MarkReached(1));
        assert!(states[0].get(0));
    }

    #[test]
    fn empty_body() {
        let body = MirBody::new();
        let cfg = compute_cfg_info(&body);
        assert!(cfg.rpo.is_empty());
        assert!(cfg.predecessors.is_empty());
    }

    #[test]
    fn self_loop_converges() {
        // bb0: branch → bb0 (self-loop), bb1
        // bb1: return ()
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let bool_ty = m.bool();
        let mut f = m.function("f", unit_ty);
        let cond = f.local("cond", bool_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(cond), Immediate::bool(true));
            b.branch(Operand::Place(Place::local(cond)), bb0, bb1);
        }
        {
            f.block_at(bb1).ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let cfg = compute_cfg_info(body);

        // Must terminate, not infinite-loop
        let states = forward_fixpoint(&cfg, body, &MarkReached(2));
        assert!(states[bb0.index()].get(0));
        assert!(states[bb1.index()].get(0));
    }
}
