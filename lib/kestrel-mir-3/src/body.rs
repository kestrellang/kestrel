use crate::block::BasicBlock;
use crate::value::ValueDef;
use crate::{BlockId, ValueId};

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
