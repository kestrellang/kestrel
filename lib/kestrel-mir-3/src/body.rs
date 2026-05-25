use crate::block::BasicBlock;
use crate::item::CopyBehavior;
use crate::ty::{MirTy, TyArena};
use crate::value::{Ownership, ValueDef};
use crate::{BlockId, MirModule, TyId, ValueId};

#[derive(Debug, Clone)]
pub struct OssaBody {
    pub values: Vec<ValueDef>,
    pub blocks: Vec<BasicBlock>,
    pub entry: BlockId,
    pub param_count: usize,
}

impl OssaBody {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            blocks: Vec::new(),
            entry: BlockId::new(0),
            param_count: 0,
        }
    }

    pub fn value(&self, id: ValueId) -> &ValueDef {
        &self.values[id.index()]
    }

    pub fn block(&self, id: BlockId) -> &BasicBlock {
        &self.blocks[id.index()]
    }

    pub fn block_mut(&mut self, id: BlockId) -> &mut BasicBlock {
        &mut self.blocks[id.index()]
    }

    pub fn alloc_value(&mut self, def: ValueDef) -> ValueId {
        let id = ValueId::new(self.values.len());
        self.values.push(def);
        id
    }

    pub fn alloc_block(&mut self) -> BlockId {
        let id = BlockId::new(self.blocks.len());
        self.blocks.push(BasicBlock::new());
        id
    }
}

impl Default for OssaBody {
    fn default() -> Self {
        Self::new()
    }
}

/// Determine ownership for a type: Bitwise → None, everything else → Owned.
pub fn ownership_for_type(ty: TyId, arena: &TyArena, module: &MirModule) -> Ownership {
    match crate::ty_query::copy_behavior(arena, module, ty, None) {
        CopyBehavior::Bitwise => Ownership::None,
        CopyBehavior::Clone(_) | CopyBehavior::None => Ownership::Owned,
    }
}

/// Shorthand: is this type trivially copyable (Bitwise)?
pub fn is_trivial(ty: TyId, arena: &TyArena) -> bool {
    matches!(
        arena.get(ty),
        MirTy::I8
            | MirTy::I16
            | MirTy::I32
            | MirTy::I64
            | MirTy::F16
            | MirTy::F32
            | MirTy::F64
            | MirTy::Bool
            | MirTy::Never
            | MirTy::Str
            | MirTy::Pointer(_)
            | MirTy::FuncThin { .. }
            | MirTy::Error
    )
}
