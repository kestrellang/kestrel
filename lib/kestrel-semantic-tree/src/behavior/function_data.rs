//! Function-specific data behavior
//!
//! This behavior stores function-specific metadata like `has_body` and `is_static`
//! that can be queried during validation passes.

use semantic_tree::behavior::Behavior;

use crate::behavior::KestrelBehaviorKind;
use crate::language::KestrelLanguage;

/// Behavior storing function-specific data
#[derive(Debug, Clone)]
pub struct FunctionDataBehavior {
    has_body: bool,
    is_static: bool,
}

impl FunctionDataBehavior {
    /// Create a new function data behavior
    pub fn new(has_body: bool, is_static: bool) -> Self {
        Self {
            has_body,
            is_static,
        }
    }

    /// Check if the function has a body
    pub fn has_body(&self) -> bool {
        self.has_body
    }

    /// Check if the function is static
    pub fn is_static(&self) -> bool {
        self.is_static
    }
}

impl Behavior<KestrelLanguage> for FunctionDataBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::FunctionData
    }
}
