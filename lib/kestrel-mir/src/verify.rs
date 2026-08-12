//! OSSA ownership verifier.
//!
//! Checks that a body satisfies the linear ownership invariant: every @owned
//! value is consumed exactly once, borrows are properly scoped, and address
//! init/uninit state is consistent.
//!
//! The algorithm is a single forward walk over the reachable blocks in RPO,
//! and each block is verified **in isolation** from an empty `FlowState`.
//!
//! That isolation is not justified. This module used to claim "no fixpoint
//! needed because the block-parameter live-in contract guarantees each block
//! can be verified in isolation" — there is no such contract. MIR lowering
//! uses the ordinary SSA dominance rule (a block may reference any definition
//! that dominates it), which is why `check_operands_defined` pools definitions
//! across every block. Enforcing the claimed contract was measured against the
//! suite and rejects the stdlib outright (audit finding F34, and the
//! "Corrections" section of `docs/fragility-audit.md`).
//!
//! The consequence is a real hole: leaks are caught, but a value consumed in
//! two *different* blocks is invisible, because `owned` starts empty in each.
//! Closing it means flowing `FlowState` along CFG edges to a fixpoint with a
//! join at merge points — not reshaping lowering.

use rustc_hash::{FxHashMap, FxHashSet};

use kestrel_hecs::Entity;
use kestrel_span::Span;

use crate::body::OssaBody;
use crate::inst::InstKind;
use crate::terminator::TerminatorKind;
use crate::ty::ParamConvention;
use crate::value::Ownership;
use crate::{BlockId, FieldIdx, MirModule, TyId, ValueId};

// ---------------------------------------------------------------------------
// Flow-sensitivity mode
// ---------------------------------------------------------------------------

/// How much of the flow-sensitive ownership walk is active.
///
/// Staged deliberately: the walk newly reports whole classes of violation that
/// have never been checked (cross-block double-consume, use-after-consume
/// across a merge), and any hit inside shipped code is a latent bug that has
/// to be fixed rather than silenced. `Warn` exists to measure that blast
/// radius against the suite before anything hard-fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlowMode {
    /// Block-local walk, exactly as before flow-sensitivity existed.
    Off,
    /// State flows; newly-reachable violations are counted and summarised on
    /// stderr but do NOT fail the build.
    Warn,
    /// State flows; newly-reachable violations are hard errors.
    Enforce,
}

impl FlowMode {
    fn current() -> FlowMode {
        use std::sync::OnceLock;
        static MODE: OnceLock<FlowMode> = OnceLock::new();
        *MODE.get_or_init(|| {
            FlowMode::parse(std::env::var("KESTREL_VERIFY_FLOW").ok().as_deref())
        })
    }

    /// Split out from `current` so the parse is testable: `current` memoises
    /// per process, so a test cannot exercise both settings through it.
    fn parse(value: Option<&str>) -> FlowMode {
        match value {
            Some("warn") => FlowMode::Warn,
            Some("enforce") => FlowMode::Enforce,
            _ => FlowMode::Off,
        }
    }

    fn flows(self) -> bool {
        self != FlowMode::Off
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct VerifyError {
    pub block: BlockId,
    /// Instruction index within the block, or None for block-level errors.
    pub inst: Option<u32>,
    pub message: String,
    /// Source span from the instruction that triggered the error.
    pub span: Option<Span>,
    /// Name of the function being verified.
    pub func_name: String,
    /// Entity of the function being verified (for DeclSpan fallback).
    pub entity: Entity,
    /// `Some` marks a user-facing diagnostic (escape-check errors, E49x);
    /// `None` is an internal compiler error (ownership invariant violation).
    /// `kestrel-compiler` renders them as `error[E49x]` vs `bug` accordingly.
    pub diag: Option<UserDiag>,
}

/// User-facing payload for coded `VerifyError`s.
#[derive(Debug, Clone)]
pub struct UserDiag {
    pub code: &'static str,
    /// Secondary label: span + message (e.g. the rooting local's definition).
    pub secondary: Option<(Span, String)>,
    pub notes: Vec<String>,
}

/// Verify that `body` satisfies OSSA ownership rules.
///
/// Returns an empty vec on success, or a list of every violation found.
pub fn verify_ossa(
    body: &OssaBody,
    module: &MirModule,
    func_name: &str,
    entity: Entity,
) -> Vec<VerifyError> {
    verify_ossa_with_mode(body, module, func_name, entity, FlowMode::current())
}

/// `verify_ossa` with the flow mode supplied rather than read from the
/// environment. The environment is consulted once per process, so tests that
/// need a specific mode must inject it here.
fn verify_ossa_with_mode(
    body: &OssaBody,
    module: &MirModule,
    func_name: &str,
    entity: Entity,
    mode: FlowMode,
) -> Vec<VerifyError> {
    let mut errors = Vec::new();

    // Check 1: ValueId uniqueness — every value defined exactly once.
    // The returned map (value -> defining block) feeds Check 1c below, which
    // needs to know where a value was defined to ask whether that definition
    // dominates the use. Built here either way, so returning it avoids a
    // second full traversal.
    let def_blocks = check_value_uniqueness(body, func_name, entity, &mut errors);

    // Check 1b: every operand must have a definition (block param or instruction result).
    check_operands_defined(body, func_name, entity, &mut errors);

    let order = reverse_postorder(body);
    let aliases = AddrAliases::build(body);

    // Check 1c: definitions must dominate their uses. Gated with the rest of
    // the flow work so `Off` remains exactly the historical behaviour, and
    // staged the same way: `Warn` reports without failing the build, only
    // `Enforce` turns a violation into a hard error. Measured at zero across
    // the 3573-file corpus, so this is a guard against regressions rather
    // than a live finding.
    let mut dominance_findings = Vec::new();
    if mode.flows() {
        check_dominance(
            body,
            &order,
            &def_blocks,
            func_name,
            entity,
            &mut dominance_findings,
        );
        match mode {
            FlowMode::Enforce => errors.append(&mut dominance_findings),
            _ => {
                if !dominance_findings.is_empty() {
                    report_flow_findings(func_name, &dominance_findings);
                }
            }
        }
    }

    // Phase 1: solve for each block's entry state.
    //
    // Off: every block starts empty, i.e. the original block-local walk, and
    // no ownership state crosses a block boundary (audit finding F34).
    //
    // Otherwise: forward dataflow to a fixpoint. Termination — the transfer
    // function is monotone (an instruction either leaves a value's state
    // untouched, or sets it to a constant independent of the incoming state),
    // `join` only ever moves a state up the lattice, and each of the finitely
    // many values has a 3-height lattice, so entry states can change only
    // finitely often.
    let mut in_states: FxHashMap<BlockId, FlowState> = FxHashMap::default();
    if mode.flows() {
        in_states.insert(body.entry, entry_state(body));

        let mut changed = true;
        while changed {
            changed = false;
            for &block_id in &order {
                let entry = in_states.get(&block_id).cloned().unwrap_or_default();
                // `report: false` — findings from a not-yet-settled state are
                // meaningless, and re-running would duplicate them.
                let outcome = verify_block(
                    body, module, block_id, func_name, entity, entry, false, mode, &aliases,
                );
                for succ in body.block(block_id).terminator.kind.successors() {
                    match in_states.get_mut(&succ) {
                        Some(existing) => {
                            if existing.join(&outcome.out_state) {
                                changed = true;
                            }
                        },
                        None => {
                            in_states.insert(succ, outcome.out_state.clone());
                            changed = true;
                        },
                    }
                }
            }
        }
    }

    // Phase 2: report once, against the settled entry states.
    let mut flow_findings = Vec::new();
    for block_id in order {
        let entry = in_states.get(&block_id).cloned().unwrap_or_default();
        let outcome = verify_block(
            body, module, block_id, func_name, entity, entry, true, mode, &aliases,
        );
        errors.extend(outcome.errors);
        flow_findings.extend(outcome.flow_findings);
    }

    if !flow_findings.is_empty() {
        report_flow_findings(func_name, &flow_findings);
    }

    errors
}

/// Ownership state on entry to the function.
///
/// Consuming parameters arrive owned and un-consumed. They are not block
/// params, so nothing else would ever track them: without seeding, every
/// consume of a parameter falls into the untracked `None` arm and the walk is
/// blind to a parameter consumed twice.
fn entry_state(body: &OssaBody) -> FlowState {
    let mut state = FlowState::default();
    for i in 0..body.param_count {
        let v = ValueId::new(i);
        if body
            .values
            .get(v.index())
            .is_some_and(|vd| vd.ownership == Ownership::Owned)
        {
            state.owned.insert(v, ValueState::Live);
        }
    }
    state
}

/// Summarise newly-visible violations on stderr. `Warn` mode only — this is a
/// measurement aid for staging the flow-sensitive walk, not a diagnostic
/// surface, so it deliberately bypasses the diagnostic machinery.
fn report_flow_findings(func_name: &str, findings: &[VerifyError]) {
    eprintln!(
        "[KESTREL_VERIFY_FLOW] {} newly-visible ownership violation(s) in '{}':",
        findings.len(),
        func_name,
    );
    for f in findings {
        eprintln!("  {:?}[{:?}]: {}", f.block, f.inst, f.message);
    }
}

// ---------------------------------------------------------------------------
// Check 1: ValueId uniqueness
// ---------------------------------------------------------------------------

/// Returns the definition map (`ValueId` -> defining block) it builds anyway,
/// so later checks don't recompute it. On a duplicate definition the *first*
/// block wins the map entry and the duplicate is reported.
fn check_value_uniqueness(
    body: &OssaBody,
    func_name: &str,
    entity: Entity,
    errors: &mut Vec<VerifyError>,
) -> FxHashMap<ValueId, BlockId> {
    // Map from ValueId -> (block that defined it).
    let mut definitions: FxHashMap<ValueId, BlockId> = FxHashMap::default();

    for (block_idx, block) in body.blocks.iter().enumerate() {
        let block_id = BlockId::new(block_idx);

        // Block params define values.
        for param in &block.params {
            if let Some(&prev_block) = definitions.get(&param.value) {
                errors.push(VerifyError {
                    block: block_id,
                    inst: None,
                    message: format!(
                        "value {:?} defined as block param in {:?} but already defined in {:?}",
                        param.value, block_id, prev_block,
                    ),
                    span: body
                        .values
                        .get(param.value.index())
                        .and_then(|vd| vd.span.clone()),
                    func_name: func_name.to_string(),
                    entity,
                    diag: None,
                });
            } else {
                definitions.insert(param.value, block_id);
            }
        }

        // Instruction results define values.
        for (inst_idx, inst) in block.insts.iter().enumerate() {
            for result in inst.kind.results() {
                if let Some(&prev_block) = definitions.get(&result) {
                    errors.push(VerifyError {
                        block: block_id,
                        inst: Some(inst_idx as u32),
                        message: format!(
                            "value {:?} defined by instruction {} in {:?} but already defined in {:?}",
                            result, inst_idx, block_id, prev_block,
                        ),
                        span: inst.span.clone(),
                        func_name: func_name.to_string(),
                        entity,
                        diag: None,
                    });
                } else {
                    definitions.insert(result, block_id);
                }
            }
        }
    }

    definitions
}

// ---------------------------------------------------------------------------
// Block ordering
// ---------------------------------------------------------------------------

/// Reachable blocks in reverse postorder.
///
/// RPO visits every block after at least one of its predecessors except across
/// back edges, which is the order a forward dataflow fixpoint wants: it
/// minimises the number of times a block has to be re-analysed. Today the walk
/// carries no state between blocks so any order would do, but the ordering is
/// the seam the flow-sensitive walk plugs into. Unreachable blocks are
/// excluded, exactly as the previous BFS excluded them.
fn reverse_postorder(body: &OssaBody) -> Vec<BlockId> {
    let mut postorder = Vec::new();
    let mut visited = FxHashSet::default();
    // Explicit stack: (block, next successor index) — recursion would blow the
    // stack on long function bodies.
    let mut stack: Vec<(BlockId, usize)> = vec![(body.entry, 0)];
    visited.insert(body.entry);

    while let Some((block_id, succ_idx)) = stack.pop() {
        let successors = body.block(block_id).terminator.kind.successors();
        if succ_idx < successors.len() {
            stack.push((block_id, succ_idx + 1));
            let succ = successors[succ_idx];
            if visited.insert(succ) {
                stack.push((succ, 0));
            }
        } else {
            postorder.push(block_id);
        }
    }

    postorder.reverse();
    postorder
}

// ---------------------------------------------------------------------------
// Dominance
// ---------------------------------------------------------------------------

/// Immediate dominators, indexed by block, for the reachable CFG.
///
/// Cooper-Harvey-Kennedy: iterate over blocks in RPO, intersecting the
/// already-computed idoms of each block's processed predecessors, until
/// nothing changes. Small and adequate here — bodies are function-sized.
///
/// `None` for the entry block and for unreachable blocks.
fn immediate_dominators(body: &OssaBody, order: &[BlockId]) -> FxHashMap<BlockId, Option<BlockId>> {
    // Position in RPO; also the "have we processed this yet" test.
    let mut rpo_index: FxHashMap<BlockId, usize> = FxHashMap::default();
    for (i, &b) in order.iter().enumerate() {
        rpo_index.insert(b, i);
    }

    let mut preds: FxHashMap<BlockId, Vec<BlockId>> = FxHashMap::default();
    for &b in order {
        for succ in body.block(b).terminator.kind.successors() {
            if rpo_index.contains_key(&succ) {
                preds.entry(succ).or_default().push(b);
            }
        }
    }

    let mut idom: FxHashMap<BlockId, Option<BlockId>> = FxHashMap::default();
    for &b in order {
        idom.insert(b, None);
    }
    // The entry dominates itself; represented by staying `None` while being
    // treated as "processed" below.
    let mut processed: FxHashSet<BlockId> = FxHashSet::default();
    processed.insert(body.entry);

    let intersect = |mut a: BlockId,
                     mut b: BlockId,
                     idom: &FxHashMap<BlockId, Option<BlockId>>,
                     rpo_index: &FxHashMap<BlockId, usize>|
     -> BlockId {
        // Walk both up the dominator tree until they meet.
        while a != b {
            while rpo_index[&a] > rpo_index[&b] {
                match idom.get(&a).copied().flatten() {
                    Some(next) => a = next,
                    None => return b,
                }
            }
            while rpo_index[&b] > rpo_index[&a] {
                match idom.get(&b).copied().flatten() {
                    Some(next) => b = next,
                    None => return a,
                }
            }
        }
        a
    };

    let mut changed = true;
    while changed {
        changed = false;
        for &b in order {
            if b == body.entry {
                continue;
            }
            let Some(bpreds) = preds.get(&b) else {
                continue;
            };
            let mut new_idom: Option<BlockId> = None;
            for &p in bpreds {
                if !processed.contains(&p) {
                    continue;
                }
                new_idom = Some(match new_idom {
                    None => p,
                    Some(cur) => intersect(p, cur, &idom, &rpo_index),
                });
            }
            if let Some(candidate) = new_idom
                && idom.get(&b).copied().flatten() != Some(candidate)
            {
                idom.insert(b, Some(candidate));
                processed.insert(b);
                changed = true;
            }
            processed.insert(b);
        }
    }

    idom
}

/// Does `a` dominate `b`? Walks `b` up the dominator tree.
fn dominates(a: BlockId, b: BlockId, idom: &FxHashMap<BlockId, Option<BlockId>>) -> bool {
    let mut cur = b;
    loop {
        if cur == a {
            return true;
        }
        match idom.get(&cur).copied().flatten() {
            Some(next) => cur = next,
            None => return false,
        }
    }
}

/// Check 1c: every operand's definition must DOMINATE its use.
///
/// This is the sound version of a check the audit originally prescribed for
/// F34 — "every operand must be defined by this block's params or an earlier
/// instruction in it". That was implemented and measured, and it rejects the
/// stdlib: lowering uses the ordinary SSA rule, where a block may reference
/// any definition that dominates it. Dominance is the property that actually
/// holds, and the one that makes a definition meaningful at a use site.
///
/// `check_operands_defined` only asks whether a definition exists ANYWHERE,
/// so it accepts a use that a definition cannot reach.
fn check_dominance(
    body: &OssaBody,
    order: &[BlockId],
    def_blocks: &FxHashMap<ValueId, BlockId>,
    func_name: &str,
    entity: Entity,
    errors: &mut Vec<VerifyError>,
) {
    let idom = immediate_dominators(body, order);

    for &block_id in order {
        let block = body.block(block_id);

        // Values usable at the top of this block without dominance analysis.
        let mut available: FxHashSet<ValueId> = FxHashSet::default();
        for param in &block.params {
            available.insert(param.value);
        }
        if block_id == body.entry {
            for i in 0..body.param_count {
                available.insert(ValueId::new(i));
            }
        }

        let check = |operand: ValueId,
                         inst: Option<u32>,
                         span: Option<Span>,
                         available: &FxHashSet<ValueId>,
                         errors: &mut Vec<VerifyError>| {
            if available.contains(&operand) {
                return;
            }
            let Some(&def_block) = def_blocks.get(&operand) else {
                // No definition at all — check_operands_defined reports that.
                return;
            };
            if def_block == block_id {
                // Defined in this block but not yet available: used before it
                // was produced.
                errors.push(VerifyError {
                    block: block_id,
                    inst,
                    message: format!(
                        "operand {operand:?} is used before it is defined in {block_id:?}"
                    ),
                    span,
                    func_name: func_name.to_string(),
                    entity,
                    diag: None,
                });
                return;
            }
            if !dominates(def_block, block_id, &idom) {
                errors.push(VerifyError {
                    block: block_id,
                    inst,
                    message: format!(
                        "operand {operand:?} is defined in {def_block:?}, which does not \
                         dominate {block_id:?} — the definition cannot be guaranteed to \
                         have executed, so it must arrive as a block parameter"
                    ),
                    span,
                    func_name: func_name.to_string(),
                    entity,
                    diag: None,
                });
            }
        };

        for (inst_idx, inst) in block.insts.iter().enumerate() {
            for operand in inst.kind.operands() {
                check(
                    operand,
                    Some(inst_idx as u32),
                    inst.span.clone(),
                    &available,
                    errors,
                );
            }
            for result in inst.kind.results() {
                available.insert(result);
            }
        }
        for operand in block.terminator.kind.operands() {
            check(
                operand,
                None,
                block.terminator.span.clone(),
                &available,
                errors,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Address aliasing
// ---------------------------------------------------------------------------

/// Which underlying slot each address-typed value refers to.
///
/// An address created by `Uninit` in one block and threaded to a successor as
/// a block argument arrives there under a *different* `ValueId`. Both name the
/// same storage, and lowering mixes them freely — the guarded-destroy shape
/// takes through the renamed param but destroys through the original id. A
/// per-`ValueId` init map is blind to that in both directions: the take
/// appears to touch an untracked address, and the destroy sees a slot that is
/// still `Init`.
///
/// So collapse every address to the `Uninit` that created it, and key init
/// state by that root instead. Flow-insensitive and computed once per body.
#[derive(Debug, Default)]
struct AddrAliases {
    /// address value -> the `Uninit` result that created its storage.
    root: FxHashMap<ValueId, ValueId>,
    /// Params reachable from two *different* roots. Nothing can be said about
    /// their storage, so they are untracked — which is exactly how the walk
    /// behaved for every cross-block address before aliasing existed.
    poisoned: FxHashSet<ValueId>,
    /// `FieldAddr` result -> (root address, field). Whole-function, so a
    /// projection taken in one block is still understood in another.
    field_of: FxHashMap<ValueId, (ValueId, FieldIdx)>,
}

impl AddrAliases {
    fn build(body: &OssaBody) -> AddrAliases {
        let mut aliases = AddrAliases::default();

        // Roots.
        for block in &body.blocks {
            for inst in &block.insts {
                if let InstKind::Uninit { result, .. } = &inst.kind {
                    aliases.root.insert(*result, *result);
                }
            }
        }

        // Propagate along block-argument bindings to a fixpoint: a param's
        // root may only become known after a later terminator is processed,
        // and back edges mean one pass is not enough.
        let mut changed = true;
        while changed {
            changed = false;
            for block in &body.blocks {
                for (succ, args) in block.terminator.kind.successor_args() {
                    let params = &body.block(succ).params;
                    for (arg, param) in args.iter().zip(params.iter()) {
                        let Some(&arg_root) = aliases.root.get(arg) else {
                            continue;
                        };
                        if aliases.poisoned.contains(&param.value) {
                            continue;
                        }
                        match aliases.root.get(&param.value) {
                            Some(&existing) if existing == arg_root => {},
                            Some(_) => {
                                // Two different slots reach this param.
                                aliases.root.remove(&param.value);
                                aliases.poisoned.insert(param.value);
                                changed = true;
                            },
                            None => {
                                aliases.root.insert(param.value, arg_root);
                                changed = true;
                            },
                        }
                    }
                }
            }
        }

        // Field projections, now that roots are settled.
        for block in &body.blocks {
            for inst in &block.insts {
                if let InstKind::FieldAddr {
                    result, base, field, ..
                } = &inst.kind
                    && let Some(&base_root) = aliases.root.get(base)
                {
                    aliases.field_of.insert(*result, (base_root, *field));
                }
            }
        }

        aliases
    }

    /// The slot `v` names, or `None` when it cannot be attributed to one.
    fn resolve(&self, v: ValueId) -> Option<ValueId> {
        if self.poisoned.contains(&v) {
            return None;
        }
        self.root.get(&v).copied()
    }
}

// ---------------------------------------------------------------------------
// Check 1b: every operand has a definition
// ---------------------------------------------------------------------------

fn check_operands_defined(
    body: &OssaBody,
    func_name: &str,
    entity: Entity,
    errors: &mut Vec<VerifyError>,
) {
    // Collect all definitions: function params + block params + instruction results.
    let mut definitions: FxHashSet<ValueId> = FxHashSet::default();
    for i in 0..body.param_count {
        definitions.insert(ValueId::new(i));
    }
    for block in &body.blocks {
        for param in &block.params {
            definitions.insert(param.value);
        }
        for inst in &block.insts {
            for result in inst.kind.results() {
                definitions.insert(result);
            }
        }
    }

    // Check instruction operands.
    for (block_idx, block) in body.blocks.iter().enumerate() {
        let block_id = BlockId::new(block_idx);
        for (inst_idx, inst) in block.insts.iter().enumerate() {
            for operand in inst.kind.operands() {
                if !definitions.contains(&operand) {
                    errors.push(VerifyError {
                        block: block_id,
                        inst: Some(inst_idx as u32),
                        message: format!(
                            "operand {:?} used but never defined (no block param or instruction produces it)",
                            operand,
                        ),
                        span: inst.span.clone(),
                        func_name: func_name.to_string(),
                        entity,
                        diag: None,
                    });
                }
            }
        }

        // Check terminator operands.
        for operand in block.terminator.kind.operands() {
            if !definitions.contains(&operand) {
                errors.push(VerifyError {
                    block: block_id,
                    inst: None,
                    message: format!("terminator operand {:?} used but never defined", operand,),
                    span: block.terminator.span.clone(),
                    func_name: func_name.to_string(),
                    entity,
                    diag: None,
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Per-block verification state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueState {
    Live,
    Consumed,
    /// Consumed on some incoming paths but not others. Legal: this is how a
    /// conditional move looks once control flow merges, and the runtime drop
    /// flag is what makes it safe. Every operation on a `Maybe` state is
    /// therefore PERMITTED — the walk reports only *definite* violations.
    MaybeConsumed,
}

impl ValueState {
    fn join(self, other: Self) -> Self {
        if self == other {
            self
        } else {
            ValueState::MaybeConsumed
        }
    }
}

#[derive(Debug, Clone)]
struct BorrowInfo {
    source: ValueId,
    is_mut: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InitState {
    Init,
    Uninit,
    /// Init on some incoming paths, uninit on others — the guarded-destroy
    /// shape (`emit_guarded_destroy`) that a drop flag resolves at runtime.
    /// Mirrors lowering's own `VarInit::MaybeUninit`; permissive, like
    /// `ValueState::MaybeConsumed`.
    MaybeInit,
}

impl InitState {
    fn join(self, other: Self) -> Self {
        if self == other {
            self
        } else {
            InitState::MaybeInit
        }
    }
}

#[derive(Debug, Clone)]
enum AddrKind {
    Whole(InitState),
    SubField {
        #[allow(dead_code)]
        ty: TyId,
        fields: FxHashMap<FieldIdx, InitState>,
    },
}

/// The part of the verifier's state that is a property of a *program point*
/// rather than of a block: what each `@owned` value's consumption state is and
/// what each address's init state is.
///
/// Split out because these are the two maps that must eventually flow along
/// CFG edges and be joined at merge points. Everything else on `FlowVerifier`
/// is either immutable context or genuinely block-scoped (borrows are required
/// to end or be forwarded within their block, which Check 4 enforces).
#[derive(Debug, Clone, Default)]
struct FlowState {
    /// Tracks @owned values: Live or Consumed.
    owned: FxHashMap<ValueId, ValueState>,
    /// Address init states.
    addrs: FxHashMap<ValueId, AddrKind>,
}

impl FlowState {
    /// Merge `other` (an incoming edge's out-state) into `self`. Returns
    /// whether anything changed, which is the fixpoint's termination signal.
    ///
    /// A key absent from one side keeps the present side's value rather than
    /// becoming `Maybe`: absence means "this path never mentioned the value",
    /// not "this path left it in the other state". Values are only reachable
    /// where they are defined, so a genuine partial-definition would have been
    /// caught by Check 1b already.
    fn join(&mut self, other: &FlowState) -> bool {
        let mut changed = false;

        for (&v, &incoming) in &other.owned {
            match self.owned.get(&v) {
                Some(&existing) => {
                    let merged = existing.join(incoming);
                    if merged != existing {
                        self.owned.insert(v, merged);
                        changed = true;
                    }
                },
                None => {
                    self.owned.insert(v, incoming);
                    changed = true;
                },
            }
        }

        for (&a, incoming) in &other.addrs {
            match self.addrs.get(&a) {
                Some(existing) => {
                    if let Some(merged) = existing.join(incoming) {
                        self.addrs.insert(a, merged);
                        changed = true;
                    }
                },
                None => {
                    self.addrs.insert(a, incoming.clone());
                    changed = true;
                },
            }
        }

        changed
    }
}

impl AddrKind {
    /// Merge two address states, returning `Some(merged)` only when the result
    /// differs from `self` (so the caller can track fixpoint progress).
    ///
    /// A `Whole` meeting a `SubField` collapses to `Whole`: the paths disagree
    /// about whether the slot is field-tracked at all, and whole-tracking is
    /// the conservative reading (any `Maybe` field makes the whole `Maybe`).
    fn join(&self, other: &AddrKind) -> Option<AddrKind> {
        let merged = match (self, other) {
            (AddrKind::Whole(a), AddrKind::Whole(b)) => AddrKind::Whole(a.join(*b)),
            (
                AddrKind::SubField { ty, fields: fa },
                AddrKind::SubField { fields: fb, .. },
            ) => {
                let mut fields = fa.clone();
                for (&idx, &b) in fb {
                    let merged = match fa.get(&idx) {
                        Some(&a) => a.join(b),
                        None => b,
                    };
                    fields.insert(idx, merged);
                }
                AddrKind::SubField { ty: *ty, fields }
            },
            // Mixed tracking granularity — collapse to whole.
            (AddrKind::Whole(a), AddrKind::SubField { fields, .. })
            | (AddrKind::SubField { fields, .. }, AddrKind::Whole(a)) => {
                let collapsed = fields
                    .values()
                    .copied()
                    .fold(*a, |acc, f| acc.join(f));
                AddrKind::Whole(collapsed)
            },
        };
        if merged.same_as(self) { None } else { Some(merged) }
    }

    fn same_as(&self, other: &AddrKind) -> bool {
        match (self, other) {
            (AddrKind::Whole(a), AddrKind::Whole(b)) => a == b,
            (AddrKind::SubField { fields: a, .. }, AddrKind::SubField { fields: b, .. }) => a == b,
            _ => false,
        }
    }
}

struct FlowVerifier<'a> {
    body: &'a OssaBody,
    _module: &'a MirModule,
    block_id: BlockId,
    func_name: &'a str,
    entity: Entity,
    /// Whether this function returns a borrowed reference (`-> &T`). The
    /// returned @guaranteed value is exempt from Check 4 (it deliberately
    /// outlives the block), and a @guaranteed return in a NON-ret_borrow
    /// function is an ICE (the copy guards must have copied it to @owned).
    ret_borrow: bool,

    /// Flowing state (owned/addr maps) — see `FlowState`.
    state: FlowState,
    /// How to treat violations that only became visible because state flowed
    /// in from a predecessor.
    mode: FlowMode,
    /// Suppress error construction during fixpoint iterations; only the final
    /// reporting pass records anything.
    report: bool,
    /// Values defined by THIS block (params + instruction results). A
    /// violation naming one of these was already reachable under the
    /// block-local walk, so it stays a hard error regardless of mode;
    /// anything else is newly-visible and is gated by `mode`.
    defined_here: FxHashSet<ValueId>,
    /// Newly-visible violations, held separately so `Warn` can report a
    /// summary without failing the build.
    flow_findings: Vec<VerifyError>,
    /// Active borrows keyed by the @guaranteed result value.
    borrows: FxHashMap<ValueId, BorrowInfo>,
    /// Live references: @guaranteed call results (only ref-returning calls
    /// produce them) keyed by result, holding the immediate borrow source.
    /// Used to attribute consume-while-borrowed to user code: a blocking
    /// borrow that a live ref chains through was kept alive *by* the ref,
    /// so the conflict is the user's (E498), not a lowering bug (ICE).
    refs: FxHashMap<ValueId, ValueId>,
    /// Whole-function address aliasing: which slot each address names.
    aliases: &'a AddrAliases,

    errors: Vec<VerifyError>,
}

impl<'a> FlowVerifier<'a> {
    fn new(
        body: &'a OssaBody,
        module: &'a MirModule,
        block_id: BlockId,
        func_name: &'a str,
        entity: Entity,
        state: FlowState,
        aliases: &'a AddrAliases,
    ) -> Self {
        let ret_borrow = module.functions.get(&entity).is_some_and(|f| {
            matches!(
                crate::item::function::ret_convention(&module.ty_arena, f.ret),
                crate::item::function::RetConvention::RefBorrow { .. }
            )
        });
        Self {
            body,
            _module: module,
            block_id,
            func_name,
            entity,
            ret_borrow,
            state,
            mode: FlowMode::current(),
            report: true,
            defined_here: FxHashSet::default(),
            flow_findings: Vec::new(),
            borrows: FxHashMap::default(),
            refs: FxHashMap::default(),
            aliases,
            errors: Vec::new(),
        }
    }

    fn err(&mut self, inst: Option<u32>, message: String) {
        let span = self.inst_span(inst);
        self.push_err(inst, span, message);
    }

    /// Like `err`, but when the triggering instruction has no span (e.g. errors
    /// raised from a terminator or block exit), fall back to the defining span
    /// of `value`. This is the common case for ownership/leak errors that name a
    /// specific value but fire at a block boundary where there's no instruction.
    fn err_val(&mut self, inst: Option<u32>, value: ValueId, message: String) {
        let span = self
            .inst_span(inst)
            .or_else(|| self.body.value(value).span.clone());
        self.push_err(inst, span, message);
    }

    fn inst_span(&self, inst: Option<u32>) -> Option<Span> {
        inst.and_then(|i| {
            self.body
                .block(self.block_id)
                .insts
                .get(i as usize)
                .and_then(|inst| inst.span.clone())
        })
    }

    fn push_err(&mut self, inst: Option<u32>, span: Option<Span>, message: String) {
        // Fixpoint iterations re-run the transfer function; only the final
        // reporting pass records. Without this, errors are duplicated once per
        // iteration and their order depends on how fast the fixpoint settles.
        if !self.report {
            return;
        }
        self.errors.push(VerifyError {
            block: self.block_id,
            inst,
            message,
            span,
            func_name: self.func_name.to_string(),
            entity: self.entity,
            diag: None,
        });
    }

    // -- Ownership helpers --

    /// Record that an @owned value has been produced. A definition OVERWRITES
    /// any inherited state, which is what makes loops sound: a value defined
    /// and consumed inside a loop body re-enters via the back edge as
    /// `Consumed`, and its defining instruction resets it to `Live`.
    fn define_owned(&mut self, v: ValueId) {
        self.state.owned.insert(v, ValueState::Live);
        self.defined_here.insert(v);
    }

    /// Report a violation that may only be visible because ownership state
    /// flowed in from a predecessor block.
    ///
    /// If the value was defined in this block the finding was already
    /// reachable block-locally, so it is a hard error as before. Otherwise it
    /// is new, and `Warn` collects it for the summary instead of failing.
    fn flow_err(&mut self, v: ValueId, inst: Option<u32>, message: String) {
        if self.defined_here.contains(&v) || self.mode == FlowMode::Enforce {
            self.err_val(inst, v, message);
            return;
        }
        if self.report && self.mode == FlowMode::Warn {
            let span = self
                .inst_span(inst)
                .or_else(|| self.body.value(v).span.clone());
            self.flow_findings.push(VerifyError {
                block: self.block_id,
                inst,
                message,
                span,
                func_name: self.func_name.to_string(),
                entity: self.entity,
                diag: None,
            });
        }
    }

    /// Attempt to consume an @owned value. Returns false if already consumed.
    fn try_consume(&mut self, v: ValueId, inst: Option<u32>) -> bool {
        self.try_consume_exempting(v, inst, None)
    }

    /// `try_consume` with an exemption set: borrows in `exempt` do not block
    /// the consume. Used by terminator forwarding — when a borrowed value and
    /// its borrow are BOTH forwarded as block args, the pair continues
    /// together in the target block (threaded ref bindings ride var slots
    /// across merges this way), so the forwarding "consume" is not an escape.
    fn try_consume_exempting(
        &mut self,
        v: ValueId,
        inst: Option<u32>,
        exempt: Option<&FxHashSet<ValueId>>,
    ) -> bool {
        let ownership = self.body.value(v).ownership;
        if ownership != Ownership::Owned {
            return true; // not tracked
        }
        match self.state.owned.get(&v) {
            Some(ValueState::Live) => {
                // Check borrow provenance: cannot consume while borrowed.
                let blocking: Vec<ValueId> = self
                    .borrows
                    .iter()
                    .filter(|(borrow_val, info)| {
                        info.source == v && !exempt.is_some_and(|e| e.contains(borrow_val))
                    })
                    .map(|(borrow_val, _)| *borrow_val)
                    .collect();
                if !blocking.is_empty() {
                    // Attribution: lowering ends plain scope borrows before a
                    // consume, so a blocking borrow at a consume site is only
                    // reachable from user code when a live reference (`&T`)
                    // chains to the consumed value and kept the borrow alive.
                    // That case is the user's error (E498); anything else is
                    // a lowering bug and stays an ICE.
                    if let Some(ref_val) = self.live_ref_rooted_at(v) {
                        self.err_consume_while_ref_live(v, inst, ref_val);
                    } else {
                        self.err_val(
                            inst,
                            v,
                            format!(
                                "cannot consume {:?}: active borrow(s) {:?} depend on it",
                                v, blocking,
                            ),
                        );
                    }
                }
                self.state.owned.insert(v, ValueState::Consumed);
                true
            },
            Some(ValueState::MaybeConsumed) => {
                // Consumed on some incoming paths only — the drop-flag shape.
                // Permitted; the runtime flag decides. Definitely consumed
                // from here on, so a *later* consume is still catchable.
                self.state.owned.insert(v, ValueState::Consumed);
                true
            },
            Some(ValueState::Consumed) => {
                self.flow_err(v, inst, format!("value {:?} consumed more than once", v));
                false
            },
            None => {
                // Not reached by any tracked definition. Under the block-local
                // walk this is the common case (the value was defined in
                // another block); once state flows it means the value is
                // genuinely untracked, e.g. a non-consuming function param.
                true
            },
        }
    }

    /// Slot-form of the consume-while-borrowed gate: a `Take` from `addr`
    /// while a borrow whose source is `addr` is live. Mirrors the value-form
    /// check in `try_consume_exempting` (E498 when a live `&T` chains to the
    /// slot; unattributable conflicts stay an ICE — a lowering bug). Does
    /// NOT touch owned-state: the address value itself stays live (it is
    /// consumed later by its own destroy).
    fn check_take_while_borrowed(&mut self, addr: ValueId, inst: Option<u32>) {
        let blocking: Vec<ValueId> = self
            .borrows
            .iter()
            .filter(|(_, info)| info.source == addr)
            .map(|(borrow_val, _)| *borrow_val)
            .collect();
        if blocking.is_empty() {
            return;
        }
        if let Some(ref_val) = self.live_ref_rooted_at(addr) {
            self.err_consume_while_ref_live(addr, inst, ref_val);
        } else {
            self.err_val(
                inst,
                addr,
                format!(
                    "cannot take from {:?}: active borrow(s) {:?} depend on it",
                    addr, blocking,
                ),
            );
        }
    }

    /// Find a live reference (@guaranteed call result) whose borrow-source
    /// chain reaches `v`. Walks `borrows` from the ref's immediate source;
    /// the chain is acyclic (each borrow's source precedes it), but the walk
    /// is bounded anyway as a hand-built-MIR guard.
    fn live_ref_rooted_at(&self, v: ValueId) -> Option<ValueId> {
        self.refs.iter().find_map(|(&ref_val, &src)| {
            let mut cur = src;
            for _ in 0..=self.borrows.len() {
                if cur == v {
                    return Some(ref_val);
                }
                match self.borrows.get(&cur) {
                    Some(info) => cur = info.source,
                    None => break,
                }
            }
            None
        })
    }

    /// E498: user-attributable consume-while-borrowed — the value is consumed
    /// (moved/destroyed) while a reference into it is still live.
    fn err_consume_while_ref_live(&mut self, v: ValueId, inst: Option<u32>, ref_val: ValueId) {
        let name = self
            .body
            .value_names
            .get(&v)
            .map(|n| format!("`{n}`"))
            .unwrap_or_else(|| "this value".to_string());
        let span = self
            .inst_span(inst)
            .or_else(|| self.body.value(v).span.clone());
        let secondary = self
            .body
            .value(ref_val)
            .span
            .clone()
            .map(|s| (s, "the reference is created here and is still live".into()));
        self.errors.push(VerifyError {
            block: self.block_id,
            inst,
            message: format!("cannot consume {name} while a reference into it is live"),
            span,
            func_name: self.func_name.to_string(),
            entity: self.entity,
            diag: Some(UserDiag {
                code: "E498",
                secondary,
                notes: vec![
                    "a reference reads its value's storage in place; consuming the value here \
                     would leave the reference dangling"
                        .into(),
                ],
            }),
        });
    }

    /// Assert a value is live (not consumed). Used for reads.
    fn assert_live(&mut self, v: ValueId, inst: Option<u32>) {
        let ownership = self.body.value(v).ownership;
        if ownership != Ownership::Owned {
            return;
        }
        // MaybeConsumed is permitted — see `ValueState::MaybeConsumed`.
        if let Some(ValueState::Consumed) = self.state.owned.get(&v) {
            self.flow_err(v, inst, format!("use of consumed value {:?}", v));
        }
    }

    /// Assert a value is live and not mut-borrowed (readable). For mut borrows
    /// the source cannot be read.
    fn assert_readable(&mut self, v: ValueId, inst: Option<u32>) {
        self.assert_live(v, inst);

        // Check 5 (mut borrow): while a mut borrow is active on v, v cannot be read.
        let mut_borrows: Vec<ValueId> = self
            .borrows
            .iter()
            .filter(|(_, info)| info.source == v && info.is_mut)
            .map(|(bv, _)| *bv)
            .collect();
        if !mut_borrows.is_empty() {
            self.err_val(
                inst,
                v,
                format!(
                    "cannot read {:?}: active mut borrow(s) {:?}",
                    v, mut_borrows,
                ),
            );
        }
    }

    // -- Address helpers --

    fn addr_require_init(&mut self, addr: ValueId, inst: Option<u32>) {
        // If this is a field addr, check the specific field.
        if let Some(&(base, field)) = self.aliases.field_of.get(&addr) {
            if let Some(AddrKind::SubField { fields, .. }) = self.state.addrs.get(&base)
                && let Some(InitState::Uninit) = fields.get(&field)
            {
                self.err(
                    inst,
                    format!("field {:?} of address {:?} is uninit", field, base),
                );
            }
            return;
        }

        // Collect error messages first to avoid borrow conflict.
        let Some(addr) = self.aliases.resolve(addr) else {
            return;
        };
        let mut errs = Vec::new();
        if let Some(ak) = self.state.addrs.get(&addr) {
            match ak {
                AddrKind::Whole(InitState::Uninit) => {
                    errs.push(format!("address {:?} is uninit", addr));
                },
                // MaybeInit: init on some paths only (guarded destroy) — the
                // drop flag decides at runtime. Permitted.
                AddrKind::Whole(InitState::MaybeInit) => {},
                AddrKind::SubField { fields, .. } => {
                    for (f, st) in fields {
                        if *st == InitState::Uninit {
                            errs.push(format!(
                                "field {:?} of sub-field-tracked address {:?} is uninit",
                                f, addr,
                            ));
                        }
                    }
                },
                AddrKind::Whole(InitState::Init) => {},
            }
        }
        for msg in errs {
            self.flow_err(addr, inst, msg);
        }
    }

    fn addr_set_uninit(&mut self, addr: ValueId, inst: Option<u32>) {
        // If this is a field addr, set that specific field.
        if let Some(&(base, field)) = self.aliases.field_of.get(&addr) {
            let mut err_msg = None;
            if let Some(AddrKind::SubField { fields, .. }) = self.state.addrs.get_mut(&base)
                && let Some(st) = fields.get_mut(&field)
            {
                if *st == InitState::Uninit {
                    err_msg = Some(format!(
                        "field {:?} of address {:?} already uninit",
                        field, base,
                    ));
                }
                *st = InitState::Uninit;
            }
            if let Some(msg) = err_msg {
                self.err(inst, msg);
            }
            return;
        }

        let Some(addr) = self.aliases.resolve(addr) else {
            return;
        };
        let mut err_msg = None;
        if let Some(ak) = self.state.addrs.get_mut(&addr) {
            match ak {
                AddrKind::Whole(st) => {
                    // MaybeInit is the guarded-destroy arm — permitted.
                    if *st == InitState::Uninit {
                        err_msg = Some(format!("address {:?} already uninit", addr));
                    }
                    *st = InitState::Uninit;
                },
                AddrKind::SubField { .. } => {
                    // Whole uninit of a sub-field tracked addr.
                    *ak = AddrKind::Whole(InitState::Uninit);
                },
            }
        }
        if let Some(msg) = err_msg {
            self.flow_err(addr, inst, msg);
        }
    }

    fn addr_store_init(&mut self, addr: ValueId, inst: Option<u32>) {
        // If this is a field addr, set that specific field.
        if let Some(&(base, field)) = self.aliases.field_of.get(&addr) {
            let mut err_msg = None;
            if let Some(AddrKind::SubField { fields, .. }) = self.state.addrs.get_mut(&base)
                && let Some(st) = fields.get_mut(&field)
            {
                if *st == InitState::Init {
                    err_msg = Some(format!(
                        "store_init on field {:?} of address {:?} but field already init",
                        field, base,
                    ));
                }
                *st = InitState::Init;
            }
            if let Some(msg) = err_msg {
                self.err(inst, msg);
            }
            return;
        }

        let Some(addr) = self.aliases.resolve(addr) else {
            return;
        };
        let mut err_msg = None;
        if let Some(ak) = self.state.addrs.get_mut(&addr) {
            match ak {
                AddrKind::Whole(st) => {
                    // MaybeInit: re-initialising a conditionally-moved slot,
                    // which is the in-loop reinit shape. Permitted.
                    if *st == InitState::Init {
                        err_msg =
                            Some(format!("store_init on address {:?} but already init", addr,));
                    }
                    *st = InitState::Init;
                },
                AddrKind::SubField { .. } => {
                    // Whole store on sub-field tracked address (e.g. var local
                    // initialized with a complete struct value) — all fields init.
                    *ak = AddrKind::Whole(InitState::Init);
                },
            }
        }
        if let Some(msg) = err_msg {
            self.flow_err(addr, inst, msg);
        }
    }

    fn addr_store_assign(&mut self, addr: ValueId, inst: Option<u32>) {
        let Some(addr) = self.aliases.resolve(addr) else {
            return;
        };
        let mut err_msg = None;
        if let Some(ak) = self.state.addrs.get(&addr)
            && let AddrKind::Whole(InitState::Uninit) = ak
        {
            err_msg = Some(format!(
                "store_assign on address {:?} but it is uninit",
                addr,
            ));
        }
        if let Some(msg) = err_msg {
            self.flow_err(addr, inst, msg);
        }
    }

    // -- Main verification --

    fn verify(mut self) -> BlockOutcome {
        let block = self.body.block(self.block_id);

        // Register block params.
        for param in &block.params {
            if param.ownership == Ownership::Owned {
                self.define_owned(param.value);
            }
            if param.ownership == Ownership::Guaranteed {
                // Track borrow from borrow_source if available.
                let def = self.body.value(param.value);
                if let Some(src) = def.borrow_source {
                    self.borrows.insert(
                        param.value,
                        BorrowInfo {
                            source: src,
                            is_mut: false,
                        },
                    );
                }
            }
        }

        // Process each instruction.
        for (inst_idx, inst) in block.insts.iter().enumerate() {
            let idx = Some(inst_idx as u32);
            self.verify_instruction(&inst.kind, idx);
        }

        // Process terminator.
        self.verify_terminator(block);

        BlockOutcome {
            errors: self.errors,
            flow_findings: self.flow_findings,
            out_state: self.state,
        }
    }

    fn verify_instruction(&mut self, kind: &InstKind, idx: Option<u32>) {
        match kind {
            // -- Value lifecycle --
            InstKind::CopyValue { result, operand } => {
                self.assert_readable(*operand, idx);
                self.define_owned(*result);
            },
            InstKind::MoveValue { result, operand } => {
                self.try_consume(*operand, idx);
                self.define_owned(*result);
            },
            InstKind::DestroyValue { operand } => {
                self.try_consume(*operand, idx);
            },
            InstKind::CoerceFnKind {
                result,
                operand,
                from,
                to,
            } => {
                if matches!((from, to), (crate::FnKind::Escaping, crate::FnKind::Consuming)) {
                    self.try_consume(*operand, idx);
                } else {
                    self.assert_readable(*operand, idx);
                }
                self.define_owned(*result);
            },

            // -- Borrowing --
            InstKind::BeginBorrow { result, operand } => {
                self.assert_live(*operand, idx);
                let source = self.body.value(*result).borrow_source.unwrap_or(*operand);
                self.borrows.insert(
                    *result,
                    BorrowInfo {
                        source,
                        is_mut: false,
                    },
                );
            },
            InstKind::EndBorrow { operand } => {
                self.borrows.remove(operand);
                self.refs.remove(operand);
            },
            InstKind::BeginMutBorrow { result, operand } => {
                self.assert_live(*operand, idx);
                let source = self.body.value(*result).borrow_source.unwrap_or(*operand);
                self.borrows.insert(
                    *result,
                    BorrowInfo {
                        source,
                        is_mut: true,
                    },
                );
            },
            InstKind::EndMutBorrow { operand } => {
                self.borrows.remove(operand);
                self.refs.remove(operand);
            },

            // -- Memory access --
            InstKind::Load { result, address } => {
                self.addr_require_init(*address, idx);
                self.define_owned(*result);
            },
            InstKind::CopyAddr {
                result: _, address, ..
            } => {
                self.addr_require_init(*address, idx);
                // Result is @owned, register it.
                if let Some(r) = kind.result()
                    && self.body.value(r).ownership == Ownership::Owned
                {
                    self.define_owned(r);
                }
            },
            InstKind::Take {
                result: _, address, ..
            } => {
                self.addr_require_init(*address, idx);
                // Taking the contents while a borrow into the slot is live
                // leaves the borrow dangling — the slot-form of the
                // consume-while-borrowed conflict (`let`/`var` locals live
                // at addresses, so `eat(b)` takes b's slot while `b.peek()`
                // is still borrowed from it). Attributed to a live `&T`
                // chaining to the slot (E498); otherwise a lowering bug.
                self.check_take_while_borrowed(*address, idx);
                self.addr_set_uninit(*address, idx);
                if let Some(r) = kind.result()
                    && self.body.value(r).ownership == Ownership::Owned
                {
                    self.define_owned(r);
                }
            },
            InstKind::BeginBorrowAddr {
                result, address, ..
            } => {
                self.addr_require_init(*address, idx);
                let source = self.body.value(*result).borrow_source.unwrap_or(*address);
                self.borrows.insert(
                    *result,
                    BorrowInfo {
                        source,
                        is_mut: false,
                    },
                );
            },
            InstKind::BeginMutBorrowAddr {
                result, address, ..
            } => {
                self.addr_require_init(*address, idx);
                let source = self.body.value(*result).borrow_source.unwrap_or(*address);
                self.borrows.insert(
                    *result,
                    BorrowInfo {
                        source,
                        is_mut: true,
                    },
                );
            },
            InstKind::StoreInit { address, value } => {
                // The stored value is consumed.
                self.try_consume(*value, idx);
                self.addr_store_init(*address, idx);
            },
            InstKind::StoreAssign { address, value } => {
                self.try_consume(*value, idx);
                self.addr_store_assign(*address, idx);
            },
            InstKind::DestroyAddr { address, .. } => {
                self.addr_require_init(*address, idx);
                self.addr_set_uninit(*address, idx);
            },

            // -- Discriminant (non-consuming read, produces @owned i32) --
            InstKind::Discriminant { result, operand } => {
                self.assert_readable(*operand, idx);
                self.define_owned(*result);
            },

            // -- Computation (non-consuming reads of scalar operands) --
            InstKind::Op1 { result, op: _, arg } => {
                self.assert_readable(*arg, idx);
                if self.body.value(*result).ownership == Ownership::Owned {
                    self.define_owned(*result);
                } else {
                    let source = self.body.value(*result).borrow_source.unwrap_or(*arg);
                    self.borrows.insert(
                        *result,
                        BorrowInfo {
                            source,
                            is_mut: false,
                        },
                    );
                }
            },
            InstKind::Op2 {
                result,
                op,
                lhs,
                rhs,
            } => {
                self.assert_readable(*lhs, idx);
                if matches!(op, crate::Op::PtrWrite(_)) {
                    // PtrWrite moves the rhs into the destination address.
                    self.try_consume(*rhs, idx);
                } else {
                    self.assert_readable(*rhs, idx);
                }
                self.define_owned(*result);
            },
            InstKind::Op3 {
                result,
                op: _,
                a,
                b,
                c,
            } => {
                self.assert_readable(*a, idx);
                self.assert_readable(*b, idx);
                self.assert_readable(*c, idx);
                self.define_owned(*result);
            },

            // -- Constants --
            InstKind::Literal { result, .. } => {
                self.define_owned(*result);
            },
            InstKind::GlobalRef { result, .. } => {
                self.define_owned(*result);
            },

            // -- Aggregate construction: operands that are @owned are consumed --
            InstKind::Struct { result, ty, fields } => {
                self.check_struct_construct_complete(*ty, fields, idx);
                for (_, v) in fields {
                    if self.body.value(*v).ownership == Ownership::Owned {
                        self.try_consume(*v, idx);
                    }
                }
                if self.body.value(*result).ownership == Ownership::Owned {
                    self.define_owned(*result);
                }
            },
            InstKind::Tuple { result, elements } => {
                for v in elements {
                    if self.body.value(*v).ownership == Ownership::Owned {
                        self.try_consume(*v, idx);
                    }
                }
                if self.body.value(*result).ownership == Ownership::Owned {
                    self.define_owned(*result);
                }
            },
            InstKind::Enum {
                result, payload, ..
            } => {
                for v in payload {
                    if self.body.value(*v).ownership == Ownership::Owned {
                        self.try_consume(*v, idx);
                    }
                }
                if self.body.value(*result).ownership == Ownership::Owned {
                    self.define_owned(*result);
                }
            },
            InstKind::Array {
                result, elements, ..
            } => {
                for v in elements {
                    if self.body.value(*v).ownership == Ownership::Owned {
                        self.try_consume(*v, idx);
                    }
                }
                if self.body.value(*result).ownership == Ownership::Owned {
                    self.define_owned(*result);
                }
            },

            // -- Aggregate extraction --
            InstKind::StructExtract {
                result, operand, ..
            }
            | InstKind::TupleExtract {
                result, operand, ..
            }
            | InstKind::EnumPayload {
                result, operand, ..
            } => {
                let op_ownership = self.body.value(*operand).ownership;
                if op_ownership == Ownership::Owned {
                    // Consuming extraction.
                    self.try_consume(*operand, idx);
                }
                // For @guaranteed, not consuming — it is a projection.
                if self.body.value(*result).ownership == Ownership::Owned {
                    self.define_owned(*result);
                }
            },

            // -- Destructuring: operand is consumed (single consume of aggregate) --
            InstKind::DestructureStruct { results, operand }
            | InstKind::DestructureTuple { results, operand } => {
                self.try_consume(*operand, idx);
                for r in results {
                    if self.body.value(*r).ownership == Ownership::Owned {
                        self.define_owned(*r);
                    }
                }
            },
            InstKind::DestructureEnum {
                results, operand, ..
            } => {
                self.try_consume(*operand, idx);
                for r in results {
                    if self.body.value(*r).ownership == Ownership::Owned {
                        self.define_owned(*r);
                    }
                }
            },

            // -- Calls --
            InstKind::Call { result, args, .. } => {
                for arg in args {
                    match arg.convention {
                        ParamConvention::Consuming => {
                            self.try_consume(arg.value, idx);
                        },
                        ParamConvention::Borrow | ParamConvention::MutBorrow => {
                            self.assert_live(arg.value, idx);
                        },
                    }
                }
                if let Some(r) = result {
                    let ty = self.body.value(*r).ty;
                    let is_never = matches!(self._module.ty_arena.get(ty), crate::ty::MirTy::Never);
                    if self.body.value(*r).ownership == Ownership::Owned && !is_never {
                        self.define_owned(*r);
                    }
                    // A @guaranteed call result is a reference (only
                    // ret_borrow callees produce them); track it for
                    // consume-while-borrowed attribution (E498).
                    if self.body.value(*r).ownership == Ownership::Guaranteed
                        && let Some(src) = self.body.value(*r).borrow_source
                    {
                        self.refs.insert(*r, src);
                    }
                }
            },
            InstKind::ApplyPartial {
                result, captures, ..
            } => {
                // Captures are consumed (they are moved into the closure).
                for v in captures {
                    if self.body.value(*v).ownership == Ownership::Owned {
                        self.try_consume(*v, idx);
                    }
                }
                if self.body.value(*result).ownership == Ownership::Owned {
                    self.define_owned(*result);
                }
            },

            // -- Address projection --
            InstKind::FieldAddr {
                result,
                base,
                field,
                ..
            } => {
                self.define_owned(*result);
                let _ = (base, field); // projections come from `AddrAliases`
            },

            // -- Uninit: creates sub-field tracking --
            InstKind::Uninit { result, ty } => {
                self.define_owned(*result);
                // Look up how many fields this type has.
                let field_count = self.struct_field_count(*ty);
                if let Some(count) = field_count {
                    let mut fields = FxHashMap::default();
                    for i in 0..count {
                        fields.insert(FieldIdx::new(i), InitState::Uninit);
                    }
                    self.state.addrs
                        .insert(*result, AddrKind::SubField { ty: *ty, fields });
                } else {
                    // Non-struct type: whole tracking, starts uninit.
                    self.state.addrs
                        .insert(*result, AddrKind::Whole(InitState::Uninit));
                }
            },
        }
    }

    /// A `Struct` instruction must supply every field of its type exactly once.
    ///
    /// Struct construction is where the independently-computed field rosters
    /// meet: the memberwise-init roster decides the arguments, the MIR layout
    /// decides the `FieldIdx` space. When those drifted apart (F3) the result
    /// was a wrong-slot write and an uninitialized field, with no diagnostic at
    /// any stage. This turns any future divergence — a new member kind, a new
    /// construction path — into a verifier error at the choke point, whichever
    /// producer introduced it.
    ///
    /// Skipped for types with no lowered struct def (nothing to check against).
    fn check_struct_construct_complete(
        &mut self,
        ty: TyId,
        fields: &[(FieldIdx, ValueId)],
        idx: Option<u32>,
    ) {
        let Some(count) = self.struct_field_count(ty) else {
            return;
        };
        let mut seen = vec![0usize; count];
        for (f, _) in fields {
            match seen.get_mut(f.index()) {
                Some(slot) => *slot += 1,
                None => {
                    let span = self.inst_span(idx);
                    self.push_err(
                        idx,
                        span,
                        format!(
                            "struct construction writes field index {} but the type has \
                             only {count} field(s)",
                            f.index()
                        ),
                    );
                },
            }
        }
        for (i, n) in seen.iter().enumerate() {
            if *n != 1 {
                let span = self.inst_span(idx);
                let what = if *n == 0 { "never written" } else { "written twice or more" };
                self.push_err(
                    idx,
                    span,
                    format!("struct construction leaves field index {i} {what}"),
                );
            }
        }
    }

    /// Returns the number of fields for a named struct type, or None if not a struct.
    fn struct_field_count(&self, ty: TyId) -> Option<usize> {
        let mir_ty = self._module.ty_arena.get(ty);
        if let crate::ty::MirTy::Named { entity, .. } = mir_ty
            && let Some(s) = self._module.structs.get(entity)
        {
            return Some(s.fields.len());
        }
        None
    }

    fn verify_terminator(&mut self, block: &crate::block::BasicBlock) {
        let term = &block.terminator.kind;

        // Collect all values forwarded as block args by the terminator.
        let mut forwarded: FxHashSet<ValueId> = FxHashSet::default();
        for (target, args) in term.successor_args() {
            let target_block = self.body.block(target);

            // Check 6: arg count must match target block param count.
            if args.len() != target_block.params.len() {
                // No single value to blame — use the terminator's own span.
                let span = block.terminator.span.clone();
                self.push_err(
                    None,
                    span,
                    format!(
                        "terminator passes {} args to {:?} but block expects {} params",
                        args.len(),
                        target,
                        target_block.params.len(),
                    ),
                );
                continue;
            }

            // Check 6: type and ownership must match.
            for (i, (arg_val, param)) in args.iter().zip(target_block.params.iter()).enumerate() {
                let arg_def = self.body.value(*arg_val);
                if arg_def.ty != param.ty {
                    self.err_val(
                        None,
                        *arg_val,
                        format!(
                            "block arg {} to {:?}: type mismatch (value {:?} has {:?}, param expects {:?})",
                            i, target, arg_val, arg_def.ty, param.ty,
                        ),
                    );
                }
                if arg_def.ownership != param.ownership {
                    self.err_val(
                        None,
                        *arg_val,
                        format!(
                            "block arg {} to {:?}: ownership mismatch (value {:?} is {:?}, param expects {:?})",
                            i, target, arg_val, arg_def.ownership, param.ownership,
                        ),
                    );
                }
            }

            for v in args {
                forwarded.insert(*v);
            }
        }

        // Check condition/discriminant liveness BEFORE consuming forwarded values,
        // because the condition/discriminant may itself be forwarded as a block arg
        // (which is the canonical way to consume it).
        match term {
            TerminatorKind::Branch { condition, .. } => {
                self.assert_live(*condition, None);
            },
            TerminatorKind::Switch { discriminant, .. } => {
                self.assert_live(*discriminant, None);
            },
            _ => {},
        }

        // Consume forwarded @owned values. Borrows forwarded by the same
        // terminator don't block: value and borrow continue together in the
        // target block (threaded ref bindings over var slots).
        let forwarded_guaranteed: FxHashSet<ValueId> = forwarded
            .iter()
            .copied()
            .filter(|v| self.body.value(*v).ownership == Ownership::Guaranteed)
            .collect();
        for v in &forwarded {
            if self.body.value(*v).ownership == Ownership::Owned {
                self.try_consume_exempting(*v, None, Some(&forwarded_guaranteed));
            }
        }

        // For Return, the returned value counts as consumed.
        if let TerminatorKind::Return(v) = term {
            self.assert_live(*v, None);
            let ownership = self.body.value(*v).ownership;
            // Return-convention hardening: a ret_borrow function must return
            // the borrow itself; everything else must return @owned (the copy
            // guards copy @guaranteed tails). A violation here is a lowering
            // bug — before this check it was a silent miscompile.
            match (self.ret_borrow, ownership) {
                (true, Ownership::Owned) => {
                    self.err_val(
                        None,
                        *v,
                        format!("ret_borrow function returns @owned value {v:?}"),
                    );
                },
                (false, Ownership::Guaranteed) => {
                    self.err_val(
                        None,
                        *v,
                        format!(
                            "function returns @guaranteed value {v:?} without the ret_borrow convention"
                        ),
                    );
                },
                _ => {},
            }
            if ownership == Ownership::Owned {
                self.try_consume(*v, None);
            }
        }

        // Check 2: every @owned value must be Consumed or forwarded by now.
        //
        // Scoped to values DEFINED in this block. Under the block-local walk
        // that was every tracked value, so this filter is a no-op there; once
        // state flows, a value merely passing through would otherwise be
        // reported as a leak by every block that inherits it while live.
        let unconsumed: Vec<ValueId> = self
            .state
            .owned
            .iter()
            .filter(|(v, state)| {
                **state == ValueState::Live && self.defined_here.contains(v)
            })
            .map(|(&v, _)| v)
            .collect();
        for v in unconsumed {
            let vd = &self.body.values[v.index()];
            let (ty, own) = (vd.ty, vd.ownership);
            self.err_val(
                None,
                v,
                format!("@owned value {:?} is live at block exit but never consumed (ty={:?}, own={:?})", v, ty, own),
            );
        }

        // Check 4: every borrow must be ended or forwarded as @guaranteed block arg.
        let mut forwarded_borrows: FxHashSet<ValueId> = forwarded
            .iter()
            .copied()
            .filter(|v| self.body.value(*v).ownership == Ownership::Guaranteed)
            .collect();
        // ret_borrow carve-out: the one returned borrow deliberately outlives
        // the block — it is the function's result.
        if self.ret_borrow
            && let TerminatorKind::Return(v) = term
            && self.body.value(*v).ownership == Ownership::Guaranteed
        {
            forwarded_borrows.insert(*v);
        }
        let open_borrows: Vec<ValueId> = self
            .borrows
            .keys()
            .filter(|bv| !forwarded_borrows.contains(bv))
            .copied()
            .collect();
        for borrow_val in open_borrows {
            self.err_val(
                None,
                borrow_val,
                format!(
                    "@guaranteed borrow {:?} is still active at block exit without EndBorrow or forwarding",
                    borrow_val,
                ),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Per-block entry point
// ---------------------------------------------------------------------------

/// What one block's transfer function produced.
struct BlockOutcome {
    errors: Vec<VerifyError>,
    flow_findings: Vec<VerifyError>,
    out_state: FlowState,
}

fn verify_block(
    body: &OssaBody,
    module: &MirModule,
    block_id: BlockId,
    func_name: &str,
    entity: Entity,
    entry_state: FlowState,
    report: bool,
    mode: FlowMode,
    aliases: &AddrAliases,
) -> BlockOutcome {
    let mut verifier =
        FlowVerifier::new(body, module, block_id, func_name, entity, entry_state, aliases);
    verifier.report = report;
    verifier.mode = mode;
    verifier.verify()
}

// ---------------------------------------------------------------------------
// Escape check (references stage 1)
// ---------------------------------------------------------------------------

/// The root rule for `-> &T` / `-> &mutating T` functions: every returned
/// borrow's `root_provenance` must be `Param` (a *mutable* one for
/// `&mutating`), `Static`, or `PointerDerived` — `Local` is the escape error.
/// Roots were stamped at value creation and copied through projections, so
/// this is a per-Return field read, never a walk.
///
/// Errors carry `diag: Some(..)` — they are user diagnostics (E494-E496),
/// not ICEs.
pub fn check_escapes(module: &MirModule) -> Vec<VerifyError> {
    use crate::item::function::{RetConvention, ret_convention};
    use crate::value::RootProvenance;

    // Two checked return shapes share the root rule:
    // - RefBorrow returns (stage 1): the returned @guaranteed borrow's root.
    // - Owned returns whose TYPE carries refs (stage 2b ref-bearing
    //   aggregates, e.g. `-> Optional[&T]`): the value's escape taint,
    //   stamped at packaging and joined through merges/copies/calls. An
    //   UNTAINTED value (its own self-root — `.None`, a ref-free arm) is
    //   always returnable.
    enum EscapeMode {
        RefBorrow { mutating: bool },
        Carrier { mutating: bool, closure: bool },
    }

    // Escape taint must survive CFG MERGES. A closure (or ref carrier) built
    // in an `if`/`else` arm reappears at the join under the target block's
    // PARAM ValueId, and `OssaBody::alloc_value` self-roots a param with no
    // borrow source — so the taint stamped at `emit_apply_partial` was lost and
    // a returned frame-view closure escaped with ZERO diagnostics (the
    // `capture_from_nested_scope.ks` hole). Recover it here: fixpoint the join
    // of every incoming argument's effective root into its block param. A
    // SELF-rooted argument is untainted and contributes nothing (joining its
    // `Local(arg)` would poison every merge).
    fn merged_roots(
        body: &crate::body::OssaBody,
        convs: &[ParamConvention],
    ) -> FxHashMap<ValueId, RootProvenance> {
        let mut roots: FxHashMap<ValueId, RootProvenance> = FxHashMap::default();
        let effective = |roots: &FxHashMap<ValueId, RootProvenance>, v: ValueId| {
            let r = roots.get(&v).copied().unwrap_or(body.value(v).root);
            (r != RootProvenance::Local(v) && !r.is_derived_placeholder()).then_some(r)
        };
        loop {
            let mut changed = false;
            for block in &body.blocks {
                for (target, args) in block.terminator.kind.successor_args() {
                    for (i, &arg) in args.iter().enumerate() {
                        let Some(src) = effective(&roots, arg) else {
                            continue;
                        };
                        let Some(param) = body.block(target).params.get(i) else {
                            continue;
                        };
                        let cur = roots.get(&param.value).copied();
                        let next = match cur {
                            Some(c) => c.join(src, convs),
                            None => src,
                        };
                        if cur != Some(next) {
                            roots.insert(param.value, next);
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                return roots;
            }
        }
    }

    let mut errors = Vec::new();
    for func in module.functions.values() {
        let Some(body) = &func.body else { continue };
        if body.values.is_empty() || body.blocks.is_empty() {
            continue;
        }
        let convs: Vec<ParamConvention> = func.params.iter().map(|p| p.convention).collect();
        let merged = merged_roots(body, &convs);
        let mode = match ret_convention(&module.ty_arena, func.ret) {
            RetConvention::RefBorrow { mutating } => EscapeMode::RefBorrow { mutating },
            _ => {
                // Deep escape-carry: a ref OR a capturing closure carried by
                // value — including through a nominal STORED FIELD (a `&Int64`
                // or closure field), which the shallow type-arg-only
                // `contains_ref`/`contains_closure` miss (#174 struct-field
                // laundering). The returned value's root is the join over its
                // ref/closure components (stamped at construction /
                // `emit_apply_partial`), so the same self-root skip + Local-root
                // rule applies. Ref takes precedence (carries the mutating bit
                // for E495); a pure-closure carrier uses the closure wording.
                let carry = module.escape_carry(func.ret);
                if carry.any_ref {
                    EscapeMode::Carrier {
                        mutating: carry.mutating_ref,
                        closure: false,
                    }
                } else if carry.view_closure {
                    EscapeMode::Carrier {
                        mutating: false,
                        closure: true,
                    }
                } else {
                    continue;
                }
            },
        };

        for (block_idx, block) in body.blocks.iter().enumerate() {
            let TerminatorKind::Return(v) = &block.terminator.kind else {
                continue;
            };
            let vd = body.value(*v);
            let vd_root = merged.get(v).copied().unwrap_or(vd.root);
            let (carrier, mutating, is_closure) = match mode {
                EscapeMode::RefBorrow { mutating } => {
                    if vd.ownership != Ownership::Guaranteed {
                        // verify_terminator's hardening reports this as an ICE.
                        continue;
                    }
                    (false, mutating, false)
                },
                EscapeMode::Carrier { mutating, closure } => {
                    if vd.ownership != Ownership::Owned
                        // Hand-built bodies may bypass alloc_value.
                        || vd_root.is_derived_placeholder()
                        // Untainted: the value's own self-root.
                        || vd_root == RootProvenance::Local(*v)
                    {
                        continue;
                    }
                    (true, mutating, closure)
                },
            };
            let mut push = |code: &'static str,
                            message: String,
                            secondary: Option<(Span, String)>,
                            notes: Vec<String>| {
                errors.push(VerifyError {
                    block: BlockId::new(block_idx),
                    inst: None,
                    message,
                    span: block.terminator.span.clone().or_else(|| vd.span.clone()),
                    func_name: func.name.clone(),
                    entity: func.entity,
                    diag: Some(UserDiag {
                        code,
                        secondary,
                        notes,
                    }),
                });
            };

            let mut root = vd_root;
            if root.is_derived_placeholder() {
                // Hand-built bodies may bypass alloc_value; treat as a local
                // with no known definition. (Carrier mode skipped these.)
                root = RootProvenance::Local(*v);
            }
            // Carrier-mode wordings name the VALUE (the ref/env rides inside it).
            // A directly-returned closure says "this closure"; a struct/tuple
            // that merely carries one says "this value: it carries a closure".
            let what = if is_closure {
                if matches!(
                    module.ty_arena.get(func.ret),
                    crate::ty::MirTy::FuncThick { .. }
                ) {
                    "this closure: it captures"
                } else {
                    "this value: it carries a closure that captures"
                }
            } else if carrier {
                "this value: it carries a reference that borrows"
            } else {
                "this reference: it borrows"
            };
            match root {
                RootProvenance::Local(local) => {
                    let name = body
                        .value_names
                        .get(&local)
                        .map(|n| format!(" `{n}`"))
                        .unwrap_or_default();
                    let secondary = body
                        .values
                        .get(local.index())
                        .and_then(|d| d.span.clone())
                        .map(|s| (s, format!("the borrowed local{name} is defined here")));
                    // FIX-IT (docs/design/closures.md, Diagnostics table:
                    // E494's "message gains a fix-it suggesting an `escaping`
                    // or `consuming` owning type"). Fix-its are `notes`
                    // strings in this data model (plan, decision 7). The kind
                    // is spelled in the EXPECTED type, never on the literal —
                    // that is the whole inference rule, so the note names the
                    // return/expected type as the place to change.
                    let mut notes = vec![
                        "only parameter-rooted or `Pointer`-derived references can be returned"
                            .into(),
                        "Pointer-derived references are not verified by the compiler".into(),
                    ];
                    if is_closure {
                        notes = vec![
                            "a normal or `mutating` closure holds VIEWS into this frame, so it \
                             cannot leave it"
                                .into(),
                            "write an owning kind in the expected/return type — `escaping (…) \
                             -> …` (shared environment, callable many times) or `consuming (…) \
                             -> …` (unique environment, called once) — and the literal is \
                             rebuilt with owned captures"
                                .into(),
                        ];
                    }
                    push(
                        "E494",
                        format!(
                            "cannot return {what} local{name}, which does not outlive the call"
                        ),
                        secondary,
                        notes,
                    );
                },
                RootProvenance::Param(idx) => {
                    let convention = func.params.get(idx as usize).map(|p| p.convention);
                    if convention == Some(ParamConvention::Consuming) {
                        let pname = func
                            .params
                            .get(idx as usize)
                            .map(|p| format!(" `{}`", p.name))
                            .unwrap_or_default();
                        let subject = if carrier {
                            "a value carrying a reference"
                        } else {
                            "a reference"
                        };
                        push(
                            "E496",
                            format!(
                                "cannot return {subject} rooted at consuming parameter{pname}: \
                                 it is destroyed when the call returns"
                            ),
                            None,
                            vec![],
                        );
                    } else if mutating && convention != Some(ParamConvention::MutBorrow) {
                        let subject = if carrier {
                            "returning a value carrying `&mutating`"
                        } else {
                            "returning `&mutating`"
                        };
                        push(
                            "E495",
                            format!(
                                "{subject} requires a mutable root: a `mutating` receiver or \
                                 parameter, or `Pointer.mutatingValue`"
                            ),
                            None,
                            vec![],
                        );
                    }
                },
                RootProvenance::Static => {
                    if mutating {
                        let subject = if carrier {
                            "returning a value carrying `&mutating`"
                        } else {
                            "returning `&mutating`"
                        };
                        push(
                            "E495",
                            format!(
                                "{subject} requires a mutable root; a static is not a \
                                 mutable root"
                            ),
                            None,
                            vec![],
                        );
                    }
                },
                RootProvenance::PointerDerived { mutable } => {
                    if mutating && !mutable {
                        let subject = if carrier {
                            "returning a value carrying `&mutating`"
                        } else {
                            "returning `&mutating`"
                        };
                        push(
                            "E495",
                            format!(
                                "{subject} requires a mutable root: use \
                                 `Pointer.mutatingValue`, not `.value`"
                            ),
                            None,
                            vec![],
                        );
                    }
                },
            }
        }
    }
    errors
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::OssaBuilder;
    use crate::callee::Callee;
    use crate::immediate::Immediate;
    use crate::inst::CallArg;
    use crate::item::struct_def::{FieldDef, StructDef};
    use crate::item::{CopyBehavior, TypeInfo};

    /// Helper: create an OssaBuilder with a Named struct type whose CopyBehavior
    /// is None (so it gets Ownership::Owned).
    fn make_owned_type(b: &mut OssaBuilder) -> (TyId, Entity) {
        let entity = b.fresh_entity();
        b.register_name(entity, "OwnedStruct");
        let ty = b.named(entity, vec![]);
        let mut def = StructDef::new(entity, "OwnedStruct");
        def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            ..TypeInfo::default()
        };
        b.add_struct(def);
        (ty, entity)
    }

    /// Helper: create a named struct with N fields (all i64), CopyBehavior::None.
    fn make_owned_struct_with_fields(b: &mut OssaBuilder, n: usize) -> (TyId, Entity) {
        let entity = b.fresh_entity();
        b.register_name(entity, "MultiFieldStruct");
        let ty = b.named(entity, vec![]);
        let i64_ty = b.i64();
        let mut def = StructDef::new(entity, "MultiFieldStruct");
        for i in 0..n {
            def.add_field(FieldDef::new(format!("field_{}", i), i64_ty));
        }
        def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            ..TypeInfo::default()
        };
        b.add_struct(def);
        (ty, entity)
    }

    fn run_verify(b: OssaBuilder) -> Vec<VerifyError> {
        let (body, module) = b.finish();
        verify_ossa(&body, &module, "test", Entity::from_raw(0))
    }

    /// Build: entry defines an @owned value, destroys it, then jumps to a
    /// second block that destroys it AGAIN without it being threaded as a
    /// block argument. That is a double-consume across a block boundary — a
    /// double free if it reached codegen.
    fn cross_block_double_consume() -> (crate::body::OssaBody, MirModule) {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let second = b.new_block();
        let entry = b.current_block();

        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }
        b.emit_destroy_value(x);
        b.emit_jump(second, vec![]);

        b.switch_to(second);
        b.emit_destroy_value(x);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        b.finish()
    }

    /// Build a diamond where a value is defined on ONE arm and used on the
    /// other. `def` cannot dominate `use` — control can reach the use without
    /// ever having executed the definition.
    fn non_dominating_def() -> (crate::body::OssaBody, MirModule) {
        let mut b = OssaBuilder::new("test");

        let then_b = b.new_block();
        let else_b = b.new_block();

        let cond = b.emit_literal(Immediate::bool(true));
        b.emit_branch(cond, then_b, vec![], else_b, vec![]);

        // Defined only on the `then` arm.
        b.switch_to(then_b);
        let only_on_then = b.emit_literal(Immediate::i64(1));
        b.emit_return(only_on_then);

        // Used on the `else` arm, which `then` does not dominate.
        b.switch_to(else_b);
        b.emit_return(only_on_then);

        b.finish()
    }

    #[test]
    fn dominance_rejects_definition_that_does_not_dominate_its_use() {
        // Guards against the check going inert. It measures zero across the
        // whole corpus, which is the desired answer only if it can still say
        // "no" — this body is the "no".
        let (body, module) = non_dominating_def();
        let order = reverse_postorder(&body);
        let mut errors = Vec::new();
        let def_blocks = check_value_uniqueness(&body, "test", Entity::from_raw(0), &mut errors);
        errors.clear();
        check_dominance(
            &body,
            &order,
            &def_blocks,
            "test",
            Entity::from_raw(0),
            &mut errors,
        );
        assert!(
            errors.iter().any(|e| e.message.contains("does not dominate")),
            "a def on one arm of a diamond must not be usable on the other: {:?}",
            errors,
        );
    }

    #[test]
    fn dominance_accepts_the_ordinary_ssa_shape() {
        // The counterpart: the audit's prescribed Check 1b ("defined by this
        // block's params or an earlier instruction in it") REJECTS this, which
        // is why it rejected the stdlib. Dominance accepts it.
        let mut b = OssaBuilder::new("test");
        let second = b.new_block();

        let defined_in_entry = b.emit_literal(Immediate::i64(7));
        b.emit_jump(second, vec![]);

        b.switch_to(second);
        b.emit_return(defined_in_entry);

        let (body, _module) = b.finish();
        let order = reverse_postorder(&body);
        let mut errors = Vec::new();
        let def_blocks = check_value_uniqueness(&body, "test", Entity::from_raw(0), &mut errors);
        errors.clear();
        check_dominance(
            &body,
            &order,
            &def_blocks,
            "test",
            Entity::from_raw(0),
            &mut errors,
        );
        assert!(
            errors.is_empty(),
            "entry dominates its successor, so the use is legal: {:?}",
            errors,
        );
    }

    #[test]
    fn dominance_rejects_use_before_def_within_a_block() {
        // Dominance alone is not enough: a block dominates itself, so an
        // operand defined LATER in the same block would slip through if the
        // walk only asked about blocks. Order within the block matters too.
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);
        let v = b.emit_uninit(owned_ty);
        b.emit_destroy_value(v);
        {
            // Swap so the destroy now precedes the definition it consumes.
            let body = b.body_mut();
            let entry = body.entry;
            body.block_mut(entry).insts.swap(0, 1);
        }
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let (body, _module) = b.finish();
        let order = reverse_postorder(&body);
        let mut errors = Vec::new();
        let def_blocks = check_value_uniqueness(&body, "test", Entity::from_raw(0), &mut errors);
        errors.clear();
        check_dominance(
            &body,
            &order,
            &def_blocks,
            "test",
            Entity::from_raw(0),
            &mut errors,
        );
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("used before it is defined")),
            "a use preceding its definition in the same block must be caught: {:?}",
            errors,
        );
    }

    #[test]
    fn cross_block_double_consume_invisible_when_flow_off() {
        // Documents F34: with the block-local walk, `second` starts from an
        // empty state, so the second destroy is unattributable and silent.
        // (`FlowMode` is read from the environment once per process, so this
        // asserts the Off-mode transfer rules directly rather than by env.)
        let (body, module) = cross_block_double_consume();
        let aliases = AddrAliases::build(&body);
        let mut v = FlowVerifier::new(
            &body,
            &module,
            BlockId::new(body.blocks.len() - 1),
            "test",
            Entity::from_raw(0),
            FlowState::default(),
            &aliases,
        );
        v.mode = FlowMode::Off;
        let outcome = v.verify();
        assert!(
            !outcome
                .errors
                .iter()
                .any(|e| e.message.contains("consumed more than once")),
            "block-local walk should not see the cross-block consume: {:?}",
            outcome.errors,
        );
    }

    #[test]
    fn cross_block_double_consume_caught_when_state_flows() {
        // The F34 payoff: hand the block the state its predecessor actually
        // left behind, and the second consume is a hard error.
        let (body, module) = cross_block_double_consume();

        let aliases = AddrAliases::build(&body);
        let mut incoming = FlowState::default();
        incoming
            .owned
            .insert(ValueId::new(0), ValueState::Consumed);

        let mut v = FlowVerifier::new(
            &body,
            &module,
            BlockId::new(body.blocks.len() - 1),
            "test",
            Entity::from_raw(0),
            incoming,
            &aliases,
        );
        v.mode = FlowMode::Enforce;
        let outcome = v.verify();
        assert!(
            outcome
                .errors
                .iter()
                .any(|e| e.message.contains("consumed more than once")),
            "flowed state should catch the cross-block double consume: {:?}",
            outcome.errors,
        );
    }

    #[test]
    fn maybe_consumed_is_permitted() {
        // A conditional move: consumed on one incoming path, live on the
        // other. The drop flag decides at runtime, so consuming here must NOT
        // be reported — otherwise every guarded destroy in the stdlib ICEs.
        let (body, module) = cross_block_double_consume();

        let aliases = AddrAliases::build(&body);
        let mut incoming = FlowState::default();
        incoming
            .owned
            .insert(ValueId::new(0), ValueState::MaybeConsumed);

        let mut v = FlowVerifier::new(
            &body,
            &module,
            BlockId::new(body.blocks.len() - 1),
            "test",
            Entity::from_raw(0),
            incoming,
            &aliases,
        );
        v.mode = FlowMode::Enforce;
        let outcome = v.verify();
        assert!(
            !outcome
                .errors
                .iter()
                .any(|e| e.message.contains("consumed more than once")),
            "MaybeConsumed must be permissive: {:?}",
            outcome.errors,
        );
    }

    #[test]
    fn driver_off_mode_misses_cross_block_double_consume() {
        // End-to-end through verify_ossa: the block-local walk is blind.
        let (body, module) = cross_block_double_consume();
        let errors = verify_ossa_with_mode(
            &body,
            &module,
            "test",
            Entity::from_raw(0),
            FlowMode::Off,
        );
        assert!(
            !errors
                .iter()
                .any(|e| e.message.contains("consumed more than once")),
            "Off mode should miss it (that IS F34): {:?}",
            errors,
        );
    }

    #[test]
    fn driver_enforce_mode_catches_cross_block_double_consume() {
        // Same body, same entry point, state now flows along the CFG edge.
        let (body, module) = cross_block_double_consume();
        let errors = verify_ossa_with_mode(
            &body,
            &module,
            "test",
            Entity::from_raw(0),
            FlowMode::Enforce,
        );
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("consumed more than once")),
            "the fixpoint should propagate Consumed across the jump: {:?}",
            errors,
        );
    }

    #[test]
    fn flow_mode_parses_from_env_value() {
        // Closes the last link: env string -> mode. (mode -> driver -> caught
        // is covered by the driver_* tests.) An unset or unrecognised value
        // must fall back to Off so an unrelated build never changes behaviour.
        assert_eq!(FlowMode::parse(Some("warn")), FlowMode::Warn);
        assert_eq!(FlowMode::parse(Some("enforce")), FlowMode::Enforce);
        assert_eq!(FlowMode::parse(None), FlowMode::Off);
        assert_eq!(FlowMode::parse(Some("")), FlowMode::Off);
        assert_eq!(FlowMode::parse(Some("1")), FlowMode::Off);
        assert!(!FlowMode::Off.flows());
        assert!(FlowMode::Warn.flows());
        assert!(FlowMode::Enforce.flows());
    }

    #[test]
    fn join_disagreeing_states_yields_maybe() {
        let mut a = FlowState::default();
        a.owned.insert(ValueId::new(0), ValueState::Live);
        let mut b = FlowState::default();
        b.owned.insert(ValueId::new(0), ValueState::Consumed);

        assert!(a.join(&b), "join must report that it changed");
        assert_eq!(a.owned[&ValueId::new(0)], ValueState::MaybeConsumed);

        // Idempotent: joining again changes nothing (fixpoint termination).
        assert!(!a.join(&b));
    }

    // -----------------------------------------------------------------------
    // Category 1: Valid bodies pass verification
    // -----------------------------------------------------------------------

    #[test]
    fn valid_trivial_return_unit() {
        let mut b = OssaBuilder::new("test");
        let _unit_ty = b.unit();
        let unit_val = b.emit_literal(Immediate::unit());
        b.emit_return(unit_val);

        let errors = run_verify(b);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn valid_owned_copy_and_destroy() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        b.body().blocks[entry.index()].params.len();
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        let y = b.emit_copy_value(x);
        b.emit_destroy_value(x);
        b.emit_return(y);

        let errors = run_verify(b);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn valid_borrow_around_call() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        let borrow_val = b.emit_begin_borrow(x);
        let callee_entity = b.fresh_entity();
        b.emit_call(
            Callee::direct(callee_entity),
            vec![CallArg {
                value: borrow_val,
                convention: ParamConvention::Borrow,
            }],
            None,
        );
        b.emit_end_borrow(borrow_val);
        b.emit_destroy_value(x);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn valid_branch_with_forwarded_owned() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);
        let bool_ty = b.bool();

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        let cond = b.new_value(bool_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            let blk = body.block_mut(entry);
            blk.params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
            blk.params.push(crate::block::BlockParam {
                value: cond,
                ty: bool_ty,
                ownership: Ownership::Owned,
            });
        }

        let (bb1, bb1_params) =
            b.new_block_with_params(&[(owned_ty, Ownership::Owned), (bool_ty, Ownership::Owned)]);
        let (bb2, bb2_params) =
            b.new_block_with_params(&[(owned_ty, Ownership::Owned), (bool_ty, Ownership::Owned)]);

        b.emit_branch(cond, bb1, vec![x, cond], bb2, vec![x, cond]);

        b.switch_to(bb1);
        b.emit_destroy_value(bb1_params[0]);
        b.emit_destroy_value(bb1_params[1]);
        let unit1 = b.emit_literal(Immediate::unit());
        b.emit_return(unit1);

        b.switch_to(bb2);
        b.emit_destroy_value(bb2_params[0]);
        b.emit_destroy_value(bb2_params[1]);
        let unit2 = b.emit_literal(Immediate::unit());
        b.emit_return(unit2);

        let errors = run_verify(b);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    // -----------------------------------------------------------------------
    // Category 2: Unconsumed @owned -> error
    // -----------------------------------------------------------------------

    #[test]
    fn error_unconsumed_owned_param() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(!errors.is_empty(), "expected unconsumed error");
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("live at block exit")),
            "expected 'live at block exit' message, got: {:?}",
            errors,
        );
    }

    #[test]
    fn error_unconsumed_owned_instruction_result() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        let _y = b.emit_copy_value(x);
        b.emit_destroy_value(x);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(!errors.is_empty(), "expected unconsumed error for y");
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("live at block exit")),
            "expected live at block exit, got: {:?}",
            errors,
        );
    }

    #[test]
    fn error_unconsumed_owned_call_result() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let callee_entity = b.fresh_entity();
        let _r = b.emit_call(
            Callee::direct(callee_entity),
            vec![],
            Some((owned_ty, Ownership::Owned)),
        );

        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("live at block exit")),
            "expected unconsumed call result, got: {:?}",
            errors,
        );
    }

    // -----------------------------------------------------------------------
    // Category 3: Double consume -> error
    // -----------------------------------------------------------------------

    #[test]
    fn error_double_destroy() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        b.emit_destroy_value(x);
        b.emit_destroy_value(x);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("consumed more than once")),
            "expected double consume error, got: {:?}",
            errors,
        );
    }

    #[test]
    fn error_move_then_destroy() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        let y = b.emit_move_value(x);
        b.emit_destroy_value(x);
        b.emit_destroy_value(y);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("consumed more than once")),
            "expected double consume error, got: {:?}",
            errors,
        );
    }

    #[test]
    fn error_double_consume_via_struct() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        let wrapper_entity = b.fresh_entity();
        let wrapper_ty = b.named(wrapper_entity, vec![]);
        let mut wrapper_def = StructDef::new(wrapper_entity, "Wrapper");
        wrapper_def.add_field(FieldDef::new("inner", owned_ty));
        wrapper_def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            ..TypeInfo::default()
        };
        b.add_struct(wrapper_def);

        let s = b.emit_struct(wrapper_ty, vec![(FieldIdx::new(0), x)]);
        b.emit_destroy_value(x);
        b.emit_destroy_value(s);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("consumed more than once")),
            "expected double consume, got: {:?}",
            errors,
        );
    }

    // -----------------------------------------------------------------------
    // Category 4: Use after consume -> error
    // -----------------------------------------------------------------------

    #[test]
    fn error_use_after_destroy() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        b.emit_destroy_value(x);
        let y = b.emit_copy_value(x);
        b.emit_destroy_value(y);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("use of consumed value")
                    || e.message.contains("consumed more than once")),
            "expected use-after-consume error, got: {:?}",
            errors,
        );
    }

    #[test]
    fn error_use_after_move() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        let _y = b.emit_move_value(x);
        let borrow = b.emit_begin_borrow(x);
        b.emit_end_borrow(borrow);
        b.emit_destroy_value(_y);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("use of consumed value")
                    || e.message.contains("consumed")),
            "expected use-after-move error, got: {:?}",
            errors,
        );
    }

    #[test]
    fn error_use_after_consume_in_call() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        let callee = b.fresh_entity();
        b.emit_call(
            Callee::direct(callee),
            vec![CallArg {
                value: x,
                convention: ParamConvention::Consuming,
            }],
            None,
        );
        let y = b.emit_copy_value(x);
        b.emit_destroy_value(y);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors.iter().any(|e| e.message.contains("consumed")),
            "expected use-after-consume, got: {:?}",
            errors,
        );
    }

    // -----------------------------------------------------------------------
    // Category 5: Missing EndBorrow -> error
    // -----------------------------------------------------------------------

    #[test]
    fn error_missing_end_borrow() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        let _borrow = b.emit_begin_borrow(x);
        b.emit_destroy_value(x);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("still active at block exit")),
            "expected open borrow error, got: {:?}",
            errors,
        );
    }

    #[test]
    fn error_missing_end_mut_borrow() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        let _mb = b.emit_begin_mut_borrow(x);
        b.emit_destroy_value(x);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("still active at block exit")),
            "expected open mut borrow error, got: {:?}",
            errors,
        );
    }

    #[test]
    fn error_end_borrow_wrong_value() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        let x2 = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            let blk = body.block_mut(entry);
            blk.params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
            blk.params.push(crate::block::BlockParam {
                value: x2,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        let _borrow_x = b.emit_begin_borrow(x);
        let borrow_x2 = b.emit_begin_borrow(x2);
        b.emit_end_borrow(borrow_x2);
        b.emit_destroy_value(x);
        b.emit_destroy_value(x2);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("still active at block exit")),
            "expected borrow not ended, got: {:?}",
            errors,
        );
    }

    // -----------------------------------------------------------------------
    // Category 6: Consume source during borrow -> error
    // -----------------------------------------------------------------------

    #[test]
    fn error_consume_source_during_borrow() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        let borrow = b.emit_begin_borrow(x);
        b.emit_destroy_value(x);
        b.emit_end_borrow(borrow);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors.iter().any(|e| e.message.contains("active borrow")),
            "expected consume-during-borrow error, got: {:?}",
            errors,
        );
    }

    #[test]
    fn error_consume_source_during_mut_borrow() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        let mb = b.emit_begin_mut_borrow(x);
        b.emit_destroy_value(x);
        b.emit_end_mut_borrow(mb);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors.iter().any(|e| e.message.contains("active borrow")),
            "expected consume-during-mut-borrow error, got: {:?}",
            errors,
        );
    }

    /// Push a ref-returning call: a Call inst whose @guaranteed result has
    /// `borrow_source = source` (mirrors mir-lower's ret_borrow registration).
    fn emit_ref_call(b: &mut OssaBuilder, ty: TyId, source: ValueId) -> ValueId {
        let callee = b.fresh_entity();
        let result = b.new_guaranteed_value(ty, source);
        let cur = b.current_block();
        b.body_mut()
            .block_mut(cur)
            .insts
            .push(crate::inst::Instruction::new(InstKind::Call {
                result: Some(result),
                callee: crate::callee::Callee::direct(callee),
                args: vec![crate::inst::CallArg {
                    value: source,
                    convention: ParamConvention::Borrow,
                }],
            }));
        result
    }

    #[test]
    fn consume_while_ref_live_is_coded_e498() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        let borrow = b.emit_begin_borrow(x);
        let ref_val = emit_ref_call(&mut b, owned_ty, borrow);
        // Consume the owner while the ref (chained x <- borrow <- ref) lives.
        let moved = b.emit_move_value(x);
        b.emit_end_borrow(ref_val);
        b.emit_end_borrow(borrow);
        b.emit_destroy_value(moved);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        let codes: Vec<_> = errors
            .iter()
            .filter_map(|e| e.diag.as_ref().map(|d| d.code))
            .collect();
        assert_eq!(codes, vec!["E498"], "got: {errors:?}");
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("while a reference into it is live")),
            "got: {errors:?}",
        );
    }

    #[test]
    fn consume_during_unrelated_ref_stays_ice() {
        // A live ref rooted at a DIFFERENT value must not user-code the
        // conflict: consuming x while only a plain borrow of x blocks it is
        // a lowering bug (uncoded ICE), even though some ref exists.
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        let y = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            for v in [x, y] {
                body.block_mut(entry).params.push(crate::block::BlockParam {
                    value: v,
                    ty: owned_ty,
                    ownership: Ownership::Owned,
                });
            }
        }

        let borrow_x = b.emit_begin_borrow(x);
        let borrow_y = b.emit_begin_borrow(y);
        let ref_y = emit_ref_call(&mut b, owned_ty, borrow_y);
        let moved = b.emit_move_value(x);
        b.emit_end_borrow(ref_y);
        b.emit_end_borrow(borrow_y);
        b.emit_end_borrow(borrow_x);
        b.emit_destroy_value(moved);
        b.emit_destroy_value(y);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors
                .iter()
                .any(|e| e.diag.is_none() && e.message.contains("active borrow")),
            "expected an uncoded consume-during-borrow ICE, got: {errors:?}",
        );
        assert!(
            errors.iter().all(|e| e.diag.is_none()),
            "no error should be coded here, got: {errors:?}",
        );
    }

    #[test]
    fn error_read_source_during_mut_borrow() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        let mb = b.emit_begin_mut_borrow(x);
        let _copy = b.emit_copy_value(x);
        b.emit_end_mut_borrow(mb);
        b.emit_destroy_value(_copy);
        b.emit_destroy_value(x);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("active mut borrow")),
            "expected read-during-mut-borrow error, got: {:?}",
            errors,
        );
    }

    // -----------------------------------------------------------------------
    // Category 7: Block arg count mismatch -> error
    // -----------------------------------------------------------------------

    #[test]
    fn error_block_arg_count_too_few() {
        let mut b = OssaBuilder::new("test");
        let i64_ty = b.i64();

        let (target, _params) =
            b.new_block_with_params(&[(i64_ty, Ownership::Owned), (i64_ty, Ownership::Owned)]);

        let lit = b.emit_literal(Immediate::i64(42));
        b.emit_jump(target, vec![lit]);

        b.switch_to(target);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("passes 1 args")
                    && e.message.contains("expects 2 params")),
            "expected arg count mismatch, got: {:?}",
            errors,
        );
    }

    #[test]
    fn error_block_arg_count_too_many() {
        let mut b = OssaBuilder::new("test");
        let i64_ty = b.i64();

        let (target, _params) = b.new_block_with_params(&[(i64_ty, Ownership::Owned)]);

        let lit1 = b.emit_literal(Immediate::i64(1));
        let lit2 = b.emit_literal(Immediate::i64(2));
        b.emit_jump(target, vec![lit1, lit2]);

        b.switch_to(target);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("passes 2 args")
                    && e.message.contains("expects 1 params")),
            "expected arg count mismatch, got: {:?}",
            errors,
        );
    }

    #[test]
    fn error_block_arg_ownership_mismatch() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let target = b.new_block();
        let guaranteed_param = b.new_guaranteed_value(owned_ty, ValueId::new(0));
        {
            let body = b.body_mut();
            body.block_mut(target)
                .params
                .push(crate::block::BlockParam {
                    value: guaranteed_param,
                    ty: owned_ty,
                    ownership: Ownership::Guaranteed,
                });
        }

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        b.emit_jump(target, vec![x]);

        b.switch_to(target);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("ownership mismatch")),
            "expected ownership mismatch, got: {:?}",
            errors,
        );
    }

    // -----------------------------------------------------------------------
    // Category 10: Valid Uninit + FieldAddr + StoreInit + Take passes
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Struct construction completeness (F3 backstop)
    //
    // Struct construction is where the independently-computed field rosters
    // meet. These assert the verifier rejects the shapes a roster divergence
    // produces, so the next one is a verifier error rather than a wrong-slot
    // write into uninitialized memory.
    // -----------------------------------------------------------------------

    #[test]
    fn struct_construct_complete_is_accepted() {
        let mut b = OssaBuilder::new("test");
        let (struct_ty, _entity) = make_owned_struct_with_fields(&mut b, 3);
        let v0 = b.emit_literal(Immediate::i64(1));
        let v1 = b.emit_literal(Immediate::i64(2));
        let v2 = b.emit_literal(Immediate::i64(3));

        let s = b.emit_struct(
            struct_ty,
            vec![
                (FieldIdx::new(0), v0),
                (FieldIdx::new(1), v1),
                (FieldIdx::new(2), v2),
            ],
        );
        b.emit_destroy_value(s);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn struct_construct_missing_field_is_rejected() {
        // The exact shape of the F3 miscompile: the caller's roster had fewer
        // fields than the layout, so a slot was never written and was later
        // read as garbage.
        let mut b = OssaBuilder::new("test");
        let (struct_ty, _entity) = make_owned_struct_with_fields(&mut b, 3);
        let v0 = b.emit_literal(Immediate::i64(1));
        let v1 = b.emit_literal(Immediate::i64(2));

        let s = b.emit_struct(struct_ty, vec![(FieldIdx::new(0), v0), (FieldIdx::new(1), v1)]);
        b.emit_destroy_value(s);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("field index 2") && e.message.contains("never written")),
            "expected a missing-field error, got: {:?}",
            errors
        );
    }

    #[test]
    fn struct_construct_duplicate_field_is_rejected() {
        let mut b = OssaBuilder::new("test");
        let (struct_ty, _entity) = make_owned_struct_with_fields(&mut b, 2);
        let v0 = b.emit_literal(Immediate::i64(1));
        let v1 = b.emit_literal(Immediate::i64(2));

        // Both arguments land in slot 0 — field 1 is left uninitialized.
        let s = b.emit_struct(struct_ty, vec![(FieldIdx::new(0), v0), (FieldIdx::new(0), v1)]);
        b.emit_destroy_value(s);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors.iter().any(|e| e.message.contains("written twice")),
            "expected a duplicate-field error, got: {:?}",
            errors
        );
    }

    #[test]
    fn struct_construct_out_of_range_field_is_rejected() {
        let mut b = OssaBuilder::new("test");
        let (struct_ty, _entity) = make_owned_struct_with_fields(&mut b, 2);
        let v0 = b.emit_literal(Immediate::i64(1));
        let v1 = b.emit_literal(Immediate::i64(2));
        let v2 = b.emit_literal(Immediate::i64(3));

        let s = b.emit_struct(
            struct_ty,
            vec![
                (FieldIdx::new(0), v0),
                (FieldIdx::new(1), v1),
                (FieldIdx::new(2), v2),
            ],
        );
        b.emit_destroy_value(s);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("only 2 field(s)")),
            "expected an out-of-range error, got: {:?}",
            errors
        );
    }

    #[test]
    fn valid_uninit_field_store_take() {
        let mut b = OssaBuilder::new("test");
        let (struct_ty, _entity) = make_owned_struct_with_fields(&mut b, 2);
        let i64_ty = b.i64();

        let addr = b.emit_uninit(struct_ty);
        let f0_addr = b.emit_field_addr(addr, i64_ty, FieldIdx::new(0));
        let f1_addr = b.emit_field_addr(addr, i64_ty, FieldIdx::new(1));

        let v0 = b.emit_literal(Immediate::i64(10));
        let v1 = b.emit_literal(Immediate::i64(20));

        b.emit_store_init(f0_addr, v0);
        b.emit_store_init(f1_addr, v1);

        let result = b.emit_take(addr, struct_ty);
        b.emit_destroy_value(f0_addr);
        b.emit_destroy_value(f1_addr);
        b.emit_destroy_value(addr);
        b.emit_destroy_value(result);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn valid_uninit_store_take_single_field() {
        let mut b = OssaBuilder::new("test");
        let (struct_ty, _entity) = make_owned_struct_with_fields(&mut b, 1);
        let i64_ty = b.i64();

        let addr = b.emit_uninit(struct_ty);
        let f0_addr = b.emit_field_addr(addr, i64_ty, FieldIdx::new(0));
        let v0 = b.emit_literal(Immediate::i64(42));
        b.emit_store_init(f0_addr, v0);

        let result = b.emit_take(addr, struct_ty);
        b.emit_destroy_value(f0_addr);
        b.emit_destroy_value(addr);
        b.emit_destroy_value(result);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn valid_uninit_destroy_addr_all_fields() {
        let mut b = OssaBuilder::new("test");
        let (struct_ty, _entity) = make_owned_struct_with_fields(&mut b, 2);
        let i64_ty = b.i64();

        let addr = b.emit_uninit(struct_ty);
        let f0_addr = b.emit_field_addr(addr, i64_ty, FieldIdx::new(0));
        let f1_addr = b.emit_field_addr(addr, i64_ty, FieldIdx::new(1));

        let v0 = b.emit_literal(Immediate::i64(1));
        let v1 = b.emit_literal(Immediate::i64(2));
        b.emit_store_init(f0_addr, v0);
        b.emit_store_init(f1_addr, v1);

        b.emit_destroy_addr(f0_addr, i64_ty);
        b.emit_destroy_addr(f1_addr, i64_ty);
        b.emit_destroy_value(f0_addr);
        b.emit_destroy_value(f1_addr);
        b.emit_destroy_value(addr);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    // -----------------------------------------------------------------------
    // Category 11: Partial init (missing field) + Take -> error
    // -----------------------------------------------------------------------

    #[test]
    fn error_partial_init_take() {
        let mut b = OssaBuilder::new("test");
        let (struct_ty, _entity) = make_owned_struct_with_fields(&mut b, 2);
        let i64_ty = b.i64();

        let addr = b.emit_uninit(struct_ty);
        let f0_addr = b.emit_field_addr(addr, i64_ty, FieldIdx::new(0));

        let v0 = b.emit_literal(Immediate::i64(10));
        b.emit_store_init(f0_addr, v0);

        let result = b.emit_take(addr, struct_ty);
        b.emit_destroy_value(result);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors.iter().any(|e| e.message.contains("uninit")),
            "expected partial-init error, got: {:?}",
            errors,
        );
    }

    #[test]
    fn error_partial_init_take_three_fields() {
        let mut b = OssaBuilder::new("test");
        let (struct_ty, _entity) = make_owned_struct_with_fields(&mut b, 3);
        let i64_ty = b.i64();

        let addr = b.emit_uninit(struct_ty);
        let f0 = b.emit_field_addr(addr, i64_ty, FieldIdx::new(0));
        let f2 = b.emit_field_addr(addr, i64_ty, FieldIdx::new(2));

        let v0 = b.emit_literal(Immediate::i64(1));
        b.emit_store_init(f0, v0);
        let v2 = b.emit_literal(Immediate::i64(3));
        b.emit_store_init(f2, v2);

        let result = b.emit_take(addr, struct_ty);
        b.emit_destroy_value(result);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors.iter().any(|e| e.message.contains("uninit")),
            "expected partial-init error (field 1 missing), got: {:?}",
            errors,
        );
    }

    #[test]
    fn error_double_store_init_same_field() {
        let mut b = OssaBuilder::new("test");
        let (struct_ty, _entity) = make_owned_struct_with_fields(&mut b, 1);
        let i64_ty = b.i64();

        let addr = b.emit_uninit(struct_ty);
        let f0 = b.emit_field_addr(addr, i64_ty, FieldIdx::new(0));

        let v1 = b.emit_literal(Immediate::i64(1));
        let v2 = b.emit_literal(Immediate::i64(2));
        b.emit_store_init(f0, v1);
        b.emit_store_init(f0, v2);

        let result = b.emit_take(addr, struct_ty);
        b.emit_destroy_value(result);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors.iter().any(|e| e.message.contains("already init")),
            "expected double store_init error, got: {:?}",
            errors,
        );
    }

    // -----------------------------------------------------------------------
    // Additional: Discriminant is non-consuming
    // -----------------------------------------------------------------------

    #[test]
    fn valid_discriminant_nonconsuming() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        let disc = b.emit_discriminant(x);
        b.emit_destroy_value(disc);
        b.emit_destroy_value(x);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    // -----------------------------------------------------------------------
    // Additional: ValueId uniqueness
    // -----------------------------------------------------------------------

    #[test]
    fn error_duplicate_value_definition() {
        let mut b = OssaBuilder::new("test");
        let _i64_ty = b.i64();
        let lit = b.emit_literal(Immediate::i64(1));

        {
            let cur = b.current_block();
            let blk = b.body_mut().block_mut(cur);
            blk.insts
                .push(crate::inst::Instruction::new(InstKind::Literal {
                    result: lit,
                    value: Immediate::i64(2),
                }));
        }

        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors.iter().any(|e| e.message.contains("already defined")),
            "expected duplicate ValueId error, got: {:?}",
            errors,
        );
    }

    // -----------------------------------------------------------------------
    // Additional: Op operand ownership checks
    // -----------------------------------------------------------------------

    #[test]
    fn valid_op2_with_guaranteed_operand() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);
        let i64_ty = b.i64();

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x,
                ty: owned_ty,
                ownership: Ownership::Owned,
            });
        }

        let borrow = b.emit_begin_borrow(x);
        let lit = b.emit_literal(Immediate::i64(1));

        let result = b.new_value(i64_ty, Ownership::Owned);
        {
            let cur = b.current_block();
            let blk = b.body_mut().block_mut(cur);
            blk.insts.push(crate::inst::Instruction::new(InstKind::Op2 {
                result,
                op: crate::Op::Add(crate::IntBits::I64, crate::Signedness::Signed),
                lhs: borrow,
                rhs: lit,
            }));
        }

        b.emit_destroy_value(result);
        b.emit_destroy_value(lit);
        b.emit_end_borrow(borrow);
        b.emit_destroy_value(x);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    // -----------------------------------------------------------------------
    // Escape check (references stage 1): the root rule for ret_borrow fns
    // -----------------------------------------------------------------------

    use crate::MirTy;
    use crate::item::function::FunctionDef;
    use crate::value::RootProvenance;

    /// Finish `b` into a module holding one FunctionDef with the given return
    /// type and param conventions (param N's ValueId is entry value N).
    fn finish_fn(
        b: OssaBuilder,
        ret: TyId,
        conventions: &[(ParamConvention, TyId)],
    ) -> (MirModule, Entity) {
        let (body, mut module) = b.finish();
        let entity = Entity::from_raw(900);
        let mut def = FunctionDef::new(entity, "escape_test", ret);
        for (i, &(conv, ty)) in conventions.iter().enumerate() {
            def.params.push(crate::item::function::ParamDef::new(
                format!("p{i}"),
                ValueId::new(i),
                ty,
                conv,
            ));
        }
        def.body = Some(body);
        module.add_function(def);
        (module, entity)
    }

    fn escape_codes(module: &MirModule) -> Vec<&'static str> {
        check_escapes(module)
            .iter()
            .map(|e| e.diag.as_ref().expect("escape errors are coded").code)
            .collect()
    }

    #[test]
    fn escape_param_root_ok() {
        let mut b = OssaBuilder::new("t");
        let i64_ty = b.i64();
        let ref_ty = b.ty(MirTy::Ref {
            pointee: i64_ty,
            mutating: false,
        });
        let p = b.new_param_value(i64_ty, Ownership::Guaranteed);
        b.emit_return(p);
        let (module, _) = finish_fn(b, ref_ty, &[(ParamConvention::Borrow, i64_ty)]);
        assert!(escape_codes(&module).is_empty());
    }

    #[test]
    fn escape_local_root_rejected() {
        let mut b = OssaBuilder::new("t");
        let i64_ty = b.i64();
        let ref_ty = b.ty(MirTy::Ref {
            pointee: i64_ty,
            mutating: false,
        });
        let local = b.emit_literal(Immediate::i64(7));
        let borrow = b.emit_begin_borrow(local);
        b.emit_return(borrow);
        let (module, _) = finish_fn(b, ref_ty, &[]);
        assert_eq!(escape_codes(&module), vec!["E494"]);
    }

    /// E494's closure-carrier FIX-IT (docs/design/closures.md, Diagnostics
    /// table: "message gains a fix-it suggesting an `escaping` or `consuming`
    /// owning type"). Pinned here rather than in testdata because the `.ks`
    /// harness matches only a diagnostic's message + primary label — a
    /// diagnostic's `notes` are not visible to `// ERROR:` annotations, and
    /// the fix-it is a note (plan, decision 7).
    #[test]
    fn escape_view_closure_return_note_suggests_owning_kind() {
        let mut b = OssaBuilder::new("t");
        let i64_ty = b.i64();
        let view_fn = b.ty(MirTy::FuncThick {
            kind: crate::FnKind::Normal,
            params: vec![],
            ret: i64_ty,
        });
        // A frame-bound closure value: an @owned FuncThick whose root is the
        // captured local, exactly what `emit_apply_partial` stamps for a view
        // kind.
        let cap = b.emit_literal(Immediate::i64(7));
        let closure = b.new_value(view_fn, Ownership::Owned);
        b.set_root(closure, RootProvenance::Local(cap));
        b.emit_return(closure);
        let (module, _) = finish_fn(b, view_fn, &[]);
        let errs = check_escapes(&module);
        assert_eq!(escape_codes(&module), vec!["E494"]);
        let notes = &errs[0].diag.as_ref().unwrap().notes;
        assert!(
            notes
                .iter()
                .any(|n| n.contains("escaping (…) -> …") && n.contains("consuming (…) -> …")),
            "E494 on a closure carrier must suggest an owning kind, got {notes:?}"
        );
    }

    #[test]
    fn escape_consuming_param_rejected() {
        let mut b = OssaBuilder::new("t");
        let i64_ty = b.i64();
        let ref_ty = b.ty(MirTy::Ref {
            pointee: i64_ty,
            mutating: false,
        });
        let p = b.new_param_value(i64_ty, Ownership::Owned);
        let borrow = b.emit_begin_borrow(p);
        b.emit_return(borrow);
        let (module, _) = finish_fn(b, ref_ty, &[(ParamConvention::Consuming, i64_ty)]);
        assert_eq!(escape_codes(&module), vec!["E496"]);
    }

    #[test]
    fn escape_mutating_needs_mutable_root() {
        let mut b = OssaBuilder::new("t");
        let i64_ty = b.i64();
        let mut_ref = b.ty(MirTy::Ref {
            pointee: i64_ty,
            mutating: true,
        });
        let p = b.new_param_value(i64_ty, Ownership::Guaranteed);
        b.emit_return(p);
        // Shared Borrow param rooting a `-> &mutating` return: const-cast guard.
        let (module, _) = finish_fn(b, mut_ref, &[(ParamConvention::Borrow, i64_ty)]);
        assert_eq!(escape_codes(&module), vec!["E495"]);
    }

    #[test]
    fn escape_mut_param_root_ok_for_mutating() {
        let mut b = OssaBuilder::new("t");
        let i64_ty = b.i64();
        let mut_ref = b.ty(MirTy::Ref {
            pointee: i64_ty,
            mutating: true,
        });
        let p = b.new_param_value(i64_ty, Ownership::Guaranteed);
        b.emit_return(p);
        let (module, _) = finish_fn(b, mut_ref, &[(ParamConvention::MutBorrow, i64_ty)]);
        assert!(escape_codes(&module).is_empty());
    }

    #[test]
    fn escape_static_root_matrix() {
        // Shared ref of a Static root: ok. `&mutating` of it: E495.
        for (mutating, expect) in [(false, vec![]), (true, vec!["E495"])] {
            let mut b = OssaBuilder::new("t");
            let i64_ty = b.i64();
            let ref_ty = b.ty(MirTy::Ref {
                pointee: i64_ty,
                mutating,
            });
            let local = b.emit_literal(Immediate::i64(7));
            let borrow = b.emit_begin_borrow(local);
            b.set_root(borrow, RootProvenance::Static);
            b.emit_return(borrow);
            let (module, _) = finish_fn(b, ref_ty, &[]);
            assert_eq!(escape_codes(&module), expect, "mutating={mutating}");
        }
    }

    #[test]
    fn escape_pointer_derived_matrix() {
        // (root mutability, ret mutating) → expected codes.
        let cases = [
            (false, false, vec![]),
            (true, false, vec![]), // mut root may decay to a shared ret
            (false, true, vec!["E495"]),
            (true, true, vec![]),
        ];
        for (mutable, mutating, expect) in cases {
            let mut b = OssaBuilder::new("t");
            let i64_ty = b.i64();
            let ref_ty = b.ty(MirTy::Ref {
                pointee: i64_ty,
                mutating,
            });
            let local = b.emit_literal(Immediate::i64(7));
            let borrow = b.emit_begin_borrow(local);
            b.set_root(borrow, RootProvenance::PointerDerived { mutable });
            b.emit_return(borrow);
            let (module, _) = finish_fn(b, ref_ty, &[]);
            assert_eq!(
                escape_codes(&module),
                expect,
                "mutable={mutable} mutating={mutating}"
            );
        }
    }

    #[test]
    fn escape_root_inherited_through_projection_chain() {
        // Borrow of a borrow of a param still roots at the param.
        let mut b = OssaBuilder::new("t");
        let i64_ty = b.i64();
        let ref_ty = b.ty(MirTy::Ref {
            pointee: i64_ty,
            mutating: false,
        });
        let p = b.new_param_value(i64_ty, Ownership::Guaranteed);
        let b1 = b.emit_begin_borrow(p);
        let b2 = b.emit_begin_borrow(b1);
        b.emit_return(b2);
        let (module, _) = finish_fn(b, ref_ty, &[(ParamConvention::Borrow, i64_ty)]);
        assert!(escape_codes(&module).is_empty());
    }

    // Return-convention hardening (verify_terminator): ICE-class, uncoded.

    #[test]
    fn guaranteed_return_without_ret_borrow_is_error() {
        let mut b = OssaBuilder::new("t");
        let i64_ty = b.i64();
        let p = b.new_param_value(i64_ty, Ownership::Guaranteed);
        b.emit_return(p);
        // Function returns i64 by value but the body returns a borrow.
        let (module, entity) = finish_fn(b, i64_ty, &[(ParamConvention::Borrow, i64_ty)]);
        let func = module.functions.get(&entity).unwrap();
        let errors = verify_ossa(func.body.as_ref().unwrap(), &module, "t", entity);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("without the ret_borrow convention")),
            "got: {errors:?}"
        );
    }

    #[test]
    fn owned_return_in_ret_borrow_fn_is_error() {
        let mut b = OssaBuilder::new("t");
        let i64_ty = b.i64();
        let ref_ty = b.ty(MirTy::Ref {
            pointee: i64_ty,
            mutating: false,
        });
        let v = b.emit_literal(Immediate::i64(7));
        b.emit_return(v);
        let (module, entity) = finish_fn(b, ref_ty, &[]);
        let func = module.functions.get(&entity).unwrap();
        let errors = verify_ossa(func.body.as_ref().unwrap(), &module, "t", entity);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("returns @owned value")),
            "got: {errors:?}"
        );
    }

    #[test]
    fn ret_borrow_return_passes_check4() {
        // The returned borrow is exempt from "still active at block exit".
        let mut b = OssaBuilder::new("t");
        let i64_ty = b.i64();
        let ref_ty = b.ty(MirTy::Ref {
            pointee: i64_ty,
            mutating: false,
        });
        let p = b.new_param_value(i64_ty, Ownership::Guaranteed);
        let borrow = b.emit_begin_borrow(p);
        b.emit_return(borrow);
        let (module, entity) = finish_fn(b, ref_ty, &[(ParamConvention::Borrow, i64_ty)]);
        let func = module.functions.get(&entity).unwrap();
        let errors = verify_ossa(func.body.as_ref().unwrap(), &module, "t", entity);
        assert!(errors.is_empty(), "got: {errors:?}");
    }
}
