//! Function bodies — locals, blocks, statements.
//!
//! Self-contained: a statement in function A never references a local in function B.

use std::collections::{HashMap, HashSet};

use crate::id::{BlockId, LocalId};
use crate::statement::Statement;
use crate::terminator::Terminator;
use crate::ty::MirTy;

/// Identifies the scope that owns a local for drop purposes.
/// Used by the drop elaboration pass to determine WHERE cleanup is inserted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeId {
    /// Function-level — cleaned at return.
    Function,
    /// Loop body — cleaned at back-edge, break, continue, and return-from-inside.
    /// header: the loop header block (where ScopeLive is emitted).
    /// exit: the block that break jumps to (outside the loop).
    Loop { header: BlockId, exit: BlockId },
}

/// A function body containing locals and basic blocks.
#[derive(Debug, Clone)]
pub struct MirBody {
    /// All locals in this function (params first, then user locals, then temps).
    pub locals: Vec<LocalDef>,
    /// All basic blocks in this function.
    pub blocks: Vec<BasicBlock>,
    /// The entry block (first block to execute).
    pub entry: BlockId,
    /// Number of parameters — the first `param_count` locals are parameters.
    pub param_count: usize,
    /// Blocks whose Return terminator is a failure path in an effectful init.
    /// The deinit pass uses this to insert partial-drop cleanup.
    pub failure_return_blocks: HashSet<BlockId>,
    /// Scope each droppable local belongs to. Locals absent from this map
    /// are function-scoped (the default). Populated by lowering so the drop
    /// elaboration pass knows which exit points trigger cleanup for which locals.
    pub local_scopes: HashMap<LocalId, ScopeId>,
}

impl MirBody {
    /// Create a new empty body.
    pub fn new() -> Self {
        Self {
            locals: Vec::new(),
            blocks: Vec::new(),
            entry: BlockId::new(0),
            param_count: 0,
            failure_return_blocks: HashSet::new(),
            local_scopes: HashMap::new(),
        }
    }

    /// Add a local and return its ID.
    pub fn add_local(&mut self, local: LocalDef) -> LocalId {
        let id = LocalId::new(self.locals.len());
        self.locals.push(local);
        id
    }

    /// Add a block and return its ID.
    pub fn add_block(&mut self, block: BasicBlock) -> BlockId {
        let id = BlockId::new(self.blocks.len());
        self.blocks.push(block);
        id
    }

    /// Get a local by ID.
    pub fn local(&self, id: LocalId) -> &LocalDef {
        &self.locals[id.index()]
    }

    /// Get a block by ID.
    pub fn block(&self, id: BlockId) -> &BasicBlock {
        &self.blocks[id.index()]
    }

    /// Get a mutable block by ID.
    pub fn block_mut(&mut self, id: BlockId) -> &mut BasicBlock {
        &mut self.blocks[id.index()]
    }
}

impl Default for MirBody {
    fn default() -> Self {
        Self::new()
    }
}

/// A basic block — a sequence of statements ending in a terminator.
///
/// Terminators are non-optional. Every block must have exactly one.
#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// Statements executed in order.
    pub stmts: Vec<Statement>,
    /// How this block exits. Always present.
    pub terminator: Terminator,
}

impl BasicBlock {
    /// Create a block with a placeholder unreachable terminator.
    /// The terminator should be replaced before the MIR is finalized.
    pub fn new() -> Self {
        Self {
            stmts: Vec::new(),
            terminator: Terminator::unreachable(),
        }
    }

    /// Get the successor blocks.
    pub fn successors(&self) -> Vec<BlockId> {
        self.terminator.successors()
    }
}

impl Default for BasicBlock {
    fn default() -> Self {
        Self::new()
    }
}

/// A local variable in a function.
#[derive(Debug, Clone)]
pub struct LocalDef {
    /// Variable name (without the `%` prefix).
    pub name: String,
    /// Type of this local.
    pub ty: MirTy,
    /// True for locals loaded from a closure env struct. These are
    /// borrowed views of the parent scope's values — the closure does
    /// not own them and drop elaboration must skip them.
    pub borrowed: bool,
}

impl LocalDef {
    pub fn new(name: impl Into<String>, ty: MirTy) -> Self {
        Self {
            name: name.into(),
            ty,
            borrowed: false,
        }
    }

    pub fn borrowed(name: impl Into<String>, ty: MirTy) -> Self {
        Self {
            name: name.into(),
            ty,
            borrowed: true,
        }
    }
}
