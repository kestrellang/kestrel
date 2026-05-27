//! OSSA ownership verifier.
//!
//! Checks that a body satisfies the linear ownership invariant: every @owned
//! value is consumed exactly once, borrows are properly scoped, and address
//! init/uninit state is consistent. The algorithm is a single forward BFS walk
//! over the CFG — no fixpoint needed because the block-parameter live-in
//! contract guarantees each block can be verified in isolation.

use std::collections::{HashMap, HashSet, VecDeque};

use kestrel_hecs::Entity;
use kestrel_span::Span;

use crate::body::OssaBody;
use crate::inst::InstKind;
use crate::terminator::TerminatorKind;
use crate::ty::ParamConvention;
use crate::value::Ownership;
use crate::{BlockId, FieldIdx, MirModule, TyId, ValueId};

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
    let mut errors = Vec::new();

    // Check 1: ValueId uniqueness — every value defined exactly once.
    check_value_uniqueness(body, func_name, entity, &mut errors);

    // Forward BFS walk from the entry block.
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(body.entry);

    while let Some(block_id) = queue.pop_front() {
        if !visited.insert(block_id) {
            continue;
        }
        verify_block(body, module, block_id, func_name, entity, &mut errors);

        // Enqueue successors.
        let block = body.block(block_id);
        for succ in block.terminator.kind.successors() {
            if !visited.contains(&succ) {
                queue.push_back(succ);
            }
        }
    }

    errors
}

// ---------------------------------------------------------------------------
// Check 1: ValueId uniqueness
// ---------------------------------------------------------------------------

fn check_value_uniqueness(
    body: &OssaBody,
    func_name: &str,
    entity: Entity,
    errors: &mut Vec<VerifyError>,
) {
    // Map from ValueId -> (block that defined it).
    let mut definitions: HashMap<ValueId, BlockId> = HashMap::new();

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
                    span: None,
                    func_name: func_name.to_string(),
                    entity,
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
                    });
                } else {
                    definitions.insert(result, block_id);
                }
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
}

#[derive(Debug, Clone)]
struct BorrowInfo {
    source: ValueId,
    is_mut: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum InitState {
    Init,
    Uninit,
}

#[derive(Debug, Clone)]
enum AddrKind {
    Whole(InitState),
    SubField {
        #[allow(dead_code)]
        ty: TyId,
        fields: HashMap<FieldIdx, InitState>,
    },
}

struct BlockVerifier<'a> {
    body: &'a OssaBody,
    _module: &'a MirModule,
    block_id: BlockId,
    func_name: &'a str,
    entity: Entity,

    /// Tracks @owned values: Live or Consumed.
    owned: HashMap<ValueId, ValueState>,
    /// Active borrows keyed by the @guaranteed result value.
    borrows: HashMap<ValueId, BorrowInfo>,
    /// Address init states.
    addrs: HashMap<ValueId, AddrKind>,
    /// Maps a FieldAddr result -> (base_addr, field_idx).
    field_addr_map: HashMap<ValueId, (ValueId, FieldIdx)>,

    errors: Vec<VerifyError>,
}

impl<'a> BlockVerifier<'a> {
    fn new(
        body: &'a OssaBody,
        module: &'a MirModule,
        block_id: BlockId,
        func_name: &'a str,
        entity: Entity,
    ) -> Self {
        Self {
            body,
            _module: module,
            block_id,
            func_name,
            entity,
            owned: HashMap::new(),
            borrows: HashMap::new(),
            addrs: HashMap::new(),
            field_addr_map: HashMap::new(),
            errors: Vec::new(),
        }
    }

    fn err(&mut self, inst: Option<u32>, message: String) {
        let span = inst.and_then(|i| {
            self.body
                .block(self.block_id)
                .insts
                .get(i as usize)
                .and_then(|inst| inst.span.clone())
        });
        self.errors.push(VerifyError {
            block: self.block_id,
            inst,
            message,
            span,
            func_name: self.func_name.to_string(),
            entity: self.entity,
        });
    }

    // -- Ownership helpers --

    /// Record that an @owned value has been produced.
    fn define_owned(&mut self, v: ValueId) {
        self.owned.insert(v, ValueState::Live);
    }

    /// Attempt to consume an @owned value. Returns false if already consumed.
    fn try_consume(&mut self, v: ValueId, inst: Option<u32>) -> bool {
        let ownership = self.body.value(v).ownership;
        if ownership != Ownership::Owned {
            return true; // not tracked
        }
        match self.owned.get(&v) {
            Some(ValueState::Live) => {
                // Check borrow provenance: cannot consume while borrowed.
                let blocking: Vec<ValueId> = self
                    .borrows
                    .iter()
                    .filter(|(_, info)| info.source == v)
                    .map(|(borrow_val, _)| *borrow_val)
                    .collect();
                if !blocking.is_empty() {
                    self.err(
                        inst,
                        format!(
                            "cannot consume {:?}: active borrow(s) {:?} depend on it",
                            v, blocking,
                        ),
                    );
                }
                self.owned.insert(v, ValueState::Consumed);
                true
            }
            Some(ValueState::Consumed) => {
                self.err(inst, format!("value {:?} consumed more than once", v));
                false
            }
            None => {
                // Value not tracked in this block — likely defined elsewhere.
                // We still flag it so the caller sees it.
                true
            }
        }
    }

    /// Assert a value is live (not consumed). Used for reads.
    fn assert_live(&mut self, v: ValueId, inst: Option<u32>) {
        let ownership = self.body.value(v).ownership;
        if ownership != Ownership::Owned {
            return;
        }
        if let Some(ValueState::Consumed) = self.owned.get(&v) {
            self.err(inst, format!("use of consumed value {:?}", v));
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
            self.err(
                inst,
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
        if let Some(&(base, field)) = self.field_addr_map.get(&addr) {
            if let Some(AddrKind::SubField { fields, .. }) = self.addrs.get(&base) {
                if let Some(InitState::Uninit) = fields.get(&field) {
                    self.err(
                        inst,
                        format!("field {:?} of address {:?} is uninit", field, base),
                    );
                }
            }
            return;
        }

        // Collect error messages first to avoid borrow conflict.
        let mut errs = Vec::new();
        if let Some(ak) = self.addrs.get(&addr) {
            match ak {
                AddrKind::Whole(InitState::Uninit) => {
                    errs.push(format!("address {:?} is uninit", addr));
                }
                AddrKind::SubField { fields, .. } => {
                    for (f, st) in fields {
                        if *st == InitState::Uninit {
                            errs.push(format!(
                                "field {:?} of sub-field-tracked address {:?} is uninit",
                                f, addr,
                            ));
                        }
                    }
                }
                AddrKind::Whole(InitState::Init) => {}
            }
        }
        for msg in errs {
            self.err(inst, msg);
        }
    }

    fn addr_set_uninit(&mut self, addr: ValueId, inst: Option<u32>) {
        // If this is a field addr, set that specific field.
        if let Some(&(base, field)) = self.field_addr_map.get(&addr) {
            let mut err_msg = None;
            if let Some(AddrKind::SubField { fields, .. }) = self.addrs.get_mut(&base) {
                if let Some(st) = fields.get_mut(&field) {
                    if *st == InitState::Uninit {
                        err_msg = Some(format!(
                            "field {:?} of address {:?} already uninit",
                            field, base,
                        ));
                    }
                    *st = InitState::Uninit;
                }
            }
            if let Some(msg) = err_msg {
                self.err(inst, msg);
            }
            return;
        }

        let mut err_msg = None;
        if let Some(ak) = self.addrs.get_mut(&addr) {
            match ak {
                AddrKind::Whole(st) => {
                    if *st == InitState::Uninit {
                        err_msg = Some(format!("address {:?} already uninit", addr));
                    }
                    *st = InitState::Uninit;
                }
                AddrKind::SubField { .. } => {
                    // Whole uninit of a sub-field tracked addr.
                    *ak = AddrKind::Whole(InitState::Uninit);
                }
            }
        }
        if let Some(msg) = err_msg {
            self.err(inst, msg);
        }
    }

    fn addr_store_init(&mut self, addr: ValueId, inst: Option<u32>) {
        // If this is a field addr, set that specific field.
        if let Some(&(base, field)) = self.field_addr_map.get(&addr) {
            let mut err_msg = None;
            if let Some(AddrKind::SubField { fields, .. }) = self.addrs.get_mut(&base) {
                if let Some(st) = fields.get_mut(&field) {
                    if *st == InitState::Init {
                        err_msg = Some(format!(
                            "store_init on field {:?} of address {:?} but field already init",
                            field, base,
                        ));
                    }
                    *st = InitState::Init;
                }
            }
            if let Some(msg) = err_msg {
                self.err(inst, msg);
            }
            return;
        }

        let mut err_msg = None;
        if let Some(ak) = self.addrs.get_mut(&addr) {
            match ak {
                AddrKind::Whole(st) => {
                    if *st == InitState::Init {
                        err_msg = Some(format!(
                            "store_init on address {:?} but already init", addr,
                        ));
                    }
                    *st = InitState::Init;
                }
                AddrKind::SubField { .. } => {
                    // Whole store on sub-field tracked address (e.g. var local
                    // initialized with a complete struct value) — all fields init.
                    *ak = AddrKind::Whole(InitState::Init);
                }
            }
        }
        if let Some(msg) = err_msg {
            self.err(inst, msg);
        }
    }

    fn addr_store_assign(&mut self, addr: ValueId, inst: Option<u32>) {
        let mut err_msg = None;
        if let Some(ak) = self.addrs.get(&addr) {
            match ak {
                AddrKind::Whole(InitState::Uninit) => {
                    err_msg = Some(format!(
                        "store_assign on address {:?} but it is uninit", addr,
                    ));
                }
                _ => {}
            }
        }
        if let Some(msg) = err_msg {
            self.err(inst, msg);
        }
    }

    // -- Main verification --

    fn verify(mut self) -> Vec<VerifyError> {
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
                        BorrowInfo { source: src, is_mut: false },
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

        self.errors
    }

    fn verify_instruction(&mut self, kind: &InstKind, idx: Option<u32>) {
        match kind {
            // -- Value lifecycle --
            InstKind::CopyValue { result, operand } => {
                self.assert_readable(*operand, idx);
                self.define_owned(*result);
            }
            InstKind::MoveValue { result, operand } => {
                self.try_consume(*operand, idx);
                self.define_owned(*result);
            }
            InstKind::DestroyValue { operand } => {
                self.try_consume(*operand, idx);
            }

            // -- Borrowing --
            InstKind::BeginBorrow { result, operand } => {
                self.assert_live(*operand, idx);
                let source = self.body.value(*result).borrow_source.unwrap_or(*operand);
                self.borrows.insert(*result, BorrowInfo { source, is_mut: false });
            }
            InstKind::EndBorrow { operand } => {
                self.borrows.remove(operand);
            }
            InstKind::BeginMutBorrow { result, operand } => {
                self.assert_live(*operand, idx);
                let source = self.body.value(*result).borrow_source.unwrap_or(*operand);
                self.borrows.insert(*result, BorrowInfo { source, is_mut: true });
            }
            InstKind::EndMutBorrow { operand } => {
                self.borrows.remove(operand);
            }

            // -- Memory access --
            InstKind::Load { result, address } => {
                self.addr_require_init(*address, idx);
                self.define_owned(*result);
            }
            InstKind::CopyAddr { result: _, address, .. } => {
                self.addr_require_init(*address, idx);
                // Result is @owned, register it.
                if let Some(r) = kind.result() {
                    if self.body.value(r).ownership == Ownership::Owned {
                        self.define_owned(r);
                    }
                }
            }
            InstKind::Take { result: _, address, .. } => {
                self.addr_require_init(*address, idx);
                self.addr_set_uninit(*address, idx);
                if let Some(r) = kind.result() {
                    if self.body.value(r).ownership == Ownership::Owned {
                        self.define_owned(r);
                    }
                }
            }
            InstKind::BeginBorrowAddr { result, address, .. } => {
                self.addr_require_init(*address, idx);
                let source = self.body.value(*result).borrow_source.unwrap_or(*address);
                self.borrows.insert(*result, BorrowInfo { source, is_mut: false });
            }
            InstKind::BeginMutBorrowAddr { result, address, .. } => {
                self.addr_require_init(*address, idx);
                let source = self.body.value(*result).borrow_source.unwrap_or(*address);
                self.borrows.insert(*result, BorrowInfo { source, is_mut: true });
            }
            InstKind::StoreInit { address, value } => {
                // The stored value is consumed.
                self.try_consume(*value, idx);
                self.addr_store_init(*address, idx);
            }
            InstKind::StoreAssign { address, value } => {
                self.try_consume(*value, idx);
                self.addr_store_assign(*address, idx);
            }
            InstKind::DestroyAddr { address, .. } => {
                self.addr_require_init(*address, idx);
                self.addr_set_uninit(*address, idx);
            }

            // -- Discriminant (non-consuming read, produces @owned i32) --
            InstKind::Discriminant { result, operand } => {
                self.assert_readable(*operand, idx);
                self.define_owned(*result);
            }

            // -- Computation (non-consuming reads of scalar operands) --
            InstKind::Op1 { result, op: _, arg } => {
                self.assert_readable(*arg, idx);
                if self.body.value(*result).ownership == Ownership::Owned {
                    self.define_owned(*result);
                } else {
                    let source = self.body.value(*result).borrow_source.unwrap_or(*arg);
                    self.borrows.insert(*result, BorrowInfo { source, is_mut: false });
                }
            }
            InstKind::Op2 { result, op, lhs, rhs } => {
                self.assert_readable(*lhs, idx);
                if matches!(op, crate::Op::PtrWrite(_)) {
                    // PtrWrite moves the rhs into the destination address.
                    self.try_consume(*rhs, idx);
                } else {
                    self.assert_readable(*rhs, idx);
                }
                self.define_owned(*result);
            }
            InstKind::Op3 { result, op: _, a, b, c } => {
                self.assert_readable(*a, idx);
                self.assert_readable(*b, idx);
                self.assert_readable(*c, idx);
                self.define_owned(*result);
            }

            // -- Constants --
            InstKind::Literal { result, .. } => {
                self.define_owned(*result);
            }
            InstKind::GlobalRef { result, .. } => {
                self.define_owned(*result);
            }

            // -- Aggregate construction: operands that are @owned are consumed --
            InstKind::Struct { result, fields, .. } => {
                for (_, v) in fields {
                    if self.body.value(*v).ownership == Ownership::Owned {
                        self.try_consume(*v, idx);
                    }
                }
                if self.body.value(*result).ownership == Ownership::Owned {
                    self.define_owned(*result);
                }
            }
            InstKind::Tuple { result, elements } => {
                for v in elements {
                    if self.body.value(*v).ownership == Ownership::Owned {
                        self.try_consume(*v, idx);
                    }
                }
                if self.body.value(*result).ownership == Ownership::Owned {
                    self.define_owned(*result);
                }
            }
            InstKind::Enum { result, payload, .. } => {
                for v in payload {
                    if self.body.value(*v).ownership == Ownership::Owned {
                        self.try_consume(*v, idx);
                    }
                }
                if self.body.value(*result).ownership == Ownership::Owned {
                    self.define_owned(*result);
                }
            }
            InstKind::Array { result, elements, .. } => {
                for v in elements {
                    if self.body.value(*v).ownership == Ownership::Owned {
                        self.try_consume(*v, idx);
                    }
                }
                if self.body.value(*result).ownership == Ownership::Owned {
                    self.define_owned(*result);
                }
            }

            // -- Aggregate extraction --
            InstKind::StructExtract { result, operand, .. }
            | InstKind::TupleExtract { result, operand, .. }
            | InstKind::EnumPayload { result, operand, .. } => {
                let op_ownership = self.body.value(*operand).ownership;
                if op_ownership == Ownership::Owned {
                    // Consuming extraction.
                    self.try_consume(*operand, idx);
                }
                // For @guaranteed, not consuming — it is a projection.
                if self.body.value(*result).ownership == Ownership::Owned {
                    self.define_owned(*result);
                }
            }

            // -- Destructuring: operand is consumed (single consume of aggregate) --
            InstKind::DestructureStruct { results, operand }
            | InstKind::DestructureTuple { results, operand } => {
                self.try_consume(*operand, idx);
                for r in results {
                    if self.body.value(*r).ownership == Ownership::Owned {
                        self.define_owned(*r);
                    }
                }
            }
            InstKind::DestructureEnum { results, operand, .. } => {
                self.try_consume(*operand, idx);
                for r in results {
                    if self.body.value(*r).ownership == Ownership::Owned {
                        self.define_owned(*r);
                    }
                }
            }

            // -- Calls --
            InstKind::Call { result, args, .. } => {
                for arg in args {
                    match arg.convention {
                        ParamConvention::Consuming => {
                            self.try_consume(arg.value, idx);
                        }
                        ParamConvention::Borrow | ParamConvention::MutBorrow => {
                            self.assert_live(arg.value, idx);
                        }
                    }
                }
                if let Some(r) = result {
                    let ty = self.body.value(*r).ty;
                    let is_never = matches!(self._module.ty_arena.get(ty), crate::ty::MirTy::Never);
                    if self.body.value(*r).ownership == Ownership::Owned && !is_never {
                        self.define_owned(*r);
                    }
                }
            }
            InstKind::ApplyPartial { result, captures, .. } => {
                // Captures are consumed (they are moved into the closure).
                for v in captures {
                    if self.body.value(*v).ownership == Ownership::Owned {
                        self.try_consume(*v, idx);
                    }
                }
                if self.body.value(*result).ownership == Ownership::Owned {
                    self.define_owned(*result);
                }
            }

            // -- Address projection --
            InstKind::FieldAddr { result, base, field, .. } => {
                self.define_owned(*result);
                if self.addrs.contains_key(base) {
                    self.field_addr_map.insert(*result, (*base, *field));
                }
            }

            // -- Uninit: creates sub-field tracking --
            InstKind::Uninit { result, ty } => {
                self.define_owned(*result);
                // Look up how many fields this type has.
                let field_count = self.struct_field_count(*ty);
                if let Some(count) = field_count {
                    let mut fields = HashMap::new();
                    for i in 0..count {
                        fields.insert(FieldIdx::new(i), InitState::Uninit);
                    }
                    self.addrs.insert(
                        *result,
                        AddrKind::SubField { ty: *ty, fields },
                    );
                } else {
                    // Non-struct type: whole tracking, starts uninit.
                    self.addrs.insert(*result, AddrKind::Whole(InitState::Uninit));
                }
            }
        }
    }

    /// Returns the number of fields for a named struct type, or None if not a struct.
    fn struct_field_count(&self, ty: TyId) -> Option<usize> {
        let mir_ty = self._module.ty_arena.get(ty);
        if let crate::ty::MirTy::Named { entity, .. } = mir_ty {
            let entity = *entity;
            for s in &self._module.structs {
                if s.entity == entity {
                    return Some(s.fields.len());
                }
            }
        }
        None
    }

    fn verify_terminator(&mut self, block: &crate::block::BasicBlock) {
        let term = &block.terminator.kind;

        // Collect all values forwarded as block args by the terminator.
        let mut forwarded: HashSet<ValueId> = HashSet::new();
        for (target, args) in term.successor_args() {
            let target_block = self.body.block(target);

            // Check 6: arg count must match target block param count.
            if args.len() != target_block.params.len() {
                self.err(
                    None,
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
                    self.err(
                        None,
                        format!(
                            "block arg {} to {:?}: type mismatch (value {:?} has {:?}, param expects {:?})",
                            i, target, arg_val, arg_def.ty, param.ty,
                        ),
                    );
                }
                if arg_def.ownership != param.ownership {
                    self.err(
                        None,
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
            }
            TerminatorKind::Switch { discriminant, .. } => {
                self.assert_live(*discriminant, None);
            }
            _ => {}
        }

        // Consume forwarded @owned values.
        for v in &forwarded {
            if self.body.value(*v).ownership == Ownership::Owned {
                self.try_consume(*v, None);
            }
        }

        // For Return, the returned value counts as consumed.
        if let TerminatorKind::Return(v) = term {
            self.assert_live(*v, None);
            if self.body.value(*v).ownership == Ownership::Owned {
                self.try_consume(*v, None);
            }
        }

        // Check 2: every @owned value must be Consumed or forwarded by now.
        let unconsumed: Vec<ValueId> = self
            .owned
            .iter()
            .filter(|(_, state)| **state == ValueState::Live)
            .map(|(&v, _)| v)
            .collect();
        for v in unconsumed {
            let vd = &self.body.values[v.index()];
            self.err(
                None,
                format!("@owned value {:?} is live at block exit but never consumed (ty={:?}, own={:?})", v, vd.ty, vd.ownership),
            );
        }

        // Check 4: every borrow must be ended or forwarded as @guaranteed block arg.
        let forwarded_borrows: HashSet<ValueId> = forwarded
            .iter()
            .copied()
            .filter(|v| self.body.value(*v).ownership == Ownership::Guaranteed)
            .collect();
        let open_borrows: Vec<ValueId> = self
            .borrows
            .keys()
            .filter(|bv| !forwarded_borrows.contains(bv))
            .copied()
            .collect();
        for borrow_val in open_borrows {
            self.err(
                None,
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

fn verify_block(
    body: &OssaBody,
    module: &MirModule,
    block_id: BlockId,
    func_name: &str,
    entity: Entity,
    errors: &mut Vec<VerifyError>,
) {
    let verifier = BlockVerifier::new(body, module, block_id, func_name, entity);
    let block_errors = verifier.verify();
    errors.extend(block_errors);
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
        def.type_info = TypeInfo { copy: CopyBehavior::None, ..TypeInfo::default() };
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
        def.type_info = TypeInfo { copy: CopyBehavior::None, ..TypeInfo::default() };
        b.add_struct(def);
        (ty, entity)
    }

    fn run_verify(b: OssaBuilder) -> Vec<VerifyError> {
        let (body, module) = b.finish();
        verify_ossa(&body, &module, "test", Entity::from_raw(0))
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
            vec![CallArg { value: borrow_val, convention: ParamConvention::Borrow }],
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
                value: x, ty: owned_ty, ownership: Ownership::Owned,
            });
            blk.params.push(crate::block::BlockParam {
                value: cond, ty: bool_ty, ownership: Ownership::Owned,
            });
        }

        let (bb1, bb1_params) = b.new_block_with_params(&[
            (owned_ty, Ownership::Owned),
            (bool_ty, Ownership::Owned),
        ]);
        let (bb2, bb2_params) = b.new_block_with_params(&[
            (owned_ty, Ownership::Owned),
            (bool_ty, Ownership::Owned),
        ]);

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
                value: x, ty: owned_ty, ownership: Ownership::Owned,
            });
        }

        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(!errors.is_empty(), "expected unconsumed error");
        assert!(
            errors.iter().any(|e| e.message.contains("live at block exit")),
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
                value: x, ty: owned_ty, ownership: Ownership::Owned,
            });
        }

        let _y = b.emit_copy_value(x);
        b.emit_destroy_value(x);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(!errors.is_empty(), "expected unconsumed error for y");
        assert!(
            errors.iter().any(|e| e.message.contains("live at block exit")),
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
            errors.iter().any(|e| e.message.contains("live at block exit")),
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
                value: x, ty: owned_ty, ownership: Ownership::Owned,
            });
        }

        b.emit_destroy_value(x);
        b.emit_destroy_value(x);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors.iter().any(|e| e.message.contains("consumed more than once")),
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
                value: x, ty: owned_ty, ownership: Ownership::Owned,
            });
        }

        let y = b.emit_move_value(x);
        b.emit_destroy_value(x);
        b.emit_destroy_value(y);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors.iter().any(|e| e.message.contains("consumed more than once")),
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
                value: x, ty: owned_ty, ownership: Ownership::Owned,
            });
        }

        let wrapper_entity = b.fresh_entity();
        let wrapper_ty = b.named(wrapper_entity, vec![]);
        let mut wrapper_def = StructDef::new(wrapper_entity, "Wrapper");
        wrapper_def.add_field(FieldDef::new("inner", owned_ty));
        wrapper_def.type_info = TypeInfo { copy: CopyBehavior::None, ..TypeInfo::default() };
        b.add_struct(wrapper_def);

        let s = b.emit_struct(wrapper_ty, vec![(FieldIdx::new(0), x)]);
        b.emit_destroy_value(x);
        b.emit_destroy_value(s);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors.iter().any(|e| e.message.contains("consumed more than once")),
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
                value: x, ty: owned_ty, ownership: Ownership::Owned,
            });
        }

        b.emit_destroy_value(x);
        let y = b.emit_copy_value(x);
        b.emit_destroy_value(y);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors.iter().any(|e| e.message.contains("use of consumed value") || e.message.contains("consumed more than once")),
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
                value: x, ty: owned_ty, ownership: Ownership::Owned,
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
            errors.iter().any(|e| e.message.contains("use of consumed value") || e.message.contains("consumed")),
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
                value: x, ty: owned_ty, ownership: Ownership::Owned,
            });
        }

        let callee = b.fresh_entity();
        b.emit_call(
            Callee::direct(callee),
            vec![CallArg { value: x, convention: ParamConvention::Consuming }],
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
                value: x, ty: owned_ty, ownership: Ownership::Owned,
            });
        }

        let _borrow = b.emit_begin_borrow(x);
        b.emit_destroy_value(x);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors.iter().any(|e| e.message.contains("still active at block exit")),
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
                value: x, ty: owned_ty, ownership: Ownership::Owned,
            });
        }

        let _mb = b.emit_begin_mut_borrow(x);
        b.emit_destroy_value(x);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors.iter().any(|e| e.message.contains("still active at block exit")),
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
                value: x, ty: owned_ty, ownership: Ownership::Owned,
            });
            blk.params.push(crate::block::BlockParam {
                value: x2, ty: owned_ty, ownership: Ownership::Owned,
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
            errors.iter().any(|e| e.message.contains("still active at block exit")),
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
                value: x, ty: owned_ty, ownership: Ownership::Owned,
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
                value: x, ty: owned_ty, ownership: Ownership::Owned,
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

    #[test]
    fn error_read_source_during_mut_borrow() {
        let mut b = OssaBuilder::new("test");
        let (owned_ty, _) = make_owned_type(&mut b);

        let entry = b.current_block();
        let x = b.new_value(owned_ty, Ownership::Owned);
        {
            let body = b.body_mut();
            body.block_mut(entry).params.push(crate::block::BlockParam {
                value: x, ty: owned_ty, ownership: Ownership::Owned,
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
            errors.iter().any(|e| e.message.contains("active mut borrow")),
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

        let (target, _params) = b.new_block_with_params(&[
            (i64_ty, Ownership::Owned),
            (i64_ty, Ownership::Owned),
        ]);

        let lit = b.emit_literal(Immediate::i64(42));
        b.emit_jump(target, vec![lit]);

        b.switch_to(target);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors.iter().any(|e| e.message.contains("passes 1 args") && e.message.contains("expects 2 params")),
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
            errors.iter().any(|e| e.message.contains("passes 2 args") && e.message.contains("expects 1 params")),
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
            body.block_mut(target).params.push(crate::block::BlockParam {
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
                value: x, ty: owned_ty, ownership: Ownership::Owned,
            });
        }

        b.emit_jump(target, vec![x]);

        b.switch_to(target);
        let unit = b.emit_literal(Immediate::unit());
        b.emit_return(unit);

        let errors = run_verify(b);
        assert!(
            errors.iter().any(|e| e.message.contains("ownership mismatch")),
            "expected ownership mismatch, got: {:?}",
            errors,
        );
    }

    // -----------------------------------------------------------------------
    // Category 10: Valid Uninit + FieldAddr + StoreInit + Take passes
    // -----------------------------------------------------------------------

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
                value: x, ty: owned_ty, ownership: Ownership::Owned,
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
            blk.insts.push(crate::inst::Instruction::new(InstKind::Literal {
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
                value: x, ty: owned_ty, ownership: Ownership::Owned,
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
}
