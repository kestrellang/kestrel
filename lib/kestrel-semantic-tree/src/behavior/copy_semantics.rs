use semantic_tree::behavior::Behavior;

use crate::{behavior::KestrelBehaviorKind, language::KestrelLanguage};

/// Copy semantics determine how values of a type are handled when assigned or passed.
///
/// - **Copyable**: The value is bitwise copied, and both the original and copy remain valid.
/// - **Cloneable**: The value is copied via the clone() method, which may perform deep copies.
/// - **NotCopyable**: The value is moved, and the original becomes invalid after the move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopySemantics {
    /// Type can be copied (bitwise copy, original remains valid).
    /// This is the default for simple value types like integers and booleans.
    Copyable,
    /// Type can be copied via clone() method.
    /// This is used for types that need custom copy logic (e.g., heap-allocated data).
    Cloneable,
    /// Type cannot be copied, only moved (original becomes invalid after move).
    /// This is used for types that manage resources or have unique ownership.
    NotCopyable,
}

/// CopySemanticsBehavior represents the copy/move semantics of a type.
///
/// This behavior is attached to type definitions (structs, enums) to indicate
/// whether values of that type can be copied or must be moved.
///
/// # Examples
///
/// - A struct with only copyable fields is typically copyable
/// - A struct that opts out via `not Copyable` has NotCopyable semantics
/// - Resource-managing types (file handles, etc.) should be NotCopyable
#[derive(Debug, Clone)]
pub struct CopySemanticsBehavior {
    /// The copy semantics for this type
    semantics: CopySemantics,
}

impl Behavior<KestrelLanguage> for CopySemanticsBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::CopySemantics
    }
}

impl CopySemanticsBehavior {
    /// Create a new CopySemanticsBehavior with the given semantics
    pub fn new(semantics: CopySemantics) -> Self {
        CopySemanticsBehavior { semantics }
    }

    /// Create a CopySemanticsBehavior indicating the type is copyable
    pub fn copyable() -> Self {
        CopySemanticsBehavior {
            semantics: CopySemantics::Copyable,
        }
    }

    /// Create a CopySemanticsBehavior indicating the type is cloneable (copy via clone())
    pub fn cloneable() -> Self {
        CopySemanticsBehavior {
            semantics: CopySemantics::Cloneable,
        }
    }

    /// Create a CopySemanticsBehavior indicating the type is not copyable (move-only)
    pub fn not_copyable() -> Self {
        CopySemanticsBehavior {
            semantics: CopySemantics::NotCopyable,
        }
    }

    /// Get the copy semantics
    pub fn semantics(&self) -> CopySemantics {
        self.semantics
    }

    /// Check if the type is copyable (either bitwise or via clone)
    pub fn is_copyable(&self) -> bool {
        matches!(
            self.semantics,
            CopySemantics::Copyable | CopySemantics::Cloneable
        )
    }

    /// Check if the type is cloneable (copies go through clone())
    pub fn is_cloneable(&self) -> bool {
        self.semantics == CopySemantics::Cloneable
    }
}
