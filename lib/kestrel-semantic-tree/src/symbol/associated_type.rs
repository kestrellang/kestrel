use std::sync::Arc;

use kestrel_span::{Name, Span};
use semantic_tree::behavior::Behavior;
use semantic_tree::symbol::{Symbol, SymbolMetadata, SymbolMetadataBuilder};

use crate::{
    behavior::{typed::TypedBehavior, visibility::VisibilityBehavior, KestrelBehaviorKind},
    language::KestrelLanguage,
    symbol::kind::KestrelSymbolKind,
    ty::Ty,
};

/// Represents an associated type declaration in a protocol.
///
/// Associated types are type placeholders that conforming types must provide
/// concrete bindings for. They can have constraints (bounds) and optional defaults.
///
/// # Example
///
/// ```kestrel
/// protocol Iterator {
///     type Item                        // Abstract
///     type Item: Equatable             // With constraint
///     type Item: Equatable = Int       // With constraint + default
/// }
/// ```
///
/// # Type Resolution
///
/// During build phase, basic symbol information is captured along with any bounds.
/// During bind phase, bounds are resolved to protocol types and defaults are resolved.
#[derive(Debug)]
pub struct AssociatedTypeSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
}

impl Symbol<KestrelLanguage> for AssociatedTypeSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl AssociatedTypeSymbol {
    /// Create a new AssociatedTypeSymbol with a name, span, visibility, and optional parent
    pub fn new(
        name: Name,
        span: Span,
        visibility: VisibilityBehavior,
        parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
    ) -> Self {
        let mut builder = SymbolMetadataBuilder::new(KestrelSymbolKind::AssociatedType)
            .with_name(name.clone())
            .with_declaration_span(name.span.clone())
            .with_span(span)
            .with_behavior(Arc::new(visibility));

        if let Some(p) = parent {
            builder = builder.with_parent(Arc::downgrade(&p));
        }

        AssociatedTypeSymbol {
            metadata: builder.build(),
        }
    }

    /// Get the constraint bounds for this associated type (e.g., `Item: Equatable, Hashable`)
    ///
    /// Returns None if no bounds behavior has been attached yet.
    pub fn bounds(&self) -> Option<Vec<Ty>> {
        self.metadata
            .behaviors()
            .into_iter()
            .find(|b| matches!(b.kind(), KestrelBehaviorKind::AssociatedTypeBounds))
            .and_then(|b| b.as_ref().downcast_ref::<AssociatedTypeBoundsBehavior>().cloned())
            .map(|b| b.bounds().to_vec())
    }

    /// Get the default type for this associated type (e.g., `= Int`)
    ///
    /// Returns None if no default is specified or not yet resolved.
    pub fn default_type(&self) -> Option<Ty> {
        self.metadata
            .behaviors()
            .into_iter()
            .find(|b| matches!(b.kind(), KestrelBehaviorKind::Typed))
            .and_then(|b| b.as_ref().downcast_ref::<TypedBehavior>().cloned())
            .map(|b| b.ty().clone())
    }

    /// Check if this associated type has a default value
    pub fn has_default(&self) -> bool {
        self.default_type().is_some()
    }

    /// Check if this associated type has constraint bounds
    pub fn has_bounds(&self) -> bool {
        self.bounds().map(|b| !b.is_empty()).unwrap_or(false)
    }
}

/// Behavior for storing resolved constraint bounds on an associated type.
///
/// This is attached during the bind phase after bounds paths are resolved
/// to concrete protocol types.
#[derive(Debug, Clone)]
pub struct AssociatedTypeBoundsBehavior {
    /// The resolved protocol bounds
    bounds: Vec<Ty>,
}

impl Behavior<KestrelLanguage> for AssociatedTypeBoundsBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::AssociatedTypeBounds
    }
}

impl AssociatedTypeBoundsBehavior {
    /// Create a new AssociatedTypeBoundsBehavior with resolved bounds
    pub fn new(bounds: Vec<Ty>) -> Self {
        AssociatedTypeBoundsBehavior { bounds }
    }

    /// Get the resolved bounds
    pub fn bounds(&self) -> &[Ty] {
        &self.bounds
    }
}
