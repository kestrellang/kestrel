mod collection;
mod metadata;
mod table;

use std::fmt::Debug;
use std::sync::atomic::{AtomicU64, Ordering};

use downcast_rs::{DowncastSync, impl_downcast};

pub use collection::SymbolCollection;
pub use metadata::SymbolMetadata;
pub use metadata::SymbolMetadataBuilder;
pub use table::SymbolTable;

use crate::language::Language;

/// Globally unique symbol identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(u64);

impl SymbolId {
    /// Create a new unique symbol ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        SymbolId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the raw ID value (useful for debugging)
    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl Default for SymbolId {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Symbol<L: Language>: Debug + Send + Sync + DowncastSync {
    fn metadata(&self) -> &SymbolMetadata<L>;
}

impl_downcast!(sync Symbol<L> where L: Language);
