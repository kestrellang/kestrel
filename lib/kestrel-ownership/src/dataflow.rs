//! Forward init/maybe-init dataflow over MIR move paths.
//!
//! Two parallel bit-sets per program point:
//! - `def_init`: paths that are *DefinitelyInit* (initialized on every CFG
//!   path leading here).
//! - `may_init`: paths that are *MaybeInit* (initialized on at least one CFG
//!   path leading here).
//!
//! Transfer function per statement:
//! - `Assign { dest, rvalue }`:
//!   - First process the RHS reads (so `let y = move x` correctly invalidates
//!     `x` before re-binding it).
//!   - `Rvalue::Move(p)`: kill `path(p)` — clears both bits.
//!   - `Rvalue::Copy/Ref/RefMut/Const(_)`: no kill.
//!   - Then gen `path(dest)` — sets both bits.
//! - `Call { dest, args }`:
//!   - For each `Value::Move(p)` arg: kill `path(p)`.
//!   - If `dest` is `Some`: gen `path(dest)`.
//!   - `Value::Copy/Ref/RefMut/Const`: no kill.
//! - Other statement kinds (`Drop` / `DropIf`) are ignored — they're
//!   compiler-inserted destructors, not user-visible moves.
//!
//! Block merge at join points:
//! - `def_init` is intersected (AND) — only paths init on every predecessor.
//! - `may_init` is unioned (OR) — paths init on any predecessor.
//!
//! Fixed-point: iterate until neither set changes for any block. For
//! Stage 4 we use a simple worklist seeded with every block.

use std::collections::{HashMap, HashSet, VecDeque};

use kestrel_mir::{
    LocalId, MirBody, Rvalue, Statement, StatementKind, TerminatorKind, Value,
};
use kestrel_span::Span;

use crate::move_path::{MovePathId, MovePathSet};

/// Init/maybe-init state at one program point.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InitState {
    pub def_init: HashSet<MovePathId>,
    pub may_init: HashSet<MovePathId>,
    /// Paths that have been moved-out (killed) on at least one reaching CFG
    /// path AND not subsequently re-initialised. Used by move-check to
    /// distinguish a real use-after-move from a CFG join over an unbound
    /// branch (e.g. the body of `while let .Some(x) = … { … }` joins the
    /// "x bound" Some arm with the "x not bound" None arm — `x` ends up
    /// `MaybeInit` at the body, but it was never *moved*; reporting that
    /// as E501 is a false positive). The set is unioned at joins (parallel
    /// to `may_init`) and cleared per-path on `mark_init` (a fresh
    /// initialisation overwrites any prior kill).
    pub moved: HashSet<MovePathId>,
    /// For each moved path, the span of (one of) the kill site(s). Used by
    /// move-check to attach a "value moved here" secondary label to
    /// E500/E501. Only one site is retained per path; on join, the existing
    /// site wins on collision — tests only need *some* valid pointer.
    pub move_sites: HashMap<MovePathId, Span>,
}

impl InitState {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Mark a path as definitely initialized at this point. Clears any
    /// recorded move site — the path has been freshly re-initialised, so a
    /// later use is not a use-after-move.
    pub fn mark_init(&mut self, path: MovePathId) {
        self.def_init.insert(path);
        self.may_init.insert(path);
        self.moved.remove(&path);
        self.move_sites.remove(&path);
    }

    /// Mark a path as uninitialized at this point. Span-less variant for
    /// callers that don't have a meaningful kill location.
    pub fn kill(&mut self, path: MovePathId) {
        self.def_init.remove(&path);
        self.may_init.remove(&path);
        self.moved.insert(path);
    }

    /// Same as [`Self::kill`], but records `site` as the move-site span for
    /// future "value moved here" labels.
    pub fn kill_with_span(&mut self, path: MovePathId, site: Span) {
        self.def_init.remove(&path);
        self.may_init.remove(&path);
        self.moved.insert(path);
        self.move_sites.insert(path, site);
    }

    /// Join with another state (predecessor merge):
    /// - def_init = self ∩ other
    /// - may_init = self ∪ other
    /// - moved    = self ∪ other
    /// - move_sites = union (existing wins on collision — arbitrary but
    ///   stable across runs).
    ///
    /// Returns true if any of the bitsets changed. `move_sites` updates
    /// don't drive worklist propagation, since the dataflow lattice is
    /// already monotone in the bitsets.
    pub fn join(&mut self, other: &InitState) -> bool {
        let mut changed = false;
        // Intersect def_init
        let new_def: HashSet<MovePathId> = self.def_init.intersection(&other.def_init).copied().collect();
        if new_def != self.def_init {
            self.def_init = new_def;
            changed = true;
        }
        // Union may_init
        let len_before = self.may_init.len();
        self.may_init.extend(other.may_init.iter().copied());
        if self.may_init.len() != len_before {
            changed = true;
        }
        // Union moved
        let len_before = self.moved.len();
        self.moved.extend(other.moved.iter().copied());
        if self.moved.len() != len_before {
            changed = true;
        }
        // Union move_sites (existing wins on collision).
        for (path, site) in &other.move_sites {
            self.move_sites.entry(*path).or_insert_with(|| site.clone());
        }
        changed
    }

    /// True iff the path is DefinitelyInit at this point.
    pub fn is_definitely_init(&self, path: MovePathId) -> bool {
        self.def_init.contains(&path)
    }

    /// True iff the path is MaybeInit (and not DefinitelyInit) at this
    /// point. Combined with [`Self::is_definitely_init`], these three
    /// classes — DefinitelyInit, MaybeInit-but-not-Def, and Uninit —
    /// partition the lattice.
    pub fn is_maybe_init(&self, path: MovePathId) -> bool {
        self.may_init.contains(&path)
    }

    /// True iff the path has been moved-out somewhere on a reaching CFG
    /// path *and* not subsequently re-initialised. Used by move-check to
    /// gate E500/E501 emission so that a path which was never actually
    /// moved doesn't produce a "may have been moved" diagnostic just from
    /// a CFG join over an unbound branch.
    pub fn was_moved(&self, path: MovePathId) -> bool {
        self.moved.contains(&path)
    }

    /// The recorded move site for `path`, if any. `None` means the path
    /// has not been killed on any reaching CFG path.
    pub fn move_site(&self, path: MovePathId) -> Option<&Span> {
        self.move_sites.get(&path)
    }
}

/// Per-block dataflow results.
#[derive(Debug, Clone)]
pub struct BlockState {
    /// State at block entry.
    pub entry: InitState,
    /// State at block exit (after all statements + terminator side-effects).
    pub exit: InitState,
    /// Whether this block's entry has been joined from at least one
    /// predecessor (or seeded as the function entry). Without this flag the
    /// initial `def_init = ∅` would clobber the first real join — set
    /// intersection bottoms out at empty, so `∅ ∩ {h} = ∅` and the path
    /// never makes it past block 0.
    pub entry_seeded: bool,
}

impl Default for BlockState {
    fn default() -> Self {
        Self {
            entry: InitState::empty(),
            exit: InitState::empty(),
            entry_seeded: false,
        }
    }
}

/// Full per-function dataflow result.
#[derive(Debug, Clone)]
pub struct DataflowResult {
    pub blocks: Vec<BlockState>,
}

/// Run the forward init dataflow over a function body. Returns one
/// [`BlockState`] per block in declaration order.
pub fn run(body: &MirBody, paths: &MovePathSet) -> DataflowResult {
    let n = body.blocks.len();
    let mut blocks: Vec<BlockState> = vec![BlockState::default(); n];

    // Entry state: parameters are DefinitelyInit; other locals are uninit.
    let mut entry_state = InitState::empty();
    for i in 0..body.param_count {
        let local = LocalId::new(i);
        if let Some(path) = paths.lookup_local(local) {
            entry_state.mark_init(path);
        }
    }

    if n == 0 {
        return DataflowResult { blocks };
    }
    let entry_idx = body.entry.index();
    blocks[entry_idx].entry = entry_state;
    blocks[entry_idx].entry_seeded = true;

    // Worklist seeded with the entry block; successors are pushed as their
    // entry states change. Each iteration recomputes a block's exit from its
    // entry and propagates to successors.
    let mut worklist: VecDeque<usize> = VecDeque::new();
    let mut in_queue: HashSet<usize> = HashSet::new();
    worklist.push_back(entry_idx);
    in_queue.insert(entry_idx);
    while let Some(bi) = worklist.pop_front() {
        in_queue.remove(&bi);
        let block = &body.blocks[bi];
        let mut state = blocks[bi].entry.clone();
        for stmt in &block.stmts {
            apply_statement(&mut state, stmt, paths);
        }
        apply_terminator_with_span(
            &mut state,
            &block.terminator.kind,
            block.terminator.span.as_ref(),
            paths,
        );

        // Always store the new exit, then propagate to successors. Fixed-point
        // termination is governed by `entry_changed` below — the previous
        // `if state != blocks[bi].exit` guard around propagation was wrong:
        // the default exit is `InitState::empty()`, so on first-visit a block
        // whose statements touch only non-tracked locals would compute an
        // empty exit, match `blocks[bi].exit`, and never seed its successors.
        // The successors then stuck at default-uninit forever.
        blocks[bi].exit = state.clone();
        for &succ in successors(&block.terminator.kind).iter() {
            let changed = if blocks[succ].entry_seeded {
                blocks[succ].entry.join(&state)
            } else {
                blocks[succ].entry = state.clone();
                blocks[succ].entry_seeded = true;
                true
            };
            if changed && !in_queue.contains(&succ) {
                worklist.push_back(succ);
                in_queue.insert(succ);
            }
        }
    }

    DataflowResult { blocks }
}

/// Apply a statement's gen/kill effects to the running state.
pub fn apply_statement(state: &mut InitState, stmt: &Statement, paths: &MovePathSet) {
    let site = stmt.span.clone();
    match &stmt.kind {
        StatementKind::Assign { dest, rvalue } => {
            // Kill RHS moves first, then gen the destination. A self-rebinding
            // shape like `x = move x` thus ends with `x` still init (the kill
            // is overwritten by the gen on the same path).
            kill_rvalue(state, rvalue, paths, site.as_ref());
            // `RefMut(p)` in an Rvalue position is Kestrel's out-parameter
            // shape — the borrowed callee may write into `p`. Conservatively
            // mark the target init (the data-flow lattice already treats
            // mark_init as idempotent on already-init paths).
            gen_rvalue_refmuts(state, rvalue, paths);
            if let Some(p) = paths.lookup_place(dest) {
                state.mark_init(p);
            }
        },
        StatementKind::Call { dest, args, .. } => {
            for arg in args {
                kill_value(state, arg, paths, site.as_ref());
            }
            // After the call returns, every `RefMut(p)` arg leaves `p` in a
            // state where the callee may have written it. Promote `p`'s
            // path to `DefinitelyInit` so a subsequent read doesn't trip
            // E500 on a value the callee just initialised (e.g.
            // `File.init(ref var %t, fd)` then `copy %t`).
            for arg in args {
                gen_value_refmuts(state, arg, paths);
            }
            if let Some(dest_place) = dest
                && let Some(p) = paths.lookup_place(dest_place)
            {
                state.mark_init(p);
            }
        },
        // Compiler-inserted drops don't move from the user's perspective.
        StatementKind::Drop { .. } | StatementKind::DropIf { .. } => {},
    }
}

fn gen_rvalue_refmuts(state: &mut InitState, rv: &Rvalue, paths: &MovePathSet) {
    match rv {
        Rvalue::RefMut(p) => {
            if let Some(path) = paths.lookup_place(p) {
                state.mark_init(path);
            }
        },
        Rvalue::Op1 { arg, .. } => gen_value_refmuts(state, arg, paths),
        Rvalue::Op2 { lhs, rhs, .. } => {
            gen_value_refmuts(state, lhs, paths);
            gen_value_refmuts(state, rhs, paths);
        },
        Rvalue::Op3 { a, b, c, .. } => {
            gen_value_refmuts(state, a, paths);
            gen_value_refmuts(state, b, paths);
            gen_value_refmuts(state, c, paths);
        },
        Rvalue::Construct { fields, .. } => {
            for (_, v) in fields {
                gen_value_refmuts(state, v, paths);
            }
        },
        Rvalue::Tuple(vs) | Rvalue::ArrayLiteral { values: vs, .. } => {
            for v in vs {
                gen_value_refmuts(state, v, paths);
            }
        },
        Rvalue::EnumVariant { payload, .. } => {
            for v in payload {
                gen_value_refmuts(state, v, paths);
            }
        },
        Rvalue::ApplyPartial { captures, .. } => {
            for v in captures {
                gen_value_refmuts(state, v, paths);
            }
        },
        Rvalue::Move(_) | Rvalue::Copy(_) | Rvalue::Ref(_) | Rvalue::Const(_) => {},
    }
}

fn gen_value_refmuts(state: &mut InitState, v: &Value, paths: &MovePathSet) {
    if let Value::RefMut(p) = v
        && let Some(path) = paths.lookup_place(p)
    {
        state.mark_init(path);
    }
}

/// Terminators can move (`Return(Value)`, `Branch.condition`). Span-less
/// convenience wrapper for callers (like drop-elab's recompute) that
/// don't carry the terminator's span around.
pub fn apply_terminator(
    state: &mut InitState,
    term: &TerminatorKind,
    paths: &MovePathSet,
) {
    apply_terminator_with_span(state, term, None, paths);
}

/// Apply terminator transfer with the terminator's own span recorded as
/// the move site for any path it kills.
pub fn apply_terminator_with_span(
    state: &mut InitState,
    term: &TerminatorKind,
    site: Option<&Span>,
    paths: &MovePathSet,
) {
    match term {
        TerminatorKind::Return(v) | TerminatorKind::Branch { condition: v, .. } => {
            kill_value(state, v, paths, site);
        },
        TerminatorKind::Switch { .. }
        | TerminatorKind::Jump(_)
        | TerminatorKind::Panic(_)
        | TerminatorKind::Unreachable => {},
    }
}

fn kill_rvalue(state: &mut InitState, rv: &Rvalue, paths: &MovePathSet, site: Option<&Span>) {
    match rv {
        Rvalue::Move(p) => {
            if let Some(path) = paths.lookup_place(p) {
                match site {
                    Some(s) => state.kill_with_span(path, s.clone()),
                    None => state.kill(path),
                }
            }
        },
        Rvalue::Copy(_) | Rvalue::Ref(_) | Rvalue::RefMut(_) | Rvalue::Const(_) => {},
        Rvalue::Op1 { arg, .. } => kill_value(state, arg, paths, site),
        Rvalue::Op2 { lhs, rhs, .. } => {
            kill_value(state, lhs, paths, site);
            kill_value(state, rhs, paths, site);
        },
        Rvalue::Op3 { a, b, c, .. } => {
            kill_value(state, a, paths, site);
            kill_value(state, b, paths, site);
            kill_value(state, c, paths, site);
        },
        Rvalue::Construct { fields, .. } => {
            for (_, v) in fields {
                kill_value(state, v, paths, site);
            }
        },
        Rvalue::Tuple(vs) | Rvalue::ArrayLiteral { values: vs, .. } => {
            for v in vs {
                kill_value(state, v, paths, site);
            }
        },
        Rvalue::EnumVariant { payload, .. } => {
            for v in payload {
                kill_value(state, v, paths, site);
            }
        },
        Rvalue::ApplyPartial { captures, .. } => {
            for v in captures {
                kill_value(state, v, paths, site);
            }
        },
    }
}

fn kill_value(state: &mut InitState, v: &Value, paths: &MovePathSet, site: Option<&Span>) {
    if let Value::Move(p) = v
        && let Some(path) = paths.lookup_place(p)
    {
        match site {
            Some(s) => state.kill_with_span(path, s.clone()),
            None => state.kill(path),
        }
    }
}

fn successors(term: &TerminatorKind) -> Vec<usize> {
    match term {
        TerminatorKind::Jump(b) => vec![b.index()],
        TerminatorKind::Branch {
            then_block,
            else_block,
            ..
        } => vec![then_block.index(), else_block.index()],
        TerminatorKind::Switch { cases, .. } => cases.iter().map(|(_, b)| b.index()).collect(),
        TerminatorKind::Return(_) | TerminatorKind::Panic(_) | TerminatorKind::Unreachable => {
            Vec::new()
        },
    }
}
