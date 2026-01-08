//! Block builder.

use crate::MirContext;
use crate::function::{
    BasicBlock, BinOp, CallArg, Callee, Immediate, Place, Rvalue, Statement, Terminator, UnOp,
    Value,
};
use crate::id::{Block, Function, Id, QualifiedName, Statement as StatementMarker, Ty};

/// Builder for constructing basic blocks.
pub struct BlockBuilder<'ctx> {
    pub(crate) ctx: &'ctx mut MirContext,
    #[allow(dead_code)]
    pub(crate) func_id: Id<Function>,
    pub(crate) id: Id<Block>,
}

impl<'ctx> BlockBuilder<'ctx> {
    /// Get the ID of the block being built.
    pub fn id(&self) -> Id<Block> {
        self.id
    }

    /// Get a reference to the block.
    pub fn block(&self) -> &BasicBlock {
        &self.ctx.blocks[self.id]
    }

    /// Get a mutable reference to the block.
    pub fn block_mut(&mut self) -> &mut BasicBlock {
        &mut self.ctx.blocks[self.id]
    }

    // === Statements ===

    /// Add a statement to this block.
    pub fn add_statement(&mut self, stmt: Statement) -> Id<StatementMarker> {
        let stmt_id = self.ctx.statements.alloc(stmt);
        self.block_mut().statements.push(stmt_id);
        stmt_id
    }

    /// Add an assignment statement.
    pub fn assign(&mut self, dest: Place, rvalue: Rvalue) -> Id<StatementMarker> {
        self.add_statement(Statement::assign(dest, rvalue))
    }

    /// Add a move assignment: `dest = move src`
    pub fn assign_move(&mut self, dest: Place, src: Place) -> Id<StatementMarker> {
        self.assign(dest, Rvalue::Move(src))
    }

    /// Add a copy assignment: `dest = copy src`
    pub fn assign_copy(&mut self, dest: Place, src: Place) -> Id<StatementMarker> {
        self.assign(dest, Rvalue::Copy(src))
    }

    /// Add a reference assignment: `dest = ref src`
    pub fn assign_ref(&mut self, dest: Place, src: Place) -> Id<StatementMarker> {
        self.assign(dest, Rvalue::Ref(src))
    }

    /// Add a mutable reference assignment: `dest = ref var src`
    pub fn assign_ref_mut(&mut self, dest: Place, src: Place) -> Id<StatementMarker> {
        self.assign(dest, Rvalue::RefMut(src))
    }

    /// Add an immediate assignment: `dest = imm`
    pub fn assign_imm(&mut self, dest: Place, imm: Immediate) -> Id<StatementMarker> {
        self.assign(dest, Rvalue::Use(imm))
    }

    /// Add a binary operation: `dest = op lhs, rhs`
    pub fn assign_binop(
        &mut self,
        dest: Place,
        op: BinOp,
        lhs: impl Into<Value>,
        rhs: impl Into<Value>,
    ) -> Id<StatementMarker> {
        self.assign(
            dest,
            Rvalue::BinaryOp {
                op,
                lhs: lhs.into(),
                rhs: rhs.into(),
            },
        )
    }

    /// Add a unary operation: `dest = op operand`
    pub fn assign_unop(
        &mut self,
        dest: Place,
        op: UnOp,
        operand: impl Into<Value>,
    ) -> Id<StatementMarker> {
        self.assign(
            dest,
            Rvalue::UnaryOp {
                op,
                operand: operand.into(),
            },
        )
    }

    /// Add a construct operation: `dest = construct Type { fields... }`
    pub fn assign_construct(
        &mut self,
        dest: Place,
        ty: Id<Ty>,
        fields: Vec<(String, Value)>,
    ) -> Id<StatementMarker> {
        self.assign(dest, Rvalue::Construct { ty, fields })
    }

    /// Add a call with return value: `dest = call callee(args...)`
    ///
    /// For convenience, all arguments default to `PassingMode::Ref` (borrow).
    /// Use `assign_call_with_modes` for explicit control.
    pub fn assign_call(
        &mut self,
        dest: Place,
        callee: Callee,
        args: Vec<Value>,
    ) -> Id<StatementMarker> {
        let call_args: Vec<CallArg> = args.into_iter().map(CallArg::borrow).collect();
        self.assign(
            dest,
            Rvalue::Call {
                callee,
                args: call_args,
            },
        )
    }

    /// Add a call with return value and explicit passing modes.
    pub fn assign_call_with_modes(
        &mut self,
        dest: Place,
        callee: Callee,
        args: Vec<CallArg>,
    ) -> Id<StatementMarker> {
        self.assign(dest, Rvalue::Call { callee, args })
    }

    /// Add a direct call with return value: `dest = call func(args...)`
    pub fn assign_call_direct(
        &mut self,
        dest: Place,
        func: Id<QualifiedName>,
        args: Vec<Value>,
    ) -> Id<StatementMarker> {
        self.assign_call(dest, Callee::direct(func), args)
    }

    /// Add a call statement (unit return): `call callee(args...)`
    ///
    /// For convenience, all arguments default to `PassingMode::Ref` (borrow).
    /// Use `call_with_modes` for explicit control.
    pub fn call(&mut self, callee: Callee, args: Vec<Value>) -> Id<StatementMarker> {
        let call_args: Vec<CallArg> = args.into_iter().map(CallArg::borrow).collect();
        self.add_statement(Statement::call(callee, call_args))
    }

    /// Add a call statement (unit return) with explicit passing modes.
    pub fn call_with_modes(&mut self, callee: Callee, args: Vec<CallArg>) -> Id<StatementMarker> {
        self.add_statement(Statement::call(callee, args))
    }

    /// Add a direct call statement (unit return): `call func(args...)`
    pub fn call_direct(
        &mut self,
        func: Id<QualifiedName>,
        args: Vec<Value>,
    ) -> Id<StatementMarker> {
        self.call(Callee::direct(func), args)
    }

    // === Terminators ===

    /// Set the terminator for this block.
    pub fn terminate(&mut self, term: Terminator) {
        self.block_mut().terminator = Some(term);
    }

    /// Add a return terminator.
    pub fn ret(&mut self, value: impl Into<Value>) {
        self.terminate(Terminator::ret(value));
    }

    /// Add a return unit terminator.
    pub fn ret_unit(&mut self) {
        self.ret(Immediate::unit());
    }

    /// Add a jump terminator.
    pub fn jump(&mut self, target: Id<Block>) {
        self.terminate(Terminator::jump(target));
    }

    /// Add a branch terminator.
    pub fn branch(
        &mut self,
        condition: impl Into<Value>,
        then_block: Id<Block>,
        else_block: Id<Block>,
    ) {
        self.terminate(Terminator::branch(condition, then_block, else_block));
    }

    /// Add a switch terminator.
    pub fn switch(&mut self, discriminant: Place, cases: Vec<(String, Id<Block>)>) {
        self.terminate(Terminator::switch(discriminant, cases));
    }

    /// Add a panic terminator.
    pub fn panic(&mut self, message: impl Into<String>) {
        self.terminate(Terminator::panic(message));
    }

    /// Add an unreachable terminator.
    pub fn unreachable(&mut self) {
        self.terminate(Terminator::unreachable());
    }
}
