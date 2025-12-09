//! Behavior accessor extensions for Kestrel symbols
//!
//! This module provides convenient accessor methods for common behavior lookups
//! on Kestrel symbols. These methods consolidate the verbose pattern of:
//! ```ignore
//! symbol.metadata().behaviors().iter()
//!     .find(|b| matches!(b.kind(), KestrelBehaviorKind::Typed))
//!     .and_then(|b| b.downcast_ref::<TypedBehavior>())
//!     .cloned()
//! ```
//! into simple method calls like `symbol.typed_behavior()`.

use std::sync::Arc;

use semantic_tree::symbol::{Symbol, SymbolMetadata};

use crate::behavior::callable::CallableBehavior;
use crate::behavior::conformances::ConformancesBehavior;
use crate::behavior::generics::GenericsBehavior;
use crate::behavior::typed::TypedBehavior;
use crate::behavior::valued::ValueBehavior;
use crate::behavior::visibility::VisibilityBehavior;
use crate::behavior::KestrelBehaviorKind;
use crate::language::KestrelLanguage;
use crate::symbol::protocol::FlattenedProtocolBehavior;

/// Extension trait for accessing typed behaviors on Kestrel symbol metadata
pub trait BehaviorExt {
    /// Get the TypedBehavior if present (cloned)
    fn typed_behavior(&self) -> Option<TypedBehavior>;

    /// Get the CallableBehavior if present (cloned)
    fn callable_behavior(&self) -> Option<CallableBehavior>;

    /// Get the VisibilityBehavior if present (cloned)
    fn visibility_behavior(&self) -> Option<VisibilityBehavior>;

    /// Get the ValueBehavior if present (cloned)
    fn value_behavior(&self) -> Option<ValueBehavior>;

    /// Get the ConformancesBehavior if present (cloned)
    fn conformances_behavior(&self) -> Option<ConformancesBehavior>;

    /// Get the GenericsBehavior if present (cloned)
    fn generics_behavior(&self) -> Option<GenericsBehavior>;

    /// Get the FlattenedProtocolBehavior if present (cloned)
    fn flattened_protocol_behavior(&self) -> Option<FlattenedProtocolBehavior>;
}

impl BehaviorExt for SymbolMetadata<KestrelLanguage> {
    fn typed_behavior(&self) -> Option<TypedBehavior> {
        self.behaviors()
            .into_iter()
            .find(|b| matches!(b.kind(), KestrelBehaviorKind::Typed))
            .and_then(|b| b.as_ref().downcast_ref::<TypedBehavior>().cloned())
    }

    fn callable_behavior(&self) -> Option<CallableBehavior> {
        self.behaviors()
            .into_iter()
            .find(|b| matches!(b.kind(), KestrelBehaviorKind::Callable))
            .and_then(|b| b.as_ref().downcast_ref::<CallableBehavior>().cloned())
    }

    fn visibility_behavior(&self) -> Option<VisibilityBehavior> {
        self.behaviors()
            .into_iter()
            .find(|b| matches!(b.kind(), KestrelBehaviorKind::Visibility))
            .and_then(|b| b.as_ref().downcast_ref::<VisibilityBehavior>().cloned())
    }

    fn value_behavior(&self) -> Option<ValueBehavior> {
        self.behaviors()
            .into_iter()
            .find(|b| matches!(b.kind(), KestrelBehaviorKind::Valued))
            .and_then(|b| b.as_ref().downcast_ref::<ValueBehavior>().cloned())
    }

    fn conformances_behavior(&self) -> Option<ConformancesBehavior> {
        self.behaviors()
            .into_iter()
            .find(|b| matches!(b.kind(), KestrelBehaviorKind::Conformances))
            .and_then(|b| b.as_ref().downcast_ref::<ConformancesBehavior>().cloned())
    }

    fn generics_behavior(&self) -> Option<GenericsBehavior> {
        self.behaviors()
            .into_iter()
            .find(|b| matches!(b.kind(), KestrelBehaviorKind::Generics))
            .and_then(|b| b.as_ref().downcast_ref::<GenericsBehavior>().cloned())
    }

    fn flattened_protocol_behavior(&self) -> Option<FlattenedProtocolBehavior> {
        self.behaviors()
            .into_iter()
            .find(|b| matches!(b.kind(), KestrelBehaviorKind::FlattenedProtocol))
            .and_then(|b| b.as_ref().downcast_ref::<FlattenedProtocolBehavior>().cloned())
    }
}

/// Extension trait for accessing typed behaviors directly on symbols
pub trait SymbolBehaviorExt {
    /// Get the TypedBehavior if present (cloned)
    fn typed_behavior(&self) -> Option<TypedBehavior>;

    /// Get the CallableBehavior if present (cloned)
    fn callable_behavior(&self) -> Option<CallableBehavior>;

    /// Get the VisibilityBehavior if present (cloned)
    fn visibility_behavior(&self) -> Option<VisibilityBehavior>;

    /// Get the ValueBehavior if present (cloned)
    fn value_behavior(&self) -> Option<ValueBehavior>;

    /// Get the ConformancesBehavior if present (cloned)
    fn conformances_behavior(&self) -> Option<ConformancesBehavior>;

    /// Get the GenericsBehavior if present (cloned)
    fn generics_behavior(&self) -> Option<GenericsBehavior>;

    /// Get the FlattenedProtocolBehavior if present (cloned)
    fn flattened_protocol_behavior(&self) -> Option<FlattenedProtocolBehavior>;
}

impl<T: Symbol<KestrelLanguage>> SymbolBehaviorExt for T {
    fn typed_behavior(&self) -> Option<TypedBehavior> {
        self.metadata().typed_behavior()
    }

    fn callable_behavior(&self) -> Option<CallableBehavior> {
        self.metadata().callable_behavior()
    }

    fn visibility_behavior(&self) -> Option<VisibilityBehavior> {
        self.metadata().visibility_behavior()
    }

    fn value_behavior(&self) -> Option<ValueBehavior> {
        self.metadata().value_behavior()
    }

    fn conformances_behavior(&self) -> Option<ConformancesBehavior> {
        self.metadata().conformances_behavior()
    }

    fn generics_behavior(&self) -> Option<GenericsBehavior> {
        self.metadata().generics_behavior()
    }

    fn flattened_protocol_behavior(&self) -> Option<FlattenedProtocolBehavior> {
        self.metadata().flattened_protocol_behavior()
    }
}

impl SymbolBehaviorExt for Arc<dyn Symbol<KestrelLanguage>> {
    fn typed_behavior(&self) -> Option<TypedBehavior> {
        self.metadata().typed_behavior()
    }

    fn callable_behavior(&self) -> Option<CallableBehavior> {
        self.metadata().callable_behavior()
    }

    fn visibility_behavior(&self) -> Option<VisibilityBehavior> {
        self.metadata().visibility_behavior()
    }

    fn value_behavior(&self) -> Option<ValueBehavior> {
        self.metadata().value_behavior()
    }

    fn conformances_behavior(&self) -> Option<ConformancesBehavior> {
        self.metadata().conformances_behavior()
    }

    fn generics_behavior(&self) -> Option<GenericsBehavior> {
        self.metadata().generics_behavior()
    }

    fn flattened_protocol_behavior(&self) -> Option<FlattenedProtocolBehavior> {
        self.metadata().flattened_protocol_behavior()
    }
}
