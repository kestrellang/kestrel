//! Builders for constructing MIR functions and blocks.

use crate::MirModule;
use crate::body::{BasicBlock, LocalDef, MirBody};
use crate::id::{BlockId, FunctionId, LocalId};
use crate::immediate::Immediate;
use crate::op::Op;
use crate::place::Place;
use crate::statement::{CallArg, Callee, Rvalue, Statement, StatementKind};
use crate::terminator::Terminator;
use crate::ty::MirTy;
use crate::value::Value;
use crate::item::{ParamDef, WhereClause, WhereConstraint};
use kestrel_hecs::Entity;

/// Builder for constructing functions.
pub struct FunctionBuilder<'a> {
    module: &'a mut MirModule,
    id: FunctionId,
}

impl<'a> FunctionBuilder<'a> {
    pub(crate) fn new(module: &'a mut MirModule, id: FunctionId) -> Self {
        // Ensure the function has a body
        let func = &mut module.functions[id.index()];
        if func.body.is_none() {
            func.body = Some(MirBody::new());
        }
        Self { module, id }
    }

    /// Get the function ID.
    pub fn id(&self) -> FunctionId {
        self.id
    }

    /// Access the function's body mutably.
    fn body_mut(&mut self) -> &mut MirBody {
        self.module.functions[self.id.index()]
            .body
            .as_mut()
            .expect("function body should be initialized")
    }

    /// Add a type parameter.
    pub fn type_param(&mut self, entity: Entity, name: impl Into<String>) {
        let tp = crate::item::TypeParamDef::new(entity, name);
        self.module.functions[self.id.index()].type_params.push(tp);
    }

    /// Add a parameter and return the local ID it's bound to.
    pub fn param(&mut self, name: impl Into<String>, ty: MirTy) -> LocalId {
        self.param_with_label(name, ty, None)
    }

    /// Add a parameter with an optional external label.
    pub fn param_with_label(
        &mut self,
        name: impl Into<String>,
        ty: MirTy,
        label: Option<String>,
    ) -> LocalId {
        let name = name.into();
        let local = LocalDef::new(name.clone(), ty.clone());
        let local_id = self.body_mut().add_local(local);
        self.body_mut().param_count += 1;

        let param = ParamDef::with_label(name.clone(), local_id, ty, label);
        let func = &mut self.module.functions[self.id.index()];
        let param_idx = func.params.len();
        func.params_by_name.insert(name.clone(), param_idx);
        func.params.push(param);
        func.locals_by_name.insert(name, local_id);

        local_id
    }

    /// Add a local variable and return its ID.
    pub fn local(&mut self, name: impl Into<String>, ty: MirTy) -> LocalId {
        let name = name.into();
        let local = LocalDef::new(name.clone(), ty);
        let local_id = self.body_mut().add_local(local);
        self.module.functions[self.id.index()]
            .locals_by_name
            .insert(name, local_id);
        local_id
    }

    /// Set the where clause.
    pub fn set_where_clause(&mut self, wc: WhereClause) {
        self.module.functions[self.id.index()].where_clause = Some(wc);
    }

    /// Add a where clause constraint.
    pub fn add_constraint(&mut self, constraint: WhereConstraint) {
        let func = &mut self.module.functions[self.id.index()];
        if func.where_clause.is_none() {
            func.where_clause = Some(WhereClause::new());
        }
        func.where_clause.as_mut().unwrap().add_constraint(constraint);
    }

    /// Add a new basic block. Sets as entry block on first call.
    pub fn add_block(&mut self) -> BlockBuilder<'_> {
        let block = BasicBlock::new();
        let block_id = self.body_mut().add_block(block);

        // First block is the entry block
        if self.body_mut().blocks.len() == 1 {
            self.body_mut().entry = block_id;
        }

        BlockBuilder {
            module: self.module,
            func_id: self.id,
            block_id,
        }
    }

    /// Get a builder for an existing block.
    pub fn block(&mut self, block_id: BlockId) -> BlockBuilder<'_> {
        BlockBuilder {
            module: self.module,
            func_id: self.id,
            block_id,
        }
    }

    /// Get the entry block ID.
    pub fn entry_block(&self) -> BlockId {
        self.module.functions[self.id.index()]
            .body
            .as_ref()
            .expect("function body should be initialized")
            .entry
    }
}

/// Builder for constructing basic blocks.
pub struct BlockBuilder<'a> {
    module: &'a mut MirModule,
    func_id: FunctionId,
    block_id: BlockId,
}

impl<'a> BlockBuilder<'a> {
    /// Get the block ID.
    pub fn id(&self) -> BlockId {
        self.block_id
    }

    /// Access the block mutably.
    fn block_mut(&mut self) -> &mut BasicBlock {
        self.module.functions[self.func_id.index()]
            .body
            .as_mut()
            .expect("function body should be initialized")
            .block_mut(self.block_id)
    }

    // === Statements ===

    /// Add a raw statement.
    pub fn add_statement(&mut self, stmt: Statement) {
        self.block_mut().stmts.push(stmt);
    }

    /// Add an assignment: `dest = rvalue`
    pub fn assign(&mut self, dest: Place, rvalue: Rvalue) {
        self.add_statement(Statement::new(StatementKind::Assign { dest, rvalue }));
    }

    /// `dest = move src`
    pub fn assign_move(&mut self, dest: Place, src: Place) {
        self.assign(dest, Rvalue::Move(src));
    }

    /// `dest = copy src`
    pub fn assign_copy(&mut self, dest: Place, src: Place) {
        self.assign(dest, Rvalue::Copy(src));
    }

    /// `dest = ref src`
    pub fn assign_ref(&mut self, dest: Place, src: Place) {
        self.assign(dest, Rvalue::Ref(src));
    }

    /// `dest = ref var src`
    pub fn assign_ref_mut(&mut self, dest: Place, src: Place) {
        self.assign(dest, Rvalue::RefMut(src));
    }

    /// `dest = <immediate>`
    pub fn assign_const(&mut self, dest: Place, imm: Immediate) {
        self.assign(dest, Rvalue::Const(imm));
    }

    /// `dest = op1 arg`
    pub fn assign_op1(&mut self, dest: Place, op: Op, arg: impl Into<Value>) {
        self.assign(dest, Rvalue::Op1 { op, arg: arg.into() });
    }

    /// `dest = op2 lhs, rhs`
    pub fn assign_op2(
        &mut self,
        dest: Place,
        op: Op,
        lhs: impl Into<Value>,
        rhs: impl Into<Value>,
    ) {
        self.assign(
            dest,
            Rvalue::Op2 {
                op,
                lhs: lhs.into(),
                rhs: rhs.into(),
            },
        );
    }

    /// `dest = op3 a, b, c`
    pub fn assign_op3(
        &mut self,
        dest: Place,
        op: Op,
        a: impl Into<Value>,
        b: impl Into<Value>,
        c: impl Into<Value>,
    ) {
        self.assign(
            dest,
            Rvalue::Op3 {
                op,
                a: a.into(),
                b: b.into(),
                c: c.into(),
            },
        );
    }

    /// `dest = construct Type { fields... }`
    pub fn assign_construct(&mut self, dest: Place, ty: MirTy, fields: Vec<(String, Value)>) {
        self.assign(dest, Rvalue::Construct { ty, fields });
    }

    /// `[dest =] call callee(args...)`
    pub fn call(&mut self, dest: Option<Place>, callee: Callee, args: Vec<CallArg>) {
        self.add_statement(Statement::new(StatementKind::Call { dest, callee, args }));
    }

    /// `dest = call func(args...)` — convenience for direct calls with borrow mode.
    pub fn call_direct(&mut self, dest: Option<Place>, func: Entity, args: Vec<Value>) {
        let call_args: Vec<CallArg> = args.into_iter().map(CallArg::borrow).collect();
        self.call(dest, Callee::direct(func), call_args);
    }

    /// `deinit <place>`
    pub fn deinit(&mut self, place: Place) {
        self.add_statement(Statement::new(StatementKind::Deinit { place }));
    }

    /// `deinit <place> if <flag>`
    pub fn deinit_if(&mut self, place: Place, flag: LocalId) {
        self.add_statement(Statement::new(StatementKind::DeinitIf { place, flag }));
    }

    /// `<flag> = true/false`
    pub fn set_deinit_flag(&mut self, flag: LocalId, value: bool) {
        self.add_statement(Statement::new(StatementKind::SetDeinitFlag { flag, value }));
    }

    // === Terminators ===

    /// Set the block's terminator.
    pub fn terminate(&mut self, term: Terminator) {
        self.block_mut().terminator = term;
    }

    /// `return <value>`
    pub fn ret(&mut self, value: impl Into<Value>) {
        self.terminate(Terminator::ret(value));
    }

    /// `return ()`
    pub fn ret_unit(&mut self) {
        self.ret(Immediate::unit());
    }

    /// `jump bb`
    pub fn jump(&mut self, target: BlockId) {
        self.terminate(Terminator::jump(target));
    }

    /// `branch if <cond>, bb_true else bb_false`
    pub fn branch(
        &mut self,
        condition: impl Into<Value>,
        then_block: BlockId,
        else_block: BlockId,
    ) {
        self.terminate(Terminator::branch(condition, then_block, else_block));
    }

    /// `switch <discriminant> { cases... }`
    pub fn switch(&mut self, discriminant: Place, cases: Vec<(String, BlockId)>) {
        self.terminate(Terminator::switch(discriminant, cases));
    }

    /// `panic "message"`
    pub fn panic(&mut self, message: impl Into<String>) {
        self.terminate(Terminator::panic(message));
    }

    /// `unreachable`
    pub fn unreachable(&mut self) {
        self.terminate(Terminator::unreachable());
    }
}
