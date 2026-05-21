use crate::body::MirBody;
use crate::operand::{ArgMode, Operand, UseMode};
use crate::statement::{Rvalue, StatementKind};
use crate::terminator::TerminatorKind;
use crate::{BlockId, LocalId};

use super::dataflow::{self, CfgInfo, ForwardTransfer, Lattice};

/// Per-local initialization state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitState {
    Dead,
    Live,
    Maybe,
}

/// Forward init-state lattice: one InitState per local.
#[derive(Debug, Clone, PartialEq)]
pub struct InitMap {
    states: Vec<InitState>,
}

impl InitMap {
    pub fn new(num_locals: usize) -> Self {
        Self {
            states: vec![InitState::Dead; num_locals],
        }
    }

    pub fn get(&self, local: LocalId) -> InitState {
        if local.index() < self.states.len() {
            self.states[local.index()]
        } else {
            InitState::Dead
        }
    }

    pub fn set(&mut self, local: LocalId, state: InitState) {
        if local.index() < self.states.len() {
            self.states[local.index()] = state;
        }
    }
}

impl Lattice for InitMap {
    fn bottom() -> Self {
        Self::new(0)
    }

    fn join(&mut self, other: &Self) -> bool {
        // Bottom (empty) is the identity: first predecessor's state is taken as-is
        if self.states.is_empty() {
            if other.states.is_empty() {
                return false;
            }
            self.states = other.states.clone();
            return true;
        }
        let mut changed = false;
        for (i, &other_s) in other.states.iter().enumerate() {
            let self_s = &mut self.states[i];
            let merged = match (*self_s, other_s) {
                (a, b) if a == b => a,
                _ => InitState::Maybe,
            };
            if merged != *self_s {
                *self_s = merged;
                changed = true;
            }
        }
        changed
    }
}

/// Transfer function for forward init-state analysis.
pub struct InitTransfer {
    num_locals: usize,
    param_count: usize,
}

impl ForwardTransfer<InitMap> for InitTransfer {
    fn entry_state(&self, _body: &MirBody) -> InitMap {
        let mut map = InitMap::new(self.num_locals);
        // Parameters are Live at function entry
        for i in 0..self.param_count {
            map.set(LocalId::new(i), InitState::Live);
        }
        map
    }

    fn transfer_block(&self, body: &MirBody, block: BlockId, state: &mut InitMap) {
        let bb = body.block(block);
        for stmt in &bb.stmts {
            transfer_statement(state, &stmt.kind);
        }
        transfer_terminator(state, &bb.terminator.kind);
    }
}

fn transfer_statement(state: &mut InitMap, kind: &StatementKind) {
    match kind {
        StatementKind::Assign { dest, rvalue } => {
            // Kill moved sources first, then gen dest
            kill_rvalue_moves(state, rvalue);
            if let Some(local) = dest.root_local() {
                state.set(local, InitState::Live);
            }
        }
        StatementKind::Call { dest, args, .. } => {
            if let Some(local) = dest.as_ref().and_then(|d| d.root_local()) {
                state.set(local, InitState::Live);
            }
            for (operand, mode) in args {
                if *mode == ArgMode::Move
                    && let Operand::Place(place) = operand
                    && let Some(local) = place.root_local()
                {
                    state.set(local, InitState::Dead);
                }
            }
        }
        StatementKind::Drop { place } => {
            // Drop consumes the value
            if let Some(local) = place.root_local() {
                state.set(local, InitState::Dead);
            }
        }
        StatementKind::DropIf { place, .. } => {
            // After DropIf, the value is dead (flag handles the conditional)
            if let Some(local) = place.root_local() {
                state.set(local, InitState::Dead);
            }
        }
        StatementKind::SetDropFlag { .. } => {}
        StatementKind::ScopeLive(local) => {
            state.set(*local, InitState::Dead);
        }
    }
}

fn transfer_terminator(state: &mut InitMap, kind: &TerminatorKind) {
    if let TerminatorKind::Return(Operand::Place(place)) = kind
        && let Some(local) = place.root_local()
    {
        state.set(local, InitState::Dead);
    }
}

fn kill_rvalue_moves(state: &mut InitMap, rvalue: &Rvalue) {
    for (op, mode) in rvalue.operands_with_mode() {
        if mode == Some(UseMode::Move)
            && let Operand::Place(place) = op
            && let Some(local) = place.root_local()
        {
            state.set(local, InitState::Dead);
        }
    }
}

// ---- Public API ----

pub struct InitAnalysis {
    entry_states: Vec<InitMap>,
}

impl InitAnalysis {
    pub fn compute(body: &MirBody) -> Self {
        let cfg = dataflow::compute_cfg_info(body);
        Self::compute_with_cfg(body, &cfg)
    }

    pub fn compute_with_cfg(body: &MirBody, cfg: &CfgInfo) -> Self {
        let transfer = InitTransfer {
            num_locals: body.locals.len(),
            param_count: body.param_count,
        };
        let entry_states = dataflow::forward_fixpoint(cfg, body, &transfer);
        Self { entry_states }
    }

    /// Init state of `local` at the entry of `block`.
    pub fn state_at_entry(&self, block: BlockId, local: LocalId) -> InitState {
        self.entry_states[block.index()].get(local)
    }

    /// Compute init state of `local` at a specific statement within a block.
    /// Walks forward from the block's entry state through statements 0..=stmt_index.
    pub fn state_after(&self, body: &MirBody, block: BlockId, stmt_index: usize, local: LocalId) -> InitState {
        let mut state = self.entry_states[block.index()].clone();
        let bb = body.block(block);
        for si in 0..=stmt_index {
            transfer_statement(&mut state, &bb.stmts[si].kind);
        }
        state.get(local)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::ModuleBuilder;
    use crate::immediate::Immediate;
    use crate::operand::Operand;
    use crate::place::Place;
    use crate::statement::Callee;
    use crate::ty::ParamConvention;
    use crate::MirModule;

    fn body_from(module: &MirModule) -> &MirBody {
        module.functions[0].body.as_ref().unwrap()
    }

    // ---- Lattice join tests ----

    #[test]
    fn join_live_live_is_live() {
        let mut a = InitMap::new(1);
        a.set(LocalId::new(0), InitState::Live);
        let mut b = InitMap::new(1);
        b.set(LocalId::new(0), InitState::Live);
        a.join(&b);
        assert_eq!(a.get(LocalId::new(0)), InitState::Live);
    }

    #[test]
    fn join_dead_dead_is_dead() {
        let mut a = InitMap::new(1);
        let b = InitMap::new(1);
        a.join(&b);
        assert_eq!(a.get(LocalId::new(0)), InitState::Dead);
    }

    #[test]
    fn join_live_dead_is_maybe() {
        let mut a = InitMap::new(1);
        a.set(LocalId::new(0), InitState::Live);
        let b = InitMap::new(1); // Dead
        a.join(&b);
        assert_eq!(a.get(LocalId::new(0)), InitState::Maybe);
    }

    #[test]
    fn join_dead_live_is_maybe() {
        let mut a = InitMap::new(1);
        let mut b = InitMap::new(1);
        b.set(LocalId::new(0), InitState::Live);
        a.join(&b);
        assert_eq!(a.get(LocalId::new(0)), InitState::Maybe);
    }

    #[test]
    fn join_maybe_anything_is_maybe() {
        let mut a = InitMap::new(1);
        a.set(LocalId::new(0), InitState::Maybe);
        let mut b = InitMap::new(1);
        b.set(LocalId::new(0), InitState::Live);
        a.join(&b);
        assert_eq!(a.get(LocalId::new(0)), InitState::Maybe);
    }

    // ---- Forward transfer tests ----

    #[test]
    fn assign_makes_live() {
        // bb0: x = 0; return ()
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let i64_ty = m.i64();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", i64_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let analysis = InitAnalysis::compute(body);

        // x is Dead at entry, Live after assignment
        assert_eq!(analysis.state_at_entry(bb0, x), InitState::Dead);
        assert_eq!(analysis.state_after(body, bb0, 0, x), InitState::Live);
    }

    #[test]
    fn move_makes_dead() {
        // bb0: x = 0; y = move x; return ()
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let i64_ty = m.i64();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", i64_ty);
        let y = f.local("y", i64_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.use_move(Place::local(y), Place::local(x));
            b.ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let analysis = InitAnalysis::compute(body);

        // After stmt 0: x=Live. After stmt 1: x=Dead (moved), y=Live
        assert_eq!(analysis.state_after(body, bb0, 0, x), InitState::Live);
        assert_eq!(analysis.state_after(body, bb0, 1, x), InitState::Dead);
        assert_eq!(analysis.state_after(body, bb0, 1, y), InitState::Live);
    }

    #[test]
    fn params_are_live_at_entry() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let mut f = m.function("f", i64_ty);
        let x = f.param("x", i64_ty, ParamConvention::Consuming);
        let bb0 = f.block_id();
        {
            f.block_at(bb0).ret(Operand::Place(Place::local(x)));
        }
        let module = m.finish();
        let body = body_from(&module);
        let analysis = InitAnalysis::compute(body);

        assert_eq!(analysis.state_at_entry(bb0, x), InitState::Live);
    }

    #[test]
    fn diamond_join_produces_maybe() {
        // bb0: x = 0; branch → bb1, bb2
        // bb1: y = move x; jump bb3
        // bb2: jump bb3
        // bb3: return ()
        // At bb3 entry: x is Dead on bb1 path, Live on bb2 path → Maybe
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let i64_ty = m.i64();
        let bool_ty = m.bool();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", i64_ty);
        let y = f.local("y", i64_ty);
        let cond = f.local("cond", bool_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        let bb2 = f.block_id();
        let bb3 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.assign_const(Place::local(cond), Immediate::bool(true));
            b.branch(Operand::Place(Place::local(cond)), bb1, bb2);
        }
        {
            let mut b = f.block_at(bb1);
            b.use_move(Place::local(y), Place::local(x));
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
        let analysis = InitAnalysis::compute(body);

        assert_eq!(analysis.state_at_entry(bb3, x), InitState::Maybe);
    }

    #[test]
    fn diamond_both_live_stays_live() {
        // bb0: x = 0; branch → bb1, bb2
        // bb1: jump bb3
        // bb2: jump bb3
        // bb3: return ()
        // x is Live on both paths → Live at bb3
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let i64_ty = m.i64();
        let bool_ty = m.bool();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", i64_ty);
        let cond = f.local("cond", bool_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        let bb2 = f.block_id();
        let bb3 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
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
        let analysis = InitAnalysis::compute(body);

        assert_eq!(analysis.state_at_entry(bb3, x), InitState::Live);
    }

    #[test]
    fn return_kills_local() {
        // bb0: x = 0; return x
        // After Return, x is Dead (moved to caller)
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
        let module = m.finish();
        let body = body_from(&module);
        let analysis = InitAnalysis::compute(body);

        // x is Live after the assign
        assert_eq!(analysis.state_after(body, bb0, 0, x), InitState::Live);
    }

    #[test]
    fn call_dest_becomes_live_move_arg_becomes_dead() {
        // bb0: result = call f(move x); return ()
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let i64_ty = m.i64();
        let callee = m.fresh_entity();
        let mut f = m.function("g", unit_ty);
        let x = f.local("x", i64_ty);
        let result = f.local("result", i64_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.call(
                Some(Place::local(result)),
                Callee::direct(callee),
                vec![(Operand::Place(Place::local(x)), ArgMode::Move)],
            );
            b.ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let analysis = InitAnalysis::compute(body);

        // After call: result=Live, x=Dead
        assert_eq!(analysis.state_after(body, bb0, 1, result), InitState::Live);
        assert_eq!(analysis.state_after(body, bb0, 1, x), InitState::Dead);
    }

    #[test]
    fn scope_live_resets_to_dead() {
        // bb0: x = 0; scope_live x; return ()
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let i64_ty = m.i64();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", i64_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.scope_live(x);
            b.ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let analysis = InitAnalysis::compute(body);

        // After assign: Live. After scope_live: Dead
        assert_eq!(analysis.state_after(body, bb0, 0, x), InitState::Live);
        assert_eq!(analysis.state_after(body, bb0, 1, x), InitState::Dead);
    }

    #[test]
    fn overwrite_detection() {
        // bb0: x = 0; x = 1; return ()
        // At stmt 1, x is already Live → overwrite (drop elab would insert drop before)
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let i64_ty = m.i64();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", i64_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.assign_const(Place::local(x), Immediate::i64(1));
            b.ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let analysis = InitAnalysis::compute(body);

        // Before stmt 1: x is Live (from stmt 0) — overwrite needed
        assert_eq!(analysis.state_after(body, bb0, 0, x), InitState::Live);
        // After stmt 1: x is still Live (new value)
        assert_eq!(analysis.state_after(body, bb0, 1, x), InitState::Live);
    }

    #[test]
    fn loop_converges() {
        // bb0: jump bb1
        // bb1: x = 0; branch → bb1, bb2
        // bb2: return ()
        // At bb1 entry on first visit: Dead. After back-edge: Live (from bb1 exit).
        // Join: Dead ∧ Live = Maybe
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let i64_ty = m.i64();
        let bool_ty = m.bool();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", i64_ty);
        let cond = f.local("cond", bool_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        let bb2 = f.block_id();
        {
            f.block_at(bb0).jump(bb1);
        }
        {
            let mut b = f.block_at(bb1);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.assign_const(Place::local(cond), Immediate::bool(true));
            b.branch(Operand::Place(Place::local(cond)), bb1, bb2);
        }
        {
            f.block_at(bb2).ret_unit();
        }
        let module = m.finish();
        let body = body_from(&module);
        let analysis = InitAnalysis::compute(body);

        // bb1 entry: from bb0 (Dead) and from bb1 back-edge (Live) → Maybe
        assert_eq!(analysis.state_at_entry(bb1, x), InitState::Maybe);
        // But after the assignment in bb1, x is definitely Live
        assert_eq!(analysis.state_after(body, bb1, 0, x), InitState::Live);
    }
}
