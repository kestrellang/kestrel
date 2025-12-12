use std::sync::Arc;

use kestrel_span::{Name, Span};
use semantic_tree::behavior::Behavior;
use semantic_tree::symbol::{Symbol, SymbolMetadata, SymbolMetadataBuilder};

use crate::{
    behavior::generics::GenericsBehavior,
    behavior::{KestrelBehaviorKind, typed::TypedBehavior, visibility::VisibilityBehavior},
    language::KestrelLanguage,
    symbol::kind::KestrelSymbolKind,
    symbol::type_parameter::TypeParameterSymbol,
    ty::{Ty, WhereClause},
};

/// Represents a type alias declaration in the semantic tree.
///
/// Type aliases provide alternative names for types.
///
/// # Type Resolution
///
/// During build phase, basic symbol information is captured.
/// During bind phase, `GenericsBehavior` is added with resolved type parameters and where clause.
/// `TypeAliasTypedBehavior` is also added with the resolved aliased type.
#[derive(Debug)]
pub struct TypeAliasSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
}

impl Symbol<KestrelLanguage> for TypeAliasSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl TypeAliasSymbol {
    /// Create a new TypeAliasSymbol with a name, span, visibility, type, and optional parent
    pub fn new(
        name: Name,
        span: Span,
        visibility: VisibilityBehavior,
        ty: TypedBehavior,
        parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
    ) -> Self {
        let mut builder = SymbolMetadataBuilder::new(KestrelSymbolKind::TypeAlias)
            .with_name(name.clone())
            .with_declaration_span(name.span.clone())
            .with_span(span)
            .with_behavior(Arc::new(visibility))
            .with_behavior(Arc::new(ty));

        if let Some(p) = parent {
            builder = builder.with_parent(Arc::downgrade(&p));
        }

        TypeAliasSymbol {
            metadata: builder.build(),
        }
    }

    /// Get the type parameters for this type alias.
    ///
    /// During BUILD phase (before GenericsBehavior is attached), this gets
    /// TypeParameter children directly. After BIND, it uses the GenericsBehavior.
    pub fn type_parameters(&self) -> Vec<Arc<TypeParameterSymbol>> {
        // First try GenericsBehavior (available after BIND)
        if let Some(g) = self.metadata.get_behavior::<GenericsBehavior>() {
            return g.type_parameters().to_vec();
        }

        // Fallback: get TypeParameter children (available during BUILD)
        self.metadata
            .children()
            .into_iter()
            .filter_map(|c| {
                if c.metadata().kind() == KestrelSymbolKind::TypeParameter {
                    c.downcast_arc::<TypeParameterSymbol>().ok()
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if this type alias is generic (has type parameters)
    pub fn is_generic(&self) -> bool {
        self.metadata
            .get_behavior::<GenericsBehavior>()
            .map(|g| g.is_generic())
            .unwrap_or(false)
    }

    /// Get the number of type parameters
    ///
    /// During BUILD phase (before GenericsBehavior is attached), this counts
    /// TypeParameter children. After BIND, it uses the GenericsBehavior.
    pub fn type_parameter_count(&self) -> usize {
        // First try GenericsBehavior (available after BIND)
        if let Some(g) = self.metadata.get_behavior::<GenericsBehavior>() {
            return g.type_parameter_count();
        }

        // Fallback: count TypeParameter children (available during BUILD)
        self.metadata
            .children()
            .iter()
            .filter(|c| c.metadata().kind() == KestrelSymbolKind::TypeParameter)
            .count()
    }

    /// Get the where clause for this type alias.
    ///
    /// Delegates to GenericsBehavior. Returns empty where clause if not yet bound.
    pub fn where_clause(&self) -> WhereClause {
        self.metadata
            .get_behavior::<GenericsBehavior>()
            .map(|g| g.where_clause().clone())
            .unwrap_or_else(WhereClause::new)
    }
}

/// TypeAliasTypedBehavior represents the resolved type information for a type alias
///
/// This behavior is added during the binding phase after resolving all path types
/// in the aliased type. The original TypedBehavior contains the syntactic type
/// (which may have unresolved Path variants), while this behavior contains the
/// fully resolved type.
#[derive(Debug, Clone)]
pub struct TypeAliasTypedBehavior {
    /// The fully resolved type that this type alias refers to
    resolved_ty: Ty,
}

impl Behavior<KestrelLanguage> for TypeAliasTypedBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::TypeAliasTyped
    }
}

impl TypeAliasTypedBehavior {
    /// Create a new TypeAliasTypedBehavior with the resolved type
    pub fn new(resolved_ty: Ty) -> Self {
        TypeAliasTypedBehavior { resolved_ty }
    }

    /// Get the resolved type
    pub fn resolved_ty(&self) -> &Ty {
        &self.resolved_ty
    }
}
