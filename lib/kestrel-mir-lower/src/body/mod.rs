pub mod call;
pub mod closure;
pub mod control;
pub mod expr;
pub mod literal;
pub mod pattern;
pub mod place;
pub mod stmt;

use std::collections::HashMap;
use std::sync::Arc;

use kestrel_hecs::Entity;
use kestrel_hir::body::{HirBlock, HirBody, HirExpr, HirExprId};
use kestrel_hir::res::LocalId as HirLocalId;
use kestrel_mir::block::BlockParam;
use kestrel_mir::body::OssaBody;
use kestrel_mir::callee::Callee;
use kestrel_mir::inst::{CallArg, InstKind, Instruction};
use kestrel_mir::terminator::{SwitchArm, Terminator, TerminatorKind};
use kestrel_mir::value::{Ownership, RootProvenance, ValueDef};
use kestrel_mir::{
    BlockId, CopyBehavior, FieldIdx, Immediate, MirTy, Op, ParamConvention, TyId, ValueId,
    VariantIdx,
};
use kestrel_reporting::{Diagnostic, Label};
use kestrel_span::Span;
use kestrel_type_infer::captures::{ClosureCaptureMap, PlaceKey};
use kestrel_type_infer::result::TypedBody;

use crate::context::LowerCtx;
use crate::ty::{lower_resolved_ty, lower_resolved_ty_preserving, lower_type};

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
    // Arc, not a deep copy: queries hand out shared HirBody allocations.
    Owned(Arc<HirBody>),
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
    // Arc, not a deep copy: queries hand out shared TypedBody allocations.
    Owned(Arc<TypedBody>),
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
        /// `true` for an inout-borrow slot (a `mutating self`/`mutating arg`
        /// param): the address is a @guaranteed pointer into the *caller's*
        /// storage, so it participates in init-state tracking (so whole-slot
        /// stores drop-vs-StoreInit correctly) but must NOT be DestroyAddr'd at
        /// scope exit — the caller owns and drops the value.
        borrowed: bool,
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
    /// Init-body `self`-field definite-init states (see `field_inits`). Saved/
    /// restored with the scope so each branch arm re-derives from the pre-branch
    /// state; the merge then joins the arms via `fold_field_inits`. Without this
    /// a then-arm's `DefInit` would leak into the else-arm and turn its first
    /// `store_init` into a `store_assign` over uninitialized memory (#154).
    pub field_inits: Vec<(FieldIdx, VarInit)>,
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
    #[allow(dead_code)]
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
    /// Static init-state of each tracked `self` field (init bodies only) at this
    /// arm's exit, joined by `fold_field_inits` at the merge (#154).
    pub field_inits: Vec<(FieldIdx, VarInit)>,
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
    /// `lower_closure_expr`. Arc-shared with the query memo cache.
    pub(crate) captures: Arc<ClosureCaptureMap>,
    /// Inside a closure body: the env value loaded for each captured *place*
    /// (e.g. `self.cap`). Consulted when lowering reads/borrows so projected
    /// captures read the env value instead of projecting from a (non-captured)
    /// receiver. Whole-local captures use `local_map` instead. Saved/restored
    /// across nested closure bodies like `local_map`.
    pub(crate) place_capture_map: HashMap<PlaceKey, ValueId>,
    /// Per-droppable-`self`-field drop-flag tracking in an init body: one entry
    /// per droppable stored field, as `(field index, substituted field type,
    /// drop-flag pointer)`. Populated for EVERY init with droppable fields (see
    /// `setup_init_field_flags`). The flag is set `true` when `self.f = v` runs;
    /// it is consulted both to flag-guard-drop the field at a failable-init
    /// failure `return` AND to drop the old value on a `MaybeUninit`
    /// reassignment. Empty in non-init bodies.
    pub(crate) init_field_flags: Vec<(FieldIdx, TyId, ValueId)>,
    /// Compile-time definite-initialization state of each droppable `self` field
    /// in an init body (the `VarInit` lattice, mirroring `var` slots): `DefUninit`
    /// until first assigned, `DefInit` after a definite assignment, `MaybeUninit`
    /// where reaching edges disagree (joined by `fold_field_inits` at merges).
    /// Drives the `self.f = v` store: `DefUninit` → `store_init`; `DefInit` →
    /// `store_assign` (drops the old value, #154); `MaybeUninit` → flag-guarded
    /// drop + `store_init`. Empty outside init bodies / for non-droppable fields.
    pub(crate) field_inits: Vec<(FieldIdx, VarInit)>,
    /// Maps an original ValueId to its current SSA representative after a
    /// block-boundary rebind. A call-argument value materialized *before* a
    /// control-flow sibling arg (`if`/`try`/`match`) is threaded through the
    /// new blocks and renamed at the merge; the held `CallArg` still names the
    /// pre-merge value. `emit_call_inner` resolves each arg (and indirect
    /// callee) value through this map so the emitted `Call` references the
    /// merged value rather than one stranded at a predecessor block's exit.
    /// Single source of truth: every cross-block rename funnels through
    /// `rebind_scope_values`, which records the mapping here.
    pub(crate) value_forwarding: HashMap<ValueId, ValueId>,
    /// Whether this body returns a borrowed reference (`-> &T`, the
    /// ret_borrow ABI). Gates `prepare_return_value` and the
    /// `set_terminator` carve-out. Saved/restored across closure bodies
    /// (closures can never ret_borrow — E491 rejects the function type).
    pub(crate) ret_borrow: bool,
    /// @guaranteed results of ret_borrow calls in this body. A ref is
    /// intra-block in stage 1: force-ending one at a non-Return terminator
    /// is the E497 "ref across a control-flow merge" error, not a silent
    /// EndBorrow. Per-body value ids — saved/restored across closures.
    pub(crate) ref_results: std::collections::HashSet<ValueId>,
    /// Named ref bindings (`let r = &expr;`, stage 1.5 item 2): the
    /// binding's @guaranteed value → its HIR local. Members are ALSO in
    /// `ref_results`; this map marks them MULTI-USE — value-context reads
    /// copy WITHOUT ending the borrow (`end_ref_if_single_use`), statement
    /// sweeps spare them, and lexical scope exit / terminators own their
    /// end. Saved/restored across closures.
    pub(crate) ref_binding_vals: HashMap<ValueId, HirLocalId>,
    /// Remaining HIR reads per ref-binding local — decremented once per
    /// read expr (`note_binding_read`). A binding live at an inside-fn
    /// terminator with remaining > 0 is the E497 binding error (bindings
    /// never cross blocks); remaining == 0 ends silently.
    pub(crate) ref_binding_remaining: HashMap<HirLocalId, usize>,
    /// Read exprs already counted against `ref_binding_remaining` — a
    /// re-lowered expr (desugar duplication) must not double-decrement.
    ref_binding_reads: std::collections::HashSet<kestrel_hir::body::HirExprId>,
    /// Stage 1.5 get→op→set writebacks pending after the owning call.
    /// Pushed by `try_lower_accessor_place_mut`'s fallback (the receiver of
    /// a mutating operation is a get/set member: the element was copied out
    /// through `get` into a temp slot); drained WATERMARK-SCOPED by the call
    /// emitters right after their call, so a nested call in a sibling
    /// argument can't steal an outer writeback. A statement-boundary
    /// drain(0) is the safety net — a dropped writeback is a lost write.
    pub(crate) pending_writebacks: Vec<PendingWriteback>,
    /// Derived address → its storage anchor (the chain's base: a param
    /// address or var slot). A `FieldAddr` result is an owned TEMP that is
    /// destroyed at scope exit; a borrow through it semantically borrows the
    /// underlying STORAGE, so `emit_begin_(mut_)borrow_addr` records the
    /// anchor as `borrow_source` — otherwise destroying the temp while a
    /// returned borrow still chains to it is a verify consume-while-borrowed
    /// ICE (and E498 would blame the temp instead of the real storage).
    pub(crate) addr_anchors: HashMap<ValueId, ValueId>,
    /// Active while inline-lowering a default-argument expression (#148): maps
    /// the callee's type params to the call site's concrete type args. The
    /// default body was inferred against the callee's *generic* params, so every
    /// type it produces (`resolve_expr_type`/`resolve_type_args`/
    /// `resolve_local_type`) is substituted through this map — otherwise the
    /// callee's `TypeParam` leaks into the (possibly non-generic) caller's body
    /// and survives to the mangler. `None` outside default-arg lowering.
    pub(crate) default_arg_subst: Option<kestrel_mir::SubstMap>,
}

/// One deferred get→op→set writeback (see `pending_writebacks`).
pub(crate) struct PendingWriteback {
    /// The member's setter child (concrete) — witness-dispatched when the
    /// member is a protocol-extension subscript (e.g. Slice's).
    setter: Entity,
    /// Receiver type — witness `self_type` / direct-callee prepend.
    receiver_ty: TyId,
    /// Raw method type args (un-prepended).
    type_args: Vec<TyId>,
    /// The receiver PLACE, evaluated once (@guaranteed mut place). The get
    /// call used a sub-borrow of it; the setter consumes it as its MutBorrow
    /// receiver.
    recv_place: ValueId,
    /// Index values, lowered once; re-borrowed per call.
    index_vals: Vec<ValueId>,
    /// Temp slot holding the element during the mutating operation.
    slot_addr: ValueId,
    elem_ty: TyId,
}

impl<'a, 'w> OssaBodyCtx<'a, 'w> {
    pub fn new(
        ctx: &'a mut LowerCtx<'w>,
        hir: &'a HirBody,
        typed: Option<&'a TypedBody>,
        captures: Arc<ClosureCaptureMap>,
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
            init_field_flags: Vec::new(),
            field_inits: Vec::new(),
            value_forwarding: HashMap::new(),
            ret_borrow: false,
            ref_results: std::collections::HashSet::new(),
            ref_binding_vals: HashMap::new(),
            ref_binding_remaining: HashMap::new(),
            ref_binding_reads: std::collections::HashSet::new(),
            pending_writebacks: Vec::new(),
            addr_anchors: HashMap::new(),
            default_arg_subst: None,
        }
    }

    /// Apply the active default-argument type substitution (#148) to a lowered
    /// type, if any. A no-op outside default-arg inline lowering.
    fn subst_default_arg_ty(&mut self, ty: TyId) -> TyId {
        let Some(subst) = self.default_arg_subst.clone() else {
            return ty;
        };
        if subst.type_params.is_empty() {
            return ty;
        }
        kestrel_mir::substitute(&mut self.ctx.module.ty_arena, ty, &subst)
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
        let param_sig_tys: Vec<TyId> = self
            .ctx
            .module
            .functions
            .get(&self.func_entity)
            .map(|f| f.params.iter().map(|p| p.ty).collect())
            .unwrap_or_default();
        self.ret_borrow = self
            .ctx
            .module
            .functions
            .get(&self.func_entity)
            .is_some_and(|f| {
                matches!(
                    kestrel_mir::item::function::ret_convention(&self.ctx.module.ty_arena, f.ret),
                    kestrel_mir::item::function::RetConvention::RefBorrow { .. }
                )
            });
        let is_init_body = self
            .ctx
            .module
            .functions
            .get(&self.func_entity)
            .map(|f| {
                matches!(
                    f.kind,
                    kestrel_mir::item::function::FunctionKind::Initializer { .. }
                )
            })
            .unwrap_or(false);
        let mut pending_ref_param_views: Vec<(ValueId, TyId, HirLocalId)> = Vec::new();
        for (i, (hir_id, local)) in locals.iter().enumerate() {
            if i >= params_len {
                break;
            }
            let ty = self.resolve_local_type(*hir_id);
            let convention = param_conventions
                .get(i)
                .copied()
                .unwrap_or(ParamConvention::Borrow);
            // All params stamp Param(i) — Consuming included, so a borrow of
            // a consuming param carries it to the escape check (E496 there).
            let root = RootProvenance::Param(i as u32);
            let val = match convention {
                ParamConvention::MutBorrow => {
                    let val = self.body.alloc_value(ValueDef {
                        ty,
                        ownership: Ownership::Guaranteed,
                        borrow_source: None,
                        root,
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
                    } else {
                        // Enroll the inout-borrow slot in the SAME init-state
                        // machinery a regular `var` uses, so a whole-slot store
                        // (`self = new`) consults its tracked init state: drop
                        // the old value when the slot is live (DefInit), but
                        // StoreInit (no drop) after a move-out (`let old = self;
                        // self = n` — the Optional.take/replace shape). Without
                        // this the slot is untracked, `var_init` returns `None`,
                        // and the store always took the drop arm — double-freeing
                        // the moved-out slot (#141). NOT enrolled for init bodies:
                        // their self is uninitialized and driven by BodyContext.
                        self.track_borrowed_var(val, ty, *hir_id);
                    }
                    val
                },
                ParamConvention::Borrow => {
                    // A REF-typed SIGNATURE param (`extend &T` methods'
                    // self/`other: Self`): the ABI value is the ref SLOT
                    // (resolve_local_type peels to the pointee — the expr
                    // seam — so it can't type this). Allocate the param at
                    // its signature type; the entry peel into the let-ref
                    // VIEW representation is deferred below the loop — the
                    // first `params_len` ValueIds must be exactly the params.
                    let sig_ref = param_sig_tys.get(i).and_then(|&t| {
                        match self.ctx.module.ty_arena.get(t) {
                            MirTy::Ref { pointee, .. } => Some((t, *pointee)),
                            _ => None,
                        }
                    });
                    if let Some((ref_ty, pointee)) = sig_ref {
                        let val = self.body.alloc_value(ValueDef {
                            ty: ref_ty,
                            ownership: Ownership::Guaranteed,
                            borrow_source: None,
                            root,
                            span: None,
                        });
                        pending_ref_param_views.push((val, pointee, *hir_id));
                        val
                    } else {
                        let val = self.body.alloc_value(ValueDef {
                            ty,
                            ownership: Ownership::Guaranteed,
                            borrow_source: None,
                            root,
                            span: None,
                        });
                        self.local_map.insert(*hir_id, LocalBinding::Ssa(val));
                        val
                    }
                },
                ParamConvention::Consuming => {
                    let ownership = self.ownership_for(ty);
                    let val = self.alloc_value(ty, ownership);
                    self.body.values[val.index()].root = root;
                    self.local_map.insert(*hir_id, LocalBinding::Ssa(val));
                    self.track_owned(val);
                    val
                },
            };
            self.body.value_names.insert(val, local.name.clone());
        }
        self.body.param_count = params_len;

        // Entry peels for ref-typed params: one fused begin_borrow each
        // (codegen loads the stored address out of the param's ref slot),
        // registered as named ref bindings so all existing let-ref use
        // paths (receiver View, arg decay, packaging) apply unchanged.
        for (param_val, pointee, hir_id) in pending_ref_param_views {
            let view = self.extract_ref_slot(param_val, pointee, |s, result, operand| {
                s.push_inst(InstKind::BeginBorrow { result, operand });
            });
            self.register_ref_binding(view, hir_id);
        }

        // Failable-init partial drop: allocate drop flags for `self`'s droppable
        // stored fields in the entry block (so they dominate every failure
        // `return`). Must run after the param loop established `self_addr`.
        self.setup_init_field_flags();

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
                // ret_borrow returns a PLACE: lower for borrow so projections
                // stay @guaranteed and carry their provenance root — the
                // owned-copy path would re-root at a fresh Local and always
                // escape-fail (E494 instead of the precise E496 etc.).
                let value = if self.ret_borrow {
                    self.lower_expr_for_ref_return(tail)
                } else {
                    self.lower_expr_for_return(tail)
                };
                if !self.is_terminated() {
                    let value = self.prepare_return_value(value);

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
            let val = match ownership {
                Ownership::Owned => self.alloc_value(ty, ownership),
                // A THREADED binding borrow continuing across the block
                // boundary (verify Check 4's forwarded form). borrow_source
                // and the provenance root are stamped from the forwarded
                // value when the block is entered (rebind_scope_values).
                Ownership::Guaranteed => self.body.alloc_value(
                    ValueDef {
                        ty,
                        ownership,
                        borrow_source: None,
                        root: RootProvenance::derived(),
                        span: None,
                    }
                    .with_span(self.current_span.clone()),
                ),
            };
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
            let ty = lower_resolved_ty(self.ctx, resolved);
            return self.subst_default_arg_ty(ty);
        }
        self.ctx.module.ty_arena.error()
    }

    pub fn resolve_local_type(&mut self, hir_id: HirLocalId) -> TyId {
        if let Some(typed) = self.typed.as_ref()
            && let Some(resolved) = typed.local_types.get(&hir_id)
        {
            let ty = lower_resolved_ty(self.ctx, resolved);
            return self.subst_default_arg_ty(ty);
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

    /// Total HIR read count for a local (0 if never read). Seeds a ref
    /// binding's remaining-use budget.
    pub(crate) fn local_use_count(&self, hir_id: HirLocalId) -> usize {
        self.local_use_counts.get(&hir_id).copied().unwrap_or(0)
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

    /// Normalize a whole-slot var address to the canonical `Pointer[T]` form.
    ///
    /// A regular `var` slot's address is already `Pointer[T]` @owned. A
    /// `mutating self`/`mutating arg` (MutBorrow) param, however, is bound as a
    /// `LocalBinding::Var` whose value has type `T` @guaranteed (the inout
    /// pointer presented as a value of the pointee type). Whole-slot reads
    /// (`Take`) and stores (`StoreInit`/`StoreAssign`) need a `Pointer[T]`
    /// address: codegen's `resolve_scalar` would otherwise *load through* a
    /// @guaranteed scalar-repr value, treating the inout pointer as a
    /// pointer-to-the-address and yielding the pointee bits instead of the
    /// address (SIGSEGV / LLVM "expected PointerValue"). Materialise `Pointer[T]`
    /// via `PtrTo` so both backends see the canonical form. Identity for an
    /// already-@owned `Pointer[T]` slot. Single source of truth for the
    /// inout-self address seam — used by every whole-self read/store site.
    pub fn whole_slot_addr(&mut self, addr: ValueId) -> ValueId {
        if self.body.value(addr).ownership != Ownership::Guaranteed {
            return addr;
        }
        let pointee_ty = self.body.value(addr).ty;
        let ptr_ty = self.ctx.module.ty_arena.pointer(pointee_ty);
        self.emit_op1(Op::PtrTo(pointee_ty), addr, ptr_ty)
    }

    fn copy_behavior_of(&self, ty: TyId) -> CopyBehavior {
        let wc = self
            .ctx
            .module
            .functions
            .get(&self.func_entity)
            .and_then(|f| f.where_clause.as_ref());
        kestrel_mir::ty_query::copy_behavior(&self.ctx.module.ty_arena, &self.ctx.module, ty, wc)
    }

    #[allow(dead_code)]
    pub fn is_copy_type(&self, ty: TyId) -> bool {
        matches!(self.copy_behavior_of(ty), CopyBehavior::Bitwise)
    }

    /// A `not Copyable` type has no clone shim — duplicating it is illegal, so
    /// an @owned transfer of such a value must be a move, never a copy.
    pub fn is_non_copyable(&self, ty: TyId) -> bool {
        matches!(self.copy_behavior_of(ty), CopyBehavior::None)
    }

    /// True when a type's copy behavior is only knowable *after*
    /// monomorphization — a bare type parameter, an associated projection, or a
    /// conditionally-Copyable container gated on one of those. Pre-mono these
    /// can report `Bitwise`, but they may resolve to a `not Copyable` type, at
    /// which point a bitwise copy is an illegal alias. Such an @owned value must
    /// be moved (consumed), never copied. See `ty_query::copy_is_mono_dependent`.
    pub fn copy_behavior_is_mono_dependent(&self, ty: TyId) -> bool {
        let wc = self
            .ctx
            .module
            .functions
            .get(&self.func_entity)
            .and_then(|f| f.where_clause.as_ref());
        kestrel_mir::ty_query::copy_is_mono_dependent(
            &self.ctx.module.ty_arena,
            &self.ctx.module,
            ty,
            wc,
        )
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

    /// A @guaranteed value with NO borrow_source and an explicit provenance
    /// root — the shape of a ref extracted from a ref-bearing aggregate
    /// (stage 2b): consuming the aggregate does not invalidate the ref (the
    /// pointer was loaded out, it borrows nothing), but the ref still
    /// carries the aggregate's escape provenance.
    pub fn alloc_guaranteed_rooted(&mut self, ty: TyId, root: RootProvenance) -> ValueId {
        self.body.alloc_value(
            ValueDef {
                ty,
                ownership: Ownership::Guaranteed,
                borrow_source: None,
                root,
                span: None,
            }
            .with_span(self.current_span.clone()),
        )
    }

    /// Shared shape of a ref-slot extraction (struct field / tuple element /
    /// enum payload whose declared slot type is `&T`): the instruction LOADS
    /// the stored address; the result is the ref itself — @guaranteed
    /// pointee-typed, rooted at the aggregate's root. An @owned aggregate is
    /// viewed through a transient borrow (ended immediately: the result
    /// doesn't depend on it). The result registers like a ref-returning call
    /// result (per-use decay; named-binding registration is the caller's).
    pub(crate) fn extract_ref_slot(
        &mut self,
        operand: ValueId,
        pointee: TyId,
        push: impl FnOnce(&mut Self, ValueId, ValueId),
    ) -> ValueId {
        let agg_root = self.body.value(operand).root;
        let owned_op = self.body.value(operand).ownership == Ownership::Owned;
        let view = if owned_op {
            self.emit_begin_borrow(operand)
        } else {
            operand
        };
        let result = self.alloc_guaranteed_rooted(pointee, agg_root);
        push(self, result, view);
        if owned_op {
            self.emit_end_borrow(view);
        }
        self.ref_results.insert(result);
        self.track_borrow(result);
        result
    }

    /// Register an extracted ref value as a NAMED ref binding (multi-use,
    /// scope-tracked, threads through control flow). The pattern-binding
    /// analog of `lower_borrow_init`'s registration.
    pub(crate) fn register_ref_binding(&mut self, v: ValueId, local: HirLocalId) {
        self.ref_binding_vals.insert(v, local);
        let uses = self.local_use_count(local);
        self.ref_binding_remaining.insert(local, uses);
        let name = self.hir.locals[local].name.clone();
        self.body.value_names.insert(v, name);
        self.local_map.insert(local, LocalBinding::Ssa(v));
    }

    /// The CURRENT function's entry-param conventions — `RootProvenance::join`
    /// ranks `Param(i)` roots by convention (roots are caller-frame
    /// provenance). Empty when the def isn't lowered yet (join then ranks
    /// params conservatively).
    pub(crate) fn current_param_convs(&self) -> Vec<ParamConvention> {
        self.ctx
            .module
            .functions
            .get(&self.func_entity)
            .map(|f| f.params.iter().map(|p| p.convention).collect())
            .unwrap_or_default()
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
        self.track_var_inner(address, content_ty, local, flag, false);
    }

    /// Track an inout-borrow slot (`mutating self`/`mutating arg` param): it
    /// participates in init-state tracking like a normal `var` (so whole-slot
    /// stores correctly choose drop-vs-StoreInit after a move-out) but is never
    /// DestroyAddr'd at scope exit — the caller owns the storage.
    pub fn track_borrowed_var(&mut self, address: ValueId, content_ty: TyId, local: HirLocalId) {
        self.track_var_inner(address, content_ty, Some(local), None, true);
    }

    fn track_var_inner(
        &mut self,
        address: ValueId,
        content_ty: TyId,
        local: Option<HirLocalId>,
        flag: Option<ValueId>,
        borrowed: bool,
    ) {
        if let Some(frame) = self.scope_stack.last_mut() {
            frame.entries.push(ScopeEntry::Var {
                addr: address,
                ty: content_ty,
                init: VarInit::DefInit,
                flag,
                local,
                borrowed,
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
                    && *l == local
                {
                    return Some(*init);
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
                    && *l == local
                {
                    *init = new_init;
                    return;
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
        self.alloc_drop_flag(true)
    }

    /// Allocate an in-memory Bool drop flag initialized to `initial` (`true` =
    /// slot owns a value). Like [`Self::alloc_var_flag`] but lets the caller pick
    /// the initial state — failable-init field flags start `false` (the field is
    /// uninitialized until `self.f = v` runs and stores `true`).
    pub fn alloc_drop_flag(&mut self, initial: bool) -> ValueId {
        let bty = self.bool_ty();
        let flag = self.emit_uninit(bty);
        let init = self.emit_literal(Immediate::bool(initial));
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

    // ----------------------------------------------------------------
    // Failable-init partial drop (drop already-initialized self fields
    // on a `return null` / `throw` exit). See `init_field_flags`.
    // ----------------------------------------------------------------

    /// In an init body, allocate a `false`-initialized drop flag in the entry
    /// block for each droppable stored field of `self`, record
    /// `(field_idx, substituted field type, flag)` in `init_field_flags`, and
    /// seed its `field_inits` state to `DefUninit`. The flag serves two roles:
    /// flag-guarded partial-drop at a failable-init failure `return`, and
    /// dropping the old value on a `MaybeUninit` reassignment (#154). No-op for
    /// every non-init body (no `init_self_addr`), so both vecs stay empty.
    fn setup_init_field_flags(&mut self) {
        let Some(self_addr) = self.body_context.init_self_addr() else {
            return;
        };
        let self_ty = self.body.value(self_addr).ty;
        let (entity, type_args) = match self.ctx.module.ty_arena.get(self_ty) {
            MirTy::Named { entity, type_args } => (*entity, type_args.clone()),
            _ => return,
        };
        // Snapshot (field_idx, raw field type) + generic params, then release the
        // module borrow before mutating the arena (substitute) / allocating flags.
        let (raw_fields, type_params): (Vec<(FieldIdx, TyId)>, Vec<Entity>) = {
            let Some(def) = self.ctx.module.structs.get(&entity) else {
                return;
            };
            let fields = (0..def.fields.len())
                .map(FieldIdx::new)
                .filter_map(|idx| self.ctx.resolve_field_ty(entity, idx).map(|ft| (idx, ft)))
                .collect();
            let params = def.type_params.iter().map(|tp| tp.entity).collect();
            (fields, params)
        };
        for (field_idx, raw_ft) in raw_fields {
            // Substitute generic params (identity for non-generic self), exactly
            // as `emit_field_addr` does, so the recorded type matches the
            // `field_addr`'s pointee at the failure-return site.
            let field_ty = if type_args.is_empty() {
                raw_ft
            } else {
                let mut subst = kestrel_mir::SubstMap::new();
                for (&tp, &ta) in type_params.iter().zip(type_args.iter()) {
                    subst.type_params.insert(tp, ta);
                }
                kestrel_mir::substitute(&mut self.ctx.module.ty_arena, raw_ft, &subst)
            };
            // `needs_drop` reads `type_info.drop`, which only reflects user
            // `deinit`s pre-`drop_fix`; OR in `is_non_copyable` to also catch
            // structs droppable purely via a non-Copyable field. A trivial
            // `destroy_addr` (e.g. an Int64 field) would no-op in expand anyway,
            // but skipping them avoids a useless flag + guard diamond.
            let droppable = kestrel_mir::ty_query::needs_drop(
                &self.ctx.module.ty_arena,
                &self.ctx.module,
                field_ty,
            ) || self.is_non_copyable(field_ty);
            if droppable {
                let flag = self.alloc_drop_flag(false);
                self.init_field_flags.push((field_idx, field_ty, flag));
                self.field_inits.push((field_idx, VarInit::DefUninit));
            }
        }
    }

    /// Compile-time init state of a droppable `self` field in an init body, or
    /// `None` for a non-droppable / untracked field (→ plain `store_init`).
    pub fn field_init(&self, field: FieldIdx) -> Option<VarInit> {
        self.field_inits
            .iter()
            .find(|(idx, _)| *idx == field)
            .map(|(_, state)| *state)
    }

    /// Set the compile-time init state of a tracked `self` field (no-op if the
    /// field isn't tracked).
    pub fn set_field_init(&mut self, field: FieldIdx, new_state: VarInit) {
        if let Some((_, state)) = self.field_inits.iter_mut().find(|(idx, _)| *idx == field) {
            *state = new_state;
        }
    }

    /// Snapshot the current `field_inits` (for save/restore around loops).
    pub fn snapshot_field_inits(&self) -> Vec<(FieldIdx, VarInit)> {
        self.field_inits.clone()
    }

    /// Store `rhs` into a `self` field inside an init body, dropping the old
    /// value when the field is already initialized (definite-initialization
    /// driven, mirroring `var`-slot assignment — #154):
    /// - untracked / non-droppable → plain `store_init`;
    /// - `DefUninit` → `store_init` (first assignment, nothing to drop);
    /// - `DefInit` → `store_assign` (straight-line reassignment drops the old
    ///   value via expand — no runtime flag needed);
    /// - `MaybeUninit` → flag-guarded drop + `store_init` (reaching edges
    ///   disagree, or inside a loop where the store re-executes).
    /// Afterwards the field is `DefInit` and its drop flag is set.
    fn store_init_self_field(&mut self, field_idx: FieldIdx, field_addr: ValueId, rhs: ValueId) {
        let Some(state) = self.field_init(field_idx) else {
            // Non-droppable / untracked field: no prior value to drop.
            self.emit_store_init(field_addr, rhs);
            return;
        };
        // Inside a loop, a `DefUninit` field's store re-executes on later
        // iterations where the field IS initialized — promote to the
        // flag-guarded path so the prior iteration's value is dropped, sound
        // regardless of trip count. `DefInit`/`MaybeUninit` already drop.
        let effective = if state == VarInit::DefUninit && !self.loop_stack.is_empty() {
            VarInit::MaybeUninit
        } else {
            state
        };
        match effective {
            VarInit::DefUninit => self.emit_store_init(field_addr, rhs),
            VarInit::DefInit => self.emit_store_assign(field_addr, rhs),
            VarInit::MaybeUninit => {
                let (field_ty, flag) = self
                    .init_field_flags
                    .iter()
                    .find(|(idx, _, _)| *idx == field_idx)
                    .map(|(_, ty, f)| (*ty, *f))
                    .expect("tracked field has a drop flag");
                // Guard threads `rhs` + `field_addr` through the diamond.
                let remapped = self.emit_guarded_destroy(flag, field_addr, field_ty, &[rhs, field_addr]);
                self.emit_store_init(remapped[1], remapped[0]);
            },
        }
        self.set_field_init(field_idx, VarInit::DefInit);
        if let Some(flag) = self.init_field_flag(field_idx) {
            self.store_drop_flag(flag, true);
        }
    }

    /// The drop flag for a stored `self` field, if this is a failable init and
    /// the field is droppable. Used by field-assign to mark the field live.
    pub fn init_field_flag(&self, field: FieldIdx) -> Option<ValueId> {
        self.init_field_flags
            .iter()
            .find(|(idx, _, _)| *idx == field)
            .map(|(_, _, flag)| *flag)
    }

    /// Is the current body a failable/throwing init (one that can fail partway
    /// and abandon a partially-initialized `self`)? Only such inits run the
    /// failure-return partial-drop. Distinguishes them from plain inits now that
    /// `init_field_flags` is populated for ALL inits (reassignment tracking) —
    /// without this gate a plain init's `return ()` would be misclassified as a
    /// failure and wrongly drop its initialized fields (double-free).
    pub fn is_failable_init(&self) -> bool {
        self.ctx
            .world
            .get::<kestrel_ast_builder::InitEffect>(self.func_entity)
            .is_some()
    }

    /// Classify a failable-init `return` value as a FAILURE exit (must drop the
    /// initialized fields) vs an early SUCCESS exit. An explicit/bare success
    /// `return` in an effectful init is lowered to `.Some(())` / `.Ok(())`
    /// (`ImplicitMember`); failure is anything else (`return null`, `throw` →
    /// `.Err`, `try` → `fromResidual`). A void return (`None`) is not a failure.
    pub fn is_init_failure_return(&self, value: &Option<HirExprId>) -> bool {
        match value {
            Some(v) => !matches!(
                &self.hir.exprs[*v],
                HirExpr::ImplicitMember { name, .. }
                    if matches!(name.as_str_or_empty(), "Some" | "Ok")
            ),
            None => false,
        }
    }

    /// At a failure `return` in a failable init, flag-guard-drop each
    /// initialized `self` field (reverse declaration order). Threads `ret_val`
    /// through the guard diamonds; returns its continuation-block value.
    pub fn emit_init_partial_drops(&mut self, mut ret_val: ValueId) -> ValueId {
        let Some(self_addr) = self.body_context.init_self_addr() else {
            return ret_val;
        };
        let self_ty = self.body.value(self_addr).ty;
        for (field_idx, field_ty, flag) in self.init_field_flags.clone().into_iter().rev() {
            let field_addr = self.emit_field_addr(self_addr, self_ty, field_idx);
            let remapped = self.emit_guarded_destroy(flag, field_addr, field_ty, &[ret_val]);
            ret_val = remapped[0];
        }
        ret_val
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
        self.emit_branch(cond, then_block, live.clone(), else_block, live.clone());

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
                    && *l == local
                {
                    return *flag;
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
            // Snapshot in reverse declaration order before mutating the scope, so
            // owned temporaries and scope-local vars are dropped innermost-first.
            let entries: Vec<ScopeEntry> = scope.entries.iter().rev().cloned().collect();
            scope.entries.retain(|e| match e {
                ScopeEntry::Owned(v) => keep.contains(v),
                ScopeEntry::Var { .. } => true,
                ScopeEntry::Borrow(_) => false,
            });
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
                    // A `var` declared in this scope (an if/match arm, loop body, or
                    // closure body) that wasn't moved out must be destroyed here:
                    // this is the only cleanup the normal arm-exit / loop-back-edge
                    // fallthrough runs (terminating exits go through
                    // `destroy_scopes_to_depth`). DefInit → drop. DefUninit → moved
                    // out, nothing to drop. MaybeUninit (conditional move) → skip:
                    // an unconditional DestroyAddr would double-free on the moved
                    // path; the flag-guarded destroy is deferred, so this stays a
                    // possible leak — never a double-free.
                    ScopeEntry::Var {
                        init: VarInit::DefInit,
                        addr,
                        ty,
                        borrowed: false,
                        ..
                    } => {
                        self.push_inst(InstKind::DestroyAddr {
                            address: *addr,
                            ty: *ty,
                        });
                    },
                    _ => {},
                }
            }
        }
    }

    pub fn destroy_scopes_to_depth(&mut self, target_depth: usize, keep: &[ValueId]) {
        let entries: Vec<ScopeEntry> = self.scope_stack[target_depth..]
            .iter()
            .rev()
            .flat_map(|scope| scope.entries.iter().rev().cloned())
            .collect();
        // End borrows first — they may reference values we're about to
        // destroy. `emit_end_borrow` also removes the scope entry, so the
        // terminator that follows every destroy site doesn't end the same
        // borrow a second time. References are deliberately left tracked:
        // `set_terminator` owns their endgame (the E497 check on inside-fn
        // jumps and the ret_borrow return carve-out) — EXCEPT named binding
        // borrows at a FUNCTION exit (depth 0): their lexical scope ends
        // right here, and the end must precede the destroys below (the
        // borrowed var slot dies in the same exit; a slot consume under an
        // open borrow is the verify error the machinery exists to catch).
        // `keep` exempts a returned borrow (ret_borrow of the binding).
        for entry in &entries {
            if let ScopeEntry::Borrow(v) = entry {
                let binding_at_exit = target_depth == 0
                    && self.ref_binding_vals.contains_key(v)
                    && !keep.contains(v);
                if !self.ref_results.contains(v) || binding_at_exit {
                    self.emit_end_borrow(*v);
                }
            }
        }
        for entry in &entries {
            match entry {
                ScopeEntry::Owned(v) if !keep.contains(v) => {
                    self.push_inst(InstKind::DestroyValue { operand: *v });
                },
                // DefUninit: the slot was moved out (Swift `load [take]`) and owns
                // nothing — emitting DestroyAddr would double-free. Skip it.
                // borrowed: inout self/arg — the caller owns the storage. Skip it.
                ScopeEntry::Var {
                    init: VarInit::DefUninit,
                    ..
                }
                | ScopeEntry::Var { borrowed: true, .. } => {},
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
                    // NAMED ref bindings thread through control flow as
                    // @guaranteed block args (references "1.75"): the borrow
                    // stays live across merges/loops and ends at its lexical
                    // scope exit. Single-use expression refs (ref_results)
                    // deliberately stay block-local (E497).
                    ScopeEntry::Borrow(v) if self.ref_binding_vals.contains_key(v) => {
                        Some((*v, self.body.value(*v).ty, Ownership::Guaranteed))
                    },
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
            field_inits: self.field_inits.clone(),
        }
    }

    pub fn restore_scope(&mut self, snapshot: &ScopeSnapshot) {
        // Single-use borrows can't cross block boundaries — strip on
        // restore. NAMED ref bindings survive: they thread through control
        // flow as @guaranteed block args, and the next rebind_scope_values
        // maps the restored (pre-branch) value to the entered block's param.
        let bindings: std::collections::HashSet<ValueId> =
            self.ref_binding_vals.keys().copied().collect();
        self.scope_stack.truncate(snapshot.scopes.len());
        for (i, frame) in self.scope_stack.iter_mut().enumerate() {
            frame.entries = snapshot.scopes[i]
                .iter()
                .filter(|e| match e {
                    ScopeEntry::Borrow(v) => bindings.contains(v),
                    _ => true,
                })
                .cloned()
                .collect();
        }
        self.local_map = snapshot.local_map.clone();
        self.tracker = snapshot.tracker.clone();
        self.field_inits = snapshot.field_inits.clone();
    }

    /// Replace scope-tracked values when entering a new block.
    /// Updates scope stack, local_map, AND the shared LiveTracker.
    pub fn rebind_scope_values(&mut self, old_vals: &[ValueId], new_vals: &[ValueId]) {
        // Threaded binding borrows: the new block's @guaranteed param IS the
        // same borrow continued. Stamp it with the original's borrow_source
        // (verify tracks it as an open borrow so the scope-exit EndBorrow
        // lands and Check 4 holds) and provenance root (the escape checker
        // still sees the original root through merges — `return r` after an
        // `if` stays E494). Register it as the binding's current name.
        for (&old, &new) in old_vals.iter().zip(new_vals.iter()) {
            if old == new {
                continue;
            }
            if let Some(&local) = self.ref_binding_vals.get(&old) {
                let (src, root, span) = {
                    let d = self.body.value(old);
                    (d.borrow_source, d.root, d.span.clone())
                };
                // Remap the borrow's source through the SAME rebinding: when
                // the borrowed var slot is itself threaded (old→new in this
                // call), the continued borrow must point at the slot's new
                // name or the next block's consume-protection goes stale.
                let remap = |x: ValueId| {
                    old_vals
                        .iter()
                        .position(|&o| o == x)
                        .map(|p| new_vals[p])
                        .unwrap_or(x)
                };
                let nd = &mut self.body.values[new.index()];
                nd.borrow_source = src.map(remap).or(Some(old));
                nd.root = root;
                if nd.span.is_none() {
                    nd.span = span;
                }
                self.ref_binding_vals.insert(new, local);
                continue;
            }
            // Stage 2b: an OWNED ref-bearing value keeps its escape taint
            // through threading — a stamped root (≠ its own self-root) is
            // copied onto the continued value, Local roots remapped through
            // the same rebinding. Untainted values keep their fresh
            // self-root (the param's own id), so the owned-return check's
            // "root == self" discriminator stays sound across merges.
            // Tainted VAR SLOTS (the G1 closure: Pointer-of-ref-bearing)
            // thread their taint the same way.
            let (ownership, root, ty) = {
                let d = self.body.value(old);
                (d.ownership, d.root, d.ty)
            };
            let carries_taint = self.ctx.module.ty_arena.contains_ref(ty)
                || matches!(
                    self.ctx.module.ty_arena.get(ty),
                    MirTy::Pointer(p) if self.ctx.module.ty_arena.contains_ref(*p)
                );
            if ownership == Ownership::Owned
                && root != RootProvenance::Local(old)
                && carries_taint
            {
                let remapped = match root {
                    RootProvenance::Local(w) => RootProvenance::Local(
                        old_vals
                            .iter()
                            .position(|&o| o == w)
                            .map(|p| new_vals[p])
                            .unwrap_or(w),
                    ),
                    r => r,
                };
                self.body.values[new.index()].root = remapped;
            }
        }
        for scope in self.scope_stack.iter_mut() {
            for entry in scope.entries.iter_mut() {
                match entry {
                    ScopeEntry::Owned(v) | ScopeEntry::Borrow(v) => {
                        if let Some(pos) = old_vals.iter().position(|&old| old == *v) {
                            *v = new_vals[pos];
                        }
                    },
                    _ => {},
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
        // Record the rename so a held value (e.g. a call argument materialized
        // before this block boundary) can be resolved to its current SSA name.
        // Overwrite semantics keep the map at the latest binding, which is the
        // one valid in the block we're about to lower into.
        for (&old, &new) in old_vals.iter().zip(new_vals.iter()) {
            if old != new {
                self.value_forwarding.insert(old, new);
            }
        }
    }

    /// Chase `value_forwarding` to the current SSA representative of `value`.
    /// Returns `value` unchanged when it was never rebound across a block
    /// boundary. Bounded to guard against any accidental cycle.
    pub fn resolve_value(&self, value: ValueId) -> ValueId {
        let mut v = value;
        for _ in 0..10_000 {
            match self.value_forwarding.get(&v) {
                Some(&next) if next != v => v = next,
                _ => break,
            }
        }
        v
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
            let owned = self.emit_copy_value(result);
            // Arm-value decay: when the arm value is a tracked ref (ret_borrow
            // call result), the copy-out above was its single use — end its
            // borrow before the jump to the merge, or `set_terminator`'s sweep
            // reports a false E497. Mirrors binding decay in `lower_stmt`.
            // (A named binding's borrow is multi-use — left for the
            // terminator policy.)
            if self.ref_results.contains(&result) {
                self.end_ref_if_single_use(result);
            }
            owned
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
            field_inits: self.field_inits.clone(),
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
                    && !out.iter().any(|(k, _)| k == l)
                {
                    out.push((*l, *init));
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
        let v = self.body.value(operand);
        let ty = v.ty;
        let ownership = v.ownership;
        // A copy of a non-Copyable @owned value is illegal (no clone shim). Such
        // a transfer is a move: consume the operand instead of duplicating it.
        if ownership == Ownership::Owned && self.is_non_copyable(ty) {
            return self.emit_move_value(operand);
        }
        // A copy of a non-Copyable @guaranteed value is a move-out-of-borrow:
        // there's no clone shim to duplicate it and we don't own it to move it.
        // The front-end move checker should reject this (E503); this is the
        // lowering backstop so it can never reach OSSA verify as an ICE-shaped
        // "copy of non-Copyable" failure. Diagnose with a span, then move the
        // borrow (the accumulated error aborts the build before codegen).
        if self.is_non_copyable(ty) {
            self.emit_move_out_of_borrow_backstop(ty);
            return self.emit_move_value(operand);
        }
        let result = self.alloc_value(ty, Ownership::Owned);
        self.push_inst(InstKind::CopyValue { result, operand });
        // Stage 2b: a copy of a ref-BEARING value aliases the same refs —
        // the escape taint travels with it (a stamped root ≠ the operand's
        // own self-root). Gated on contains_ref so nothing else changes.
        self.carry_ref_taint(result, operand);
        self.track_owned(result);
        result
    }

    /// Copy the operand's provenance root onto `result` when the operand is
    /// a TAINTED ref-bearing value (root differs from its own self-root).
    fn carry_ref_taint(&mut self, result: ValueId, operand: ValueId) {
        let (ty, root) = {
            let d = self.body.value(operand);
            (d.ty, d.root)
        };
        if root != RootProvenance::Local(operand) && self.ctx.module.ty_arena.contains_ref(ty) {
            self.stamp_root(result, root);
        }
    }

    /// Backstop diagnostic for the lowering of a move-out-of-borrow (duplicating
    /// a non-Copyable value held only by borrow). Normally the HIR move checker
    /// rejects this first (E503); this fires only for shapes it can't see, so
    /// the build fails with a real error instead of an OSSA-verify ICE.
    fn emit_move_out_of_borrow_backstop(&mut self, ty: TyId) {
        let span = self
            .current_span
            .clone()
            .unwrap_or_else(|| Span::synthetic(0));
        let ty_str = kestrel_mir::display::ty_to_string(ty, &self.ctx.module);
        self.ctx.query.accumulate(
            Diagnostic::error()
                .with_code("E503")
                .with_message(format!(
                    "cannot move non-copyable value of type `{ty_str}` out of a borrow"
                ))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range()).with_message(
                        "a non-copyable value cannot be moved out of a borrowed place",
                    ),
                ]),
        );
    }

    /// Move an @owned value: produces a fresh @owned result and consumes the
    /// operand (removing it from scope + tracker so it isn't dropped or
    /// forwarded again). Used for non-Copyable transfers.
    pub fn emit_move_value(&mut self, operand: ValueId) -> ValueId {
        let ty = self.body.value(operand).ty;
        let result = self.alloc_value(ty, Ownership::Owned);
        self.push_inst(InstKind::MoveValue { result, operand });
        self.carry_ref_taint(result, operand);
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
    /// The borrow's source is the address's storage ANCHOR (the chain base),
    /// not an intermediate `FieldAddr` temp — the temp is destroyed at scope
    /// exit, and a returned borrow must not chain to it (see `addr_anchors`).
    pub fn emit_begin_borrow_addr(&mut self, address: ValueId, ty: TyId) -> ValueId {
        let anchor = self.addr_anchors.get(&address).copied().unwrap_or(address);
        let result = self.alloc_guaranteed(ty, anchor);
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

    /// The function-exit ownership gate, shared by the block-tail and
    /// explicit-`return` paths. Non-ret_borrow functions must return @owned —
    /// a @guaranteed tail is copied out (the historical copy guard, verbatim).
    /// A ret_borrow function returns the borrow itself: @guaranteed passes
    /// through; an @owned result is borrowed in place — its provenance root
    /// decides escape legality downstream (an owned temporary roots `Local`
    /// and is rejected by E494; verify's Return hardening backstops both
    /// directions). The returned borrow is untracked: it deliberately
    /// outlives the body (Check 4's ret_borrow carve-out forwards it).
    pub fn prepare_return_value(&mut self, value: ValueId) -> ValueId {
        let guaranteed = self.body.value(value).ownership == Ownership::Guaranteed;
        if self.ret_borrow {
            let v = if guaranteed {
                value
            } else {
                self.emit_begin_borrow(value)
            };
            self.deferred_end_borrows.retain(|&x| x != v);
            self.untrack_borrow(v);
            return v;
        }
        if guaranteed {
            let owned = self.emit_copy_value(value);
            self.emit_end_borrow(value);
            return owned;
        }
        value
    }

    pub fn drain_deferred_borrows(&mut self) {
        let borrows: Vec<ValueId> = self.deferred_end_borrows.drain(..).collect();
        for v in borrows {
            self.untrack_borrow(v);
            self.push_inst(InstKind::EndBorrow { operand: v });
        }
    }

    /// End still-tracked refs (ret_borrow call results) allocated at or
    /// after `mark`. Refs are single-use: at an expression boundary (a
    /// lowered `if` condition, the end of a statement) a ref born inside
    /// that expression and still scope-tracked has been fully used — its
    /// copy-out just didn't flow through a path that ends it (e.g. a
    /// @guaranteed field projection consumed downstream). Ending it here
    /// keeps it out of the next terminator, where it would be a false E497.
    /// The watermark spares refs born EARLIER that are legitimately pending
    /// (`add(h.peek(), if c { .. })` — the peek ref must survive to the
    /// `add` call so the arm terminator correctly reports E497).
    pub fn end_stale_refs_since(&mut self, mark: usize) {
        let stale: Vec<ValueId> = self
            .scope_stack
            .iter()
            .flat_map(|s| s.entries.iter())
            .filter_map(|e| match e {
                // Named bindings are multi-use — they survive statement/
                // condition sweeps and end at scope exit or a terminator.
                ScopeEntry::Borrow(v)
                    if v.index() >= mark
                        && self.ref_results.contains(v)
                        && !self.ref_binding_vals.contains_key(v) =>
                {
                    Some(*v)
                },
                _ => None,
            })
            .collect();
        for v in stale {
            self.emit_end_borrow(v);
        }
    }

    /// Next value index — the watermark for `end_stale_refs_since`.
    pub fn ref_watermark(&self) -> usize {
        self.body.values.len()
    }

    pub fn emit_begin_mut_borrow(&mut self, operand: ValueId) -> ValueId {
        let ty = self.body.value(operand).ty;
        let result = self.alloc_guaranteed(ty, operand);
        self.push_inst(InstKind::BeginMutBorrow { result, operand });
        self.track_borrow(result);
        result
    }

    pub fn emit_begin_mut_borrow_addr(&mut self, address: ValueId, ty: TyId) -> ValueId {
        // Anchored like `emit_begin_borrow_addr` — see that doc comment.
        let anchor = self.addr_anchors.get(&address).copied().unwrap_or(address);
        let result = self.alloc_guaranteed(ty, anchor);
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

    /// Coerce a value destined for an @owned aggregate slot (tuple element,
    /// struct field, enum payload) to @owned. A @guaranteed element is a
    /// *borrow*; packing it into an @owned aggregate (which then `consume`s it)
    /// would make the aggregate alias the borrow, so once the borrow's source
    /// drops the aggregate dangles — and double-frees when the aggregate drops.
    /// Clone borrows to @owned. (Arises when a by-value — i.e. borrowed —
    /// param escapes into storage, e.g. `Headers.add` doing
    /// `self.entries.append((name, value))`.) Non-Copyable borrows are left
    /// alone: cloning them is illegal and the frontend rejects such escapes.
    fn own_aggregate_element(&mut self, v: ValueId) -> ValueId {
        // Resolve any element value renamed by a block-boundary rebind since it
        // was prepared. When a *later* aggregate element expression splits the
        // current block (a `try`/`if`/`match` field), the values of the earlier
        // elements are threaded through the new blocks and renamed at the merge
        // (recorded in `value_forwarding`); the held element value still names
        // the stranded pre-split id. Without this, the aggregate would consume
        // the stale id while the live threaded twin stays tracked in scope and
        // is dropped at scope exit — an over-release that double-frees any
        // COW-shared element. Mirrors the same resolution in `emit_call_inner`.
        let v = self.resolve_value(v);
        let vd = self.body.value(v);
        let guaranteed = vd.ownership == Ownership::Guaranteed;
        let ty = vd.ty;
        if guaranteed && !self.is_non_copyable(ty) {
            self.emit_copy_value(v)
        } else {
            v
        }
    }

    /// Stage 2b packaging: prepare one element bound for a REF slot. The
    /// borrow itself is the operand — the address is the payload: no
    /// copy-to-owned (that would clone the POINTEE) and, for @guaranteed
    /// operands, no consume (a borrow isn't owned; consuming would also
    /// un-thread a named binding). The ref's provenance joins the
    /// aggregate's taint; a fresh single-use expression ref has done its
    /// job once packaged (collected in `end_after`, ended after the inst).
    fn prep_ref_slot_element(
        &mut self,
        v: ValueId,
        taint: &mut Option<RootProvenance>,
        end_after: &mut Vec<ValueId>,
    ) -> ValueId {
        let v = self.resolve_value(v);
        let (root, guaranteed) = {
            let d = self.body.value(v);
            (d.root, d.ownership == Ownership::Guaranteed)
        };
        let convs = self.current_param_convs();
        *taint = Some(match taint.take() {
            None => root,
            Some(j) => j.join(root, &convs),
        });
        if guaranteed && self.ref_results.contains(&v) && !self.ref_binding_vals.contains_key(&v) {
            end_after.push(v);
        }
        v
    }

    /// Is the declared (substituted) slot type at `slot` a `&T`?
    pub(crate) fn slot_is_ref(&self, slot_tys: &[TyId], slot: usize) -> bool {
        slot_tys
            .get(slot)
            .is_some_and(|&t| matches!(self.ctx.module.ty_arena.get(t), MirTy::Ref { .. }))
    }

    pub fn emit_struct(&mut self, ty: TyId, fields: Vec<(FieldIdx, ValueId)>) -> ValueId {
        let slot_tys = self.struct_field_tys(ty);
        let result = self.alloc_value(ty, Ownership::Owned);
        let mut taint: Option<RootProvenance> = None;
        let mut end_after: Vec<ValueId> = Vec::new();
        let fields: Vec<(FieldIdx, ValueId)> = fields
            .into_iter()
            .map(|(idx, v)| {
                if self.slot_is_ref(&slot_tys, idx.index()) {
                    (idx, self.prep_ref_slot_element(v, &mut taint, &mut end_after))
                } else {
                    (idx, self.own_aggregate_element(v))
                }
            })
            .collect();
        for &(_, v) in &fields {
            // @guaranteed ref-slot operands are borrows — never consumed.
            if self.body.value(v).ownership == Ownership::Owned {
                self.consume(v);
            }
        }
        if let Some(root) = taint {
            self.stamp_root(result, root);
        }
        self.push_inst(InstKind::Struct { result, ty, fields });
        for v in end_after {
            self.end_ref_if_single_use(v);
        }
        self.track_owned(result);
        result
    }

    pub fn emit_tuple(&mut self, ty: TyId, elements: Vec<ValueId>) -> ValueId {
        let slot_tys = self.tuple_elem_tys(ty);
        let result = self.alloc_value(ty, Ownership::Owned);
        let mut taint: Option<RootProvenance> = None;
        let mut end_after: Vec<ValueId> = Vec::new();
        let elements: Vec<ValueId> = elements
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                if self.slot_is_ref(&slot_tys, i) {
                    self.prep_ref_slot_element(v, &mut taint, &mut end_after)
                } else {
                    self.own_aggregate_element(v)
                }
            })
            .collect();
        for &v in &elements {
            if self.body.value(v).ownership == Ownership::Owned {
                self.consume(v);
            }
        }
        if let Some(root) = taint {
            self.stamp_root(result, root);
        }
        self.push_inst(InstKind::Tuple { result, elements });
        for v in end_after {
            self.end_ref_if_single_use(v);
        }
        self.track_owned(result);
        result
    }

    pub fn emit_enum_variant(
        &mut self,
        enum_ty: TyId,
        variant: VariantIdx,
        payload: Vec<ValueId>,
    ) -> ValueId {
        let slot_tys = self.enum_variant_payload_tys(enum_ty, variant);
        let result = self.alloc_value(enum_ty, Ownership::Owned);
        let mut taint: Option<RootProvenance> = None;
        let mut end_after: Vec<ValueId> = Vec::new();
        let payload: Vec<ValueId> = payload
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                if self.slot_is_ref(&slot_tys, i) {
                    self.prep_ref_slot_element(v, &mut taint, &mut end_after)
                } else {
                    self.own_aggregate_element(v)
                }
            })
            .collect();
        for &v in &payload {
            if self.body.value(v).ownership == Ownership::Owned {
                self.consume(v);
            }
        }
        if let Some(root) = taint {
            self.stamp_root(result, root);
        }
        self.push_inst(InstKind::Enum {
            result,
            enum_ty,
            variant,
            payload,
        });
        for v in end_after {
            self.end_ref_if_single_use(v);
        }
        self.track_owned(result);
        result
    }

    pub fn emit_struct_extract(
        &mut self,
        operand: ValueId,
        field: FieldIdx,
        result_ty: TyId,
    ) -> ValueId {
        // Stage 2b ref slot: the extraction loads the stored address — see
        // `extract_ref_slot`. Never the copy path (it would clone the
        // POINTEE).
        if let MirTy::Ref { pointee, .. } = self.ctx.module.ty_arena.get(result_ty) {
            let pointee = *pointee;
            return self.extract_ref_slot(operand, pointee, |s, result, view| {
                s.push_inst(InstKind::StructExtract {
                    result,
                    operand: view,
                    field,
                });
            });
        }
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
        // Stage 2b ref slot — see `extract_ref_slot`.
        if let MirTy::Ref { pointee, .. } = self.ctx.module.ty_arena.get(result_ty) {
            let pointee = *pointee;
            return self.extract_ref_slot(operand, pointee, |s, result, view| {
                s.push_inst(InstKind::TupleExtract {
                    result,
                    operand: view,
                    index,
                });
            });
        }
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

    /// Is this `Def` entity a stored global — a module-level `var`/`let` or a
    /// `static var`/`let` member — whose storage is a GlobalRef? Both are
    /// `NodeKind::Field` with no `Callable` (a computed `get`/`set` member is
    /// `Callable`; an instance field is `Field`+`!Callable` too but never
    /// appears as a bare `Def`, only `self.x`). This is the criterion
    /// `lower_def` already uses to READ a global, and it is timing-independent:
    /// a forward-referenced global isn't yet in `module.statics` while an
    /// earlier body lowers, but codegen registers every static before it runs,
    /// so the GlobalRef resolves. Checking `module.statics` here silently
    /// missed forward refs (a lost write); the `Static` component alone is too
    /// broad (static methods carry it).
    pub fn is_stored_global_def(&self, entity: Entity) -> bool {
        self.ctx.world.get::<kestrel_ast_builder::NodeKind>(entity)
            == Some(&kestrel_ast_builder::NodeKind::Field)
            && self
                .ctx
                .world
                .get::<kestrel_ast_builder::Callable>(entity)
                .is_none()
    }

    pub fn emit_global_ref(&mut self, entity: Entity) -> ValueId {
        let i64_ty = self.ctx.module.ty_arena.i64();
        let result = self.alloc_value(i64_ty, Ownership::Owned);
        // Globals outlive every call — borrows rooted here may escape returns.
        self.stamp_root(result, RootProvenance::Static);
        self.push_inst(InstKind::GlobalRef { result, entity });
        self.track_owned(result);
        result
    }

    /// Override a value's provenance root after allocation. Used where the
    /// root is known better than the alloc-time default: globals (`Static`)
    /// and the `ptr_ref`/`ptr_mut_ref` intrinsics (`PointerDerived`).
    pub fn stamp_root(&mut self, value: ValueId, root: RootProvenance) {
        self.body.values[value.index()].root = root;
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
        // Tuple container: the element type is the pointee directly (no
        // entity/field substitution). Mirrors the struct branch's result —
        // codegen's `struct_field_offset` likewise special-cases tuples.
        if let MirTy::Tuple(elems) = self.ctx.module.ty_arena.get(ty) {
            let field_ty = elems[field.index()];
            return self.finish_field_addr(base, ty, field, field_ty);
        }
        let field_ty = if let MirTy::Named { entity, type_args } = self.ctx.module.ty_arena.get(ty)
        {
            let entity = *entity;
            let type_args = type_args.clone();
            if let Some(raw_ft) = self.ctx.resolve_field_ty(entity, field) {
                if type_args.is_empty() {
                    raw_ft
                } else {
                    let mut subst = kestrel_mir::SubstMap::new();
                    if let Some(def) = self.ctx.module.structs.get(&entity) {
                        for (tp, &ta) in def.type_params.iter().zip(type_args.iter()) {
                            subst.type_params.insert(tp.entity, ta);
                        }
                    }
                    kestrel_mir::substitute(&mut self.ctx.module.ty_arena, raw_ft, &subst)
                }
            } else {
                ty
            }
        } else {
            ty
        };
        self.finish_field_addr(base, ty, field, field_ty)
    }

    /// Tail of `emit_field_addr`: allocate the `Pointer[field_ty]` result,
    /// inherit the base's provenance root + storage anchor, and push the
    /// FieldAddr inst. Shared by struct and tuple containers.
    fn finish_field_addr(
        &mut self,
        base: ValueId,
        ty: TyId,
        field: FieldIdx,
        field_ty: TyId,
    ) -> ValueId {
        let ptr_ty = self.ctx.module.ty_arena.pointer(field_ty);
        let result = self.alloc_value(ptr_ty, Ownership::Owned);
        // A field address lives exactly where its base lives: inherit the
        // base's provenance root. Without this the address self-roots
        // `Local` (the alloc_value funnel only chases borrow_source, and
        // addresses are owned), severing the Param(i) root of a `mutating`
        // receiver — every `-> &T { self.v }` projection then failed E494
        // "borrows local" even though the verifier accepts Param roots.
        // Chains (`self.a.b`) compose transitively; Begin(Mut)BorrowAddr
        // results inherit from the address via borrow_source as before.
        let base_root = self.body.value(base).root;
        self.stamp_root(result, base_root);
        // Thread the storage anchor: borrows through this derived address
        // record the chain's BASE as their source (see `addr_anchors`).
        let anchor = self.addr_anchors.get(&base).copied().unwrap_or(base);
        self.addr_anchors.insert(result, anchor);
        self.push_inst(InstKind::FieldAddr {
            result,
            base,
            ty,
            field,
        });
        self.track_owned(result);
        result
    }

    /// G1 closure (stage 2b): a TAINTED ref-bearing value stored into a
    /// var slot taints the SLOT's root (monotone join — reassignments only
    /// tighten), and loads inherit it — so
    /// `var o = .Some(&local); return o` stays E494 instead of laundering
    /// the taint through memory. Param-rooted stores keep slots (and their
    /// loads) returnable: the cursor-in-a-var flagship is unaffected.
    fn taint_slot_from_store(&mut self, address: ValueId, value: ValueId) {
        let (v_root, v_ty) = {
            let d = self.body.value(value);
            (d.root, d.ty)
        };
        if v_root == RootProvenance::Local(value) || !self.ctx.module.ty_arena.contains_ref(v_ty)
        {
            return;
        }
        let slot_root = self.body.value(address).root;
        let new_root = if slot_root == RootProvenance::Local(address) {
            v_root
        } else {
            let convs = self.current_param_convs();
            slot_root.join(v_root, &convs)
        };
        self.stamp_root(address, new_root);
    }

    /// The load half of the G1 closure: a load of a ref-bearing type from
    /// a tainted slot carries the slot's root.
    fn inherit_slot_taint(&mut self, result: ValueId, address: ValueId, ty: TyId) {
        let slot_root = self.body.value(address).root;
        if slot_root != RootProvenance::Local(address)
            && self.ctx.module.ty_arena.contains_ref(ty)
        {
            self.stamp_root(result, slot_root);
        }
    }

    pub fn emit_store_init(&mut self, address: ValueId, value: ValueId) {
        let value = if self.body.value(value).ownership == Ownership::Guaranteed {
            self.emit_copy_value(value)
        } else {
            value
        };
        self.taint_slot_from_store(address, value);
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
        self.taint_slot_from_store(address, value);
        self.push_inst(InstKind::StoreAssign { address, value });
        self.consume(value);
    }

    pub fn emit_load(&mut self, address: ValueId, ty: TyId) -> ValueId {
        let result = self.alloc_value(ty, Ownership::Owned);
        self.inherit_slot_taint(result, address, ty);
        self.push_inst(InstKind::Load { result, address });
        self.track_owned(result);
        result
    }

    pub fn emit_take(&mut self, address: ValueId, ty: TyId) -> ValueId {
        let result = self.alloc_value(ty, Ownership::Owned);
        self.inherit_slot_taint(result, address, ty);
        // Default to independent (memcpy); the post-mono mark_independent_takes
        // pass flips provably-safe moves to aliasing.
        self.push_inst(InstKind::Take {
            result,
            address,
            ty,
            independent: true,
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
        mut args: Vec<CallArg>,
        result_ty: Option<TyId>,
    ) -> Option<ValueId> {
        // Resolve any arg / indirect-callee value that was renamed by a
        // block-boundary rebind since it was prepared (e.g. an @owned arg
        // materialized before a control-flow sibling arg, threaded through the
        // new blocks and renamed at the merge). Without this the Call would
        // reference a value stranded at a predecessor block's exit.
        for a in args.iter_mut() {
            a.value = self.resolve_value(a.value);
        }
        let callee = match callee {
            Callee::Thin(v) => Callee::Thin(self.resolve_value(v)),
            Callee::Thick(v) => Callee::Thick(self.resolve_value(v)),
            other => other,
        };
        // ret_borrow callee? Only Direct callees can be: function values
        // (Thin/Thick) are rejected by E491 and witness ref-returns are out
        // of stage 1's scope. `result_ty` is already the PEELED pointee
        // (ResolvedTy lowering peels Ref), so the convention must come from
        // the callee's signature, not the result type.
        // THE single ret-convention source is the entity-keyed front-end
        // query (`CallableRefReturn`), NOT the callee's lowered FunctionDef:
        // `module.functions` fills incrementally as declarations lower, so a
        // FunctionDef lookup misses callees whose defining file lowers later
        // (array.ks lowers before pointer.ks — `Pointer.value` would fall
        // back to a value-convention result and break the returned ref).
        let ret_ref_mutating: Option<bool> = match &callee {
            Callee::Direct { func, .. } => self
                .ctx
                .query
                .query(kestrel_hir_lower::CallableRefReturn {
                    entity: *func,
                    root: self.ctx.root,
                })
                .map(|r| r.mutating),
            // Stage 2d: a witness call to a `-> &Self.Item`-shaped
            // requirement is a ret_borrow call too — derive from the
            // PROTOCOL method's declared return (the E458 exact-shape rule
            // guarantees every impl agrees). Without this arm the returned
            // raw pointer registered as an owned value and the pointer
            // BITS read as the pointee (generic-dispatch corruption).
            Callee::Witness {
                protocol, method, ..
            } => self
                .find_protocol_method_entity(*protocol, method)
                .and_then(|entity| {
                    self.ctx.query.query(kestrel_hir_lower::CallableRefReturn {
                        entity,
                        root: self.ctx.root,
                    })
                })
                .map(|r| r.mutating),
            _ => None,
        };
        // Stage 2b: a generic `-> T` callee instantiated at `T = &U`
        // (unwrap()/or() on Optional[&U], `identity[&U]`) returns the REF
        // by value — the mono instance's ret IS `MirTy::Ref` while
        // ret_borrow stays false (derived from the DECLARED ret). The
        // caller registers the result exactly like a ret_borrow result:
        // @guaranteed pointee, rooted at the borrowable args (extraction
        // roots at the aggregate's root). Derivable here from the declared
        // return + instantiated type args — no expr context needed.
        let ret_ref_mutating = ret_ref_mutating.or_else(|| {
            let Callee::Direct {
                func, type_args, ..
            } = &callee
            else {
                return None;
            };
            let kestrel_hir::ty::HirTy::Param(p, _) =
                self.ctx.query.query(kestrel_hir_lower::LowerCallableReturnType {
                    entity: *func,
                    root: self.ctx.root,
                })
            else {
                return None;
            };
            // `type_args` is [enclosing type/extension params] ++ [own
            // params] (prepend_receiver_type_args) — search both lists.
            let own: Vec<kestrel_hecs::Entity> = self
                .ctx
                .world
                .get::<kestrel_ast_builder::TypeParams>(*func)
                .map(|tp| tp.0.clone())
                .unwrap_or_default();
            let parent: Vec<kestrel_hecs::Entity> = self
                .ctx
                .world
                .parent_of(*func)
                .and_then(|par| {
                    self.ctx
                        .world
                        .get::<kestrel_ast_builder::TypeParams>(par)
                        .map(|tp| tp.0.clone())
                })
                .unwrap_or_default();
            let idx = parent
                .iter()
                .position(|&e| e == p)
                .or_else(|| own.iter().position(|&e| e == p).map(|i| parent.len() + i))?;
            match self.ctx.module.ty_arena.get(*type_args.get(idx)?) {
                MirTy::Ref { mutating, .. } => Some(*mutating),
                _ => None,
            }
        });
        // PointerDerived originates at the `ptr_ref`/`ptr_mut_ref` intrinsics:
        // a callee that is a thin intrinsic wrapper (Pointer.value /
        // .mutatingValue) returns a view inheriting the raw pointer's
        // contract, not its receiver temp's lifetime — re-rooting it at the
        // receiver would make `Array.at` a false E494.
        let ret_pointer_derived = ret_ref_mutating.is_some()
            && match &callee {
                Callee::Direct { func, .. } => {
                    self.ctx
                        .query
                        .query(kestrel_type_infer::RetRefPointerDerived {
                            entity: *func,
                            root: self.ctx.root,
                        })
                },
                _ => false,
            };
        let result = result_ty.map(|ty| {
            if let Some(mutating) = ret_ref_mutating {
                // Register the returned ref @guaranteed against the borrow it
                // travels from: the receiver / unique borrow-convention arg
                // (the E493 decl rule makes it unique; for methods that is
                // args[0]). With no borrowable arg the callee's root must be
                // Static/PointerDerived — inherit the pointer's contract.
                let source = args
                    .iter()
                    .find(|a| self.body.value(a.value).ownership == Ownership::Guaranteed)
                    .map(|a| a.value);
                match source {
                    Some(src) => {
                        let v = self.alloc_guaranteed(ty, src);
                        if ret_pointer_derived {
                            self.stamp_root(v, RootProvenance::PointerDerived {
                                mutable: mutating,
                            });
                        }
                        v
                    },
                    None => self.body.alloc_value(
                        ValueDef {
                            ty,
                            ownership: Ownership::Guaranteed,
                            borrow_source: None,
                            root: RootProvenance::PointerDerived { mutable: mutating },
                            span: None,
                        }
                        .with_span(self.current_span.clone()),
                    ),
                }
            } else {
                let ownership = self.ownership_for(ty);
                let v = self.alloc_value(ty, ownership);
                // Stage 2b: an owned result that CARRIES refs (a ref-bearing
                // aggregate, e.g. `-> Optional[&T]`) roots at the join of
                // the borrowable args — the callee can only have rooted its
                // refs at its params (mirror of the ret_borrow source rule).
                // No candidate args ⇒ the callee used statics/pointers —
                // inherit the unverified pointer contract.
                if self.ctx.module.ty_arena.contains_ref(ty) {
                    let convs = self.current_param_convs();
                    let mut joined: Option<RootProvenance> = None;
                    for a in &args {
                        let (a_root, a_guaranteed, a_ty) = {
                            let d = self.body.value(a.value);
                            (d.root, d.ownership == Ownership::Guaranteed, d.ty)
                        };
                        let candidate = a_guaranteed
                            || (a.convention == ParamConvention::Consuming
                                && self.ctx.module.ty_arena.contains_ref(a_ty));
                        if candidate {
                            joined = Some(match joined {
                                None => a_root,
                                Some(j) => j.join(a_root, &convs),
                            });
                        }
                    }
                    let root = joined
                        .unwrap_or(RootProvenance::PointerDerived { mutable: false });
                    self.stamp_root(v, root);
                }
                v
            }
        });
        let mut borrows: Vec<ValueId> = args
            .iter()
            .filter(|a| self.body.value(a.value).ownership == Ownership::Guaranteed)
            .map(|a| a.value)
            .collect();
        if let Some(cv) = callee.value()
            && self.body.value(cv).ownership == Ownership::Guaranteed
        {
            borrows.push(cv);
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
        if ret_ref_mutating.is_none() {
            for borrow_val in borrows {
                let src = self.body.value(borrow_val).borrow_source;
                self.emit_end_borrow(borrow_val);
                // A sub-borrow of a ref (receiver/arg prep) was that ref's
                // single use — refs aren't nameable, so once the dependent
                // borrow ends the ref is dead. End it here so it never
                // reaches a later terminator (a stale ref at a Branch would
                // be an E497 false positive). Named bindings ARE nameable —
                // their borrow outlives any one call.
                if let Some(src) = src
                    && self.ref_results.contains(&src)
                {
                    self.end_ref_if_single_use(src);
                }
            }
        }
        // else: the arg borrows stay scope-tracked — every potential root
        // must outlive the returned ref (cheap under may-alias); the scope
        // machinery ends them at scope exit or the next terminator.
        self.drain_deferred_borrows();
        if let (Some(ty), Some(r)) = (result_ty, result) {
            if matches!(self.ctx.module.ty_arena.get(ty), MirTy::Never) {
                self.destroy_scopes_to_depth(0, &[]);
                self.set_terminator(TerminatorKind::Panic("noreturn".to_string()));
                return Some(r);
            }
            if ret_ref_mutating.is_some() {
                self.track_borrow(r);
                self.ref_results.insert(r);
            } else {
                self.track_owned(r);
            }
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
        // Values this terminator forwards as block args. A NAMED binding
        // borrow among them is THREADED (references "1.75"): verify Check 4
        // accepts forwarded @guaranteed block args, the scope entry survives,
        // and the entered block rebinds it to the matching @guaranteed param
        // (rebind_scope_values). All control-flow constructs forward the
        // live-tracked set — and bindings joined it via all_live_tracked —
        // so this is the normal path; the E497 below is the fallback for
        // any jump emitted outside the tracker pattern.
        let forwarded: std::collections::HashSet<ValueId> = match &kind {
            TerminatorKind::Jump { args, .. } => args.iter().copied().collect(),
            TerminatorKind::Branch {
                then_args,
                else_args,
                ..
            } => then_args.iter().chain(else_args.iter()).copied().collect(),
            TerminatorKind::Switch { cases, .. } => cases
                .iter()
                .flat_map(|c| c.args.iter())
                .copied()
                .collect(),
            _ => Default::default(),
        };
        // Threading is keyed by the binding's LOCAL, not the ValueId: arms
        // are lowered sequentially but their exit jumps are emitted later
        // under whatever scope state lowering currently holds, so the scope
        // entry may name a SIBLING arm's param for the same binding. The
        // forwarded arg always names the exiting path's own value; a scope
        // borrow for the same local is the same logical binding.
        let threaded_locals: std::collections::HashSet<HirLocalId> = forwarded
            .iter()
            .filter_map(|v| self.ref_binding_vals.get(v).copied())
            .collect();
        let is_threaded = |vals: &HashMap<ValueId, HirLocalId>, v: &ValueId| {
            vals.get(v).is_some_and(|l| threaded_locals.contains(l))
        };
        // End all scope-tracked borrows EXCEPT threaded bindings before the
        // terminator — single-use borrows can't cross block boundaries.
        let all_borrows: Vec<ValueId> = self
            .scope_stack
            .iter()
            .flat_map(|s| s.entries.iter())
            .filter_map(|e| match e {
                ScopeEntry::Borrow(v) if !is_threaded(&self.ref_binding_vals, v) => Some(*v),
                _ => None,
            })
            .collect();
        let binding_vals = &self.ref_binding_vals;
        for scope in &mut self.scope_stack {
            scope.entries.retain(|e| match e {
                ScopeEntry::Borrow(v) => is_threaded(binding_vals, v),
                _ => true,
            });
        }
        // ret_borrow carve-out: the one returned borrow IS the function's
        // result — it outlives the body (belt-and-suspenders with
        // prepare_return_value's untrack; covers Returns routed elsewhere).
        let returned = match &kind {
            TerminatorKind::Return(v) if self.ret_borrow => Some(*v),
            _ => None,
        };
        let ends_block_inside_fn = !matches!(
            kind,
            TerminatorKind::Return(_) | TerminatorKind::Panic(_) | TerminatorKind::Unreachable
        );
        for v in all_borrows {
            if Some(v) == returned {
                continue;
            }
            // A ret_borrow call result crossing a Jump/Branch/Switch would
            // dangle past the merge — single-use refs stay intra-block
            // (E497). Still EndBorrow for IR sanity; the error aborts
            // compilation. A NAMED binding only lands here on a jump that
            // did NOT forward it (a non-tracker-pattern edge): silent end
            // when fully used, the binding E497 otherwise.
            if ends_block_inside_fn {
                if let Some(&local) = self.ref_binding_vals.get(&v) {
                    if self.ref_binding_remaining.get(&local).copied().unwrap_or(0) > 0 {
                        kestrel_debug::ktrace!(
                            "xblock",
                            "E497 fallback: v={:?} local={:?} forwarded={:?} kind={:?}",
                            v,
                            local,
                            forwarded,
                            kind
                        );
                        self.emit_binding_across_merge_error(v, local);
                    }
                } else if self.ref_results.contains(&v) {
                    self.emit_ref_across_merge_error(v);
                }
            }
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

    /// E497: a reference (ret_borrow call result) used as a place across a
    /// control-flow merge. Deliberate v1 limitation — suggest hoisting.
    fn emit_ref_across_merge_error(&mut self, v: ValueId) {
        let span = self
            .body
            .value(v)
            .span
            .clone()
            .or_else(|| self.current_span.clone())
            .unwrap_or_else(|| Span::synthetic(0));
        self.ctx.query.accumulate(
            Diagnostic::error()
                .with_code("E497")
                .with_message(
                    "a reference cannot stay live across a control-flow merge in this version",
                )
                .with_labels(vec![Label::primary(span.file_id, span.range()).with_message(
                    "this reference would cross an `if`/`match`/loop boundary",
                )])
                .with_notes(vec![
                    "bind the value first (`let x = ...;`) or hoist the branching expression \
                     into its own binding"
                        .into(),
                ]),
        );
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
        // Drop all in-scope owned values before the diverging `Panic`
        // terminator so OSSA verification passes — owned intermediates (e.g.
        // a `fatalError` message's formatting temps, or a match scrutinee in
        // a non-exhaustive fallback) would otherwise be "live at block exit
        // but never consumed". Mirrors the Never-returning call path
        // (`emit_call_inner`), which already cleans up before `Panic`.
        self.destroy_scopes_to_depth(0, &[]);
        self.set_terminator(TerminatorKind::Panic(msg.to_string()));
    }

    // ================================================================
    // Var-local address access — the address walk lives in `place.rs`
    // (`try_field_addr_chain` / `try_var_addr`), the place resolver.
    // ================================================================

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

    /// Lower a ret_borrow function's return expression as a PLACE: the
    /// resolver keeps the provenance root for the escape check — value-mode
    /// lowering's Copyable-field snapshot would re-root at a fresh Local and
    /// turn every `&self.field` return into E494. `FieldViews::Allow` makes
    /// resolution total for stored-field shapes (borrowing receivers project
    /// @guaranteed views); everything else falls back to the ordinary paths.
    /// Used ONLY at ret_borrow return sites.
    pub fn lower_expr_for_ref_return(&mut self, expr_id: HirExprId) -> ValueId {
        if let Some(place) = self.lower_place(expr_id, place::FieldViews::Allow) {
            return match place.repr {
                place::PlaceRepr::Addr(addr) => self.emit_begin_borrow_addr(addr, place.pointee),
                place::PlaceRepr::View(v) => v,
            };
        }
        self.lower_expr_for_borrow(expr_id)
    }

    pub fn lower_expr_for_borrow(&mut self, expr_id: HirExprId) -> ValueId {
        // Local/captured places resolve through the place resolver: var
        // locals borrow their address in place (value-mode would
        // `emit_copy_addr` — an illegal copy for a non-Copyable var); owned
        // SSA locals come back RAW (callers own the borrow decision —
        // borrowing here stranded Iterator.fold's receiver on a temp).
        // Field exprs deliberately fall to lower_expr: value-context reads
        // keep the Copyable-field snapshot and its clone counts.
        if !matches!(self.hir.exprs[expr_id], HirExpr::Field { .. })
            && let Some(place) = self.lower_place(expr_id, place::FieldViews::Forbid)
        {
            return match place.repr {
                place::PlaceRepr::Addr(addr) => self.emit_begin_borrow_addr(addr, place.pointee),
                place::PlaceRepr::View(v) => v,
            };
        }
        self.lower_expr(expr_id)
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

    /// Stage-1.5 decay shape for ALWAYS-DECAY value positions (literal
    /// elements): a tracked ref result is copied out — the position owns a
    /// value, never the place — and the copy is the ref's single use, so
    /// its borrow ends here. Mirrors binding decay in `lower_stmt` and the
    /// arm decay in `capture_arm_exit`. Non-ref values pass through.
    pub fn decay_if_ref(&mut self, value: ValueId) -> ValueId {
        if self.ref_results.contains(&value) {
            let owned = self.emit_copy_value(value);
            self.end_ref_if_single_use(value);
            owned
        } else {
            value
        }
    }

    /// End a ref after a value-use ONLY when it is single-use. Stage-1
    /// expression refs are unnameable — their copy-out is their one use,
    /// so the borrow ends with it. A NAMED binding's value is multi-use:
    /// the borrow stays live until lexical scope exit / the next
    /// terminator. Every "the ref's single use — end it" site funnels here.
    pub fn end_ref_if_single_use(&mut self, v: ValueId) {
        if !self.ref_binding_vals.contains_key(&v) {
            self.emit_end_borrow(v);
        }
    }

    /// Count a read of a ref-binding local (once per HIR expr — desugar
    /// re-lowering must not double-decrement). Drives the terminator
    /// policy: remaining > 0 at an inside-fn terminator = E497.
    pub fn note_binding_read(&mut self, local: HirLocalId, expr_id: HirExprId) {
        if let Some(remaining) = self.ref_binding_remaining.get_mut(&local)
            && self.ref_binding_reads.insert(expr_id)
        {
            *remaining = remaining.saturating_sub(1);
        }
    }

    /// Lower a `let r = &expr;` initializer to the binding's @guaranteed
    /// value (stage 1.5 item 2). `&expr` is "evaluate `expr` as a place and
    /// borrow it" — exactly a borrow-convention argument, so the whole
    /// place matrix is delegated to `prepare_call_arg_for_expr`:
    /// var slots → `BeginBorrowAddr`/`BeginMutBorrowAddr` (writes through
    /// the var stay visible — may-alias), accessor-backed members → the
    /// `mutating ref` child (MutBorrow), SSA locals / @guaranteed values →
    /// in-place borrow (no spurious clone), ref-returning calls →
    /// pass-through. Rvalue borrows are rejected by analyze (E499) but
    /// lower soundly (scope exit ends the borrow before the temp dies).
    ///
    /// A `&mutating` of a get/set-only member falls to the WRITEBACK path,
    /// whose statement-boundary write-back would strand later stores —
    /// analyze rejects that shape (no `mutating ref` provider).
    pub fn lower_borrow_init(&mut self, inner: HirExprId, mutating: bool) -> ValueId {
        // Reads through `lower_expr_for_borrow` bypass the Local arm's
        // counter — count a binding read (re-borrow `&r`) here.
        if let HirExpr::Local(l, _) = &self.hir.exprs[inner]
            && !self.is_var_local(l)
        {
            let l = *l;
            self.note_binding_read(l, inner);
        }
        let convention = if mutating {
            ParamConvention::MutBorrow
        } else {
            ParamConvention::Borrow
        };
        let v = self.prepare_call_arg_for_expr(inner, convention).value;
        // Re-borrow of an existing NAMED binding: the new binding needs its
        // own lifetime — two locals must never share one borrow value.
        if self.ref_binding_vals.contains_key(&v) {
            return self.emit_begin_borrow(v);
        }
        v
    }

    /// E497, binding wording: a named ref binding still has uses after an
    /// inside-fn terminator — bindings never cross blocks (no @guaranteed
    /// block params in this version).
    fn emit_binding_across_merge_error(&mut self, v: ValueId, local: HirLocalId) {
        let name = self.hir.locals[local].name.clone();
        let span = self
            .body
            .value(v)
            .span
            .clone()
            .or_else(|| Some(self.hir.locals[local].span.clone()))
            .unwrap_or_else(|| Span::synthetic(0));
        self.ctx.query.accumulate(
            Diagnostic::error()
                .with_code("E497")
                .with_message(format!(
                    "ref binding '{name}' cannot stay live across a control-flow merge"
                ))
                .with_labels(vec![Label::primary(span.file_id, span.range()).with_message(
                    "this binding is still used after an `if`/`match`/loop boundary",
                )])
                .with_notes(vec![
                    "a binding's last use must come before the branch; re-borrow inside \
                     the branch or bind the value (`let x = ...;`) instead"
                        .into(),
                ]),
        );
    }

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
            // Place-resolved: Addr borrows the address (writes go through);
            // a View (SSA owned receiver like a `consuming` func's self,
            // loaded ref-slot field) routes through prepare_call_arg —
            // borrowing the value in place, never lower_expr's copy_value,
            // which would strand the mutation on a throwaway copy (the
            // Iterator.fold/reduce infinite loop).
            if let Some(p) = self.lower_place(expr_id, place::FieldViews::Forbid) {
                return match p.repr {
                    place::PlaceRepr::Addr(addr) => {
                        let borrow = self.emit_begin_mut_borrow_addr(addr, p.pointee);
                        CallArg {
                            value: borrow,
                            convention,
                        }
                    },
                    place::PlaceRepr::View(v) => self.prepare_call_arg(v, convention),
                };
            }
            // Stage 1.5 accessor place: an accessor-backed member in mutable
            // position (`x(i) += v` receiver, `x(i).mutate()`) fabricates its
            // place through the `mutating ref` accessor. The @guaranteed ref
            // result IS the by-reference address (see prepare_call_arg's
            // MutBorrow pass-through); scope machinery ends its borrow at the
            // statement boundary, exactly like the shipped
            // `arr.mutableAt(index: i) += 1` shape. Accessor-backed members
            // are None for lower_place (not plain stored), so the probe
            // order place → accessor → value matches the old
            // var-addr → accessor → value chain.
            if let Some(arg) = self.try_lower_accessor_place_mut(expr_id) {
                return arg;
            }
            let val = self.lower_expr(expr_id);
            return self.prepare_call_arg(val, convention);
        }
        if convention == ParamConvention::Borrow {
            // Place-resolved: Addr (var locals / var-rooted field chains)
            // borrows the address; a View (SSA/@guaranteed receiver, closure
            // param, loaded ref-slot field) goes through prepare_call_arg —
            // its sub-borrow semantics protect named-binding multi-use, and
            // it never emits the spurious copy_value lower_expr would (a
            // clone() for Cloneable receivers, corrupting @guaranteed
            // aggregates, e.g. `valuePtr().with { v in v.clone() }`).
            if let Some(p) = self.lower_place(expr_id, place::FieldViews::Forbid) {
                return match p.repr {
                    place::PlaceRepr::Addr(addr) => {
                        let borrow = self.emit_begin_borrow_addr(addr, p.pointee);
                        CallArg {
                            value: borrow,
                            convention,
                        }
                    },
                    place::PlaceRepr::View(v) => self.prepare_call_arg(v, convention),
                };
            }
            let val = self.lower_expr(expr_id);
            return self.prepare_call_arg(val, convention);
        }
        // Single-use SSA local with Consuming convention: move directly,
        // bypassing the emit_value_use copy. Only safe at function top-level
        // scope — loops re-execute the block, and conditional branches need
        // the value alive for the other arm's cleanup/forwarding. The consume
        // is deferred to `emit_call_inner` (see prepare_call_arg) so the value
        // stays tracked across any later control-flow sibling arg.
        if convention == ParamConvention::Consuming && self.scope_stack.len() == 1 {
            let expr = self.hir.exprs[expr_id].clone();
            if let HirExpr::Local(hir_local, _) = &expr
                && !self.is_var_local(hir_local)
                && self.is_single_use(*hir_local)
            {
                let val = self.map_local(*hir_local);
                if self.body.value(val).ownership == Ownership::Owned {
                    return CallArg {
                        value: val,
                        convention,
                    };
                }
            }
        }
        let val = self.lower_expr(expr_id);
        self.prepare_call_arg(val, convention)
    }

    /// Stage 1.5 accessor place (the `mutating ref` half of the provider
    /// rule): when an accessor-backed member expression (`x(i)`, `x.first`)
    /// sits in MUTABLE position — compound-assign receiver, mutating-method
    /// receiver — fabricate its place by calling the member's `mutating ref`
    /// accessor. Receiver and index args are lowered ONCE; the @guaranteed
    /// ref result is passed through as the by-reference MutBorrow argument
    /// (see prepare_call_arg). Returns None when the member has no
    /// `mutating ref` accessor (get/set members keep today's behavior until
    /// the writeback fallback lands).
    fn try_lower_accessor_place_mut(&mut self, expr_id: HirExprId) -> Option<CallArg> {
        let (receiver_expr, index_args, member, is_static) =
            self.accessor_member_prelude(expr_id)?;
        let pointee_ty = self.resolve_expr_type(expr_id);

        if let Some(accessor) = self.ctx.find_ref_accessor_child(member, true) {
            self.ctx.register_name(accessor);
            let mut call_args: Vec<CallArg> = Vec::new();
            let type_args = if is_static {
                let self_type = self.type_from_type_ref(receiver_expr);
                self.prepend_receiver_type_args(self_type, vec![])
            } else {
                let receiver_ty = self.resolve_expr_type(receiver_expr);
                let ta = self.resolve_type_args(expr_id);
                call_args
                    .push(self.prepare_call_arg_for_expr(receiver_expr, ParamConvention::MutBorrow));
                self.prepend_receiver_type_args(receiver_ty, ta)
            };
            for a in &index_args {
                let v = self.lower_expr(a.value);
                call_args.push(self.prepare_call_arg(v, ParamConvention::Borrow));
            }
            let callee = Callee::direct_with_args(accessor, type_args, None);
            let ref_val = self.emit_call_returning(callee, call_args, pointee_ty);
            return Some(CallArg {
                value: ref_val,
                convention: ParamConvention::MutBorrow,
            });
        }

        // get→op→set WRITEBACK fallback: computed accessors can't fabricate
        // addresses, so the element is copied out through `get` into a temp
        // slot, the mutating operation runs on the slot, and the owning call
        // emitter (or the statement-boundary drain) writes the slot back
        // through `set`. The slot-borrow IS the by-reference receiver arg.
        let slot_addr =
            self.fabricate_setter_writeback_slot(expr_id, receiver_expr, &index_args, member)?;
        let slot_borrow = self.emit_begin_mut_borrow_addr(slot_addr, pointee_ty);
        Some(CallArg {
            value: slot_borrow,
            convention: ParamConvention::MutBorrow,
        })
    }

    /// Copy an accessor-backed member's value out through its read provider
    /// into a fresh stack slot and register the set-back (`pending_writebacks`,
    /// drained by the owning call or the statement boundary). Returns the
    /// slot's ADDRESS so the caller can mutate it in place (a mut-borrow for a
    /// call receiver, or a direct field store for `o.member.field = v`). The
    /// member must have a setter, be non-static, and be Copyable (a
    /// non-Copyable element can't ride the copy-out — E503). `expr_id` is the
    /// accessor member expression; `receiver_expr`/`index_args`/`member` are
    /// its already-resolved prelude (see `accessor_member_prelude`).
    fn fabricate_setter_writeback_slot(
        &mut self,
        expr_id: HirExprId,
        receiver_expr: HirExprId,
        index_args: &[kestrel_hir::body::HirCallArg],
        member: kestrel_hecs::Entity,
    ) -> Option<ValueId> {
        // Statics keep today's rejection (not carved by the analyzer either).
        if self
            .ctx
            .world
            .get::<kestrel_ast_builder::Static>(member)
            .is_some()
        {
            return None;
        }
        let pointee_ty = self.resolve_expr_type(expr_id);
        let setter = self.ctx.find_setter_child(member)?;
        // NotCopyable elements can't ride a copy-out — backstop the analyzer
        // (mirrors emit_move_out_of_borrow_backstop's accumulate pattern).
        if self.is_non_copyable(pointee_ty) {
            let span = self
                .current_span
                .clone()
                .unwrap_or_else(|| kestrel_span::Span::synthetic(0));
            let ty_str = kestrel_mir::display::ty_to_string(pointee_ty, &self.ctx.module);
            self.ctx.query.accumulate(
                kestrel_reporting::Diagnostic::error()
                    .with_code("E503")
                    .with_message(format!(
                        "cannot mutate a non-copyable `{ty_str}` element through get/set \
                         accessors: the writeback copies the element out"
                    ))
                    .with_labels(vec![kestrel_reporting::Label::primary(
                        span.file_id,
                        span.range(),
                    )
                    .with_message("this mutation needs an in-place element reference")])
                    .with_notes(vec![
                        "add a `mutating ref` accessor to mutate elements in place".into(),
                    ]),
            );
            return None;
        }
        self.ctx.register_name(setter);

        let receiver_ty = self.resolve_expr_type(receiver_expr);
        let type_args = self.resolve_type_args(expr_id);
        // Receiver place evaluated ONCE; the get call below uses a
        // sub-borrow so the place survives the call (emit_call_inner ends
        // arg borrows after non-ret_borrow calls), and the setter consumes
        // the place itself as its MutBorrow receiver at drain time.
        let recv_place = self
            .prepare_call_arg_for_expr(receiver_expr, ParamConvention::MutBorrow)
            .value;
        let index_vals: Vec<ValueId> = index_args
            .iter()
            .map(|a| self.lower_expr(a.value))
            .collect();

        // GET: copy the element out.
        let mut get_args: Vec<CallArg> = vec![CallArg {
            value: self.emit_begin_borrow(recv_place),
            convention: ParamConvention::Borrow,
        }];
        for &v in &index_vals {
            get_args.push(self.prepare_call_arg(v, ParamConvention::Borrow));
        }
        // Read through the member's READ provider: the `ref` accessor child
        // when present (a ref+set cross-mix leaves the parent bodyless;
        // emit_store_init below copies out of the @guaranteed ref result),
        // else the getter on the member itself.
        let read_callee = self
            .ctx
            .find_ref_accessor_child(member, false)
            .unwrap_or(member);
        let got = if let Some(protocol) = self.ctx.is_protocol_method(read_callee) {
            self.ctx.register_name(protocol);
            let key = self.ctx.witness_method_key(read_callee);
            let callee = Callee::Witness {
                protocol,
                method: key,
                self_type: receiver_ty,
                method_type_args: type_args.clone(),
            };
            self.emit_call_returning(callee, get_args, pointee_ty)
        } else {
            self.ctx.register_name(read_callee);
            let ta = self.prepend_receiver_type_args(receiver_ty, type_args.clone());
            let callee = Callee::direct_with_args(read_callee, ta, None);
            self.emit_call_returning(callee, get_args, pointee_ty)
        };

        // Slot: the mutating operation runs on the slot's address.
        let slot_addr = self.emit_uninit(pointee_ty);
        self.emit_store_init(slot_addr, got);
        self.pending_writebacks.push(PendingWriteback {
            setter,
            receiver_ty,
            type_args,
            recv_place,
            index_vals,
            slot_addr,
            elem_ty: pointee_ty,
        });
        Some(slot_addr)
    }

    /// `o.member.field = v` where `member` is a value-returning computed
    /// property (get/set, no `mutating ref`): get the member into a slot,
    /// store `v` into the slot's `field`, and let the writeback drain call the
    /// setter — a get→modify→set rewrite. Returns `true` when handled. Without
    /// this the stored-field assign fell back to mutating a getter temp that
    /// was then dropped, silently losing the write (#139).
    pub(crate) fn try_lower_field_assign_through_setter(
        &mut self,
        base: HirExprId,
        base_ty: TyId,
        field_idx: FieldIdx,
        rhs: ValueId,
    ) -> bool {
        let Some((receiver_expr, index_args, member, _)) = self.accessor_member_prelude(base)
        else {
            return false;
        };
        let Some(slot_addr) =
            self.fabricate_setter_writeback_slot(base, receiver_expr, &index_args, member)
        else {
            return false;
        };
        // Store the new field value directly into the slot (no intermediate
        // borrow → no conflict with the drain's `take` of the same slot). The
        // slot field already holds the gotten value, so store_assign drops it.
        let field_addr = self.emit_field_addr(slot_addr, base_ty, field_idx);
        self.emit_store_assign(field_addr, rhs);
        // The pending writeback drains at the owning statement boundary,
        // calling `member`'s setter with the mutated slot.
        true
    }

    /// Resolve an accessor-backed member expression (`x(i)`, `x.first`,
    /// `o.proxy`) to its prelude: `(receiver_expr, index_args, member,
    /// is_static)`. `None` when the expression isn't a Subscript/Field member
    /// call (so not an accessor place).
    fn accessor_member_prelude(
        &mut self,
        expr_id: HirExprId,
    ) -> Option<(HirExprId, Vec<kestrel_hir::body::HirCallArg>, kestrel_hecs::Entity, bool)> {
        let expr = self.hir.exprs[expr_id].clone();
        let (receiver_expr, index_args): (HirExprId, Vec<kestrel_hir::body::HirCallArg>) = match &expr
        {
            HirExpr::Call { callee, args, .. } => (*callee, args.clone()),
            HirExpr::Field { base, .. } => (*base, Vec::new()),
            _ => return None,
        };
        let member = self
            .typed
            .as_ref()
            .and_then(|t| t.resolutions.get(&expr_id))
            .copied()?;
        if !matches!(
            self.ctx.world.get::<kestrel_ast_builder::NodeKind>(member),
            Some(kestrel_ast_builder::NodeKind::Subscript | kestrel_ast_builder::NodeKind::Field)
        ) {
            return None;
        }
        let is_static = self
            .ctx
            .world
            .get::<kestrel_ast_builder::Static>(member)
            .is_some();
        Some((receiver_expr, index_args, member, is_static))
    }

    /// Drain writebacks pushed at or above `watermark`: take the mutated
    /// element back out of its slot and call the member's setter. Invoked by
    /// the call emitters right after their call (watermark recorded before
    /// their arg prep), plus a drain(0) safety net at statement boundaries.
    pub(crate) fn drain_writebacks(&mut self, watermark: usize) {
        while self.pending_writebacks.len() > watermark {
            let wb = self.pending_writebacks.pop().expect("len checked");
            let new_val = self.emit_take(wb.slot_addr, wb.elem_ty);
            let mut args: Vec<CallArg> = vec![CallArg {
                value: wb.recv_place,
                convention: ParamConvention::MutBorrow,
            }];
            for &v in &wb.index_vals {
                args.push(self.prepare_call_arg(v, ParamConvention::Borrow));
            }
            args.push(self.prepare_call_arg(new_val, ParamConvention::Borrow));
            if let Some(protocol) = self.ctx.is_protocol_method(wb.setter) {
                self.ctx.register_name(protocol);
                let key = self.ctx.witness_setter_key(wb.setter);
                let callee = Callee::Witness {
                    protocol,
                    method: key,
                    self_type: wb.receiver_ty,
                    method_type_args: wb.type_args,
                };
                self.emit_call_void(callee, args);
            } else {
                let ta = self.prepend_receiver_type_args(wb.receiver_ty, wb.type_args);
                let callee = Callee::direct_with_args(wb.setter, ta, None);
                self.emit_call_void(callee, args);
            }
        }
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
                // An already-@guaranteed value is itself the by-reference
                // address (e.g. from `ptr_mut_borrow` or `lower_expr_for_borrow`
                // of a mut place). Re-borrowing a scalar address spills it to a
                // stack slot and passes `&slot`, adding a spurious indirection
                // that corrupts the pointee — pass it through directly.
                if self.body.value(value).ownership == Ownership::Guaranteed {
                    return CallArg { value, convention };
                }
                let borrow = self.emit_begin_mut_borrow(value);
                CallArg {
                    value: borrow,
                    convention,
                }
            },
            ParamConvention::Consuming => {
                // Do NOT consume here. The owning `Call` is emitted only after
                // every sibling arg is lowered; if a later arg introduces
                // control flow (`if`/`try`/`match`), this @owned value must
                // stay tracked so it threads through the new blocks. Consuming
                // now would strand it (OSSA: "live at block exit but never
                // consumed"). `emit_call_inner` resolves the (possibly rebound)
                // value and consumes it after pushing the call.
                if self.body.value(value).ownership == Ownership::Owned {
                    CallArg { value, convention }
                } else {
                    let copy = self.emit_copy_value(value);
                    // Copy-out was a ref's single use — end it (stage-1
                    // refs are unnameable, nothing can use it again).
                    // Named bindings stay live for later reads.
                    if self.ref_results.contains(&value) {
                        self.end_ref_if_single_use(value);
                    }
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
        // Clone the resolved args so the `self.typed` borrow is released before
        // the `self.ctx` / `self.subst_default_arg_ty` mutable borrows below.
        let Some(resolved_args) = self
            .typed
            .as_ref()
            .and_then(|t| t.type_args.get(&expr_id))
            .cloned()
        else {
            return Vec::new();
        };
        // Type-side position: a `&T` type argument (stage 2b) is the type
        // itself, not an expression value — never peel it, or `(Optional, [&T])`
        // collapses into `(Optional, [T])`.
        resolved_args
            .iter()
            .map(|ty| {
                let t = lower_resolved_ty_preserving(self.ctx, ty);
                self.subst_default_arg_ty(t)
            })
            .collect()
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
            let entity = *entity;
            let args: Vec<TyId> = hir_args.iter().map(|a| lower_type(self.ctx, a)).collect();
            self.ctx.register_name(entity);
            let ty = crate::ty::lower_named_type(self.ctx, entity, args);
            self.subst_default_arg_ty(ty)
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

/// Synthesize the MIR body of a getter for a stored `static var` that witnesses
/// a protocol `static var { get }` requirement. A stored static var has no
/// accessor function, so witness dispatch (`T.field` through a type param) has
/// nothing to bind (#147). The body is `return <clone of the global>` — mirrors
/// the direct static-read lowering (global_ref + copy_addr, which clones so the
/// global retains ownership). Drives the OSSA emit helpers directly off an empty
/// HIR body. The FunctionDef (entity, name, ret = field type, no params) must
/// already exist in the module.
pub(crate) fn synthesize_static_var_getter(
    ctx: &mut LowerCtx,
    getter_entity: Entity,
    field_entity: Entity,
    field_ty: TyId,
) {
    let empty = HirBody::empty();
    let captures = Arc::new(ClosureCaptureMap::default());
    let mut bctx = OssaBodyCtx::new(ctx, &empty, None, captures, getter_entity, false);
    let entry = bctx.new_block();
    bctx.body.entry = entry;
    bctx.current_block = Some(entry);
    bctx.push_scope();
    let addr = bctx.emit_global_ref(field_entity);
    let result = bctx.emit_copy_addr(addr, field_ty);
    bctx.emit_destroy_value(addr);
    bctx.emit_ret(result);
    let body = bctx.body;
    if let Some(f) = ctx.module.functions.get_mut(&getter_entity) {
        f.body = Some(body);
    }
}

/// Synthesize the MIR body of a setter for a stored `static var` witnessing a
/// protocol `static var { set }` requirement (twin of
/// [`synthesize_static_var_getter`]). The body is `global = value` — a
/// `StoreAssign` (drops the old value, stores the consumed param). The
/// FunctionDef must already exist with one consuming `value` param at
/// `ValueId(0)` and a unit return type.
pub(crate) fn synthesize_static_var_setter(
    ctx: &mut LowerCtx,
    setter_entity: Entity,
    field_entity: Entity,
    field_ty: TyId,
) {
    let empty = HirBody::empty();
    let captures = Arc::new(ClosureCaptureMap::default());
    let mut bctx = OssaBodyCtx::new(ctx, &empty, None, captures, setter_entity, false);
    let entry = bctx.new_block();
    bctx.body.entry = entry;
    bctx.current_block = Some(entry);
    bctx.push_scope();
    // The incoming `value` param is ValueId(0) (matches the FunctionDef's
    // param). `param_count` seeds the verifier's defined-on-entry set.
    bctx.body.param_count = 1;
    let value = bctx.body.alloc_value(ValueDef::owned(field_ty));
    let addr = bctx.emit_global_ref(field_entity);
    bctx.emit_store_assign(addr, value);
    bctx.emit_destroy_value(addr);
    let unit = bctx.emit_literal(kestrel_mir::Immediate::unit());
    bctx.emit_ret(unit);
    let body = bctx.body;
    if let Some(f) = ctx.module.functions.get_mut(&setter_entity) {
        f.body = Some(body);
    }
}

/// Synthesize the MIR body of a getter for a stored INSTANCE var that witnesses
/// a protocol `var { get }` requirement (the instance analogue of
/// [`synthesize_static_var_getter`]). The body is `return <clone of self.field>`
/// — extract the field from the (borrowed) `self` param, then CopyValue to an
/// @owned result. The FunctionDef must already exist with one borrowed `self`
/// param at `ValueId(0)` and the field type as its return type.
pub(crate) fn synthesize_instance_var_getter(
    ctx: &mut LowerCtx,
    getter_entity: Entity,
    self_ty: TyId,
    field_idx: FieldIdx,
    field_ty: TyId,
) {
    let empty = HirBody::empty();
    let captures = Arc::new(ClosureCaptureMap::default());
    let mut bctx = OssaBodyCtx::new(ctx, &empty, None, captures, getter_entity, false);
    let entry = bctx.new_block();
    bctx.body.entry = entry;
    bctx.current_block = Some(entry);
    bctx.push_scope();
    bctx.body.param_count = 1;
    let self_val = bctx.body.alloc_value(ValueDef {
        ty: self_ty,
        ownership: Ownership::Guaranteed,
        borrow_source: None,
        root: RootProvenance::Param(0),
        span: None,
    });
    let field_view = bctx.emit_struct_extract(self_val, field_idx, field_ty);
    let result = bctx.emit_copy_value(field_view);
    bctx.emit_ret(result);
    let body = bctx.body;
    if let Some(f) = ctx.module.functions.get_mut(&getter_entity) {
        f.body = Some(body);
    }
}

/// Synthesize the MIR body of a setter for a stored INSTANCE var witnessing a
/// protocol `var { set }` requirement. The body is `self.field = value` — a
/// `StoreAssign` through the field address of the (mutably-borrowed) `self`. The
/// FunctionDef must already exist with a mutating `self` param at `ValueId(0)`,
/// a consuming `value` param at `ValueId(1)`, and a unit return type.
pub(crate) fn synthesize_instance_var_setter(
    ctx: &mut LowerCtx,
    setter_entity: Entity,
    self_ty: TyId,
    field_idx: FieldIdx,
    field_ty: TyId,
) {
    let empty = HirBody::empty();
    let captures = Arc::new(ClosureCaptureMap::default());
    let mut bctx = OssaBodyCtx::new(ctx, &empty, None, captures, setter_entity, false);
    let entry = bctx.new_block();
    bctx.body.entry = entry;
    bctx.current_block = Some(entry);
    bctx.push_scope();
    bctx.body.param_count = 2;
    // self: mutating borrow — an address usable by emit_field_addr (ValueId 0).
    let self_val = bctx.body.alloc_value(ValueDef {
        ty: self_ty,
        ownership: Ownership::Guaranteed,
        borrow_source: None,
        root: RootProvenance::Param(0),
        span: None,
    });
    // value: consuming param (ValueId 1).
    let value = bctx.body.alloc_value(ValueDef::owned(field_ty));
    let field_addr = bctx.emit_field_addr(self_val, self_ty, field_idx);
    bctx.emit_store_assign(field_addr, value);
    bctx.emit_destroy_value(field_addr); // consume the @owned field pointer
    let unit = bctx.emit_literal(kestrel_mir::Immediate::unit());
    bctx.emit_ret(unit);
    let body = bctx.body;
    if let Some(f) = ctx.module.functions.get_mut(&setter_entity) {
        f.body = Some(body);
    }
}

/// Span of the value-producing expression: descends through `Block` wrappers
/// to the tail expression, so an arm `=> { ...; expr }` diagnoses at `expr`.
/// Used to point arm-value decay diagnostics (E503 on a NotCopyable copy-out
/// in `capture_arm_exit`) at the arm value, not the enclosing statement.
pub(crate) fn value_expr_span(hir: &HirBody, id: HirExprId) -> Span {
    let mut id = id;
    while let kestrel_hir::body::HirExpr::Block { body, .. } = &hir.exprs[id] {
        match body.tail_expr {
            Some(tail) => id = tail,
            None => break,
        }
    }
    expr_span(hir, id)
}

/// Extract span from an HirExpr.
pub(crate) fn expr_span(hir: &HirBody, id: HirExprId) -> Span {
    match &hir.exprs[id] {
        kestrel_hir::body::HirExpr::Literal { span, .. }
        | kestrel_hir::body::HirExpr::Local(_, span)
        | kestrel_hir::body::HirExpr::Tuple { span, .. }
        | kestrel_hir::body::HirExpr::Borrow { span, .. }
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
        typed.as_deref(),
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
