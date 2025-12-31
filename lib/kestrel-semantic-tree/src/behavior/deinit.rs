use semantic_tree::behavior::Behavior;
use semantic_tree::symbol::SymbolId;

use crate::language::KestrelLanguage;

use super::KestrelBehaviorKind;

/// Behavior attached to struct symbols that have a deinit block.
///
/// This behavior tracks the deinit symbol associated with the struct,
/// allowing the compiler to:
/// - Validate that at most one deinit exists per struct
/// - Generate drop glue that calls the deinit
/// - Warn if a Copyable type has a deinit
#[derive(Debug, Clone)]
pub struct DeinitBehavior {
    /// The symbol ID of the deinit declaration
    deinit_symbol: SymbolId,
}

impl Behavior<KestrelLanguage> for DeinitBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::Deinit
    }
}

impl DeinitBehavior {
    /// Create a new DeinitBehavior with the given deinit symbol
    pub fn new(deinit_symbol: SymbolId) -> Self {
        Self { deinit_symbol }
    }

    /// Get the symbol ID of the deinit declaration
    pub fn deinit_symbol(&self) -> SymbolId {
        self.deinit_symbol
    }
}
