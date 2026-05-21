use kestrel_hecs::Entity;

use crate::body::{BasicBlock, LocalDef, MirBody};
use crate::immediate::Immediate;
use crate::item::enum_def::EnumDef;
use crate::item::function::{FunctionDef, FunctionKind, ParamDef};
use crate::item::protocol::ProtocolDef;
use crate::item::static_def::StaticDef;
use crate::item::struct_def::StructDef;
use crate::item::witness::WitnessDef;
use crate::operand::{ArgMode, UseMode};
use crate::statement::{Callee, Rvalue, Statement, StatementKind};
use crate::terminator::{SwitchCase, Terminator, TerminatorKind};
use crate::ty::{MirTy, ParamConvention, TyArena};
use crate::{BlockId, FieldIdx, FunctionIdx, LocalId, MirModule, Operand, Op, Place, TyId, VariantIdx};

pub struct ModuleBuilder {
    module: MirModule,
    next_entity: u32,
}

impl ModuleBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            module: MirModule::new(name),
            next_entity: 1,
        }
    }

    /// Allocate a fresh entity for test use. Starts at 1 and increments.
    /// These IDs will collide with real ECS entities — only use in tests
    /// or self-contained contexts where no ECS world is involved.
    pub fn fresh_entity(&mut self) -> Entity {
        let e = Entity::from_raw(self.next_entity);
        self.next_entity += 1;
        e
    }

    pub fn arena(&self) -> &TyArena {
        &self.module.ty_arena
    }

    pub fn arena_mut(&mut self) -> &mut TyArena {
        &mut self.module.ty_arena
    }

    pub fn register_name(&mut self, entity: Entity, name: &str) {
        self.module.register_name(entity, name);
    }

    // Type interning shortcuts

    pub fn ty(&mut self, ty: MirTy) -> TyId {
        self.module.ty_arena.intern(ty)
    }

    pub fn i8(&mut self) -> TyId {
        self.module.ty_arena.i8()
    }

    pub fn i16(&mut self) -> TyId {
        self.module.ty_arena.i16()
    }

    pub fn i32(&mut self) -> TyId {
        self.module.ty_arena.i32()
    }

    pub fn i64(&mut self) -> TyId {
        self.module.ty_arena.i64()
    }

    pub fn f32(&mut self) -> TyId {
        self.module.ty_arena.f32()
    }

    pub fn f64(&mut self) -> TyId {
        self.module.ty_arena.f64()
    }

    pub fn bool(&mut self) -> TyId {
        self.module.ty_arena.bool()
    }

    pub fn unit(&mut self) -> TyId {
        self.module.ty_arena.unit()
    }

    pub fn pointer(&mut self, pointee: TyId) -> TyId {
        self.module.ty_arena.pointer(pointee)
    }

    pub fn named(&mut self, entity: Entity, type_args: Vec<TyId>) -> TyId {
        self.module.ty_arena.named(entity, type_args)
    }

    // Item builders

    pub fn add_struct(&mut self, def: StructDef) -> crate::StructIdx {
        self.module.add_struct(def)
    }

    pub fn add_enum(&mut self, def: EnumDef) -> crate::EnumIdx {
        self.module.add_enum(def)
    }

    pub fn add_protocol(&mut self, def: ProtocolDef) -> crate::ProtocolIdx {
        self.module.add_protocol(def)
    }

    pub fn add_witness(&mut self, def: WitnessDef) -> crate::WitnessIdx {
        self.module.add_witness(def)
    }

    pub fn add_static(&mut self, def: StaticDef) -> crate::StaticIdx {
        self.module.add_static(def)
    }

    pub fn function(&mut self, name: &str, ret: TyId) -> FunctionBuilder<'_> {
        let entity = self.fresh_entity();
        self.module.register_name(entity, name);
        let func = FunctionDef::new(entity, name, ret);
        let idx = self.module.add_function(func);
        self.module.functions[idx.index()].body = Some(MirBody::new());
        FunctionBuilder {
            module: &mut self.module,
            func_idx: idx,
            temp_counter: 0,
        }
    }

    pub fn function_with_entity(
        &mut self,
        entity: Entity,
        name: &str,
        ret: TyId,
    ) -> FunctionBuilder<'_> {
        self.module.register_name(entity, name);
        let func = FunctionDef::new(entity, name, ret);
        let idx = self.module.add_function(func);
        self.module.functions[idx.index()].body = Some(MirBody::new());
        FunctionBuilder {
            module: &mut self.module,
            func_idx: idx,
            temp_counter: 0,
        }
    }

    pub fn finish(self) -> MirModule {
        self.module
    }
}

pub struct FunctionBuilder<'a> {
    module: &'a mut MirModule,
    func_idx: FunctionIdx,
    temp_counter: u32,
}

impl<'a> FunctionBuilder<'a> {
    pub fn entity(&self) -> Entity {
        self.module.functions[self.func_idx.index()].entity
    }

    pub fn index(&self) -> crate::FunctionIdx {
        self.func_idx
    }

    fn body_mut(&mut self) -> &mut MirBody {
        self.module.functions[self.func_idx.index()]
            .body
            .as_mut()
            .expect("function body should be initialized")
    }

    pub fn set_kind(&mut self, kind: FunctionKind) {
        self.module.functions[self.func_idx.index()].kind = kind;
    }

    pub fn param(
        &mut self,
        name: &str,
        ty: TyId,
        convention: ParamConvention,
    ) -> LocalId {
        // Local type = logical type regardless of convention (Option B)
        let local_id = self.body_mut().add_local(LocalDef::new(name, ty));
        self.body_mut().param_count += 1;
        let param = ParamDef::new(name, local_id, ty, convention);
        self.module.functions[self.func_idx.index()].params.push(param);
        local_id
    }

    pub fn local(&mut self, name: &str, ty: TyId) -> LocalId {
        self.body_mut().add_local(LocalDef::new(name, ty))
    }

    pub fn temp(&mut self, ty: TyId) -> LocalId {
        let name = format!("_t{}", self.temp_counter);
        self.temp_counter += 1;
        self.body_mut().add_local(LocalDef::new(name, ty))
    }

    pub fn block(&mut self) -> BlockBuilder<'_> {
        let id = self.body_mut().add_block(BasicBlock::new());
        BlockBuilder {
            module: self.module,
            func_idx: self.func_idx,
            block_id: id,
        }
    }

    pub fn block_id(&mut self) -> BlockId {
        self.body_mut().add_block(BasicBlock::new())
    }

    pub fn block_at(&mut self, id: BlockId) -> BlockBuilder<'_> {
        BlockBuilder {
            module: self.module,
            func_idx: self.func_idx,
            block_id: id,
        }
    }
}

pub struct BlockBuilder<'a> {
    module: &'a mut MirModule,
    func_idx: FunctionIdx,
    block_id: BlockId,
}

impl<'a> BlockBuilder<'a> {
    pub fn id(&self) -> BlockId {
        self.block_id
    }

    fn block_mut(&mut self) -> &mut BasicBlock {
        self.module.functions[self.func_idx.index()]
            .body
            .as_mut()
            .expect("body")
            .block_mut(self.block_id)
    }

    fn push_stmt(&mut self, kind: StatementKind) {
        self.block_mut()
            .stmts
            .push(Statement::new(kind));
    }

    // Assignments

    pub fn assign(&mut self, dest: Place, rvalue: Rvalue) {
        self.push_stmt(StatementKind::Assign { dest, rvalue });
    }

    pub fn use_copy(&mut self, dest: Place, src: Place) {
        self.assign(dest, Rvalue::Use(Operand::Place(src), UseMode::Copy));
    }

    pub fn use_move(&mut self, dest: Place, src: Place) {
        self.assign(dest, Rvalue::Use(Operand::Place(src), UseMode::Move));
    }

    pub fn assign_ref(&mut self, dest: Place, src: Place) {
        self.assign(dest, Rvalue::Ref(src));
    }

    pub fn assign_ref_mut(&mut self, dest: Place, src: Place) {
        self.assign(dest, Rvalue::RefMut(src));
    }

    pub fn assign_const(&mut self, dest: Place, imm: Immediate) {
        self.assign(dest, Rvalue::Use(Operand::Const(imm), UseMode::Copy));
    }

    pub fn assign_op1(&mut self, dest: Place, op: Op, arg: Operand) {
        self.assign(dest, Rvalue::Op1 { op, arg });
    }

    pub fn assign_op2(&mut self, dest: Place, op: Op, lhs: Operand, rhs: Operand) {
        self.assign(dest, Rvalue::Op2 { op, lhs, rhs });
    }

    pub fn assign_op3(&mut self, dest: Place, op: Op, a: Operand, b: Operand, c: Operand) {
        self.assign(dest, Rvalue::Op3 { op, a, b, c });
    }

    pub fn assign_construct(
        &mut self,
        dest: Place,
        ty: TyId,
        fields: Vec<(FieldIdx, Operand, UseMode)>,
    ) {
        self.assign(dest, Rvalue::Construct { ty, fields });
    }

    pub fn assign_tuple(&mut self, dest: Place, elems: Vec<(Operand, UseMode)>) {
        self.assign(dest, Rvalue::Tuple(elems));
    }

    pub fn assign_enum(
        &mut self,
        dest: Place,
        enum_ty: TyId,
        variant: VariantIdx,
        payload: Vec<(Operand, UseMode)>,
    ) {
        self.assign(
            dest,
            Rvalue::EnumVariant {
                enum_ty,
                variant,
                payload,
            },
        );
    }

    // Calls

    pub fn call(&mut self, dest: Option<Place>, callee: Callee, args: Vec<(Operand, ArgMode)>) {
        self.push_stmt(StatementKind::Call { dest, callee, args });
    }

    pub fn call_direct(
        &mut self,
        dest: Option<Place>,
        func: Entity,
        args: Vec<(Operand, ArgMode)>,
    ) {
        self.call(dest, Callee::direct(func), args);
    }

    // Drop statements

    pub fn drop(&mut self, place: Place) {
        self.push_stmt(StatementKind::Drop { place });
    }

    pub fn drop_if(&mut self, place: Place, flag: LocalId) {
        self.push_stmt(StatementKind::DropIf { place, flag });
    }

    pub fn set_drop_flag(&mut self, flag: LocalId, value: bool) {
        self.push_stmt(StatementKind::SetDropFlag { flag, value });
    }

    pub fn scope_live(&mut self, local: LocalId) {
        self.push_stmt(StatementKind::ScopeLive(local));
    }

    // Terminators

    pub fn ret(&mut self, operand: Operand) {
        self.block_mut().terminator = Terminator::ret(operand);
    }

    pub fn ret_unit(&mut self) {
        self.ret(Operand::Const(Immediate::unit()));
    }

    pub fn jump(&mut self, target: BlockId) {
        self.block_mut().terminator = Terminator::jump(target);
    }

    pub fn branch(&mut self, cond: Operand, then_block: BlockId, else_block: BlockId) {
        self.block_mut().terminator = Terminator::branch(cond, then_block, else_block);
    }

    pub fn switch(&mut self, disc: Place, cases: Vec<(SwitchCase, BlockId)>) {
        self.block_mut().terminator = Terminator::switch(disc, cases);
    }

    pub fn panic(&mut self, msg: &str) {
        self.block_mut().terminator = Terminator::panic(msg);
    }

    pub fn unreachable(&mut self) {
        self.block_mut().terminator = Terminator::unreachable();
    }

    pub fn terminate(&mut self, kind: TerminatorKind) {
        self.block_mut().terminator = Terminator::new(kind);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{IntBits, Signedness};

    #[test]
    fn basic_function() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();

        let mut f = m.function("example.add", i64_ty);
        let x = f.param("x", i64_ty, ParamConvention::Consuming);
        let y = f.param("y", i64_ty, ParamConvention::Consuming);
        let result = f.local("result", i64_ty);

        {
            let mut bb = f.block();
            bb.assign_op2(
                Place::local(result),
                Op::Add(IntBits::I64, Signedness::Signed),
                Operand::Place(Place::local(x)),
                Operand::Place(Place::local(y)),
            );
            bb.ret(Operand::Place(Place::local(result)));
        }

        let module = m.finish();
        assert_eq!(module.functions.len(), 1);
        let body = module.functions[0].body.as_ref().unwrap();
        assert_eq!(body.locals.len(), 3);
        assert_eq!(body.blocks.len(), 1);
        assert_eq!(body.param_count, 2);
    }

    #[test]
    fn control_flow() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let bool_ty = m.bool();

        let mut f = m.function("example.abs", i64_ty);
        let x = f.param("x", i64_ty, ParamConvention::Consuming);
        let result = f.local("result", i64_ty);
        let is_neg = f.local("is_neg", bool_ty);

        let bb0 = f.block_id();
        let bb_neg = f.block_id();
        let bb_pos = f.block_id();
        let bb_ret = f.block_id();

        {
            let mut b = f.block_at(bb0);
            b.assign_op2(
                Place::local(is_neg),
                Op::Lt(IntBits::I64, Signedness::Signed),
                Operand::Place(Place::local(x)),
                Operand::Const(Immediate::i64(0)),
            );
            b.branch(Operand::Place(Place::local(is_neg)), bb_neg, bb_pos);
        }
        {
            let mut b = f.block_at(bb_neg);
            b.assign_op1(
                Place::local(result),
                Op::Neg(IntBits::I64),
                Operand::Place(Place::local(x)),
            );
            b.jump(bb_ret);
        }
        {
            let mut b = f.block_at(bb_pos);
            b.use_copy(Place::local(result), Place::local(x));
            b.jump(bb_ret);
        }
        {
            let mut b = f.block_at(bb_ret);
            b.ret(Operand::Place(Place::local(result)));
        }

        let module = m.finish();
        let body = module.functions[0].body.as_ref().unwrap();
        assert_eq!(body.blocks.len(), 4);
    }

    #[test]
    fn construct_struct() {
        let mut m = ModuleBuilder::new("test");
        let _i64_ty = m.i64();
        let point_entity = m.fresh_entity();
        let point_ty = m.named(point_entity, vec![]);

        let mut f = m.function("make_point", point_ty);
        let result = f.local("result", point_ty);

        {
            let mut bb = f.block();
            bb.assign_construct(
                Place::local(result),
                point_ty,
                vec![
                    (FieldIdx::new(0), Operand::Const(Immediate::i64(1)), UseMode::Copy),
                    (FieldIdx::new(1), Operand::Const(Immediate::i64(2)), UseMode::Copy),
                ],
            );
            bb.ret(Operand::Place(Place::local(result)));
        }

        let module = m.finish();
        let body = module.functions[0].body.as_ref().unwrap();
        assert_eq!(body.blocks[0].stmts.len(), 1);
    }

    #[test]
    fn call_with_arg_modes() {
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        let i64_ty = m.i64();
        let callee_entity = m.fresh_entity();
        m.register_name(callee_entity, "print_num");

        let mut f = m.function("caller", unit_ty);
        let x = f.param("x", i64_ty, ParamConvention::Consuming);

        {
            let mut bb = f.block();
            bb.call_direct(
                None,
                callee_entity,
                vec![(Operand::Place(Place::local(x)), ArgMode::Ref)],
            );
            bb.ret_unit();
        }

        let module = m.finish();
        let body = module.functions[0].body.as_ref().unwrap();
        match &body.blocks[0].stmts[0].kind {
            StatementKind::Call { args, .. } => {
                assert_eq!(args[0].1, ArgMode::Ref);
            }
            _ => panic!("expected Call"),
        }
    }

    #[test]
    fn fresh_entity_increments() {
        let mut m = ModuleBuilder::new("test");
        let e1 = m.fresh_entity();
        let e2 = m.fresh_entity();
        let e3 = m.fresh_entity();
        assert_ne!(e1, e2);
        assert_ne!(e2, e3);
        assert_eq!(e1, Entity::from_raw(1));
        assert_eq!(e2, Entity::from_raw(2));
        assert_eq!(e3, Entity::from_raw(3));
    }

    #[test]
    fn temp_unique_names() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();

        let mut f = m.function("foo", i64_ty);
        let t0 = f.temp(i64_ty);
        let t1 = f.temp(i64_ty);
        assert_ne!(t0, t1);

        let module = m.finish();
        let body = module.functions[0].body.as_ref().unwrap();
        assert_eq!(body.local(t0).name, "_t0");
        assert_eq!(body.local(t1).name, "_t1");
    }

    #[test]
    fn borrow_param_gets_pointer_type() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let unit_ty = m.unit();

        let mut f = m.function("reader", unit_ty);
        let x = f.param("x", i64_ty, ParamConvention::Borrow);

        let module = m.finish();
        let body = module.functions[0].body.as_ref().unwrap();
        // Option B: local type = logical type, not Pointer-wrapped
        assert_eq!(body.local(x).ty, i64_ty);

        let param = &module.functions[0].params[0];
        assert_eq!(param.ty, i64_ty);
        assert_eq!(param.convention, ParamConvention::Borrow);
    }

    #[test]
    fn display_integration() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();

        let mut f = m.function("main", i64_ty);
        let x = f.param("x", i64_ty, ParamConvention::Consuming);
        {
            let mut bb = f.block();
            bb.ret(Operand::Place(Place::local(x)));
        }

        let module = m.finish();
        let output = module.display().to_string();
        assert!(output.contains("fn main"));
        assert!(output.contains("x: i64"));
    }
}
