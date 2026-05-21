use crate::body::MirBody;
use crate::operand::Operand;
use crate::statement::{Rvalue, StatementKind};
use crate::terminator::TerminatorKind;
use crate::{BlockId, LocalId};

use super::dataflow::{self, BackwardTransfer, CfgInfo, Lattice};

// ---- Bit-vector lattice ----

#[derive(Clone, Debug)]
pub struct BitVec {
    words: Vec<u64>,
    num_bits: usize,
}

impl BitVec {
    pub fn new(num_bits: usize) -> Self {
        let num_words = num_bits.div_ceil(64);
        Self {
            words: vec![0; num_words],
            num_bits,
        }
    }

    pub fn set(&mut self, bit: usize) {
        let word = bit / 64;
        if word >= self.words.len() {
            self.words.resize(word + 1, 0);
            self.num_bits = self.num_bits.max(bit + 1);
        }
        self.words[word] |= 1u64 << (bit % 64);
    }

    pub fn clear(&mut self, bit: usize) {
        let word = bit / 64;
        if word >= self.words.len() {
            return;
        }
        self.words[word] &= !(1u64 << (bit % 64));
    }

    pub fn get(&self, bit: usize) -> bool {
        if bit / 64 >= self.words.len() {
            return false;
        }
        (self.words[bit / 64] >> (bit % 64)) & 1 != 0
    }

    fn union_with(&mut self, other: &BitVec) -> bool {
        if self.words.len() < other.words.len() {
            self.words.resize(other.words.len(), 0);
            self.num_bits = other.num_bits;
        }
        let mut changed = false;
        for (a, &b) in self.words.iter_mut().zip(other.words.iter()) {
            let old = *a;
            *a |= b;
            changed |= *a != old;
        }
        changed
    }
}

impl PartialEq for BitVec {
    fn eq(&self, other: &Self) -> bool {
        self.words == other.words
    }
}

impl Lattice for BitVec {
    fn bottom() -> Self {
        Self::new(0)
    }

    fn join(&mut self, other: &Self) -> bool {
        self.union_with(other)
    }
}

// ---- Transfer ----

struct LivenessTransfer {
    num_locals: usize,
}

impl BackwardTransfer<BitVec> for LivenessTransfer {
    fn exit_state(&self, _body: &MirBody) -> BitVec {
        BitVec::new(self.num_locals)
    }

    fn transfer_block(&self, body: &MirBody, block: BlockId, state: &mut BitVec) {
        let bb = body.block(block);
        transfer_terminator(state, &bb.terminator.kind);
        for stmt in bb.stmts.iter().rev() {
            transfer_statement(state, &stmt.kind);
        }
    }
}

fn transfer_statement(live: &mut BitVec, kind: &StatementKind) {
    match kind {
        StatementKind::Assign { dest, rvalue } => {
            // Kill dest first, then gen reads — `x = copy x` should gen x
            if let Some(id) = dest.root_local() {
                live.clear(id.index());
            }
            gen_rvalue(live, rvalue);
        }
        StatementKind::Call { dest, callee, args } => {
            if let Some(id) = dest.as_ref().and_then(|d| d.root_local()) {
                live.clear(id.index());
            }
            for (operand, _) in args {
                gen_operand(live, operand);
            }
            // Indirect call targets are live reads
            match callee {
                crate::Callee::Thin(place) | crate::Callee::Thick(place) => {
                    gen_place(live, place);
                }
                _ => {}
            }
        }
        StatementKind::Drop { place } => {
            gen_place(live, place);
        }
        StatementKind::DropIf { place, flag } => {
            gen_place(live, place);
            live.set(flag.index());
        }
        StatementKind::SetDropFlag { flag, .. } => {
            live.clear(flag.index());
        }
        StatementKind::ScopeLive(local) => {
            live.set(local.index());
        }
    }
}

fn transfer_terminator(live: &mut BitVec, kind: &TerminatorKind) {
    match kind {
        TerminatorKind::Return(op) => gen_operand(live, op),
        TerminatorKind::Branch { condition, .. } => gen_operand(live, condition),
        TerminatorKind::Switch { discriminant, .. } => gen_place(live, discriminant),
        TerminatorKind::Jump(_) | TerminatorKind::Panic(_) | TerminatorKind::Unreachable => {}
    }
}

fn gen_operand(live: &mut BitVec, operand: &Operand) {
    if let Operand::Place(place) = operand {
        gen_place(live, place);
    }
}

fn gen_place(live: &mut BitVec, place: &crate::Place) {
    if let Some(id) = place.root_local() {
        live.set(id.index());
    }
}

fn gen_rvalue(live: &mut BitVec, rvalue: &Rvalue) {
    for op in rvalue.operands() {
        gen_operand(live, op);
    }
    for place in rvalue.referenced_places() {
        gen_place(live, place);
    }
}

// ---- Public API ----

pub struct Liveness {
    /// Per-block exit states (liveness at the bottom of each block,
    /// after all statements but before the terminator takes effect on
    /// control flow).
    exit_states: Vec<BitVec>,
    num_locals: usize,
}

impl Liveness {
    pub fn compute(body: &MirBody) -> Self {
        let cfg = dataflow::compute_cfg_info(body);
        Self::compute_with_cfg(body, &cfg)
    }

    pub fn compute_with_cfg(body: &MirBody, cfg: &CfgInfo) -> Self {
        let num_locals = body.locals.len();
        let transfer = LivenessTransfer { num_locals };
        let exit_states = dataflow::backward_fixpoint(cfg, body, &transfer);
        Self {
            exit_states,
            num_locals,
        }
    }

    /// Precompute liveness-after for every statement in a block.
    ///
    /// Returns a vec where `result[i].get(local)` is true iff `local` is
    /// live immediately after statement `i`. Call BEFORE modifying the block.
    pub fn block_liveness_after(&self, body: &MirBody, block: BlockId) -> Vec<BitVec> {
        let bb = body.block(block);
        let num_stmts = bb.stmts.len();
        let mut result = Vec::with_capacity(num_stmts);
        result.resize_with(num_stmts, || BitVec::new(self.num_locals));

        let mut live = self.exit_states[block.index()].clone();
        transfer_terminator(&mut live, &bb.terminator.kind);

        for i in (0..num_stmts).rev() {
            result[i] = live.clone();
            transfer_statement(&mut live, &bb.stmts[i].kind);
        }

        result
    }

    /// Is `local` live immediately after statement `stmt_index` in `block`?
    pub fn is_live_after(
        &self,
        body: &MirBody,
        block: BlockId,
        stmt_index: usize,
        local: LocalId,
    ) -> bool {
        let after = self.block_liveness_after(body, block);
        after[stmt_index].get(local.index())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::ModuleBuilder;
    use crate::immediate::Immediate;
    use crate::operand::{ArgMode, UseMode};
    use crate::place::Place;
    use crate::statement::{Callee, Rvalue};
    use crate::{IntBits, MirModule, Op, Signedness};

    fn body_from(module: &MirModule) -> &MirBody {
        module.functions[0].body.as_ref().unwrap()
    }

    // ---- Basic liveness ----

    #[test]
    fn live_after_read() {
        // bb0: x = 0; y = x + 1; return y
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let mut f = m.function("f", i64_ty);
        let x = f.local("x", i64_ty);
        let y = f.local("y", i64_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));          // stmt 0
            b.assign_op2(                                                  // stmt 1
                Place::local(y),
                Op::Add(IntBits::I64, Signedness::Signed),
                Operand::Place(Place::local(x)),
                Operand::Const(Immediate::i64(1)),
            );
            b.ret(Operand::Place(Place::local(y)));
        }
        let module = m.finish();
        let body = body_from(&module);
        let liveness = Liveness::compute(body);

        // After stmt 0 (x = 0): x is live (read in stmt 1)
        assert!(liveness.is_live_after(body, bb0, 0, x));
        // After stmt 0: y is NOT live yet
        assert!(!liveness.is_live_after(body, bb0, 0, y));
        // After stmt 1 (y = x + 1): y is live (returned), x is dead
        assert!(liveness.is_live_after(body, bb0, 1, y));
        assert!(!liveness.is_live_after(body, bb0, 1, x));
    }

    #[test]
    fn dead_local_never_live() {
        // bb0: x = 0; return ()
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let i64_ty = m.i64();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", i64_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0)); // stmt 0
            b.ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let liveness = Liveness::compute(body);

        // x is written but never read
        assert!(!liveness.is_live_after(body, bb0, 0, x));
    }

    #[test]
    fn last_use_is_dead_after() {
        // bb0: x = 0; y = x; return y
        // After "y = x", x's only use is consumed — x should be dead
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let mut f = m.function("f", i64_ty);
        let x = f.local("x", i64_ty);
        let y = f.local("y", i64_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));         // stmt 0
            b.assign(                                                     // stmt 1
                Place::local(y),
                Rvalue::Use(Operand::Place(Place::local(x)), UseMode::Copy),
            );
            b.ret(Operand::Place(Place::local(y)));
        }
        let module = m.finish();
        let body = body_from(&module);
        let liveness = Liveness::compute(body);

        // After stmt 1 (y = copy x): x is dead (no more reads), y is live
        assert!(!liveness.is_live_after(body, bb0, 1, x));
        assert!(liveness.is_live_after(body, bb0, 1, y));
    }

    // ---- Control flow ----

    #[test]
    fn diamond_live_on_one_path() {
        // bb0: branch → bb1, bb2
        // bb1: tmp = copy x; jump bb3
        // bb2: jump bb3
        // bb3: return ()
        // x is live at bb0 exit because bb1 reads it
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let i64_ty = m.i64();
        let bool_ty = m.bool();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", i64_ty);
        let cond = f.local("cond", bool_ty);
        let tmp = f.local("tmp", i64_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        let bb2 = f.block_id();
        let bb3 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(cond), Immediate::bool(true));     // stmt 0
            b.branch(Operand::Place(Place::local(cond)), bb1, bb2);
        }
        {
            let mut b = f.block_at(bb1);
            b.assign(                                                       // stmt 0
                Place::local(tmp),
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
        let liveness = Liveness::compute(body);

        // x is live after stmt 0 of bb0 (union: live on bb1 path)
        assert!(liveness.is_live_after(body, bb0, 0, x));
    }

    #[test]
    fn loop_keeps_local_live() {
        // bb0: x = 0; jump bb1
        // bb1: tmp = copy x; branch → bb1, bb2
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
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0)); // stmt 0
            b.jump(bb1);
        }
        {
            let mut b = f.block_at(bb1);
            b.assign(                                             // stmt 0
                Place::local(tmp),
                Rvalue::Use(Operand::Place(Place::local(x)), UseMode::Copy),
            );
            b.assign_const(Place::local(cond), Immediate::bool(true)); // stmt 1
            b.branch(Operand::Place(Place::local(cond)), bb1, bb2);
        }
        {
            f.block_at(bb2).ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let liveness = Liveness::compute(body);

        // x is live after stmt 0 of bb0 (loop back-edge keeps it live)
        assert!(liveness.is_live_after(body, bb0, 0, x));
        // x is live after stmt 0 of bb1 (back-edge: read again next iteration)
        assert!(liveness.is_live_after(body, bb1, 0, x));
    }

    // ---- Self-assign: x = copy x ----

    #[test]
    fn self_assign_keeps_live() {
        // bb0: x = 0; x = copy x; return x
        // After "x = copy x": x is live (returned). The gen from reading x
        // must happen after the kill from writing x.
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let mut f = m.function("f", i64_ty);
        let x = f.local("x", i64_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));         // stmt 0
            b.assign(                                                     // stmt 1
                Place::local(x),
                Rvalue::Use(Operand::Place(Place::local(x)), UseMode::Copy),
            );
            b.ret(Operand::Place(Place::local(x)));
        }
        let module = m.finish();
        let body = body_from(&module);
        let liveness = Liveness::compute(body);

        // After stmt 1 (x = copy x): x is live (returned)
        assert!(liveness.is_live_after(body, bb0, 1, x));
        // After stmt 0 (x = 0): x is live (read in stmt 1)
        assert!(liveness.is_live_after(body, bb0, 0, x));
    }

    // ---- Call statement ----

    #[test]
    fn call_kills_dest_gens_args() {
        // bb0: result = call f(x) [ref]; return result
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let callee = m.fresh_entity();
        let mut f = m.function("g", i64_ty);
        let x = f.local("x", i64_ty);
        let result = f.local("result", i64_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.call(                                                        // stmt 0
                Some(Place::local(result)),
                Callee::direct(callee),
                vec![(Operand::Place(Place::local(x)), ArgMode::Ref)],
            );
            b.ret(Operand::Place(Place::local(result)));
        }
        let module = m.finish();
        let body = body_from(&module);
        let liveness = Liveness::compute(body);

        // After stmt 0: result is live (returned), x is dead (last use was arg)
        assert!(liveness.is_live_after(body, bb0, 0, result));
        assert!(!liveness.is_live_after(body, bb0, 0, x));
    }

    // ---- Drop/DropIf ----

    #[test]
    fn drop_gens_place() {
        // bb0: x = 0; drop x; return ()
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let i64_ty = m.i64();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", i64_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0)); // stmt 0
            b.drop(Place::local(x));                              // stmt 1
            b.ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let liveness = Liveness::compute(body);

        // After stmt 0: x is live (drop reads it)
        assert!(liveness.is_live_after(body, bb0, 0, x));
        // After stmt 1: x is dead (drop consumed it)
        assert!(!liveness.is_live_after(body, bb0, 1, x));
    }

    #[test]
    fn drop_if_gens_place_and_flag() {
        // bb0: flag = true; x = 0; drop x if flag; return ()
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let i64_ty = m.i64();
        let bool_ty = m.bool();
        let mut f = m.function("f", unit_ty);
        let flag = f.local("flag", bool_ty);
        let x = f.local("x", i64_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.set_drop_flag(flag, true);                          // stmt 0
            b.assign_const(Place::local(x), Immediate::i64(0));  // stmt 1
            b.drop_if(Place::local(x), flag);                     // stmt 2
            b.ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let liveness = Liveness::compute(body);

        // After stmt 1: x is live (drop_if reads it), flag is live (drop_if reads it)
        assert!(liveness.is_live_after(body, bb0, 1, x));
        assert!(liveness.is_live_after(body, bb0, 1, flag));
    }

    // ---- Ref/RefMut gens the place ----

    #[test]
    fn ref_gens_place() {
        // bb0: x = 0; r = ref x; return ()
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let i64_ty = m.i64();
        let ptr_ty = m.pointer(i64_ty);
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", i64_ty);
        let r = f.local("r", ptr_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0)); // stmt 0
            b.assign_ref(Place::local(r), Place::local(x));       // stmt 1
            b.ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let liveness = Liveness::compute(body);

        // After stmt 0: x is live (ref reads it)
        assert!(liveness.is_live_after(body, bb0, 0, x));
    }

    // ---- block_liveness_after ----

    #[test]
    fn block_liveness_after_precompute() {
        // bb0: x = 0; y = x + 1; return y
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let mut f = m.function("f", i64_ty);
        let x = f.local("x", i64_ty);
        let y = f.local("y", i64_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.assign_op2(
                Place::local(y),
                Op::Add(IntBits::I64, Signedness::Signed),
                Operand::Place(Place::local(x)),
                Operand::Const(Immediate::i64(1)),
            );
            b.ret(Operand::Place(Place::local(y)));
        }
        let module = m.finish();
        let body = body_from(&module);
        let liveness = Liveness::compute(body);

        let after = liveness.block_liveness_after(body, bb0);
        assert_eq!(after.len(), 2);
        // after[0]: liveness after stmt 0 (x = 0)
        assert!(after[0].get(x.index()));   // x live
        assert!(!after[0].get(y.index()));  // y not live
        // after[1]: liveness after stmt 1 (y = x + 1)
        assert!(!after[1].get(x.index())); // x dead
        assert!(after[1].get(y.index()));  // y live
    }

    // ---- Return gens operand ----

    #[test]
    fn return_gens_place() {
        // bb0: x = 0; return x
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let mut f = m.function("f", i64_ty);
        let x = f.local("x", i64_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0)); // stmt 0
            b.ret(Operand::Place(Place::local(x)));
        }
        let module = m.finish();
        let body = body_from(&module);
        let liveness = Liveness::compute(body);

        // After stmt 0: x is live (returned)
        assert!(liveness.is_live_after(body, bb0, 0, x));
    }

    // ---- Branch condition is live ----

    #[test]
    fn branch_condition_is_live() {
        // bb0: cond = true; branch cond → bb1, bb1
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
            b.assign_const(Place::local(cond), Immediate::bool(true)); // stmt 0
            b.branch(Operand::Place(Place::local(cond)), bb1, bb1);
        }
        {
            f.block_at(bb1).ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let liveness = Liveness::compute(body);

        // After stmt 0: cond is live (used by branch terminator)
        assert!(liveness.is_live_after(body, bb0, 0, cond));
    }
}
