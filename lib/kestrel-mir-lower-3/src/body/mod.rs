pub mod call;
pub mod closure;
pub mod control;
pub mod expr;
pub mod literal;
pub mod pattern;
pub mod stmt;

use std::collections::{HashMap, HashSet};

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

pub(crate) struct ScopeFrame {
    pub tracked_values: Vec<ValueId>,
    /// Stack-allocated var locals needing DestroyAddr on scope exit.
    pub var_addrs: Vec<(ValueId, TyId)>,
}

/// Snapshot of the scope stack + local_map for save/restore across
/// parallel control flow arms (if/else, match).
#[derive(Clone)]
pub(crate) struct ScopeSnapshot {
    pub scopes: Vec<Vec<ValueId>>,
    pub local_map: HashMap<HirLocalId, ValueId>,
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
pub(crate) struct LiveTracker {
    slots: Vec<(ValueId, TyId, Ownership)>,
}

impl LiveTracker {
    pub fn from_live(live: &[(ValueId, TyId, Ownership)]) -> Self {
        Self { slots: live.to_vec() }
    }

    /// Current values to forward as block args.
    pub fn values(&self) -> Vec<ValueId> {
        self.slots.iter().map(|&(v, _, _)| v).collect()
    }

    /// Type descriptors for creating block params.
    pub fn descs(&self) -> Vec<(TyId, Ownership)> {
        self.slots.iter().map(|&(_, ty, ownership)| (ty, ownership)).collect()
    }

    /// Update slots when entering a new block whose params replace old values.
    pub fn rebind(&mut self, old: &[ValueId], new: &[ValueId]) {
        for slot in &mut self.slots {
            if let Some(pos) = old.iter().position(|&v| v == slot.0) {
                slot.0 = new[pos];
            }
        }
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }
}

pub(crate) struct OssaBodyCtx<'a, 'w> {
    pub ctx: &'a mut LowerCtx<'w>,
    pub hir: HirRef<'a>,
    pub typed: Option<TypedRef<'a>>,
    pub func_entity: Entity,
    pub func_idx: usize,
    pub in_protocol_extension: bool,
    pub body: OssaBody,
    pub current_block: Option<BlockId>,
    pub local_map: HashMap<HirLocalId, ValueId>,
    /// Locals using address semantics (MutBorrow params, var locals).
    /// Reads go through Load, field assignments use the address directly.
    pub var_locals: HashSet<HirLocalId>,
    pub loop_stack: Vec<LoopInfo>,
    pub scope_stack: Vec<ScopeFrame>,
    /// Shared tracker for threading @owned values through control flow.
    /// Updated by rebind_scope_values, saved/restored with snapshots.
    pub tracker: LiveTracker,
    pub temp_counter: u32,
    pub current_span: Option<Span>,
}

impl<'a, 'w> OssaBodyCtx<'a, 'w> {
    pub fn new(
        ctx: &'a mut LowerCtx<'w>,
        hir: &'a HirBody,
        typed: Option<&'a TypedBody>,
        func_entity: Entity,
        func_idx: usize,
        in_protocol_extension: bool,
    ) -> Self {
        Self {
            ctx,
            hir: HirRef::Borrowed(hir),
            typed: typed.map(TypedRef::Borrowed),
            func_entity,
            func_idx,
            in_protocol_extension,
            body: OssaBody::new(),
            current_block: None,
            local_map: HashMap::new(),
            var_locals: HashSet::new(),
            loop_stack: Vec::new(),
            scope_stack: Vec::new(),
            tracker: LiveTracker::from_live(&[]),
            temp_counter: 0,
            current_span: None,
        }
    }

    // ================================================================
    // Main entry
    // ================================================================

    pub fn lower_body(&mut self) {
        let locals: Vec<_> = self.hir.locals.iter().map(|(id, l)| (id, l.clone())).collect();
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
        // and need var_locals treatment — reads go through Load, field
        // assignments use the address directly.
        let param_conventions: Vec<ParamConvention> = self.ctx.module.functions
            .get(self.func_idx)
            .map(|f| f.params.iter().map(|p| p.convention).collect())
            .unwrap_or_default();
        for (i, (hir_id, _local)) in locals.iter().enumerate() {
            if i >= params_len { break; }
            let ty = self.resolve_local_type(*hir_id);
            let convention = param_conventions.get(i).copied()
                .unwrap_or(ParamConvention::Borrow);
            match convention {
                ParamConvention::MutBorrow => {
                    let val = self.body.alloc_value(ValueDef {
                        ty,
                        ownership: Ownership::Guaranteed,
                        borrow_source: None,
                    });
                    self.local_map.insert(*hir_id, val);
                    self.var_locals.insert(*hir_id);
                }
                ParamConvention::Borrow => {
                    let val = self.body.alloc_value(ValueDef {
                        ty,
                        ownership: Ownership::Guaranteed,
                        borrow_source: None,
                    });
                    self.local_map.insert(*hir_id, val);
                }
                ParamConvention::Consuming => {
                    let ownership = self.ownership_for(ty);
                    let val = self.alloc_value(ty, ownership);
                    self.local_map.insert(*hir_id, val);
                    self.track_owned(val);
                }
            }
        }
        self.body.param_count = params_len;

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
                    let prev = self.current_span.replace(tail_span);
                    // Destroy ALL scopes except the return value
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

    pub fn new_block_with_params(&mut self, params: &[(TyId, Ownership)]) -> (BlockId, Vec<ValueId>) {
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
            .map(|b| !matches!(self.body.block(b).terminator.kind, TerminatorKind::Unreachable))
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

    pub fn map_local(&mut self, hir_id: HirLocalId) -> ValueId {
        if let Some(&val) = self.local_map.get(&hir_id) {
            return val;
        }
        // Lazy allocation for locals referenced before their let-statement
        // (e.g. deinit of an uninitialized local, closure captures).
        let ty = self.resolve_local_type(hir_id);
        let ownership = self.ownership_for(ty);
        let val = self.alloc_value(ty, ownership);
        self.local_map.insert(hir_id, val);
        val
    }

    pub fn is_copy_type(&self, ty: TyId) -> bool {
        let wc = self.ctx.module.functions.get(self.func_idx)
            .and_then(|f| f.where_clause.as_ref());
        matches!(
            kestrel_mir_3::ty_query::copy_behavior(&self.ctx.module.ty_arena, &self.ctx.module, ty, wc),
            CopyBehavior::Bitwise
        )
    }

    pub fn ownership_for(&self, ty: TyId) -> Ownership {
        kestrel_mir_3::body::ownership_for_type(ty, &self.ctx.module.ty_arena, &self.ctx.module)
    }

    // ================================================================
    // Value allocation
    // ================================================================

    pub fn alloc_value(&mut self, ty: TyId, ownership: Ownership) -> ValueId {
        let def = match ownership {
            Ownership::Owned => ValueDef::owned(ty),
            Ownership::Guaranteed => panic!("use alloc_guaranteed for @guaranteed"),
        };
        self.body.alloc_value(def)
    }

    pub fn alloc_value_auto(&mut self, ty: TyId) -> ValueId {
        let ownership = self.ownership_for(ty);
        self.alloc_value(ty, ownership)
    }

    pub fn alloc_guaranteed(&mut self, ty: TyId, source: ValueId) -> ValueId {
        self.body.alloc_value(ValueDef::guaranteed(ty, source))
    }

    // ================================================================
    // Scope tracking
    // ================================================================

    pub fn push_scope(&mut self) {
        self.scope_stack.push(ScopeFrame {
            tracked_values: Vec::new(),
            var_addrs: Vec::new(),
        });
    }

    pub fn track_var(&mut self, address: ValueId, content_ty: TyId) {
        if let Some(frame) = self.scope_stack.last_mut() {
            frame.var_addrs.push((address, content_ty));
        }
    }

    pub fn track_owned(&mut self, value: ValueId) {
        if self.is_terminated() {
            return;
        }
        if let Some(frame) = self.scope_stack.last_mut() {
            if !frame.tracked_values.contains(&value) {
                frame.tracked_values.push(value);
            }
        }
    }

    /// Mark a value as consumed — removes from scope tracking.
    pub fn consume(&mut self, value: ValueId) {
        for scope in self.scope_stack.iter_mut().rev() {
            if let Some(pos) = scope.tracked_values.iter().position(|&v| v == value) {
                scope.tracked_values.remove(pos);
                return;
            }
        }
    }

    pub fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }

    pub fn destroy_scope_except(&mut self, keep: &[ValueId]) {
        if let Some(scope) = self.scope_stack.last_mut() {
            let mut surviving = Vec::new();
            for &value in scope.tracked_values.iter().rev() {
                if keep.contains(&value) {
                    surviving.push(value);
                } else {
                    self.body.block_mut(self.current_block.unwrap()).insts.push(
                        Instruction::new(InstKind::DestroyValue { operand: value }),
                    );
                }
            }
            surviving.reverse();
            scope.tracked_values = surviving;
        }
    }

    pub fn destroy_scopes_to_depth(&mut self, target_depth: usize, keep: &[ValueId]) {
        for scope in self.scope_stack[target_depth..].iter().rev() {
            for &value in scope.tracked_values.iter().rev() {
                if !keep.contains(&value) {
                    self.body.block_mut(self.current_block.unwrap()).insts.push(
                        Instruction::new(InstKind::DestroyValue { operand: value }),
                    );
                }
            }
            for &(address, ty) in scope.var_addrs.iter().rev() {
                self.body.block_mut(self.current_block.unwrap()).insts.push(
                    Instruction::new(InstKind::DestroyAddr { address, ty }),
                );
            }
        }
    }

    /// All tracked values for threading through block params.
    pub fn all_live_tracked(&self) -> Vec<(ValueId, TyId, Ownership)> {
        self.scope_stack
            .iter()
            .flat_map(|s| {
                s.tracked_values.iter()
                    .map(|&v| (v, self.body.value(v).ty, Ownership::Owned))
            })
            .collect()
    }

    /// Save scope stack + local_map + tracker for restoration between parallel arms.
    pub fn snapshot_scope(&self) -> ScopeSnapshot {
        ScopeSnapshot {
            scopes: self.scope_stack.iter().map(|s| s.tracked_values.clone()).collect(),
            local_map: self.local_map.clone(),
            tracker: self.tracker.clone(),
        }
    }

    /// Restore scope stack + local_map + tracker from a snapshot.
    /// Truncates extra frames that may remain from terminated arms.
    pub fn restore_scope(&mut self, snapshot: &ScopeSnapshot) {
        self.scope_stack.truncate(snapshot.scopes.len());
        for (i, frame) in self.scope_stack.iter_mut().enumerate() {
            frame.tracked_values = snapshot.scopes[i].clone();
        }
        self.local_map = snapshot.local_map.clone();
        self.tracker = snapshot.tracker.clone();
    }

    /// Replace scope-tracked values when entering a new block.
    /// Updates scope stack, local_map, AND the shared LiveTracker.
    pub fn rebind_scope_values(&mut self, old_vals: &[ValueId], new_vals: &[ValueId]) {
        for scope in self.scope_stack.iter_mut() {
            for val in scope.tracked_values.iter_mut() {
                if let Some(pos) = old_vals.iter().position(|&v| v == *val) {
                    *val = new_vals[pos];
                }
            }
        }
        for (_, val) in self.local_map.iter_mut() {
            if let Some(pos) = old_vals.iter().position(|&v| v == *val) {
                *val = new_vals[pos];
            }
        }
        self.tracker.rebind(old_vals, new_vals);
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
        let result = self.alloc_value(ty, Ownership::Owned);
        self.push_inst(InstKind::CopyValue { result, operand });
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
        result
    }

    pub fn emit_end_borrow(&mut self, operand: ValueId) {
        self.push_inst(InstKind::EndBorrow { operand });
    }

    pub fn emit_begin_mut_borrow(&mut self, operand: ValueId) -> ValueId {
        let ty = self.body.value(operand).ty;
        let result = self.alloc_guaranteed(ty, operand);
        self.push_inst(InstKind::BeginMutBorrow { result, operand });
        result
    }

    pub fn emit_begin_mut_borrow_addr(&mut self, address: ValueId, ty: TyId) -> ValueId {
        let result = self.alloc_guaranteed(ty, address);
        self.push_inst(InstKind::BeginMutBorrowAddr { result, address, ty });
        result
    }

    pub fn emit_end_mut_borrow(&mut self, operand: ValueId) {
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
        self.push_inst(InstKind::Op2 { result, op, lhs, rhs });
        self.track_owned(result);
        result
    }

    pub fn emit_op3(&mut self, op: Op, a: ValueId, b: ValueId, c: ValueId, result_ty: TyId) -> ValueId {
        let result = self.alloc_value(result_ty, Ownership::Owned);
        self.push_inst(InstKind::Op3 { result, op, a, b, c });
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

    pub fn emit_enum_variant(&mut self, enum_ty: TyId, variant: VariantIdx, payload: Vec<ValueId>) -> ValueId {
        let result = self.alloc_value(enum_ty, Ownership::Owned);
        for &v in &payload {
            self.consume(v);
        }
        self.push_inst(InstKind::Enum { result, enum_ty, variant, payload });
        self.track_owned(result);
        result
    }

    pub fn emit_struct_extract(&mut self, operand: ValueId, field: FieldIdx, result_ty: TyId) -> ValueId {
        let operand_ownership = self.body.value(operand).ownership;
        if operand_ownership == Ownership::Guaranteed {
            let result = self.alloc_guaranteed(result_ty, operand);
            self.push_inst(InstKind::StructExtract { result, operand, field });
            result
        } else {
            // Borrow → extract (@guaranteed) → copy (@owned). Operand stays alive
            // for the tracker and further extractions.
            let borrow = self.emit_begin_borrow(operand);
            let field_ref = self.alloc_guaranteed(result_ty, borrow);
            self.push_inst(InstKind::StructExtract { result: field_ref, operand: borrow, field });
            let result = self.emit_copy_value(field_ref);
            self.emit_end_borrow(borrow);
            result
        }
    }

    pub fn emit_tuple_extract(&mut self, operand: ValueId, index: u32, result_ty: TyId) -> ValueId {
        let operand_ownership = self.body.value(operand).ownership;
        if operand_ownership == Ownership::Guaranteed {
            let result = self.alloc_guaranteed(result_ty, operand);
            self.push_inst(InstKind::TupleExtract { result, operand, index });
            result
        } else {
            // Borrow → extract (@guaranteed) → copy (@owned). Operand stays alive.
            let borrow = self.emit_begin_borrow(operand);
            let elem_ref = self.alloc_guaranteed(result_ty, borrow);
            self.push_inst(InstKind::TupleExtract { result: elem_ref, operand: borrow, index });
            let result = self.emit_copy_value(elem_ref);
            self.emit_end_borrow(borrow);
            result
        }
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
        let ptr_ty = self.ctx.module.ty_arena.pointer(ty);
        let result = self.alloc_value(ptr_ty, Ownership::Owned);
        self.push_inst(InstKind::FieldAddr { result, base, ty, field });
        self.track_owned(result);
        result
    }

    pub fn emit_store_init(&mut self, address: ValueId, value: ValueId) {
        let value = if self.body.value(value).ownership == Ownership::Guaranteed {
            self.emit_copy_value(value)
        } else { value };
        self.push_inst(InstKind::StoreInit { address, value });
        self.consume(value);
    }

    pub fn emit_store_assign(&mut self, address: ValueId, value: ValueId) {
        let value = if self.body.value(value).ownership == Ownership::Guaranteed {
            self.emit_copy_value(value)
        } else { value };
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
        self.push_inst(InstKind::Take { result, address, ty });
        self.track_owned(result);
        result
    }

    pub fn emit_copy_addr(&mut self, address: ValueId, ty: TyId) -> ValueId {
        let result = self.alloc_value(ty, Ownership::Owned);
        self.push_inst(InstKind::CopyAddr { result, address, ty });
        self.track_owned(result);
        result
    }

    pub fn emit_apply_partial(&mut self, func: Entity, captures: Vec<ValueId>, result_ty: TyId) -> ValueId {
        let result = self.alloc_value(result_ty, Ownership::Owned);
        for &v in &captures {
            self.consume(v);
        }
        self.push_inst(InstKind::ApplyPartial { result, func, captures });
        self.track_owned(result);
        result
    }

    // ================================================================
    // Emit calls
    // ================================================================

    /// Emit a call and return the result ValueId. Handles borrow insertion.
    /// If the callee returns Never, emits Unreachable after the call.
    pub fn emit_call_returning(
        &mut self,
        callee: Callee,
        args: Vec<CallArg>,
        result_ty: TyId,
    ) -> ValueId {
        let ownership = self.ownership_for(result_ty);
        let result = self.alloc_value(result_ty, ownership);
        // Collect @guaranteed values from args AND callee for EndBorrow
        let mut borrows: Vec<ValueId> = args.iter()
            .filter(|a| self.body.value(a.value).ownership == Ownership::Guaranteed)
            .map(|a| a.value)
            .collect();
        if let Some(cv) = callee.value() {
            if self.body.value(cv).ownership == Ownership::Guaranteed {
                borrows.push(cv);
            }
        }
        // Consuming args are consumed by the call — remove from scope tracking
        let consuming: Vec<ValueId> = args.iter()
            .filter(|a| a.convention == ParamConvention::Consuming)
            .map(|a| a.value)
            .collect();
        self.push_inst(InstKind::Call {
            result: Some(result),
            callee,
            args,
        });
        for v in consuming {
            self.consume(v);
        }
        for borrow_val in borrows {
            self.emit_end_borrow(borrow_val);
        }
        // Never-returning calls terminate the block — destroy all live values first
        if matches!(self.ctx.module.ty_arena.get(result_ty), MirTy::Never) {
            self.destroy_scopes_to_depth(0, &[]);
            self.set_terminator(TerminatorKind::Panic("noreturn".to_string()));
            return result;
        }
        self.track_owned(result);
        result
    }

    /// Emit a void call (no result).
    pub fn emit_call_void(&mut self, callee: Callee, args: Vec<CallArg>) {
        let mut borrows: Vec<ValueId> = args.iter()
            .filter(|a| self.body.value(a.value).ownership == Ownership::Guaranteed)
            .map(|a| a.value)
            .collect();
        // Consuming args are consumed by the call — remove from scope tracking
        let consuming: Vec<ValueId> = args.iter()
            .filter(|a| a.convention == ParamConvention::Consuming)
            .map(|a| a.value)
            .collect();
        if let Some(cv) = callee.value() {
            if self.body.value(cv).ownership == Ownership::Guaranteed {
                borrows.push(cv);
            }
        }
        self.push_inst(InstKind::Call {
            result: None,
            callee,
            args,
        });
        for v in consuming {
            self.consume(v);
        }
        for borrow_val in borrows {
            self.emit_end_borrow(borrow_val);
        }
    }

    // ================================================================
    // Emit terminators
    // ================================================================

    pub fn set_terminator(&mut self, kind: TerminatorKind) {
        let term = Terminator { kind, span: self.current_span.clone() };
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
        self.set_terminator(TerminatorKind::Switch { discriminant, cases });
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
                if self.var_locals.contains(&hir_local) {
                    self.local_map.get(&hir_local).copied()
                } else {
                    None
                }
            }
            kestrel_hir::body::HirExpr::Field { base, name, .. } => {
                let base_addr = self.try_field_addr_chain(base)?;
                let base_ty = self.resolve_expr_type(base);
                let field_name = name.as_str_or_empty();
                let struct_entity = match self.ctx.module.ty_arena.get(base_ty) {
                    MirTy::Named { entity, .. } => Some(*entity),
                    _ => None,
                };
                let field_idx = struct_entity
                    .and_then(|e| self.ctx.resolve_field_idx(e, field_name))
                    .unwrap_or(FieldIdx::new(0));
                Some(self.emit_field_addr(base_addr, base_ty, field_idx))
            }
            _ => None,
        }
    }

    /// If `expr_id` resolves to a var local (possibly through a field chain),
    /// return its address.
    pub fn try_var_addr(&mut self, expr_id: HirExprId) -> Option<ValueId> {
        self.try_field_addr_chain(expr_id)
    }

    /// Resolve an expression to a value suitable for borrowing — returns the
    /// original value without copying. For non-local expressions, falls back
    /// to lower_expr (which may copy).
    pub fn lower_expr_for_borrow(&mut self, expr_id: HirExprId) -> ValueId {
        let expr = self.hir.exprs[expr_id].clone();
        match &expr {
            HirExpr::Local(hir_local, _) if !self.var_locals.contains(hir_local) => {
                self.map_local(*hir_local)
            }
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
            HirExpr::Local(hir_local, _) if !self.var_locals.contains(hir_local) => {
                let val = self.map_local(*hir_local);
                self.consume(val);
                val
            }
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
    pub fn prepare_call_arg_for_expr(&mut self, expr_id: HirExprId, convention: ParamConvention) -> CallArg {
        if convention == ParamConvention::MutBorrow {
            if let Some(addr) = self.try_var_addr(expr_id) {
                let ty = self.resolve_expr_type(expr_id);
                let borrow = self.emit_begin_mut_borrow_addr(addr, ty);
                return CallArg { value: borrow, convention };
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
                CallArg { value: borrow, convention }
            }
            ParamConvention::MutBorrow => {
                let borrow = self.emit_begin_mut_borrow(value);
                CallArg { value: borrow, convention }
            }
            ParamConvention::Consuming => {
                let copy = self.emit_copy_value(value);
                self.consume(copy);
                CallArg { value: copy, convention }
            }
        }
    }

    // ================================================================
    // Helpers
    // ================================================================

    pub fn resolve_type_args(&mut self, expr_id: HirExprId) -> Vec<TyId> {
        if let Some(typed) = self.typed.as_ref()
            && let Some(resolved_args) = typed.type_args.get(&expr_id)
        {
            return resolved_args.iter().map(|ty| lower_resolved_ty(self.ctx, ty)).collect();
        }
        Vec::new()
    }

    pub fn prepend_receiver_type_args(&self, receiver_ty: TyId, method_args: Vec<TyId>) -> Vec<TyId> {
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
pub(crate) fn lower_function_body(ctx: &mut LowerCtx, entity: Entity, func_idx: usize) {
    use kestrel_hir_lower::LowerBody;
    use kestrel_name_res::ExtensionTargetEntity;
    use kestrel_type_infer::InferBody;

    let Some(hir) = ctx.query.query(LowerBody { entity, root: ctx.root }) else {
        return;
    };

    let typed = ctx.query.query(InferBody { entity, root: ctx.root });

    let in_protocol_extension = ctx.world.parent_of(entity).is_some_and(|parent| {
        matches!(
            ctx.world.get::<kestrel_ast_builder::NodeKind>(parent),
            Some(kestrel_ast_builder::NodeKind::Extension)
        ) && ctx
            .query
            .query(ExtensionTargetEntity { extension: parent, root: ctx.root })
            .is_some_and(|target| {
                matches!(
                    ctx.world.get::<kestrel_ast_builder::NodeKind>(target),
                    Some(kestrel_ast_builder::NodeKind::Protocol)
                )
            })
    });

    let mut bctx = OssaBodyCtx::new(ctx, &hir, typed.as_ref(), entity, func_idx, in_protocol_extension);
    bctx.lower_body();
    let ossa_body = bctx.body;

    let func = &mut ctx.module.functions[func_idx];
    for (pi, param) in func.params.iter_mut().enumerate() {
        param.value = ValueId::new(pi);
        if pi < ossa_body.values.len() {
            param.ty = ossa_body.values[pi].ty;
        }
    }
    func.body = Some(ossa_body);
}
