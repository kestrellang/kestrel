pub mod call;
pub mod closure;
pub mod control;
pub mod expr;
pub mod literal;
pub mod pattern;
pub mod stmt;

use std::collections::HashMap;

use kestrel_hecs::Entity;
use kestrel_hir::body::{HirBlock, HirBody, HirExpr, HirExprId};
use kestrel_hir::res::LocalId as HirLocalId;
use kestrel_mir_3::block::BlockParam;
use kestrel_mir_3::body::OssaBody;
use kestrel_mir_3::callee::Callee;
use kestrel_mir_3::inst::{CallArg, InstKind, Instruction};
use kestrel_mir_3::terminator::{SwitchArm, Terminator, TerminatorKind};
use kestrel_mir_3::value::{Ownership, ValueDef};
use kestrel_mir_3::{
    BlockId, CopyBehavior, FieldIdx, Immediate, MirTy, Op, ParamConvention, TyId, ValueId,
    VariantIdx,
};
use kestrel_span::Span;
use kestrel_type_infer::captures::{ClosureCaptureMap, PlaceKey};
use kestrel_type_infer::result::TypedBody;

use crate::context::LowerCtx;
use crate::ty::{lower_resolved_ty, lower_type};

pub(crate) struct LoopInfo {
    pub header_block: BlockId,
    pub exit_block: BlockId,
    pub label: Option<String>,
    pub scope_depth: usize,
    /// Number of slots in the loop's tracker. Break takes the first
    /// N values from the active tracker to thread to the exit block.
    pub tracker_len: usize,
}

pub(crate) enum HirRef<'a> {
    Borrowed(&'a HirBody),
    Owned(HirBody),
}

impl std::ops::Deref for HirRef<'_> {
    type Target = HirBody;
    fn deref(&self) -> &HirBody {
        match self {
            HirRef::Borrowed(r) => r,
            HirRef::Owned(o) => o,
        }
    }
}

pub(crate) enum TypedRef<'a> {
    Borrowed(&'a TypedBody),
    Owned(TypedBody),
}

impl std::ops::Deref for TypedRef<'_> {
    type Target = TypedBody;
    fn deref(&self) -> &TypedBody {
        match self {
            TypedRef::Borrowed(r) => r,
            TypedRef::Owned(o) => o,
        }
    }
}

/// How a local variable is bound: directly as an SSA value, or
/// indirectly through a stack address (for mutable vars/mutating params).
#[derive(Clone, Copy)]
pub(crate) enum LocalBinding {
    Ssa(ValueId),
    Var(ValueId),
}

impl LocalBinding {
    pub fn value(self) -> ValueId {
        match self {
            LocalBinding::Ssa(v) | LocalBinding::Var(v) => v,
        }
    }
}

/// What kind of function body we're lowering — determines receiver type
/// resolution and field store semantics.
#[derive(Clone)]
pub(crate) enum BodyContext {
    Normal,
    ProtocolExtension,
    Initializer { self_addr: ValueId },
    ProtocolExtensionInit { self_addr: ValueId },
}

impl BodyContext {
    pub fn is_protocol_extension(&self) -> bool {
        matches!(
            self,
            BodyContext::ProtocolExtension | BodyContext::ProtocolExtensionInit { .. }
        )
    }

    pub fn init_self_addr(&self) -> Option<ValueId> {
        match self {
            BodyContext::Initializer { self_addr }
            | BodyContext::ProtocolExtensionInit { self_addr } => Some(*self_addr),
            _ => None,
        }
    }
}

/// Static initialization state of a `var` (address) slot — whether the slot
/// still owns a value at a given program point. Mirrors Swift's Definite
/// Initialization availability: `DefInit` (owns a value), `DefUninit` (moved out /
/// `load [take]`'d), and `MaybeUninit` (consumed on some control-flow paths only),
/// which is reconciled with a runtime drop flag (Swift's `dynamic_lifetime`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum VarInit {
    DefInit,
    MaybeUninit,
    DefUninit,
}

impl VarInit {
    /// Lattice join over reaching control-flow edges.
    pub fn join(self, other: VarInit) -> VarInit {
        use VarInit::*;
        match (self, other) {
            (DefInit, DefInit) => DefInit,
            (DefUninit, DefUninit) => DefUninit,
            _ => MaybeUninit,
        }
    }
}

#[derive(Clone)]
pub(crate) enum ScopeEntry {
    Owned(ValueId),
    Var {
        addr: ValueId,
        ty: TyId,
        /// Whether the slot currently owns a value (drives scope-exit destroy).
        init: VarInit,
        /// In-memory `Bool` drop-flag slot (original pointer) for a
        /// conditionally-moved var; `None` for vars that are never conditionally
        /// moved. `true` in memory = slot owns a value (must drop).
        flag: Option<ValueId>,
        /// Stable identity of the slot across block-merge rebinds (`addr` itself is
        /// stable today, but reads key by the HIR local). `None` for self/params.
        local: Option<HirLocalId>,
    },
    /// @guaranteed borrow needing EndBorrow at scope exit.
    Borrow(ValueId),
}

pub(crate) struct ScopeFrame {
    pub entries: Vec<ScopeEntry>,
}

#[derive(Clone)]
pub(crate) struct ScopeSnapshot {
    pub scopes: Vec<Vec<ScopeEntry>>,
    pub local_map: HashMap<HirLocalId, LocalBinding>,
    pub tracker: LiveTracker,
}

/// Tracks a fixed set of @owned values across control flow merges.
///
/// Created at the entry of an if/match with the current live @owned values.
/// The slot count never changes — only the current ValueId per slot updates
/// when values are rebound at block boundaries. This ensures the merge block
/// always receives the correct number of arguments, regardless of what values
/// are created or consumed during arm execution.
#[derive(Clone)]
struct TrackerSlot {
    value: ValueId,
    ty: TyId,
    ownership: Ownership,
    /// False once the value has been moved/consumed. Dead slots keep their
    /// position so callers can reason about liveness positionally across
    /// nested control flow (which rebinds slots in place), but are excluded
    /// from `values()` / `descs()` so they are no longer forwarded.
    alive: bool,
}

#[derive(Clone)]
pub(crate) struct LiveTracker {
    slots: Vec<TrackerSlot>,
}

impl LiveTracker {
    pub fn from_live(live: &[(ValueId, TyId, Ownership)]) -> Self {
        Self {
            slots: live
                .iter()
                .map(|&(value, ty, ownership)| TrackerSlot {
                    value,
                    ty,
                    ownership,
                    alive: true,
                })
                .collect(),
        }
    }

    /// Current values to forward as block args (alive slots only).
    pub fn values(&self) -> Vec<ValueId> {
        self.slots
            .iter()
            .filter(|s| s.alive)
            .map(|s| s.value)
            .collect()
    }

    /// Type descriptors for creating block params (alive slots only).
    pub fn descs(&self) -> Vec<(TyId, Ownership)> {
        self.slots
            .iter()
            .filter(|s| s.alive)
            .map(|s| (s.ty, s.ownership))
            .collect()
    }

    /// Update slots when entering a new block whose params replace old values.
    pub fn rebind(&mut self, old: &[ValueId], new: &[ValueId]) {
        for slot in &mut self.slots {
            if let Some(pos) = old.iter().position(|&v| v == slot.value) {
                slot.value = new[pos];
            }
        }
    }

    /// Number of values currently forwarded (alive slots).
    pub fn len(&self) -> usize {
        self.slots.iter().filter(|s| s.alive).count()
    }

    /// Positional view over all slots (alive and dead) as `(current value,
    /// alive)`, used to reconcile divergent liveness at a branch merge.
    pub fn slot_states(&self) -> Vec<(ValueId, bool)> {
        self.slots.iter().map(|s| (s.value, s.alive)).collect()
    }

    /// Mark a value dead (e.g. when it has been moved) so it is no longer
    /// forwarded. The slot keeps its position for positional liveness checks.
    pub fn remove(&mut self, value: ValueId) {
        for slot in &mut self.slots {
            if slot.value == value {
                slot.alive = false;
            }
        }
    }

    /// Whether `value` is still alive in the forwarded set.
    pub fn contains(&self, value: ValueId) -> bool {
        self.slots.iter().any(|s| s.alive && s.value == value)
    }
}

/// Exit state of one branch/arm that reaches a merge. `slots[i]` is the
/// `(current value, alive)` of the i-th pre-branch live value at the exit —
/// `alive == false` means the value was moved/consumed on this edge. Shared by
/// `lower_if` (control.rs) and `lower_match` (pattern.rs).
pub(crate) struct ArmExit {
    pub block: BlockId,
    pub result: ValueId,
    pub slots: Vec<(ValueId, bool)>,
    /// Static init-state of each in-scope `var` (by HIR local) at this arm's exit,
    /// for drop-flag reconciliation at the merge.
    pub var_inits: Vec<(HirLocalId, VarInit)>,
}

pub(crate) struct OssaBodyCtx<'a, 'w> {
    pub ctx: &'a mut LowerCtx<'w>,
    pub hir: HirRef<'a>,
    pub typed: Option<TypedRef<'a>>,
    pub func_entity: Entity,
    pub body_context: BodyContext,
    pub body: OssaBody,
    pub current_block: Option<BlockId>,
    pub local_map: HashMap<HirLocalId, LocalBinding>,
    pub loop_stack: Vec<LoopInfo>,
    pub scope_stack: Vec<ScopeFrame>,
    pub tracker: LiveTracker,
    pub deferred_end_borrows: Vec<ValueId>,
    pub temp_counter: u32,
    pub current_span: Option<Span>,
    /// Remaining use count per SSA local — decremented on each lower_expr.
    /// When the count hits zero, the local is moved instead of copied.
    local_use_counts: HashMap<HirLocalId, usize>,
    /// Place-based closure capture plan for this body, keyed by closure
    /// `HirExprId`. Computed once (post-inference) and consumed by
    /// `lower_closure_expr`.
    pub(crate) captures: ClosureCaptureMap,
    /// Inside a closure body: the env value loaded for each captured *place*
    /// (e.g. `self.cap`). Consulted when lowering reads/borrows so projected
    /// captures read the env value instead of projecting from a (non-captured)
    /// receiver. Whole-local captures use `local_map` instead. Saved/restored
    /// across nested closure bodies like `local_map`.
    pub(crate) place_capture_map: HashMap<PlaceKey, ValueId>,
}

impl<'a, 'w> OssaBodyCtx<'a, 'w> {
    pub fn new(
        ctx: &'a mut LowerCtx<'w>,
        hir: &'a HirBody,
        typed: Option<&'a TypedBody>,
        captures: ClosureCaptureMap,
        func_entity: Entity,
        in_protocol_extension: bool,
    ) -> Self {
        Self {
            ctx,
            hir: HirRef::Borrowed(hir),
            typed: typed.map(TypedRef::Borrowed),
            captures,
            place_capture_map: HashMap::new(),
            func_entity,
            body_context: if in_protocol_extension {
                BodyContext::ProtocolExtension
            } else {
                BodyContext::Normal
            },
            body: OssaBody::new(),
            current_block: None,
            local_map: HashMap::new(),
            loop_stack: Vec::new(),
            scope_stack: Vec::new(),
            tracker: LiveTracker::from_live(&[]),
            deferred_end_borrows: Vec::new(),
            temp_counter: 0,
            current_span: None,
            local_use_counts: HashMap::new(),
        }
    }

    // ================================================================
    // Main entry
    // ================================================================

    pub fn lower_body(&mut self) {
        // Build use counts for SSA locals — enables move-on-last-use.
        self.local_use_counts = self.count_local_uses();

        let locals: Vec<_> = self
            .hir
            .locals
            .iter()
            .map(|(id, l)| (id, l.clone()))
            .collect();
        let params_len = self.hir.params.len();
        let statements: Vec<_> = self.hir.statements.clone();

        let entry = self.new_block();
        self.body.entry = entry;
        self.current_block = Some(entry);
        self.push_scope();

        // Only pre-allocate function parameters. Non-param locals get their
        // ValueId when their let-statement or pattern binding fires.
        // This prevents orphan ValueIds (with no defining instruction) from
        // leaking through scope snapshots across match/if-let boundaries.
        // Check param conventions from the MIR function def.
        // MutBorrow params (mutating self, mutating args) receive an address
        // and are bound as LocalBinding::Var — reads go through Load, field
        // assignments use the address directly.
        let param_conventions: Vec<ParamConvention> = self
            .ctx
            .module
            .functions
            .get(&self.func_entity)
            .map(|f| f.params.iter().map(|p| p.convention).collect())
            .unwrap_or_default();
        let is_init_body = self
            .ctx
            .module
            .functions
            .get(&self.func_entity)
            .map(|f| {
                matches!(
                    f.kind,
                    kestrel_mir_3::item::function::FunctionKind::Initializer { .. }
                )
            })
            .unwrap_or(false);
        for (i, (hir_id, _local)) in locals.iter().enumerate() {
            if i >= params_len {
                break;
            }
            let ty = self.resolve_local_type(*hir_id);
            let convention = param_conventions
                .get(i)
                .copied()
                .unwrap_or(ParamConvention::Borrow);
            match convention {
                ParamConvention::MutBorrow => {
                    let val = self.body.alloc_value(ValueDef {
                        ty,
                        ownership: Ownership::Guaranteed,
                        borrow_source: None,
                        span: None,
                    });
                    self.local_map.insert(*hir_id, LocalBinding::Var(val));
                    if is_init_body && i == 0 {
                        self.body_context = match self.body_context {
                            BodyContext::ProtocolExtension => {
                                BodyContext::ProtocolExtensionInit { self_addr: val }
                            },
                            _ => BodyContext::Initializer { self_addr: val },
                        };
                    }
                },
                ParamConvention::Borrow => {
                    let val = self.body.alloc_value(ValueDef {
                        ty,
                        ownership: Ownership::Guaranteed,
                        borrow_source: None,
                        span: None,
                    });
                    self.local_map.insert(*hir_id, LocalBinding::Ssa(val));
                },
                ParamConvention::Consuming => {
                    let ownership = self.ownership_for(ty);
                    let val = self.alloc_value(ty, ownership);
                    self.local_map.insert(*hir_id, LocalBinding::Ssa(val));
                    self.track_owned(val);
                },
            }
        }
        self.body.param_count = params_len;

        // Consuming params stay as SSA @owned values — no var_local
        // promotion. Mutating calls use BeginMutBorrow on the SSA value
        // directly, and last-use forwarding avoids unnecessary clones.

        // Lower top-level statements
        for &stmt_id in &statements {
            self.lower_stmt(stmt_id);
            if self.is_terminated() {
                break;
            }
        }

        // Lower tail expression and return
        if !self.is_terminated() {
            if let Some(tail) = self.hir.tail_expr {
                let tail_span = expr_span(&self.hir, tail);
                let value = self.lower_expr(tail);
                if !self.is_terminated() {
                    // @guaranteed values can't be returned — copy to @owned first
                    let value = if self.body.value(value).ownership == Ownership::Guaranteed {
                        let owned = self.emit_copy_value(value);
                        self.emit_end_borrow(value);
                        owned
                    } else {
                        value
                    };

                    self.drain_deferred_borrows();
                    let prev = self.current_span.replace(tail_span);
                    self.destroy_scopes_to_depth(0, &[value]);
                    self.set_terminator(TerminatorKind::Return(value));
                    self.current_span = prev;
                }
            } else {
                let unit = self.emit_literal(Immediate::unit());
                self.destroy_scopes_to_depth(0, &[unit]);
                self.set_terminator(TerminatorKind::Return(unit));
            }
        }
        self.pop_scope();
    }

    // ================================================================
    // Block management
    // ================================================================

    pub fn new_block(&mut self) -> BlockId {
        self.body.alloc_block()
    }

    pub fn new_block_with_params(
        &mut self,
        params: &[(TyId, Ownership)],
    ) -> (BlockId, Vec<ValueId>) {
        let block = self.body.alloc_block();
        let mut values = Vec::new();
        for &(ty, ownership) in params {
            let val = self.alloc_value(ty, ownership);
            self.body.block_mut(block).params.push(BlockParam {
                value: val,
                ty,
                ownership,
            });
            values.push(val);
        }
        (block, values)
    }

    pub fn switch_to(&mut self, block: BlockId) {
        self.current_block = Some(block);
    }

    pub fn is_terminated(&self) -> bool {
        self.current_block
            .map(|b| {
                !matches!(
                    self.body.block(b).terminator.kind,
                    TerminatorKind::Unreachable
                )
            })
            .unwrap_or(true)
    }

    // ================================================================
    // Type queries
    // ================================================================

    pub fn resolve_expr_type(&mut self, expr_id: HirExprId) -> TyId {
        if let Some(typed) = self.typed.as_ref()
            && let Some(resolved) = typed.expr_types.get(&expr_id)
        {
            return lower_resolved_ty(self.ctx, resolved);
        }
        self.ctx.module.ty_arena.error()
    }

    pub fn resolve_local_type(&mut self, hir_id: HirLocalId) -> TyId {
        if let Some(typed) = self.typed.as_ref()
            && let Some(resolved) = typed.local_types.get(&hir_id)
        {
            return lower_resolved_ty(self.ctx, resolved);
        }
        self.ctx.module.ty_arena.error()
    }

    pub fn is_var_local(&self, hir_id: &HirLocalId) -> bool {
        matches!(self.local_map.get(hir_id), Some(LocalBinding::Var(_)))
    }

    /// Count how many times each local appears as `HirExpr::Local` in the body.
    fn count_local_uses(&self) -> HashMap<HirLocalId, usize> {
        let mut counts = HashMap::new();
        for (_, expr) in self.hir.exprs.iter() {
            if let HirExpr::Local(id, _) = expr {
                *counts.entry(*id).or_default() += 1;
            }
        }
        counts
    }

    /// Returns true if this SSA local is referenced exactly once in the HIR,
    /// meaning this reference is the only use and the value can be moved.
    fn is_single_use(&self, hir_id: HirLocalId) -> bool {
        self.local_use_counts.get(&hir_id).copied().unwrap_or(0) == 1
    }

    pub fn map_local(&mut self, hir_id: HirLocalId) -> ValueId {
        if let Some(&binding) = self.local_map.get(&hir_id) {
            return binding.value();
        }
        // Lazy allocation for locals referenced before their let-statement
        // (e.g. deinit of an uninitialized local, closure captures).
        let ty = self.resolve_local_type(hir_id);
        let ownership = self.ownership_for(ty);
        let val = self.alloc_value(ty, ownership);
        self.local_map.insert(hir_id, LocalBinding::Ssa(val));
        val
    }

    fn copy_behavior_of(&self, ty: TyId) -> CopyBehavior {
        let wc = self
            .ctx
            .module
            .functions
            .get(&self.func_entity)
            .and_then(|f| f.where_clause.as_ref());
        kestrel_mir_3::ty_query::copy_behavior(&self.ctx.module.ty_arena, &self.ctx.module, ty, wc)
    }

    pub fn is_copy_type(&self, ty: TyId) -> bool {
        matches!(self.copy_behavior_of(ty), CopyBehavior::Bitwise)
    }

    /// A `not Copyable` type has no clone shim — duplicating it is illegal, so
    /// an @owned transfer of such a value must be a move, never a copy.
    pub fn is_non_copyable(&self, ty: TyId) -> bool {
        matches!(self.copy_behavior_of(ty), CopyBehavior::None)
    }

    pub fn ownership_for(&self, _ty: TyId) -> Ownership {
        Ownership::Owned
    }

    // ================================================================
    // Value allocation
    // ================================================================

    pub fn alloc_value(&mut self, ty: TyId, ownership: Ownership) -> ValueId {
        let def = match ownership {
            Ownership::Owned => ValueDef::owned(ty),
            Ownership::Guaranteed => panic!("use alloc_guaranteed for @guaranteed"),
        };
        // Stamp the value with the current expr/stmt span, mirroring push_inst —
        // gives verifier ICEs a precise location for this value's definition.
        self.body
            .alloc_value(def.with_span(self.current_span.clone()))
    }

    pub fn alloc_value_auto(&mut self, ty: TyId) -> ValueId {
        let ownership = self.ownership_for(ty);
        self.alloc_value(ty, ownership)
    }

    pub fn alloc_guaranteed(&mut self, ty: TyId, source: ValueId) -> ValueId {
        self.body
            .alloc_value(ValueDef::guaranteed(ty, source).with_span(self.current_span.clone()))
    }

    // ================================================================
    // Scope tracking
    // ================================================================

    pub fn push_scope(&mut self) {
        self.scope_stack.push(ScopeFrame {
            entries: Vec::new(),
        });
    }

    pub fn track_var(
        &mut self,
        address: ValueId,
        content_ty: TyId,
        local: Option<HirLocalId>,
        flag: Option<ValueId>,
    ) {
        if let Some(frame) = self.scope_stack.last_mut() {
            frame.entries.push(ScopeEntry::Var {
                addr: address,
                ty: content_ty,
                init: VarInit::DefInit,
                flag,
                local,
            });
        }
    }

    /// Current static init-state of the `var` slot for HIR `local`, searching
    /// inner scopes first. Keyed by `HirLocalId` (stable across block-merge
    /// rebinds), not address. `None` if not a tracked var local.
    pub fn var_init(&self, local: HirLocalId) -> Option<VarInit> {
        for scope in self.scope_stack.iter().rev() {
            for entry in scope.entries.iter().rev() {
                if let ScopeEntry::Var {
                    local: Some(l),
                    init,
                    ..
                } = entry
                {
                    if *l == local {
                        return Some(*init);
                    }
                }
            }
        }
        None
    }

    /// Set the static init-state of the `var` slot for HIR `local`.
    pub fn set_var_init(&mut self, local: HirLocalId, new_init: VarInit) {
        for scope in self.scope_stack.iter_mut().rev() {
            for entry in scope.entries.iter_mut().rev() {
                if let ScopeEntry::Var {
                    local: Some(l),
                    init,
                    ..
                } = entry
                {
                    if *l == local {
                        *init = new_init;
                        return;
                    }
                }
            }
        }
    }

    /// The MIR `Bool` type id.
    pub fn bool_ty(&mut self) -> TyId {
        Immediate::bool(false).ty(&mut self.ctx.module.ty_arena)
    }

    /// Allocate an in-memory drop flag (Swift's `dynamic_lifetime` control
    /// variable) initialized to `true` (= slot owns a value). Returns the flag
    /// slot pointer; store/load it via this original pointer in any later block
    /// (the verifier permits it — see Phase 3 design note). The pointer is
    /// tracked like the var address: threaded through merges and `DestroyValue`'d
    /// (a no-op on a trivial `Bool` slot) at scope exit.
    pub fn alloc_var_flag(&mut self) -> ValueId {
        let bty = self.bool_ty();
        let flag = self.emit_uninit(bty);
        let init = self.emit_literal(Immediate::bool(true));
        self.emit_store_init(flag, init);
        flag
    }

    /// Allocate a drop flag for a non-Copyable var (which may be conditionally
    /// moved); `None` for Copyable vars (never moved, never need a flag).
    pub fn maybe_alloc_var_flag(&mut self, ty: TyId) -> Option<ValueId> {
        if self.is_non_copyable(ty) {
            Some(self.alloc_var_flag())
        } else {
            None
        }
    }

    /// Store `full` into a drop-flag slot (`true` = slot owns a value).
    pub fn store_drop_flag(&mut self, flag: ValueId, full: bool) {
        let v = self.emit_literal(Immediate::bool(full));
        self.emit_store_assign(flag, v);
    }

    pub fn emit_destroy_addr(&mut self, address: ValueId, ty: TyId) {
        self.push_inst(InstKind::DestroyAddr { address, ty });
    }

    /// Emit `if load(flag) { destroy_addr(slot) }` — Swift's drop-flag-guarded
    /// cleanup of a conditionally-moved var. Threads all live owned values
    /// through the diamond and returns `thread` remapped to the continuation
    /// block (where `current_block` is left). The `DestroyAddr` consumes no owned
    /// SSA value, so both edges carry the same live set and the merge is trivial.
    pub fn emit_guarded_destroy(
        &mut self,
        flag: ValueId,
        slot: ValueId,
        ty: TyId,
        thread: &[ValueId],
    ) -> Vec<ValueId> {
        let bty = self.bool_ty();
        let cond = self.emit_load(flag, bty);

        let saved_tracker = self.tracker.clone();
        self.tracker = LiveTracker::from_live(&self.all_live_tracked());
        let live = self.tracker.values();
        let descs = self.tracker.descs();

        let (then_block, then_params) = self.new_block_with_params(&descs);
        let (else_block, else_params) = self.new_block_with_params(&descs);
        let (merge_block, merge_params) = self.new_block_with_params(&descs);
        self.emit_branch(
            cond,
            then_block,
            live.clone(),
            else_block,
            live.clone(),
        );

        // then arm: run the deinit on the slot, then forward the live set.
        self.switch_to(then_block);
        self.emit_destroy_addr(slot, ty);
        self.emit_jump(merge_block, then_params.clone());

        // else arm: forward unchanged.
        self.switch_to(else_block);
        self.emit_jump(merge_block, else_params);

        // continuation: rebind scope/tracker to the merge params.
        self.switch_to(merge_block);
        self.rebind_scope_values(&live, &merge_params);
        self.tracker = saved_tracker;
        self.tracker.rebind(&live, &merge_params);

        thread
            .iter()
            .map(|&t| match live.iter().position(|&v| v == t) {
                Some(pos) => merge_params[pos],
                None => t,
            })
            .collect()
    }

    /// Drop-flag slot (original pointer) of the `var` for HIR `local`, if any.
    pub fn var_flag(&self, local: HirLocalId) -> Option<ValueId> {
        for scope in self.scope_stack.iter().rev() {
            for entry in scope.entries.iter().rev() {
                if let ScopeEntry::Var {
                    local: Some(l),
                    flag,
                    ..
                } = entry
                {
                    if *l == local {
                        return *flag;
                    }
                }
            }
        }
        None
    }

    pub fn track_owned(&mut self, value: ValueId) {
        if self.is_terminated() {
            return;
        }
        if let Some(frame) = self.scope_stack.last_mut() {
            let already = frame
                .entries
                .iter()
                .any(|e| matches!(e, ScopeEntry::Owned(v) if *v == value));
            if !already {
                frame.entries.push(ScopeEntry::Owned(value));
            }
        }
    }

    pub fn consume(&mut self, value: ValueId) {
        // A genuinely consumed (moved) value must no longer be forwarded
        // through a branch merge.
        self.tracker.remove(value);
        self.pop_owned_from_scope(value);
    }

    /// Remove an owned value from scope tracking WITHOUT marking it dead in the
    /// `tracker`. Used for re-threading bookkeeping (e.g. loop block params),
    /// where the value lives on as a block parameter rather than being moved.
    pub fn pop_owned_from_scope(&mut self, value: ValueId) {
        for scope in self.scope_stack.iter_mut().rev() {
            if let Some(pos) = scope
                .entries
                .iter()
                .position(|e| matches!(e, ScopeEntry::Owned(v) if *v == value))
            {
                scope.entries.remove(pos);
                return;
            }
        }
    }

    pub fn track_borrow(&mut self, value: ValueId) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.entries.push(ScopeEntry::Borrow(value));
        }
    }

    fn untrack_borrow(&mut self, value: ValueId) {
        for scope in self.scope_stack.iter_mut().rev() {
            if let Some(pos) = scope
                .entries
                .iter()
                .position(|e| matches!(e, ScopeEntry::Borrow(v) if *v == value))
            {
                scope.entries.remove(pos);
                return;
            }
        }
    }

    pub fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }

    pub fn destroy_scope_except(&mut self, keep: &[ValueId]) {
        if let Some(scope) = self.scope_stack.last_mut() {
            let borrows: Vec<ValueId> = scope
                .entries
                .iter()
                .rev()
                .filter_map(|e| match e {
                    ScopeEntry::Borrow(v) => Some(*v),
                    _ => None,
                })
                .collect();
            let to_destroy: Vec<ValueId> = scope
                .entries
                .iter()
                .rev()
                .filter_map(|e| match e {
                    ScopeEntry::Owned(v) if !keep.contains(v) => Some(*v),
                    _ => None,
                })
                .collect();
            scope.entries.retain(|e| match e {
                ScopeEntry::Owned(v) => keep.contains(v),
                ScopeEntry::Var { .. } => true,
                ScopeEntry::Borrow(_) => false,
            });
            for v in borrows {
                self.push_inst(InstKind::EndBorrow { operand: v });
            }
            for value in to_destroy {
                self.push_inst(InstKind::DestroyValue { operand: value });
            }
        }
    }

    pub fn destroy_scopes_to_depth(&mut self, target_depth: usize, keep: &[ValueId]) {
        let entries: Vec<ScopeEntry> = self.scope_stack[target_depth..]
            .iter()
            .rev()
            .flat_map(|scope| scope.entries.iter().rev().cloned())
            .collect();
        // End borrows first — they may reference values we're about to destroy.
        for entry in &entries {
            if let ScopeEntry::Borrow(v) = entry {
                self.push_inst(InstKind::EndBorrow { operand: *v });
            }
        }
        for entry in &entries {
            match entry {
                ScopeEntry::Owned(v) if !keep.contains(v) => {
                    self.push_inst(InstKind::DestroyValue { operand: *v });
                },
                // DefUninit: the slot was moved out (Swift `load [take]`) and owns
                // nothing — emitting DestroyAddr would double-free. Skip it.
                ScopeEntry::Var {
                    init: VarInit::DefUninit,
                    ..
                } => {},
                ScopeEntry::Var { addr, ty, .. } => {
                    self.push_inst(InstKind::DestroyAddr {
                        address: *addr,
                        ty: *ty,
                    });
                },
                _ => {},
            }
        }
    }

    pub fn all_live_tracked(&self) -> Vec<(ValueId, TyId, Ownership)> {
        self.scope_stack
            .iter()
            .flat_map(|s| {
                s.entries.iter().filter_map(|e| match e {
                    ScopeEntry::Owned(v) => Some((*v, self.body.value(*v).ty, Ownership::Owned)),
                    _ => None,
                })
            })
            .collect()
    }

    pub fn snapshot_scope(&self) -> ScopeSnapshot {
        ScopeSnapshot {
            scopes: self.scope_stack.iter().map(|s| s.entries.clone()).collect(),
            local_map: self.local_map.clone(),
            tracker: self.tracker.clone(),
        }
    }

    pub fn restore_scope(&mut self, snapshot: &ScopeSnapshot) {
        self.scope_stack.truncate(snapshot.scopes.len());
        for (i, frame) in self.scope_stack.iter_mut().enumerate() {
            // Borrows can't cross block boundaries — strip on restore.
            frame.entries = snapshot.scopes[i]
                .iter()
                .filter(|e| !matches!(e, ScopeEntry::Borrow(_)))
                .cloned()
                .collect();
        }
        self.local_map = snapshot.local_map.clone();
        self.tracker = snapshot.tracker.clone();
    }

    /// Replace scope-tracked values when entering a new block.
    /// Updates scope stack, local_map, AND the shared LiveTracker.
    pub fn rebind_scope_values(&mut self, old_vals: &[ValueId], new_vals: &[ValueId]) {
        for scope in self.scope_stack.iter_mut() {
            for entry in scope.entries.iter_mut() {
                if let ScopeEntry::Owned(v) = entry {
                    if let Some(pos) = old_vals.iter().position(|&old| old == *v) {
                        *v = new_vals[pos];
                    }
                }
            }
        }
        for (_, binding) in self.local_map.iter_mut() {
            let v = binding.value();
            if let Some(pos) = old_vals.iter().position(|&old| old == v) {
                match binding {
                    LocalBinding::Ssa(val) => *val = new_vals[pos],
                    LocalBinding::Var(val) => *val = new_vals[pos],
                }
            }
        }
        self.tracker.rebind(old_vals, new_vals);
    }

    /// Finish a branch/arm: materialize an @owned result, drop arm-local owned
    /// values (keeping the result + threaded tracker values), and capture the
    /// arm's exit state. Returns `None` if the arm diverged (already terminated).
    /// Shared by `lower_if` and the match decision-tree walk.
    pub fn capture_arm_exit(&mut self, result: ValueId) -> Option<ArmExit> {
        if self.is_terminated() {
            return None;
        }
        // A merge param can't carry a borrow — materialize an @owned result.
        let result = if self.body.value(result).ownership == Ownership::Guaranteed {
            self.emit_copy_value(result)
        } else {
            result
        };
        // Drop owned values local to this arm; keep the result + threaded values.
        let mut keep = vec![result];
        keep.extend(self.tracker.values());
        self.destroy_scope_except(&keep);
        let block = self.current_block.expect("arm has a current block");
        Some(ArmExit {
            block,
            result,
            slots: self.tracker.slot_states(),
            var_inits: self.scope_var_inits(),
        })
    }

    /// Per-`var` static init-state (keyed by HIR local) across all in-scope
    /// frames, innermost wins. Used to reconcile conditional moves at merges.
    pub fn scope_var_inits(&self) -> Vec<(HirLocalId, VarInit)> {
        let mut out: Vec<(HirLocalId, VarInit)> = Vec::new();
        for scope in self.scope_stack.iter().rev() {
            for entry in scope.entries.iter().rev() {
                if let ScopeEntry::Var {
                    local: Some(l),
                    init,
                    ..
                } = entry
                {
                    if !out.iter().any(|(k, _)| k == l) {
                        out.push((*l, *init));
                    }
                }
            }
        }
        out
    }

    // ================================================================
    // Emit instructions
    // ================================================================

    pub fn push_inst(&mut self, kind: InstKind) {
        if self.is_terminated() {
            return;
        }
        let inst = match &self.current_span {
            Some(s) => Instruction::with_span(kind, s.clone()),
            None => Instruction::new(kind),
        };
        if let Some(block_id) = self.current_block {
            self.body.block_mut(block_id).insts.push(inst);
        }
    }

    pub fn emit_literal(&mut self, imm: Immediate) -> ValueId {
        let ty = imm.ty(&mut self.ctx.module.ty_arena);
        let result = self.alloc_value(ty, Ownership::Owned);
        self.push_inst(InstKind::Literal { result, value: imm });
        self.track_owned(result);
        result
    }

    pub fn emit_copy_value(&mut self, operand: ValueId) -> ValueId {
        let ty = self.body.value(operand).ty;
        // A copy of a non-Copyable @owned value is illegal (no clone shim). Such
        // a transfer is a move: consume the operand instead of duplicating it.
        if self.body.value(operand).ownership == Ownership::Owned && self.is_non_copyable(ty) {
            return self.emit_move_value(operand);
        }
        let result = self.alloc_value(ty, Ownership::Owned);
        self.push_inst(InstKind::CopyValue { result, operand });
        self.track_owned(result);
        result
    }

    /// Move an @owned value: produces a fresh @owned result and consumes the
    /// operand (removing it from scope + tracker so it isn't dropped or
    /// forwarded again). Used for non-Copyable transfers.
    pub fn emit_move_value(&mut self, operand: ValueId) -> ValueId {
        let ty = self.body.value(operand).ty;
        let result = self.alloc_value(ty, Ownership::Owned);
        self.push_inst(InstKind::MoveValue { result, operand });
        self.consume(operand);
        self.track_owned(result);
        result
    }

    pub fn emit_destroy_value(&mut self, operand: ValueId) {
        self.push_inst(InstKind::DestroyValue { operand });
        self.consume(operand);
    }

    pub fn emit_begin_borrow(&mut self, operand: ValueId) -> ValueId {
        let ty = self.body.value(operand).ty;
        let result = self.alloc_guaranteed(ty, operand);
        self.push_inst(InstKind::BeginBorrow { result, operand });
        self.track_borrow(result);
        result
    }

    /// Borrow directly from an address (e.g. a mutable self parameter).
    /// Avoids the copy_addr + begin_borrow pattern which creates an @owned
    /// copy whose destruction runs drop shims (breaking refcount for RcBox etc.)
    pub fn emit_begin_borrow_addr(&mut self, address: ValueId, ty: TyId) -> ValueId {
        let result = self.alloc_guaranteed(ty, address);
        self.push_inst(InstKind::BeginBorrowAddr {
            result,
            address,
            ty,
        });
        self.track_borrow(result);
        result
    }

    pub fn emit_end_borrow(&mut self, operand: ValueId) {
        self.deferred_end_borrows.retain(|&v| v != operand);
        self.untrack_borrow(operand);
        self.push_inst(InstKind::EndBorrow { operand });
    }

    pub fn drain_deferred_borrows(&mut self) {
        let borrows: Vec<ValueId> = self.deferred_end_borrows.drain(..).collect();
        for v in borrows {
            self.untrack_borrow(v);
            self.push_inst(InstKind::EndBorrow { operand: v });
        }
    }

    pub fn emit_begin_mut_borrow(&mut self, operand: ValueId) -> ValueId {
        let ty = self.body.value(operand).ty;
        let result = self.alloc_guaranteed(ty, operand);
        self.push_inst(InstKind::BeginMutBorrow { result, operand });
        self.track_borrow(result);
        result
    }

    pub fn emit_begin_mut_borrow_addr(&mut self, address: ValueId, ty: TyId) -> ValueId {
        let result = self.alloc_guaranteed(ty, address);
        self.push_inst(InstKind::BeginMutBorrowAddr {
            result,
            address,
            ty,
        });
        self.track_borrow(result);
        result
    }

    pub fn emit_end_mut_borrow(&mut self, operand: ValueId) {
        self.untrack_borrow(operand);
        self.push_inst(InstKind::EndMutBorrow { operand });
    }

    pub fn emit_op1(&mut self, op: Op, arg: ValueId, result_ty: TyId) -> ValueId {
        let result = self.alloc_value(result_ty, Ownership::Owned);
        self.push_inst(InstKind::Op1 { result, op, arg });
        self.track_owned(result);
        result
    }

    pub fn emit_op2(&mut self, op: Op, lhs: ValueId, rhs: ValueId, result_ty: TyId) -> ValueId {
        let result = self.alloc_value(result_ty, Ownership::Owned);
        self.push_inst(InstKind::Op2 {
            result,
            op,
            lhs,
            rhs,
        });
        self.track_owned(result);
        result
    }

    pub fn emit_op3(
        &mut self,
        op: Op,
        a: ValueId,
        b: ValueId,
        c: ValueId,
        result_ty: TyId,
    ) -> ValueId {
        let result = self.alloc_value(result_ty, Ownership::Owned);
        self.push_inst(InstKind::Op3 {
            result,
            op,
            a,
            b,
            c,
        });
        self.track_owned(result);
        result
    }

    pub fn emit_struct(&mut self, ty: TyId, fields: Vec<(FieldIdx, ValueId)>) -> ValueId {
        let result = self.alloc_value(ty, Ownership::Owned);
        for &(_, v) in &fields {
            self.consume(v);
        }
        self.push_inst(InstKind::Struct { result, ty, fields });
        self.track_owned(result);
        result
    }

    pub fn emit_tuple(&mut self, ty: TyId, elements: Vec<ValueId>) -> ValueId {
        let result = self.alloc_value(ty, Ownership::Owned);
        for &v in &elements {
            self.consume(v);
        }
        self.push_inst(InstKind::Tuple { result, elements });
        self.track_owned(result);
        result
    }

    pub fn emit_enum_variant(
        &mut self,
        enum_ty: TyId,
        variant: VariantIdx,
        payload: Vec<ValueId>,
    ) -> ValueId {
        let result = self.alloc_value(enum_ty, Ownership::Owned);
        for &v in &payload {
            self.consume(v);
        }
        self.push_inst(InstKind::Enum {
            result,
            enum_ty,
            variant,
            payload,
        });
        self.track_owned(result);
        result
    }

    pub fn emit_struct_extract(
        &mut self,
        operand: ValueId,
        field: FieldIdx,
        result_ty: TyId,
    ) -> ValueId {
        let operand_ownership = self.body.value(operand).ownership;
        if operand_ownership == Ownership::Guaranteed {
            let result = self.alloc_guaranteed(result_ty, operand);
            self.push_inst(InstKind::StructExtract {
                result,
                operand,
                field,
            });
            result
        } else {
            // Borrow → extract (@guaranteed) → copy (@owned). Operand stays alive
            // for the tracker and further extractions.
            let borrow = self.emit_begin_borrow(operand);
            let field_ref = self.alloc_guaranteed(result_ty, borrow);
            self.push_inst(InstKind::StructExtract {
                result: field_ref,
                operand: borrow,
                field,
            });
            if self.is_non_copyable(result_ty) {
                // A non-Copyable field can't be duplicated — hand back the
                // @guaranteed view. The borrow is left open (ended at scope
                // exit via the tracker) so the field_ref stays valid for reads.
                return field_ref;
            }
            let result = self.emit_copy_value(field_ref);
            self.emit_end_borrow(borrow);
            result
        }
    }

    pub fn emit_tuple_extract(&mut self, operand: ValueId, index: u32, result_ty: TyId) -> ValueId {
        let operand_ownership = self.body.value(operand).ownership;
        if operand_ownership == Ownership::Guaranteed {
            let result = self.alloc_guaranteed(result_ty, operand);
            self.push_inst(InstKind::TupleExtract {
                result,
                operand,
                index,
            });
            result
        } else {
            // Borrow → extract (@guaranteed) → copy (@owned). Operand stays alive.
            let borrow = self.emit_begin_borrow(operand);
            let elem_ref = self.alloc_guaranteed(result_ty, borrow);
            self.push_inst(InstKind::TupleExtract {
                result: elem_ref,
                operand: borrow,
                index,
            });
            let result = self.emit_copy_value(elem_ref);
            self.emit_end_borrow(borrow);
            result
        }
    }

    /// Consume an @owned enum value, moving ALL payload fields of `variant` out
    /// as @owned results (one per `field_tys`). Used by match move-out so a
    /// non-Copyable payload is moved rather than illegally copied.
    pub fn emit_destructure_enum(
        &mut self,
        operand: ValueId,
        variant: VariantIdx,
        field_tys: &[TyId],
    ) -> Vec<ValueId> {
        let results: Vec<ValueId> = field_tys
            .iter()
            .map(|&ty| self.alloc_value(ty, Ownership::Owned))
            .collect();
        self.push_inst(InstKind::DestructureEnum {
            results: results.clone(),
            operand,
            variant,
        });
        self.consume(operand);
        for &r in &results {
            self.track_owned(r);
        }
        results
    }

    /// Consume an @owned struct value, moving ALL fields out as @owned results.
    pub fn emit_destructure_struct(
        &mut self,
        operand: ValueId,
        field_tys: &[TyId],
    ) -> Vec<ValueId> {
        let results: Vec<ValueId> = field_tys
            .iter()
            .map(|&ty| self.alloc_value(ty, Ownership::Owned))
            .collect();
        self.push_inst(InstKind::DestructureStruct {
            results: results.clone(),
            operand,
        });
        self.consume(operand);
        for &r in &results {
            self.track_owned(r);
        }
        results
    }

    /// Consume an @owned tuple value, moving ALL elements out as @owned results.
    pub fn emit_destructure_tuple(&mut self, operand: ValueId, elem_tys: &[TyId]) -> Vec<ValueId> {
        let results: Vec<ValueId> = elem_tys
            .iter()
            .map(|&ty| self.alloc_value(ty, Ownership::Owned))
            .collect();
        self.push_inst(InstKind::DestructureTuple {
            results: results.clone(),
            operand,
        });
        self.consume(operand);
        for &r in &results {
            self.track_owned(r);
        }
        results
    }

    pub fn emit_discriminant(&mut self, operand: ValueId) -> ValueId {
        let i32_ty = self.ctx.module.ty_arena.i32();
        let result = self.alloc_value(i32_ty, Ownership::Owned);
        self.push_inst(InstKind::Discriminant { result, operand });
        self.track_owned(result);
        result
    }

    pub fn emit_global_ref(&mut self, entity: Entity) -> ValueId {
        let i64_ty = self.ctx.module.ty_arena.i64();
        let result = self.alloc_value(i64_ty, Ownership::Owned);
        self.push_inst(InstKind::GlobalRef { result, entity });
        self.track_owned(result);
        result
    }

    pub fn emit_uninit(&mut self, ty: TyId) -> ValueId {
        let ptr_ty = self.ctx.module.ty_arena.pointer(ty);
        let result = self.alloc_value(ptr_ty, Ownership::Owned);
        self.push_inst(InstKind::Uninit { result, ty });
        self.track_owned(result);
        result
    }

    pub fn emit_field_addr(&mut self, base: ValueId, ty: TyId, field: FieldIdx) -> ValueId {
        // Result type is Pointer[field_type], not Pointer[struct_type].
        // The expand pass uses the pointer's pointee type to decide which
        // drop shim to call for StoreAssign; using the struct type would
        // destroy the whole struct starting at the field's address.
        let field_ty = if let MirTy::Named { entity, type_args } = self.ctx.module.ty_arena.get(ty)
        {
            let entity = *entity;
            let type_args = type_args.clone();
            if let Some(raw_ft) = self.ctx.resolve_field_ty(entity, field) {
                if type_args.is_empty() {
                    raw_ft
                } else {
                    let mut subst = kestrel_mir_3::SubstMap::new();
                    if let Some(def) = self.ctx.module.structs.get(&entity) {
                        for (tp, &ta) in def.type_params.iter().zip(type_args.iter()) {
                            subst.type_params.insert(tp.entity, ta);
                        }
                    }
                    kestrel_mir_3::substitute(&mut self.ctx.module.ty_arena, raw_ft, &subst)
                }
            } else {
                ty
            }
        } else {
            ty
        };
        let ptr_ty = self.ctx.module.ty_arena.pointer(field_ty);
        let result = self.alloc_value(ptr_ty, Ownership::Owned);
        self.push_inst(InstKind::FieldAddr {
            result,
            base,
            ty,
            field,
        });
        self.track_owned(result);
        result
    }

    pub fn emit_store_init(&mut self, address: ValueId, value: ValueId) {
        let value = if self.body.value(value).ownership == Ownership::Guaranteed {
            self.emit_copy_value(value)
        } else {
            value
        };
        self.push_inst(InstKind::StoreInit { address, value });
        self.consume(value);
    }

    /// Store a borrowed (`@guaranteed`) value's bits into freshly-`Uninit`
    /// storage without duplicating ownership — no `CopyValue`, no consume.
    /// Codegen lowers this to a bitwise `copy_aggregate`/store, so the slot
    /// ends up aliasing the borrow's underlying storage (for a non-copyable
    /// value, its `env_ptr`). Sound only when the destination never outlives
    /// the borrow and is never dropped — e.g. a non-escaping closure's
    /// capture env, which is caller-stack-allocated and not destroyed. Used to
    /// borrow-capture a called-not-stored closure (comparator/predicate)
    /// without the illegal copy of a non-Copyable `@thick` value.
    pub fn emit_store_init_borrowed(&mut self, address: ValueId, value: ValueId) {
        debug_assert_eq!(
            self.body.value(value).ownership,
            Ownership::Guaranteed,
            "emit_store_init_borrowed expects a @guaranteed value; use emit_store_init for @owned",
        );
        self.push_inst(InstKind::StoreInit { address, value });
    }

    pub fn emit_store_assign(&mut self, address: ValueId, value: ValueId) {
        let value = if self.body.value(value).ownership == Ownership::Guaranteed {
            self.emit_copy_value(value)
        } else {
            value
        };
        self.push_inst(InstKind::StoreAssign { address, value });
        self.consume(value);
    }

    pub fn emit_load(&mut self, address: ValueId, ty: TyId) -> ValueId {
        let result = self.alloc_value(ty, Ownership::Owned);
        self.push_inst(InstKind::Load { result, address });
        self.track_owned(result);
        result
    }

    pub fn emit_take(&mut self, address: ValueId, ty: TyId) -> ValueId {
        let result = self.alloc_value(ty, Ownership::Owned);
        self.push_inst(InstKind::Take {
            result,
            address,
            ty,
        });
        self.track_owned(result);
        result
    }

    pub fn emit_copy_addr(&mut self, address: ValueId, ty: TyId) -> ValueId {
        // Borrow the address, then CopyValue to get an @owned clone.
        // CopyValue goes through the expand pass → proper clone for Named
        // types (e.g. RcBox refcount bump), balancing the later DestroyValue.
        let borrow = self.emit_begin_borrow_addr(address, ty);
        let result = self.emit_copy_value(borrow);
        self.emit_end_borrow(borrow);
        result
    }

    pub fn emit_apply_partial(
        &mut self,
        callee: Callee,
        captures: Vec<ValueId>,
        result_ty: TyId,
    ) -> ValueId {
        let result = self.alloc_value(result_ty, Ownership::Owned);
        for &v in &captures {
            self.consume(v);
        }
        self.push_inst(InstKind::ApplyPartial {
            result,
            callee,
            captures,
        });
        self.track_owned(result);
        result
    }

    // ================================================================
    // Emit calls
    // ================================================================

    fn emit_call_inner(
        &mut self,
        callee: Callee,
        args: Vec<CallArg>,
        result_ty: Option<TyId>,
    ) -> Option<ValueId> {
        let result = result_ty.map(|ty| {
            let ownership = self.ownership_for(ty);
            self.alloc_value(ty, ownership)
        });
        let mut borrows: Vec<ValueId> = args
            .iter()
            .filter(|a| self.body.value(a.value).ownership == Ownership::Guaranteed)
            .map(|a| a.value)
            .collect();
        if let Some(cv) = callee.value() {
            if self.body.value(cv).ownership == Ownership::Guaranteed {
                borrows.push(cv);
            }
        }
        let consuming: Vec<ValueId> = args
            .iter()
            .filter(|a| a.convention == ParamConvention::Consuming)
            .map(|a| a.value)
            .collect();
        self.push_inst(InstKind::Call {
            result,
            callee,
            args,
        });
        for v in consuming {
            self.consume(v);
        }
        for borrow_val in borrows {
            self.emit_end_borrow(borrow_val);
        }
        self.drain_deferred_borrows();
        if let (Some(ty), Some(r)) = (result_ty, result) {
            if matches!(self.ctx.module.ty_arena.get(ty), MirTy::Never) {
                self.destroy_scopes_to_depth(0, &[]);
                self.set_terminator(TerminatorKind::Panic("noreturn".to_string()));
                return Some(r);
            }
            self.track_owned(r);
        }
        result
    }

    pub fn emit_call_returning(
        &mut self,
        callee: Callee,
        args: Vec<CallArg>,
        result_ty: TyId,
    ) -> ValueId {
        self.emit_call_inner(callee, args, Some(result_ty)).unwrap()
    }

    pub fn emit_call_void(&mut self, callee: Callee, args: Vec<CallArg>) {
        self.emit_call_inner(callee, args, None);
    }

    // ================================================================
    // Emit terminators
    // ================================================================

    pub fn set_terminator(&mut self, kind: TerminatorKind) {
        self.drain_deferred_borrows();
        // End any scope-tracked borrows before the terminator — borrows
        // can't cross block boundaries.
        let all_borrows: Vec<ValueId> = self
            .scope_stack
            .iter()
            .flat_map(|s| s.entries.iter())
            .filter_map(|e| match e {
                ScopeEntry::Borrow(v) => Some(*v),
                _ => None,
            })
            .collect();
        for scope in &mut self.scope_stack {
            scope
                .entries
                .retain(|e| !matches!(e, ScopeEntry::Borrow(_)));
        }
        for v in all_borrows {
            self.push_inst(InstKind::EndBorrow { operand: v });
        }
        let term = Terminator {
            kind,
            span: self.current_span.clone(),
        };
        if let Some(block_id) = self.current_block {
            self.body.block_mut(block_id).terminator = term;
        }
    }

    pub fn emit_ret(&mut self, value: ValueId) {
        self.set_terminator(TerminatorKind::Return(value));
    }

    pub fn emit_jump(&mut self, target: BlockId, args: Vec<ValueId>) {
        self.set_terminator(TerminatorKind::Jump { target, args });
    }

    pub fn emit_branch(
        &mut self,
        cond: ValueId,
        then_block: BlockId,
        then_args: Vec<ValueId>,
        else_block: BlockId,
        else_args: Vec<ValueId>,
    ) {
        self.set_terminator(TerminatorKind::Branch {
            condition: cond,
            then_block,
            then_args,
            else_block,
            else_args,
        });
    }

    pub fn emit_switch(&mut self, discriminant: ValueId, cases: Vec<SwitchArm>) {
        self.set_terminator(TerminatorKind::Switch {
            discriminant,
            cases,
        });
    }

    pub fn emit_panic(&mut self, msg: &str) {
        self.set_terminator(TerminatorKind::Panic(msg.to_string()));
    }

    // ================================================================
    // Var-local address access
    // ================================================================

    /// Walk a chain of HirExpr::Field nodes to find a root var local.
    /// If found, emit a chain of FieldAddr instructions and return the
    /// final address. Returns None if the root isn't addressable.
    pub fn try_field_addr_chain(&mut self, expr_id: HirExprId) -> Option<ValueId> {
        let expr = self.hir.exprs[expr_id].clone();
        match expr {
            kestrel_hir::body::HirExpr::Local(hir_local, _) => {
                match self.local_map.get(&hir_local).copied() {
                    Some(LocalBinding::Var(addr)) => Some(addr),
                    _ => None,
                }
            },
            kestrel_hir::body::HirExpr::Field { base, name, .. } => {
                let base_addr = self.try_field_addr_chain(base)?;
                let base_ty = self.resolve_expr_type(base);
                let field_name = name.as_str_or_empty();
                let struct_entity = match self.ctx.module.ty_arena.get(base_ty) {
                    MirTy::Named { entity, .. } => Some(*entity),
                    _ => None,
                };
                let field_idx =
                    struct_entity.and_then(|e| self.ctx.resolve_field_idx(e, field_name))?;
                Some(self.emit_field_addr(base_addr, base_ty, field_idx))
            },
            _ => None,
        }
    }

    /// If `expr_id` resolves to a var local (possibly through a field chain),
    /// return its address.
    pub fn try_var_addr(&mut self, expr_id: HirExprId) -> Option<ValueId> {
        self.try_field_addr_chain(expr_id)
    }

    /// Inside a closure body: if `expr_id` is a captured *projected* place
    /// (e.g. `self.cap`), return the env value loaded for it. Returns `None`
    /// when not in a closure, when the expr isn't a captured place, or for
    /// whole-local captures (those bind through `local_map`).
    pub(crate) fn captured_place_value(&self, expr_id: HirExprId) -> Option<ValueId> {
        if self.place_capture_map.is_empty() {
            return None;
        }
        let typed = self.typed.as_ref()?;
        let key =
            kestrel_type_infer::captures::place_key_of(&self.ctx.query, typed, &self.hir, expr_id)?;
        self.place_capture_map.get(&key).copied()
    }

    /// Resolve an expression to a value suitable for borrowing — returns the
    /// original value without copying. For non-local expressions, falls back
    /// to lower_expr (which may copy).
    pub fn lower_expr_for_borrow(&mut self, expr_id: HirExprId) -> ValueId {
        // A captured projected place reads its env value directly.
        if let Some(v) = self.captured_place_value(expr_id) {
            return v;
        }
        let expr = self.hir.exprs[expr_id].clone();
        match &expr {
            HirExpr::Local(hir_local, _) if !self.is_var_local(hir_local) => {
                self.map_local(*hir_local)
            },
            HirExpr::Local(hir_local, _) => {
                // var-local: borrow the address directly (Swift `load [borrow]`).
                // Falling through to lower_expr would `emit_copy_addr` — an
                // illegal copy for a non-Copyable var.
                let addr = self.map_local(*hir_local);
                let ty = self.resolve_local_type(*hir_local);
                self.emit_begin_borrow_addr(addr, ty)
            },
            _ => self.lower_expr(expr_id),
        }
    }

    /// Lower an expression for a consuming context — moves ownership
    /// instead of copying. For SSA locals, returns the value directly
    /// and consumes it from scope (no bitwise copy_value). Var locals
    /// and complex expressions fall back to lower_expr.
    pub fn lower_expr_for_consuming(&mut self, expr_id: HirExprId) -> ValueId {
        let expr = self.hir.exprs[expr_id].clone();
        match &expr {
            HirExpr::Local(hir_local, _) if !self.is_var_local(hir_local) => {
                let val = self.map_local(*hir_local);
                self.consume(val);
                val
            },
            _ => self.lower_expr(expr_id),
        }
    }

    // ================================================================
    // Value transfer: the OSSA copy/move decision
    // ================================================================

    /// Transfer a value for use — conservative: always copies @owned.
    /// The copy_optimize pass will eliminate unnecessary copies later.
    /// Transfer a value for use — copies @owned values.
    pub fn emit_value_use(&mut self, value: ValueId) -> ValueId {
        let ownership = self.body.value(value).ownership;
        match ownership {
            Ownership::Owned => self.emit_copy_value(value),
            Ownership::Guaranteed => value,
        }
    }

    /// Prepare a call argument from an HIR expression, respecting var locals.
    /// For MutBorrow on var locals / MutBorrow params, emits BeginMutBorrowAddr
    /// so mutations write through to the original storage.
    pub fn prepare_call_arg_for_expr(
        &mut self,
        expr_id: HirExprId,
        convention: ParamConvention,
    ) -> CallArg {
        if convention == ParamConvention::MutBorrow {
            if let Some(addr) = self.try_var_addr(expr_id) {
                let ty = self.resolve_expr_type(expr_id);
                let borrow = self.emit_begin_mut_borrow_addr(addr, ty);
                return CallArg {
                    value: borrow,
                    convention,
                };
            }
            // SSA owned receiver (e.g. a `consuming` func's `self`): borrow the
            // value in place via lower_expr_for_borrow. lower_expr would emit
            // copy_value here, stranding the mutation on the throwaway copy while
            // the loop-carried original never advances — the infinite loop in
            // Iterator.fold/reduce (`while let .Some = self.next()`).
            let val = self.lower_expr_for_borrow(expr_id);
            return self.prepare_call_arg(val, convention);
        }
        if convention == ParamConvention::Borrow {
            if let Some(addr) = self.try_var_addr(expr_id) {
                let ty = self.resolve_expr_type(expr_id);
                let borrow = self.emit_begin_borrow_addr(addr, ty);
                return CallArg {
                    value: borrow,
                    convention,
                };
            }
            // SSA / @guaranteed receiver (e.g. a closure param or an already-borrowed
            // value used as a borrowing method's receiver): borrow it in place.
            // lower_expr would emit a spurious copy_value, which for a Cloneable type
            // expands to a clone() — double-cloning the receiver and corrupting
            // @guaranteed aggregate values (e.g. `valuePtr().with { v in v.clone() }`).
            // Mirrors the MutBorrow path above.
            let val = self.lower_expr_for_borrow(expr_id);
            return self.prepare_call_arg(val, convention);
        }
        // Single-use SSA local with Consuming convention: move directly,
        // bypassing the emit_value_use copy. Only safe at function top-level
        // scope — loops re-execute the block, and conditional branches need
        // the value alive for the other arm's cleanup/forwarding.
        if convention == ParamConvention::Consuming && self.scope_stack.len() == 1 {
            let expr = self.hir.exprs[expr_id].clone();
            if let HirExpr::Local(hir_local, _) = &expr {
                if !self.is_var_local(hir_local) && self.is_single_use(*hir_local) {
                    let val = self.map_local(*hir_local);
                    if self.body.value(val).ownership == Ownership::Owned {
                        self.consume(val);
                        return CallArg {
                            value: val,
                            convention,
                        };
                    }
                }
            }
        }
        let val = self.lower_expr(expr_id);
        self.prepare_call_arg(val, convention)
    }

    /// Prepare a value for a call argument with a given convention.
    pub fn prepare_call_arg(&mut self, value: ValueId, convention: ParamConvention) -> CallArg {
        match convention {
            ParamConvention::Borrow => {
                let borrow = self.emit_begin_borrow(value);
                CallArg {
                    value: borrow,
                    convention,
                }
            },
            ParamConvention::MutBorrow => {
                let borrow = self.emit_begin_mut_borrow(value);
                CallArg {
                    value: borrow,
                    convention,
                }
            },
            ParamConvention::Consuming => {
                if self.body.value(value).ownership == Ownership::Owned {
                    self.consume(value);
                    CallArg { value, convention }
                } else {
                    let copy = self.emit_copy_value(value);
                    self.consume(copy);
                    CallArg {
                        value: copy,
                        convention,
                    }
                }
            },
        }
    }

    // ================================================================
    // Helpers
    // ================================================================

    pub fn resolve_type_args(&mut self, expr_id: HirExprId) -> Vec<TyId> {
        if let Some(typed) = self.typed.as_ref()
            && let Some(resolved_args) = typed.type_args.get(&expr_id)
        {
            return resolved_args
                .iter()
                .map(|ty| lower_resolved_ty(self.ctx, ty))
                .collect();
        }
        Vec::new()
    }

    pub fn prepend_receiver_type_args(
        &self,
        receiver_ty: TyId,
        method_args: Vec<TyId>,
    ) -> Vec<TyId> {
        let parent_args = match self.ctx.module.ty_arena.get(receiver_ty) {
            MirTy::Named { type_args, .. } => type_args.clone(),
            _ => Vec::new(),
        };
        if parent_args.is_empty() {
            return method_args;
        }
        let mut result = parent_args;
        result.extend(method_args);
        result
    }

    pub fn type_from_type_ref(&mut self, expr_id: HirExprId) -> TyId {
        use kestrel_hir::body::HirExpr;
        let expr = &self.hir.exprs[expr_id];
        if let HirExpr::Def(entity, hir_args, _) = expr {
            let args: Vec<TyId> = hir_args.iter().map(|a| lower_type(self.ctx, a)).collect();
            self.ctx.register_name(*entity);
            crate::ty::lower_named_type(self.ctx, *entity, args)
        } else {
            self.resolve_expr_type(expr_id)
        }
    }

    /// Lower a block of HIR statements + optional tail expression.
    pub fn lower_hir_block(&mut self, block: &HirBlock) -> ValueId {
        for &stmt_id in &block.stmts {
            self.lower_stmt(stmt_id);
            if self.is_terminated() {
                return self.emit_literal(Immediate::unit());
            }
        }
        if let Some(tail) = block.tail_expr {
            self.lower_expr(tail)
        } else {
            self.emit_literal(Immediate::unit())
        }
    }
}

/// Extract span from an HirExpr.
pub(crate) fn expr_span(hir: &HirBody, id: HirExprId) -> Span {
    match &hir.exprs[id] {
        kestrel_hir::body::HirExpr::Literal { span, .. }
        | kestrel_hir::body::HirExpr::Local(_, span)
        | kestrel_hir::body::HirExpr::Tuple { span, .. }
        | kestrel_hir::body::HirExpr::Field { span, .. }
        | kestrel_hir::body::HirExpr::TupleIndex { span, .. }
        | kestrel_hir::body::HirExpr::Def(_, _, span)
        | kestrel_hir::body::HirExpr::OverloadSet { span, .. }
        | kestrel_hir::body::HirExpr::ImplicitMember { span, .. }
        | kestrel_hir::body::HirExpr::Call { span, .. }
        | kestrel_hir::body::HirExpr::MethodCall { span, .. }
        | kestrel_hir::body::HirExpr::ProtocolCall { span, .. }
        | kestrel_hir::body::HirExpr::If { span, .. }
        | kestrel_hir::body::HirExpr::Loop { span, .. }
        | kestrel_hir::body::HirExpr::Break { span, .. }
        | kestrel_hir::body::HirExpr::Continue { span, .. }
        | kestrel_hir::body::HirExpr::Return { span, .. }
        | kestrel_hir::body::HirExpr::Assign { span, .. }
        | kestrel_hir::body::HirExpr::Match { span, .. }
        | kestrel_hir::body::HirExpr::Array { span, .. }
        | kestrel_hir::body::HirExpr::Dict { span, .. }
        | kestrel_hir::body::HirExpr::Closure { span, .. }
        | kestrel_hir::body::HirExpr::Block { span, .. }
        | kestrel_hir::body::HirExpr::Sugar { span, .. }
        | kestrel_hir::body::HirExpr::Error { span, .. } => span.clone(),
    }
}

/// Lower a function body to OSSA.
///
/// `hir_entity` is the entity whose HIR body is lowered (used for LowerBody/InferBody queries).
/// `func_entity` is the key in `module.functions` where the result is stored.
/// For normal functions these are the same; for static init thunks they differ
/// (the static's entity provides the body, the thunk's entity keys the function).
pub(crate) fn lower_function_body(ctx: &mut LowerCtx, hir_entity: Entity, func_entity: Entity) {
    use kestrel_hir_lower::LowerBody;
    use kestrel_name_res::ExtensionTargetEntity;
    use kestrel_type_infer::{ClosureCaptures, InferBody};

    let Some(hir) = ctx.query.query(LowerBody {
        entity: hir_entity,
        root: ctx.root,
    }) else {
        return;
    };

    let typed = ctx.query.query(InferBody {
        entity: hir_entity,
        root: ctx.root,
    });

    // Place-based closure capture plan (single source of truth — see
    // kestrel-type-infer/src/captures.rs). Consumed by lower_closure_expr.
    let captures = ctx.query.query(ClosureCaptures {
        entity: hir_entity,
        root: ctx.root,
    });

    let in_protocol_extension = ctx.world.parent_of(hir_entity).is_some_and(|parent| {
        matches!(
            ctx.world.get::<kestrel_ast_builder::NodeKind>(parent),
            Some(kestrel_ast_builder::NodeKind::Extension)
        ) && ctx
            .query
            .query(ExtensionTargetEntity {
                extension: parent,
                root: ctx.root,
            })
            .is_some_and(|target| {
                matches!(
                    ctx.world.get::<kestrel_ast_builder::NodeKind>(target),
                    Some(kestrel_ast_builder::NodeKind::Protocol)
                )
            })
    });

    let mut bctx = OssaBodyCtx::new(
        ctx,
        &hir,
        typed.as_ref(),
        captures,
        func_entity,
        in_protocol_extension,
    );
    bctx.lower_body();
    let ossa_body = bctx.body;

    let func = ctx.module.functions.get_mut(&func_entity).unwrap();
    for (pi, param) in func.params.iter_mut().enumerate() {
        param.value = ValueId::new(pi);
        if pi < ossa_body.values.len() {
            param.ty = ossa_body.values[pi].ty;
        }
    }
    func.body = Some(ossa_body);
}
