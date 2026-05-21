use std::collections::{HashMap, HashSet};

use crate::statement::Statement;
use crate::terminator::Terminator;
use crate::{BlockId, LocalId, TyId};

#[derive(Debug, Clone, PartialEq)]
pub struct LocalDef {
    pub name: String,
    pub ty: TyId,
}

impl LocalDef {
    pub fn new(name: impl Into<String>, ty: TyId) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasicBlock {
    pub stmts: Vec<Statement>,
    pub terminator: Terminator,
}

impl BasicBlock {
    pub fn new() -> Self {
        Self {
            stmts: Vec::new(),
            terminator: Terminator::unreachable(),
        }
    }
}

impl Default for BasicBlock {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScopeId {
    Function,
    Loop { header: BlockId, exit: BlockId },
}

#[derive(Debug, Clone)]
pub struct MirBody {
    pub locals: Vec<LocalDef>,
    pub blocks: Vec<BasicBlock>,
    pub param_count: usize,
    pub entry: BlockId,
    pub local_scopes: HashMap<LocalId, ScopeId>,
    pub failure_return_blocks: HashSet<BlockId>,
}

impl MirBody {
    pub fn new() -> Self {
        Self {
            locals: Vec::new(),
            blocks: Vec::new(),
            param_count: 0,
            entry: BlockId::new(0),
            local_scopes: HashMap::new(),
            failure_return_blocks: HashSet::new(),
        }
    }

    pub fn add_local(&mut self, local: LocalDef) -> LocalId {
        let id = LocalId::new(self.locals.len());
        self.locals.push(local);
        id
    }

    pub fn add_block(&mut self, block: BasicBlock) -> BlockId {
        let id = BlockId::new(self.blocks.len());
        self.blocks.push(block);
        id
    }

    pub fn block(&self, id: BlockId) -> &BasicBlock {
        &self.blocks[id.index()]
    }

    pub fn block_mut(&mut self, id: BlockId) -> &mut BasicBlock {
        &mut self.blocks[id.index()]
    }

    pub fn local(&self, id: LocalId) -> &LocalDef {
        &self.locals[id.index()]
    }
}

impl Default for MirBody {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_starts_empty() {
        let body = MirBody::new();
        assert!(body.locals.is_empty());
        assert!(body.blocks.is_empty());
        assert_eq!(body.param_count, 0);
        assert_eq!(body.entry, BlockId::new(0));
    }

    #[test]
    fn add_local() {
        let mut body = MirBody::new();
        let ty = TyId::new(0);
        let id = body.add_local(LocalDef::new("x", ty));
        assert_eq!(id, LocalId::new(0));
        assert_eq!(body.locals.len(), 1);
        assert_eq!(body.local(id).name, "x");
        assert_eq!(body.local(id).ty, ty);
    }

    #[test]
    fn add_multiple_locals() {
        let mut body = MirBody::new();
        let ty = TyId::new(0);
        let a = body.add_local(LocalDef::new("a", ty));
        let b = body.add_local(LocalDef::new("b", ty));
        assert_eq!(a, LocalId::new(0));
        assert_eq!(b, LocalId::new(1));
        assert_eq!(body.locals.len(), 2);
    }

    #[test]
    fn add_block() {
        let mut body = MirBody::new();
        let bb = BasicBlock::new();
        let id = body.add_block(bb);
        assert_eq!(id, BlockId::new(0));
        assert_eq!(body.blocks.len(), 1);
    }

    #[test]
    fn block_access() {
        let mut body = MirBody::new();
        let id = body.add_block(BasicBlock::new());
        assert!(body.block(id).stmts.is_empty());
    }

    #[test]
    fn block_mut_access() {
        let mut body = MirBody::new();
        let id = body.add_block(BasicBlock::new());
        body.block_mut(id)
            .stmts
            .push(Statement::new(crate::StatementKind::ScopeLive(
                LocalId::new(0),
            )));
        assert_eq!(body.block(id).stmts.len(), 1);
    }

    #[test]
    fn param_count_tracking() {
        let mut body = MirBody::new();
        let ty = TyId::new(0);
        body.add_local(LocalDef::new("p0", ty));
        body.add_local(LocalDef::new("p1", ty));
        body.param_count = 2;
        body.add_local(LocalDef::new("tmp", ty));
        assert_eq!(body.param_count, 2);
        assert_eq!(body.locals.len(), 3);
    }

    #[test]
    fn basic_block_default_terminator() {
        let bb = BasicBlock::new();
        assert_eq!(bb.terminator.kind, crate::TerminatorKind::Unreachable);
    }

    #[test]
    fn scope_id_variants() {
        let func = ScopeId::Function;
        let loop_scope = ScopeId::Loop {
            header: BlockId::new(1),
            exit: BlockId::new(5),
        };
        assert_ne!(func, loop_scope);
    }

    #[test]
    fn local_scopes_and_failure_blocks() {
        let mut body = MirBody::new();
        let local = body.add_local(LocalDef::new("x", TyId::new(0)));
        body.local_scopes.insert(local, ScopeId::Function);
        assert!(body.local_scopes.contains_key(&local));

        let block = body.add_block(BasicBlock::new());
        body.failure_return_blocks.insert(block);
        assert!(body.failure_return_blocks.contains(&block));
    }
}
