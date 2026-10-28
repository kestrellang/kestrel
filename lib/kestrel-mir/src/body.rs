use rustc_hash::FxHashMap;

use crate::block::BasicBlock;
use crate::value::{RootProvenance, ValueDef};
use crate::{BlockId, ValueId};

#[derive(Debug, Clone)]
pub struct OssaBody {
    pub values: Vec<ValueDef>,
    pub blocks: Vec<BasicBlock>,
    pub entry: BlockId,
    pub param_count: usize,
    /// Source-level names for named values (params, `let`/`var` bindings),
    /// filled during lowering. Diagnostics-only (escape errors say
    /// "borrows local `x`"); absence is always fine.
    pub value_names: FxHashMap<ValueId, String>,
}

impl OssaBody {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            blocks: Vec::new(),
            entry: BlockId::new(0),
            param_count: 0,
            value_names: FxHashMap::default(),
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

    pub fn alloc_value(&mut self, mut def: ValueDef) -> ValueId {
        let id = ValueId::new(self.values.len());
        // THE provenance funnel: borrows/projections inherit their source's
        // root (carries Param/Static/PointerDerived through whole chains in
        // O(1)); everything else self-roots as a fresh Local. Sites that know
        // better (params, globals, ptr_ref intrinsics) pass an explicit root.
        if def.root.is_derived_placeholder() {
            // `.get` not indexing: hand-built (test) bodies may record a
            // borrow_source that isn't allocated yet — self-root those.
            def.root = def
                .borrow_source
                .and_then(|src| self.values.get(src.index()).map(|v| v.root))
                .unwrap_or(RootProvenance::Local(id));
        }
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
