//! Backward liveness analysis over MIR.
//!
//! Computes which locals are live (have a future read) at each block
//! boundary. Used by clone elaboration to decide whether a `Copy` of a
//! Clone type needs a clone call — a local that is dead after the copy
//! can be moved instead of cloned.

use std::collections::VecDeque;

use crate::id::LocalId;
use crate::place::Place;
use crate::statement::{Rvalue, StatementKind};
use crate::terminator::TerminatorKind;
use crate::value::Value;
use crate::MirBody;

// ---- Bit-vector ----

#[derive(Clone, Debug)]
pub struct BitVec {
    words: Vec<u64>,
}

impl BitVec {
    fn new(num_bits: usize) -> Self {
        let num_words = (num_bits + 63) / 64;
        Self {
            words: vec![0; num_words],
        }
    }

    fn set(&mut self, bit: usize) {
        self.words[bit / 64] |= 1u64 << (bit % 64);
    }

    fn clear(&mut self, bit: usize) {
        self.words[bit / 64] &= !(1u64 << (bit % 64));
    }

    fn get(&self, bit: usize) -> bool {
        (self.words[bit / 64] >> (bit % 64)) & 1 != 0
    }

    /// `self |= other`. Returns true if self changed.
    fn union_with(&mut self, other: &BitVec) -> bool {
        let mut changed = false;
        for (a, b) in self.words.iter_mut().zip(other.words.iter()) {
            let old = *a;
            *a |= b;
            changed |= *a != old;
        }
        changed
    }

    fn clone_from_other(&mut self, other: &BitVec) {
        self.words.copy_from_slice(&other.words);
    }
}

// ---- Liveness result ----

#[allow(dead_code)]
pub struct Liveness {
    live_in: Vec<BitVec>,
    live_out: Vec<BitVec>,
    num_locals: usize,
}

impl Liveness {
    /// Run backward liveness analysis on a MIR body.
    pub fn compute(body: &MirBody) -> Self {
        let num_blocks = body.blocks.len();
        let num_locals = body.locals.len();
        let empty = || BitVec::new(num_locals);

        let mut live_in: Vec<BitVec> = (0..num_blocks).map(|_| empty()).collect();
        let mut live_out: Vec<BitVec> = (0..num_blocks).map(|_| empty()).collect();

        // Build predecessor map for the worklist.
        let mut predecessors: Vec<Vec<usize>> = vec![Vec::new(); num_blocks];
        for bi in 0..num_blocks {
            for succ in &body.blocks[bi].successors() {
                predecessors[succ.index()].push(bi);
            }
        }

        // Reverse-postorder for convergence.
        let rpo = compute_rpo(body);

        // Worklist — seeded with all blocks in RPO order.
        let mut in_queue = vec![true; num_blocks];
        let mut queue: VecDeque<usize> = rpo.iter().copied().collect();

        let mut scratch = empty();

        while let Some(bi) = queue.pop_front() {
            in_queue[bi] = false;
            let block = &body.blocks[bi];

            // live_out[bi] = ∪ live_in[succ] for all successors
            let mut out = empty();
            for succ in &block.successors() {
                out.union_with(&live_in[succ.index()]);
            }
            live_out[bi].clone_from_other(&out);

            // Transfer backward through the block: start from live_out,
            // process terminator, then statements in reverse.
            scratch.clone_from_other(&out);
            transfer_terminator(&mut scratch, &block.terminator.kind);
            for stmt in block.stmts.iter().rev() {
                transfer_statement(&mut scratch, &stmt.kind);
            }

            // If live_in changed, enqueue predecessors.
            if scratch.words != live_in[bi].words {
                live_in[bi].clone_from_other(&scratch);
                for &pred in &predecessors[bi] {
                    if !in_queue[pred] {
                        in_queue[pred] = true;
                        queue.push_back(pred);
                    }
                }
            }
        }

        Self {
            live_in,
            live_out,
            num_locals,
        }
    }

    /// Precompute liveness-after for every statement in block `bi`.
    ///
    /// Returns a vec where `result[si].get(local)` is true iff `local`
    /// is live immediately after original statement `si`. Must be called
    /// BEFORE the block is modified (inserted clone calls would corrupt
    /// the index mapping).
    pub fn block_liveness_after(&self, body: &MirBody, bi: usize) -> Vec<BitVec> {
        let block = &body.blocks[bi];
        let num_stmts = block.stmts.len();
        let mut result = vec![BitVec::new(self.num_locals); num_stmts];
        let mut live = self.live_out[bi].clone();
        transfer_terminator(&mut live, &block.terminator.kind);
        for i in (0..num_stmts).rev() {
            result[i] = live.clone();
            transfer_statement(&mut live, &block.stmts[i].kind);
        }
        result
    }

    /// Is `local` live in the given precomputed liveness-after set?
    pub fn is_live_in(set: &BitVec, local: LocalId) -> bool {
        set.get(local.index())
    }
}

// ---- Transfer functions ----

fn transfer_statement(live: &mut BitVec, kind: &StatementKind) {
    match kind {
        StatementKind::Assign { dest, rvalue } => {
            // Kill the destination first, then gen the rvalue reads.
            // Order matters: `x = copy x` should gen x.
            if let Some(id) = dest.root_local() {
                live.clear(id.index());
            }
            gen_rvalue(live, rvalue);
        }
        StatementKind::Call { dest, args, .. } => {
            if let Some(d) = dest {
                if let Some(id) = d.root_local() {
                    live.clear(id.index());
                }
            }
            for arg in args {
                gen_value(live, arg);
            }
        }
        StatementKind::Drop { place } | StatementKind::Deinit { place } => {
            gen_place(live, place);
        }
        StatementKind::DropIf { place, flag } | StatementKind::DeinitIf { place, flag } => {
            gen_place(live, place);
            live.set(flag.index());
        }
        StatementKind::SetDeinitFlag { flag, .. } => {
            live.clear(flag.index());
        }
        StatementKind::ScopeLive(local) => {
            live.set(local.index());
        }
    }
}

fn transfer_terminator(live: &mut BitVec, kind: &TerminatorKind) {
    match kind {
        TerminatorKind::Return(val) => gen_value(live, val),
        TerminatorKind::Branch { condition, .. } => gen_value(live, condition),
        TerminatorKind::Switch { discriminant, .. } => gen_place(live, discriminant),
        TerminatorKind::Jump(_) | TerminatorKind::Panic(_) | TerminatorKind::Unreachable => {}
    }
}

fn gen_value(live: &mut BitVec, value: &Value) {
    if let Some(place) = value.as_place() {
        gen_place(live, place);
    }
}

fn gen_place(live: &mut BitVec, place: &Place) {
    if let Some(id) = place.root_local() {
        live.set(id.index());
    }
}

fn gen_rvalue(live: &mut BitVec, rvalue: &Rvalue) {
    match rvalue {
        Rvalue::Move(p) | Rvalue::Copy(p) | Rvalue::Ref(p) | Rvalue::RefMut(p) => {
            gen_place(live, p);
        }
        Rvalue::Const(_) => {}
        Rvalue::Op1 { arg, .. } => gen_value(live, arg),
        Rvalue::Op2 { lhs, rhs, .. } => {
            gen_value(live, lhs);
            gen_value(live, rhs);
        }
        Rvalue::Op3 { a, b, c, .. } => {
            gen_value(live, a);
            gen_value(live, b);
            gen_value(live, c);
        }
        Rvalue::Construct { fields, .. } => {
            for (_, v) in fields {
                gen_value(live, v);
            }
        }
        Rvalue::Tuple(values) | Rvalue::ArrayLiteral { values, .. } => {
            for v in values {
                gen_value(live, v);
            }
        }
        Rvalue::EnumVariant { payload, .. } => {
            for v in payload {
                gen_value(live, v);
            }
        }
        Rvalue::ApplyPartial { captures, .. } => {
            for v in captures {
                gen_value(live, v);
            }
        }
    }
}

// ---- RPO computation ----

fn compute_rpo(body: &MirBody) -> Vec<usize> {
    let num_blocks = body.blocks.len();
    let mut visited = vec![false; num_blocks];
    let mut postorder = Vec::with_capacity(num_blocks);

    fn dfs(
        bi: usize,
        body: &MirBody,
        visited: &mut [bool],
        postorder: &mut Vec<usize>,
    ) {
        if visited[bi] {
            return;
        }
        visited[bi] = true;
        for succ in &body.blocks[bi].successors() {
            dfs(succ.index(), body, visited, postorder);
        }
        postorder.push(bi);
    }

    dfs(body.entry.index(), body, &mut visited, &mut postorder);

    // Include unreachable blocks so the analysis is complete.
    for bi in 0..num_blocks {
        if !visited[bi] {
            dfs(bi, body, &mut visited, &mut postorder);
        }
    }

    postorder.reverse();
    postorder
}
