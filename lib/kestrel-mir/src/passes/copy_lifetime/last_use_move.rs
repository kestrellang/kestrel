//! Move a `let`/`var` slot's value out at its last read instead of copying it.
//!
//! Every local lives at a stack slot (`uninit`), and every by-value read of it
//! copies: `copy_addr %slot`, or `begin_borrow_addr` + `copy_value` +
//! `end_borrow`. When nothing can read the slot again before it is destroyed
//! or overwritten, that copy is a clone of a value about to die. This pass
//! turns such a read into `take %slot` (a move) and drops the work the copy
//! forced:
//!
//! - a later `destroy_addr %slot` the take reaches is deleted, and
//! - a later `store_assign %slot, %new` becomes `store_init` (nothing to drop).
//!
//! For `let left = parse(); Node(kind: .Bin(left: left))` this moves `left`
//! into the payload; for the `var left = ...; while .. { left = wrap(left) }`
//! loop it moves the old value into the new node and re-initializes the slot.
//! Without it, wrapping a child in an `indirect` parent deep-copies the
//! child's whole subtree, which makes bottom-up tree building quadratic.
//!
//! **Soundness.** Per slot, a backward liveness pass over the slot's events
//! (reads, kills) proves no path from the read reaches another read before a
//! kill. A forward pass then checks every kill a take reaches is reached ONLY
//! by moved-out paths (there are no drop flags); takes that reach a kill also
//! reachable from a still-initialized path are dropped until none conflict.
//! A slot whose address leaves the known set of address instructions (a call
//! argument, a closure capture, a raw pointer, a returned value) is never
//! touched: something else could read it.
//!
//! Slots of observable types (see [`TypeFacts::observable`]) are skipped: the
//! move removes one drop of the original, which `@observabledrop` types may
//! notice.

use kestrel_debug::ktrace;
use rustc_hash::{FxHashMap, FxHashSet};

use super::{TypeFacts, terminator_value_use};
use crate::ValueId;
use crate::body::OssaBody;
use crate::inst::{InstKind, Instruction};
use crate::mono::types::MonoModule;
use crate::ty::{FnKind, MirTy};
use crate::value::Ownership;

pub(super) fn run(mono: &mut MonoModule, facts: &mut TypeFacts) -> usize {
    let mut total = 0usize;
    // Bodies are mutated while type tables are read: take bodies out first.
    let mut funcs = std::mem::take(&mut mono.functions);
    for func in funcs.iter_mut() {
        let Some(body) = &mut func.body else { continue };
        let n = move_in_function(mono, body, facts);
        if n > 0 {
            ktrace!("last-use-move", "{}: {n} slot reads moved", func.name);
        }
        total += n;
    }
    mono.functions = funcs;
    total
}

/// What one instruction does to a slot's contents.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Event {
    /// Needs the contents initialized (borrow, copy, field access, ...).
    Read,
    /// A by-value read that could become a move (index into `Slot::cands`).
    Cand(usize),
    /// `store_init` of the whole slot.
    Init,
    /// `store_assign` of the whole slot: drops the old value, stores a new one.
    Assign,
    /// `destroy_addr` of the whole slot.
    Destroy,
}

impl Event {
    fn is_kill(self) -> bool {
        matches!(self, Event::Init | Event::Assign | Event::Destroy)
    }
}

/// A by-value read of the whole slot.
#[derive(Clone, Copy)]
enum CandShape {
    /// `%c = copy_addr %slot`.
    CopyAddr { inst: usize },
    /// `%b = begin_borrow_addr %slot; %c = copy_value %b; end_borrow %b`, where
    /// `%b` has no other use.
    BorrowCopy {
        begin: usize,
        copy: usize,
        end: usize,
    },
}

#[derive(Clone, Copy)]
struct Cand {
    block: usize,
    shape: CandShape,
    /// The copy's result, which becomes the take's result.
    result: ValueId,
    address: ValueId,
}

#[derive(Default)]
struct Slot {
    /// (block, inst index, event), in program order within each block.
    events: Vec<(usize, usize, Event)>,
    cands: Vec<Cand>,
    /// The address escapes the instructions we model: never touch the slot.
    escapes: bool,
}

/// Union-find over value ids, joining a slot with its field addresses and the
/// block params it is threaded through.
struct Names(Vec<u32>);

impl Names {
    fn find(&mut self, v: ValueId) -> usize {
        let mut i = v.index();
        while self.0[i] as usize != i {
            let p = self.0[i] as usize;
            self.0[i] = self.0[p];
            i = p;
        }
        i
    }

    fn union(&mut self, a: ValueId, b: ValueId) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.0[ra] = rb as u32;
        }
    }
}

fn move_in_function(mono: &MonoModule, body: &mut OssaBody, facts: &mut TypeFacts) -> usize {
    let n_values = body.values.len();
    let mut names = Names((0..n_values as u32).collect());

    // 1. Name classes: field addresses join their base, params their args.
    //    `whole` marks names that denote the entire slot, not a field of it.
    let mut def_kind: Vec<Def> = vec![Def::Other; n_values];
    for block in &body.blocks {
        for p in &block.params {
            def_kind[p.value.index()] = Def::Param;
        }
        for inst in &block.insts {
            match inst.kind {
                InstKind::Uninit { result, .. } => def_kind[result.index()] = Def::Slot,
                InstKind::FieldAddr { result, base, .. } => {
                    def_kind[result.index()] = Def::Field;
                    names.union(result, base);
                },
                _ => {},
            }
        }
    }
    let mut edges: Vec<(ValueId, ValueId)> = Vec::new();
    for block in &body.blocks {
        for (target, args) in block.terminator.kind.successor_args() {
            for (a, p) in args.iter().zip(&body.blocks[target.index()].params) {
                names.union(*a, p.value);
                edges.push((*a, p.value));
            }
        }
    }
    let whole = whole_names(&def_kind, &edges);

    // A class is a slot iff it holds exactly one `uninit` and otherwise only
    // field addresses and params. Anything else (function params, call
    // results) means we can't see every write to it.
    let mut slot_of: FxHashMap<usize, usize> = FxHashMap::default();
    let mut bad: FxHashSet<usize> = FxHashSet::default();
    let mut slot_ty = Vec::new();
    for block in &body.blocks {
        for inst in &block.insts {
            if let InstKind::Uninit { result, ty } = inst.kind {
                let root = names.find(result);
                if slot_of.insert(root, slot_ty.len()).is_some() {
                    bad.insert(root);
                }
                slot_ty.push(ty);
            }
        }
    }
    for (v, def) in def_kind.iter().enumerate() {
        let root = names.find(ValueId::new(v));
        if *def == Def::Other && slot_of.contains_key(&root) {
            bad.insert(root);
        }
    }
    if slot_of.is_empty() {
        return 0;
    }

    // Borrows of a slot (and the params they are forwarded through) end with
    // `end_borrow`: that end is a read, since the borrow was live until then.
    let mut borrow_slot: FxHashMap<usize, usize> = FxHashMap::default();
    for block in &body.blocks {
        for inst in &block.insts {
            if let InstKind::BeginBorrowAddr {
                result, address, ..
            }
            | InstKind::BeginMutBorrowAddr {
                result, address, ..
            } = inst.kind
            {
                let root = names.find(address);
                if let Some(&s) = slot_of.get(&root) {
                    let b = names.find(result);
                    borrow_slot.insert(b, s);
                }
            }
        }
    }

    let mut uses = vec![0u32; n_values];
    for block in &body.blocks {
        for inst in &block.insts {
            for op in inst.kind.operands() {
                uses[op.index()] += 1;
            }
        }
        for op in block.terminator.kind.operands() {
            uses[op.index()] += 1;
        }
    }

    let handles = Handles::new(mono, body, &mut names, &slot_of, &edges);

    // 2. Events per slot, in program order.
    let mut slots: Vec<Slot> = (0..slot_ty.len()).map(|_| Slot::default()).collect();
    for (bi, block) in body.blocks.iter().enumerate() {
        // `begin_borrow_addr` index → (copy index, copy result) awaiting its end.
        let mut open: FxHashMap<ValueId, (usize, Option<(usize, ValueId)>)> = FxHashMap::default();
        for (ii, inst) in block.insts.iter().enumerate() {
            let mut slot_ops: Vec<(usize, ValueId)> = Vec::new();
            for op in inst.kind.operands() {
                let root = names.find(op);
                if let Some(&s) = slot_of.get(&root) {
                    slot_ops.push((s, op));
                }
            }
            match inst.kind {
                InstKind::EndBorrow { operand } | InstKind::EndMutBorrow { operand } => {
                    let Some(&s) = borrow_slot.get(&names.find(operand)) else {
                        continue;
                    };
                    let event = match open.remove(&operand) {
                        Some((begin, Some((copy, result)))) => {
                            let c = slots[s].cands.len();
                            slots[s].cands.push(Cand {
                                block: bi,
                                shape: CandShape::BorrowCopy {
                                    begin,
                                    copy,
                                    end: ii,
                                },
                                result,
                                address: begin_address(&block.insts[begin]),
                            });
                            Event::Cand(c)
                        },
                        _ => Event::Read,
                    };
                    slots[s].events.push((bi, ii, event));
                    continue;
                },
                InstKind::CopyValue { result, operand } => {
                    if let Some(entry) = open.get_mut(&operand)
                        && entry.1.is_none()
                        && body.value(result).ownership == Ownership::Owned
                    {
                        entry.1 = Some((ii, result));
                    }
                },
                _ => {},
            }
            for (s, op) in slot_ops {
                let is_whole = whole[op.index()];
                let event = match inst.kind {
                    InstKind::BeginBorrowAddr { result, .. } => {
                        // A shared borrow used only by one copy and its end is
                        // a by-value read; anything else is a plain read.
                        if is_whole && uses[result.index()] == 2 {
                            open.insert(result, (ii, None));
                        }
                        Event::Read
                    },
                    InstKind::BeginMutBorrowAddr { .. } | InstKind::Load { .. } => Event::Read,
                    InstKind::Take { .. } => Event::Read,
                    InstKind::CopyAddr {
                        result, address, ..
                    } => {
                        if is_whole && body.value(result).ownership == Ownership::Owned {
                            let c = slots[s].cands.len();
                            slots[s].cands.push(Cand {
                                block: bi,
                                shape: CandShape::CopyAddr { inst: ii },
                                result,
                                address,
                            });
                            Event::Cand(c)
                        } else {
                            Event::Read
                        }
                    },
                    InstKind::StoreInit { address, .. } if address == op => {
                        if is_whole {
                            Event::Init
                        } else {
                            Event::Read
                        }
                    },
                    InstKind::StoreAssign { address, .. } if address == op => {
                        if is_whole {
                            Event::Assign
                        } else {
                            Event::Read
                        }
                    },
                    InstKind::DestroyAddr { address, .. } if address == op => {
                        if is_whole {
                            Event::Destroy
                        } else {
                            Event::Read
                        }
                    },
                    // Releasing the slot's own storage, or projecting a field
                    // (whose uses are events of their own), doesn't touch the
                    // contents.
                    InstKind::DestroyValue { .. } | InstKind::FieldAddr { .. } => continue,
                    // The address is copied into a view closure; `handles`
                    // turns every later use of it into an event.
                    InstKind::CopyValue { .. }
                    | InstKind::MoveValue { .. }
                    | InstKind::ApplyPartial { .. }
                        if handles.covers(inst, s) =>
                    {
                        Event::Read
                    },
                    _ => {
                        slots[s].escapes = true;
                        continue;
                    },
                };
                slots[s].events.push((bi, ii, event));
            }
        }
        if let Some(v) = terminator_value_use(&block.terminator.kind)
            && let Some(&s) = slot_of.get(&names.find(v))
        {
            slots[s].escapes = true;
        }
    }
    handles.add_events(body, &mut slots);
    for (&root, &s) in &slot_of {
        if bad.contains(&root) {
            slots[s].escapes = true;
        }
    }

    for slot in &mut slots {
        slot.events.sort_by_key(|&(b, i, _)| (b, i));
    }

    // 3. Per slot: pick the reads that are last uses, then rewrite.
    let cfg = Cfg::new(body);
    let mut rewrites = Rewrites::default();
    let mut moved = 0usize;
    for (s, slot) in slots.iter().enumerate() {
        if slot.escapes || slot.cands.is_empty() || facts.observable(mono, slot_ty[s]) {
            continue;
        }
        moved += plan_slot(&cfg, body.entry.index(), slot, &mut rewrites);
    }
    if moved > 0 {
        rewrites.apply(body);
    }
    moved
}

/// Values that carry a slot's address beyond the address instructions:
/// copies of the pointer and the view closures capturing it (`a and b` and
/// `a or b` lower `b` to such a closure). A view closure cannot outlive the
/// frame, so the slot is only reachable through it while some copy, borrow or
/// call of it is still to come: each of those uses is a read of the slot.
/// Any other use (stored, returned, captured by an owning closure, a bare
/// pointer passed to a call) lets the address escape.
struct Handles {
    /// Handle → the slots it reaches, and whether it is a closure.
    of: FxHashMap<ValueId, (Vec<usize>, bool)>,
    /// Instructions that derive a handle straight from a slot address.
    roots: FxHashSet<(ValueId, usize)>,
}

impl Handles {
    fn new(
        mono: &MonoModule,
        body: &OssaBody,
        names: &mut Names,
        slot_of: &FxHashMap<usize, usize>,
        edges: &[(ValueId, ValueId)],
    ) -> Self {
        let mut h = Handles {
            of: FxHashMap::default(),
            roots: FxHashSet::default(),
        };
        loop {
            let before: usize = h.of.values().map(|(s, _)| s.len()).sum();
            for block in &body.blocks {
                for inst in &block.insts {
                    let (result, sources, closure) = match &inst.kind {
                        InstKind::CopyValue { result, operand }
                        | InstKind::MoveValue { result, operand }
                        | InstKind::BeginBorrow { result, operand }
                        | InstKind::CoerceFnKind {
                            result, operand, ..
                        } => (*result, vec![*operand], None),
                        InstKind::ApplyPartial {
                            result, captures, ..
                        } if is_view_closure(mono, body, *result) => {
                            (*result, captures.clone(), Some(true))
                        },
                        _ => continue,
                    };
                    let mut reach = Vec::new();
                    let mut is_closure = closure.unwrap_or(false);
                    for src in sources {
                        if let Some(&s) = slot_of.get(&names.find(src)) {
                            h.roots.insert((result, s));
                            reach.push(s);
                        } else if let Some((ss, c)) = h.of.get(&src) {
                            reach.extend(ss.iter().copied());
                            is_closure |= *c;
                        }
                    }
                    if !reach.is_empty() {
                        h.add(result, reach, is_closure);
                    }
                }
            }
            for &(a, p) in edges {
                if let Some((ss, c)) = h.of.get(&a).cloned() {
                    h.add(p, ss, c);
                }
            }
            let after: usize = h.of.values().map(|(s, _)| s.len()).sum();
            if after == before {
                return h;
            }
        }
    }

    fn add(&mut self, v: ValueId, slots: Vec<usize>, closure: bool) {
        let entry = self.of.entry(v).or_insert_with(|| (Vec::new(), closure));
        entry.1 |= closure;
        for s in slots {
            if !entry.0.contains(&s) {
                entry.0.push(s);
            }
        }
    }

    /// `inst` derives a tracked handle from slot `s`'s address.
    fn covers(&self, inst: &Instruction, s: usize) -> bool {
        inst.kind
            .result()
            .is_some_and(|r| self.roots.contains(&(r, s)))
    }

    /// Every use of a handle is a read of its slots, or an escape.
    fn add_events(&self, body: &OssaBody, slots: &mut [Slot]) {
        if self.of.is_empty() {
            return;
        }
        for (bi, block) in body.blocks.iter().enumerate() {
            for (ii, inst) in block.insts.iter().enumerate() {
                for op in inst.kind.operands() {
                    let Some((reach, closure)) = self.of.get(&op) else {
                        continue;
                    };
                    let derives = inst.kind.result().is_some_and(|r| self.of.contains_key(&r));
                    let ok = match inst.kind {
                        InstKind::EndBorrow { .. } | InstKind::DestroyValue { .. } => true,
                        InstKind::CopyValue { .. }
                        | InstKind::MoveValue { .. }
                        | InstKind::BeginBorrow { .. }
                        | InstKind::CoerceFnKind { .. }
                        | InstKind::ApplyPartial { .. } => derives,
                        // Calling or passing a view closure runs it no later
                        // than the call; a bare pointer could be kept.
                        InstKind::Call { .. } => *closure,
                        _ => false,
                    };
                    for &s in reach {
                        if ok {
                            slots[s].events.push((bi, ii, Event::Read));
                        } else {
                            slots[s].escapes = true;
                        }
                    }
                }
            }
            if let Some(v) = terminator_value_use(&block.terminator.kind)
                && let Some((reach, _)) = self.of.get(&v)
            {
                for &s in reach {
                    slots[s].escapes = true;
                }
            }
        }
    }
}

/// A closure that only views its captures (it lives in the frame).
fn is_view_closure(mono: &MonoModule, body: &OssaBody, v: ValueId) -> bool {
    matches!(
        mono.ty_arena.get(body.value(v).ty),
        MirTy::FuncThick {
            kind: FnKind::Normal | FnKind::Mutating,
            ..
        }
    )
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Def {
    Slot,
    Field,
    Param,
    Other,
}

fn begin_address(inst: &Instruction) -> ValueId {
    match inst.kind {
        InstKind::BeginBorrowAddr { address, .. } => address,
        _ => unreachable!("BorrowCopy candidate starts at a begin_borrow_addr"),
    }
}

/// Names denoting the whole slot: the `uninit` itself and params whose every
/// incoming argument is a whole name. Field addresses (and params carrying
/// them) are not.
fn whole_names(def_kind: &[Def], edges: &[(ValueId, ValueId)]) -> Vec<bool> {
    let mut whole: Vec<bool> = def_kind
        .iter()
        .map(|d| matches!(d, Def::Slot | Def::Param))
        .collect();
    loop {
        let mut changed = false;
        for &(a, p) in edges {
            if whole[p.index()] && !whole[a.index()] {
                whole[p.index()] = false;
                changed = true;
            }
        }
        if !changed {
            return whole;
        }
    }
}

struct Cfg {
    succs: Vec<Vec<usize>>,
}

impl Cfg {
    fn new(body: &OssaBody) -> Self {
        let succs = body
            .blocks
            .iter()
            .map(|b| {
                b.terminator
                    .kind
                    .successors()
                    .iter()
                    .map(|t| t.index())
                    .collect()
            })
            .collect();
        Self { succs }
    }
}

/// Moved-out state of a slot on entry to a point, joined over paths.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Moved {
    Unreached,
    No,
    Yes,
    Maybe,
}

impl Moved {
    fn join(self, other: Moved) -> Moved {
        match (self, other) {
            (Moved::Unreached, x) | (x, Moved::Unreached) => x,
            (a, b) if a == b => a,
            _ => Moved::Maybe,
        }
    }
}

/// Decide which of `slot`'s candidate reads become moves, recording the edits.
/// Returns how many reads were moved.
fn plan_slot(cfg: &Cfg, entry: usize, slot: &Slot, rewrites: &mut Rewrites) -> usize {
    let n_blocks = cfg.succs.len();
    let mut per_block: Vec<Vec<(usize, Event)>> = vec![Vec::new(); n_blocks];
    for &(b, i, e) in &slot.events {
        per_block[b].push((i, e));
    }

    // Backward liveness: is the content read again before a kill?
    let mut live_in = vec![false; n_blocks];
    loop {
        let mut changed = false;
        for b in (0..n_blocks).rev() {
            let v = match per_block[b].first() {
                Some((_, e)) => !e.is_kill(),
                None => cfg.succs[b].iter().any(|&t| live_in[t]),
            };
            if v != live_in[b] {
                live_in[b] = v;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let live_out = |b: usize| cfg.succs[b].iter().any(|&t| live_in[t]);

    // A candidate is a last use when the next event after it is a kill (or,
    // with none left in its block, the content is dead on block exit).
    let mut chosen: FxHashSet<usize> = FxHashSet::default();
    for evs in &per_block {
        for (k, &(_, e)) in evs.iter().enumerate() {
            let Event::Cand(c) = e else { continue };
            let dead = match evs.get(k + 1) {
                Some((_, next)) => next.is_kill(),
                None => !live_out(slot.cands[c].block),
            };
            if dead && shape_is_contiguous(slot, evs, k) {
                chosen.insert(c);
            }
        }
    }

    // Forward: every kill a move reaches must be reached only by moves.
    // Drop moves that reach a contested kill until none remain.
    loop {
        if chosen.is_empty() {
            return 0;
        }
        let (site_state, ok) = forward_states(cfg, entry, &per_block, &chosen);
        if !ok {
            return 0;
        }
        let contested: FxHashSet<(usize, usize)> = site_state
            .iter()
            .filter(|(_, m)| **m == Moved::Maybe)
            .map(|(site, _)| *site)
            .collect();
        if contested.is_empty() {
            for (site, m) in &site_state {
                if *m == Moved::Yes {
                    rewrites.kills.push(*site);
                }
            }
            for &c in &chosen {
                rewrites.cands.push(slot.cands[c]);
            }
            return chosen.len();
        }
        let before = chosen.len();
        chosen.retain(|&c| {
            let cand = slot.cands[c];
            reached_kills(cfg, &per_block, cand.block, event_index(&per_block, c))
                .is_disjoint(&contested)
        });
        if chosen.len() == before {
            return 0;
        }
    }
}

/// A `BorrowCopy` candidate is rewritten to a take at the copy: no other
/// event of the slot may sit between the begin and the end.
fn shape_is_contiguous(slot: &Slot, evs: &[(usize, Event)], k: usize) -> bool {
    let Event::Cand(c) = evs[k].1 else {
        return false;
    };
    match slot.cands[c].shape {
        CandShape::CopyAddr { .. } => true,
        CandShape::BorrowCopy { begin, .. } => k > 0 && evs[k - 1].0 == begin,
    }
}

fn event_index(per_block: &[Vec<(usize, Event)>], c: usize) -> (usize, usize) {
    for (b, evs) in per_block.iter().enumerate() {
        for (k, &(_, e)) in evs.iter().enumerate() {
            if e == Event::Cand(c) {
                return (b, k);
            }
        }
    }
    unreachable!("candidate has an event")
}

/// Moved-out state at each kill site, plus `false` if a read can see a moved
/// slot (liveness should rule that out; bail rather than miscompile).
fn forward_states(
    cfg: &Cfg,
    entry: usize,
    per_block: &[Vec<(usize, Event)>],
    chosen: &FxHashSet<usize>,
) -> (FxHashMap<(usize, usize), Moved>, bool) {
    let n_blocks = cfg.succs.len();
    let mut state_in = vec![Moved::Unreached; n_blocks];
    state_in[entry] = Moved::No;
    let mut sites: FxHashMap<(usize, usize), Moved> = FxHashMap::default();
    let mut work: Vec<usize> = vec![entry];
    let mut ok = true;
    while let Some(b) = work.pop() {
        let mut st = state_in[b];
        for &(i, e) in &per_block[b] {
            match e {
                Event::Cand(c) if chosen.contains(&c) => {
                    if st != Moved::No {
                        ok = false;
                    }
                    st = Moved::Yes;
                },
                Event::Cand(_) | Event::Read => {
                    if st != Moved::No {
                        ok = false;
                    }
                },
                Event::Init | Event::Assign | Event::Destroy => {
                    let slot = sites.entry((b, i)).or_insert(Moved::Unreached);
                    *slot = slot.join(st);
                    st = Moved::No;
                },
            }
        }
        for &t in &cfg.succs[b] {
            let joined = state_in[t].join(st);
            if joined != state_in[t] {
                state_in[t] = joined;
                work.push(t);
            }
        }
    }
    // Sites were joined from every visit; a site first seen as `No` and later
    // as `Yes` is `Maybe`. Init kills need no rewrite whatever their state.
    (sites, ok)
}

/// Kill sites reachable from event `k` of block `b` before any other kill.
fn reached_kills(
    cfg: &Cfg,
    per_block: &[Vec<(usize, Event)>],
    b: usize,
    (_, k): (usize, usize),
) -> FxHashSet<(usize, usize)> {
    let mut out = FxHashSet::default();
    if let Some(&(i, e)) = per_block[b].get(k + 1) {
        if e.is_kill() {
            out.insert((b, i));
        }
        return out;
    }
    let mut seen: FxHashSet<usize> = FxHashSet::default();
    let mut work: Vec<usize> = cfg.succs[b].clone();
    while let Some(t) = work.pop() {
        if !seen.insert(t) {
            continue;
        }
        match per_block[t].first() {
            Some(&(i, e)) => {
                if e.is_kill() {
                    out.insert((t, i));
                }
            },
            None => work.extend(cfg.succs[t].iter().copied()),
        }
    }
    out
}

/// Edits collected across all slots of a body, applied at once.
#[derive(Default)]
struct Rewrites {
    cands: Vec<Cand>,
    /// (block, inst) of kills reached only by moves.
    kills: Vec<(usize, usize)>,
}

impl Rewrites {
    fn apply(self, body: &mut OssaBody) {
        let mut delete: FxHashSet<(usize, usize)> = FxHashSet::default();
        for cand in &self.cands {
            let (at, extra) = match cand.shape {
                CandShape::CopyAddr { inst } => (inst, [None, None]),
                CandShape::BorrowCopy { begin, copy, end } => (copy, [Some(begin), Some(end)]),
            };
            let ty = match body.blocks[cand.block].insts[extra[0].unwrap_or(at)].kind {
                InstKind::CopyAddr { ty, .. } | InstKind::BeginBorrowAddr { ty, .. } => ty,
                _ => unreachable!("candidate shape"),
            };
            body.blocks[cand.block].insts[at].kind = InstKind::Take {
                result: cand.result,
                address: cand.address,
                ty,
                independent: true,
            };
            for i in extra.into_iter().flatten() {
                delete.insert((cand.block, i));
            }
        }
        for &(b, i) in &self.kills {
            let inst = &mut body.blocks[b].insts[i];
            match inst.kind {
                InstKind::StoreAssign { address, value } => {
                    inst.kind = InstKind::StoreInit { address, value };
                },
                InstKind::DestroyAddr { .. } => {
                    delete.insert((b, i));
                },
                _ => {},
            }
        }
        for (b, block) in body.blocks.iter_mut().enumerate() {
            let mut i = 0usize;
            block.insts.retain(|_| {
                let keep = !delete.contains(&(b, i));
                i += 1;
                keep
            });
        }
    }
}
