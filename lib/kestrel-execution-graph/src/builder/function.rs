//! Function builder.

use crate::MirContext;
use crate::builder::BlockBuilder;
use crate::function::{BasicBlock, LocalDef, TypeParamDef, TypeParamOwner};
use crate::id::{Block, Function, Id, Local, Ty, TypeParam};
use crate::item::{FunctionDef, ParamDef, WhereClause, WhereConstraint};
use crate::metadata::Metadata;

/// Builder for constructing functions.
pub struct FunctionBuilder<'ctx> {
    pub(crate) ctx: &'ctx mut MirContext,
    pub(crate) id: Id<Function>,
}

impl<'ctx> FunctionBuilder<'ctx> {
    /// Get the ID of the function being built.
    pub fn id(&self) -> Id<Function> {
        self.id
    }

    /// Get a reference to the function definition.
    pub fn def(&self) -> &FunctionDef {
        &self.ctx.functions[self.id]
    }

    /// Get a mutable reference to the function definition.
    pub fn def_mut(&mut self) -> &mut FunctionDef {
        &mut self.ctx.functions[self.id]
    }

    /// Add a type parameter to this function.
    pub fn type_param(&mut self, name: impl Into<String>) -> Id<TypeParam> {
        let tp = TypeParamDef::new(name, TypeParamOwner::Function(self.id));
        let tp_id = self.ctx.type_params.alloc(tp);
        self.def_mut().type_params.push(tp_id);
        tp_id
    }

    /// Add a parameter to this function.
    pub fn param(&mut self, name: impl Into<String>, ty: Id<Ty>) -> Id<Local> {
        let name = name.into();

        let local = LocalDef::new(name.clone(), ty);
        let local_id = self.ctx.locals.alloc(local);

        let param = ParamDef::new(name.clone(), local_id, ty);
        let param_id = self.ctx.params.alloc(param);

        let def = self.def_mut();
        def.params.push(param_id);
        def.params_by_name.insert(name.clone(), param_id);
        def.locals.push(local_id);
        def.locals_by_name.insert(name, local_id);

        local_id
    }

    /// Add a local variable to this function.
    pub fn local(&mut self, name: impl Into<String>, ty: Id<Ty>) -> Id<Local> {
        let name = name.into();

        let local = LocalDef::new(name.clone(), ty);
        let local_id = self.ctx.locals.alloc(local);

        let def = self.def_mut();
        def.locals.push(local_id);
        def.locals_by_name.insert(name, local_id);

        local_id
    }

    /// Set the where clause for this function.
    pub fn set_where_clause(&mut self, where_clause: WhereClause) {
        self.def_mut().where_clause = Some(where_clause);
    }

    /// Add a constraint to the where clause.
    pub fn add_constraint(&mut self, constraint: WhereConstraint) {
        let def = self.def_mut();
        if def.where_clause.is_none() {
            def.where_clause = Some(WhereClause::new());
        }
        def.where_clause
            .as_mut()
            .unwrap()
            .add_constraint(constraint);
    }

    /// Add a new basic block to this function.
    pub fn add_block(&mut self) -> BlockBuilder<'_> {
        let block = BasicBlock::new();
        let block_id = self.ctx.blocks.alloc(block);

        let def = self.def_mut();
        if def.entry_block.is_none() {
            def.entry_block = Some(block_id);
        }
        def.blocks.push(block_id);

        BlockBuilder {
            ctx: self.ctx,
            func_id: self.id,
            id: block_id,
        }
    }

    /// Get a builder for an existing block.
    pub fn block(&mut self, block_id: Id<Block>) -> BlockBuilder<'_> {
        BlockBuilder {
            ctx: self.ctx,
            func_id: self.id,
            id: block_id,
        }
    }

    /// Get the entry block ID.
    pub fn entry_block(&self) -> Option<Id<Block>> {
        self.def().entry_block
    }

    /// Get all block IDs in this function.
    pub fn blocks(&self) -> &[Id<Block>] {
        &self.def().blocks
    }

    /// Set metadata for this function.
    pub fn set_metadata(&mut self, meta: Metadata) {
        self.def_mut().meta = meta;
    }
}
