use kestrel_hecs::Entity;

use crate::block::BlockParam;
use crate::body::OssaBody;
use crate::callee::Callee;
use crate::immediate::Immediate;
use crate::inst::{CallArg, InstKind, Instruction};
use crate::item::enum_def::EnumDef;
use crate::item::protocol::ProtocolDef;
use crate::item::struct_def::StructDef;
use crate::terminator::{SwitchArm, Terminator, TerminatorKind};
use crate::ty::MirTy;
use crate::value::{Ownership, ValueDef};
use crate::{BlockId, FieldIdx, MirModule, Op, TyId, ValueId, VariantIdx};

/// Builder for constructing OSSA bodies programmatically.
pub struct OssaBuilder {
    module: MirModule,
    body: OssaBody,
    current_block: BlockId,
    next_entity: u32,
}

impl OssaBuilder {
    pub fn new(name: &str) -> Self {
        let module = MirModule::new(name);
        let mut body = OssaBody::new();
        let entry = body.alloc_block();
        body.entry = entry;
        Self {
            module,
            body,
            current_block: entry,
            next_entity: 1,
        }
    }

    pub fn fresh_entity(&mut self) -> Entity {
        let e = Entity::from_raw(self.next_entity);
        self.next_entity += 1;
        e
    }

    pub fn register_name(&mut self, entity: Entity, name: &str) {
        self.module.register_name(entity, name);
    }

    // -- Type interning --

    pub fn ty(&mut self, ty: MirTy) -> TyId {
        self.module.ty_arena.intern(ty)
    }
    pub fn i64(&mut self) -> TyId {
        self.module.ty_arena.i64()
    }
    pub fn i32(&mut self) -> TyId {
        self.module.ty_arena.i32()
    }
    pub fn bool(&mut self) -> TyId {
        self.module.ty_arena.bool()
    }
    pub fn unit(&mut self) -> TyId {
        self.module.ty_arena.unit()
    }
    pub fn str_ty(&mut self) -> TyId {
        self.module.ty_arena.str_ty()
    }
    pub fn never(&mut self) -> TyId {
        self.module.ty_arena.never()
    }
    pub fn pointer(&mut self, pointee: TyId) -> TyId {
        self.module.ty_arena.pointer(pointee)
    }
    pub fn named(&mut self, entity: Entity, type_args: Vec<TyId>) -> TyId {
        self.module.ty_arena.named(entity, type_args)
    }

    // -- Type metadata --

    pub fn add_struct(&mut self, def: StructDef) {
        self.module.add_struct(def);
    }
    pub fn add_enum(&mut self, def: EnumDef) {
        self.module.add_enum(def);
    }
    pub fn add_protocol(&mut self, def: ProtocolDef) {
        self.module.add_protocol(def);
    }

    // -- Value allocation --

    pub fn new_value(&mut self, ty: TyId, ownership: Ownership) -> ValueId {
        let def = match ownership {
            Ownership::Owned => ValueDef::owned(ty),
            Ownership::Guaranteed => panic!("use new_guaranteed_value for @guaranteed"),
        };
        self.body.alloc_value(def)
    }

    pub fn new_guaranteed_value(&mut self, ty: TyId, source: ValueId) -> ValueId {
        self.body.alloc_value(ValueDef::guaranteed(ty, source))
    }

    /// Allocate a value with ownership derived from type.
    pub fn new_value_auto(&mut self, ty: TyId) -> ValueId {
        self.new_value(ty, Ownership::Owned)
    }

    // -- Block management --

    pub fn new_block(&mut self) -> BlockId {
        self.body.alloc_block()
    }

    /// Create a block with typed, ownership-annotated params. Returns (block_id, param_value_ids).
    pub fn new_block_with_params(
        &mut self,
        params: &[(TyId, Ownership)],
    ) -> (BlockId, Vec<ValueId>) {
        let block = self.body.alloc_block();
        let mut values = Vec::new();
        for &(ty, ownership) in params {
            let def = match ownership {
                Ownership::Owned => ValueDef::owned(ty),
                Ownership::Guaranteed => panic!("use add_guaranteed_block_param for @guaranteed"),
            };
            let val = self.body.alloc_value(def);
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
        self.current_block = block;
    }

    pub fn current_block(&self) -> BlockId {
        self.current_block
    }

    // -- Instruction emission --

    fn emit(&mut self, kind: InstKind) {
        self.body
            .block_mut(self.current_block)
            .insts
            .push(Instruction::new(kind));
    }

    pub fn emit_copy_value(&mut self, operand: ValueId) -> ValueId {
        let ty = self.body.value(operand).ty;
        let result = self.new_value(ty, Ownership::Owned);
        self.emit(InstKind::CopyValue { result, operand });
        result
    }

    pub fn emit_move_value(&mut self, operand: ValueId) -> ValueId {
        let ty = self.body.value(operand).ty;
        let result = self.new_value(ty, Ownership::Owned);
        self.emit(InstKind::MoveValue { result, operand });
        result
    }

    pub fn emit_destroy_value(&mut self, operand: ValueId) {
        self.emit(InstKind::DestroyValue { operand });
    }

    pub fn emit_begin_borrow(&mut self, operand: ValueId) -> ValueId {
        let ty = self.body.value(operand).ty;
        let result = self.new_guaranteed_value(ty, operand);
        self.emit(InstKind::BeginBorrow { result, operand });
        result
    }

    pub fn emit_end_borrow(&mut self, operand: ValueId) {
        self.emit(InstKind::EndBorrow { operand });
    }

    pub fn emit_begin_mut_borrow(&mut self, operand: ValueId) -> ValueId {
        let ty = self.body.value(operand).ty;
        let result = self.new_guaranteed_value(ty, operand);
        self.emit(InstKind::BeginMutBorrow { result, operand });
        result
    }

    pub fn emit_end_mut_borrow(&mut self, operand: ValueId) {
        self.emit(InstKind::EndMutBorrow { operand });
    }

    pub fn emit_load(&mut self, address: ValueId, result_ty: TyId) -> ValueId {
        let result = self.new_value(result_ty, Ownership::Owned);
        self.emit(InstKind::Load { result, address });
        result
    }

    pub fn emit_copy_addr(&mut self, address: ValueId, ty: TyId) -> ValueId {
        let result = self.new_value(ty, Ownership::Owned);
        self.emit(InstKind::CopyAddr {
            result,
            address,
            ty,
        });
        result
    }

    pub fn emit_take(&mut self, address: ValueId, ty: TyId) -> ValueId {
        let result = self.new_value(ty, Ownership::Owned);
        self.emit(InstKind::Take {
            result,
            address,
            ty,
        });
        result
    }

    pub fn emit_begin_borrow_addr(&mut self, address: ValueId, ty: TyId) -> ValueId {
        let result = self.new_guaranteed_value(ty, address);
        self.emit(InstKind::BeginBorrowAddr {
            result,
            address,
            ty,
        });
        result
    }

    pub fn emit_begin_mut_borrow_addr(&mut self, address: ValueId, ty: TyId) -> ValueId {
        let result = self.new_guaranteed_value(ty, address);
        self.emit(InstKind::BeginMutBorrowAddr {
            result,
            address,
            ty,
        });
        result
    }

    pub fn emit_store_init(&mut self, address: ValueId, value: ValueId) {
        self.emit(InstKind::StoreInit { address, value });
    }

    pub fn emit_store_assign(&mut self, address: ValueId, value: ValueId) {
        self.emit(InstKind::StoreAssign { address, value });
    }

    pub fn emit_destroy_addr(&mut self, address: ValueId, ty: TyId) {
        self.emit(InstKind::DestroyAddr { address, ty });
    }

    pub fn emit_discriminant(&mut self, operand: ValueId) -> ValueId {
        let i32_ty = self.i32();
        let result = self.new_value(i32_ty, Ownership::Owned);
        self.emit(InstKind::Discriminant { result, operand });
        result
    }

    pub fn emit_op1(&mut self, op: Op, arg: ValueId, result_ty: TyId) -> ValueId {
        let result = self.new_value(result_ty, Ownership::Owned);
        self.emit(InstKind::Op1 { result, op, arg });
        result
    }

    pub fn emit_op2(&mut self, op: Op, lhs: ValueId, rhs: ValueId, result_ty: TyId) -> ValueId {
        let result = self.new_value(result_ty, Ownership::Owned);
        self.emit(InstKind::Op2 {
            result,
            op,
            lhs,
            rhs,
        });
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
        let result = self.new_value(result_ty, Ownership::Owned);
        self.emit(InstKind::Op3 {
            result,
            op,
            a,
            b,
            c,
        });
        result
    }

    pub fn emit_literal(&mut self, value: Immediate) -> ValueId {
        let ty = value.ty(&mut self.module.ty_arena);
        let result = self.new_value(ty, Ownership::Owned);
        self.emit(InstKind::Literal { result, value });
        result
    }

    pub fn emit_global_ref(&mut self, entity: Entity) -> ValueId {
        let i64_ty = self.i64();
        let result = self.new_value(i64_ty, Ownership::Owned);
        self.emit(InstKind::GlobalRef { result, entity });
        result
    }

    pub fn emit_struct(&mut self, ty: TyId, fields: Vec<(FieldIdx, ValueId)>) -> ValueId {
        let result = self.new_value(ty, Ownership::Owned);
        self.emit(InstKind::Struct { result, ty, fields });
        result
    }

    pub fn emit_tuple(&mut self, ty: TyId, elements: Vec<ValueId>) -> ValueId {
        let result = self.new_value(ty, Ownership::Owned);
        self.emit(InstKind::Tuple { result, elements });
        result
    }

    pub fn emit_enum(
        &mut self,
        enum_ty: TyId,
        variant: VariantIdx,
        payload: Vec<ValueId>,
    ) -> ValueId {
        let result = self.new_value(enum_ty, Ownership::Owned);
        self.emit(InstKind::Enum {
            result,
            enum_ty,
            variant,
            payload,
        });
        result
    }

    pub fn emit_array(
        &mut self,
        element_ty: TyId,
        elements: Vec<ValueId>,
        array_ty: TyId,
    ) -> ValueId {
        let result = self.new_value(array_ty, Ownership::Owned);
        self.emit(InstKind::Array {
            result,
            element_ty,
            elements,
        });
        result
    }

    pub fn emit_struct_extract(
        &mut self,
        operand: ValueId,
        field: FieldIdx,
        result_ty: TyId,
    ) -> ValueId {
        let result = self.new_value(result_ty, Ownership::Owned);
        self.emit(InstKind::StructExtract {
            result,
            operand,
            field,
        });
        result
    }

    pub fn emit_tuple_extract(&mut self, operand: ValueId, index: u32, result_ty: TyId) -> ValueId {
        let result = self.new_value(result_ty, Ownership::Owned);
        self.emit(InstKind::TupleExtract {
            result,
            operand,
            index,
        });
        result
    }

    pub fn emit_enum_payload(
        &mut self,
        operand: ValueId,
        variant: VariantIdx,
        field: FieldIdx,
        result_ty: TyId,
    ) -> ValueId {
        let result = self.new_value(result_ty, Ownership::Owned);
        self.emit(InstKind::EnumPayload {
            result,
            operand,
            variant,
            field,
        });
        result
    }

    pub fn emit_destructure_struct(
        &mut self,
        operand: ValueId,
        field_types: &[(TyId, Ownership)],
    ) -> Vec<ValueId> {
        let results: Vec<ValueId> = field_types
            .iter()
            .map(|&(ty, ownership)| self.new_value(ty, ownership))
            .collect();
        self.emit(InstKind::DestructureStruct {
            results: results.clone(),
            operand,
        });
        results
    }

    pub fn emit_destructure_tuple(
        &mut self,
        operand: ValueId,
        elem_types: &[(TyId, Ownership)],
    ) -> Vec<ValueId> {
        let results: Vec<ValueId> = elem_types
            .iter()
            .map(|&(ty, ownership)| self.new_value(ty, ownership))
            .collect();
        self.emit(InstKind::DestructureTuple {
            results: results.clone(),
            operand,
        });
        results
    }

    pub fn emit_destructure_enum(
        &mut self,
        operand: ValueId,
        variant: VariantIdx,
        field_types: &[(TyId, Ownership)],
    ) -> Vec<ValueId> {
        let results: Vec<ValueId> = field_types
            .iter()
            .map(|&(ty, ownership)| self.new_value(ty, ownership))
            .collect();
        self.emit(InstKind::DestructureEnum {
            results: results.clone(),
            operand,
            variant,
        });
        results
    }

    pub fn emit_call(
        &mut self,
        callee: Callee,
        args: Vec<CallArg>,
        result_ty: Option<(TyId, Ownership)>,
    ) -> Option<ValueId> {
        let result = result_ty.map(|(ty, ownership)| self.new_value(ty, ownership));
        self.emit(InstKind::Call {
            result,
            callee,
            args,
        });
        result
    }

    pub fn emit_apply_partial(
        &mut self,
        callee: Callee,
        captures: Vec<ValueId>,
        result_ty: TyId,
    ) -> ValueId {
        let result = self.new_value(result_ty, Ownership::Owned);
        self.emit(InstKind::ApplyPartial {
            result,
            callee,
            captures,
        });
        result
    }

    pub fn emit_field_addr(&mut self, base: ValueId, ty: TyId, field: FieldIdx) -> ValueId {
        let ptr_ty = self.pointer(ty);
        let result = self.new_value(ptr_ty, Ownership::Owned);
        self.emit(InstKind::FieldAddr {
            result,
            base,
            ty,
            field,
        });
        result
    }

    pub fn emit_uninit(&mut self, ty: TyId) -> ValueId {
        let ptr_ty = self.pointer(ty);
        let result = self.new_value(ptr_ty, Ownership::Owned);
        self.emit(InstKind::Uninit { result, ty });
        result
    }

    // -- Terminators --

    pub fn emit_return(&mut self, value: ValueId) {
        self.body.block_mut(self.current_block).terminator =
            Terminator::new(TerminatorKind::Return(value));
    }

    pub fn emit_jump(&mut self, target: BlockId, args: Vec<ValueId>) {
        self.body.block_mut(self.current_block).terminator =
            Terminator::new(TerminatorKind::Jump { target, args });
    }

    pub fn emit_branch(
        &mut self,
        condition: ValueId,
        then_block: BlockId,
        then_args: Vec<ValueId>,
        else_block: BlockId,
        else_args: Vec<ValueId>,
    ) {
        self.body.block_mut(self.current_block).terminator =
            Terminator::new(TerminatorKind::Branch {
                condition,
                then_block,
                then_args,
                else_block,
                else_args,
            });
    }

    pub fn emit_switch(&mut self, discriminant: ValueId, cases: Vec<SwitchArm>) {
        self.body.block_mut(self.current_block).terminator =
            Terminator::new(TerminatorKind::Switch {
                discriminant,
                cases,
            });
    }

    pub fn emit_panic(&mut self, message: impl Into<String>) {
        self.body.block_mut(self.current_block).terminator =
            Terminator::new(TerminatorKind::Panic(message.into()));
    }

    pub fn emit_unreachable(&mut self) {
        self.body.block_mut(self.current_block).terminator =
            Terminator::new(TerminatorKind::Unreachable);
    }

    // -- Finalize --

    pub fn finish(self) -> (OssaBody, MirModule) {
        (self.body, self.module)
    }

    /// Access the module for type queries during building.
    pub fn module(&self) -> &MirModule {
        &self.module
    }

    /// Access the body for inspection during building.
    pub fn body(&self) -> &OssaBody {
        &self.body
    }

    /// Access the body mutably for advanced test scenarios.
    pub fn body_mut(&mut self) -> &mut OssaBody {
        &mut self.body
    }
}
