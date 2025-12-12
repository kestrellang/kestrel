use std::sync::Arc;

use kestrel_span::{Name, Span};
use semantic_tree::symbol::{Symbol, SymbolMetadata, SymbolMetadataBuilder};

use crate::{
    behavior::generics::GenericsBehavior, behavior::visibility::VisibilityBehavior,
    language::KestrelLanguage, symbol::kind::KestrelSymbolKind,
    symbol::type_parameter::TypeParameterSymbol, ty::WhereClause,
};

/// Represents a struct declaration in the semantic tree.
///
/// Structs are composite types with fields, methods, and optional generic parameters.
///
/// # Type Resolution
///
/// During build phase, basic symbol information is captured.
/// During bind phase, `GenericsBehavior` is added with resolved type parameters and where clause.
/// Query methods like `type_parameters()` and `where_clause()` delegate to this behavior.
#[derive(Debug)]
pub struct StructSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
}

impl Symbol<KestrelLanguage> for StructSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl StructSymbol {
    /// Create a new StructSymbol with a name, span, visibility, and optional parent
    pub fn new(
        name: Name,
        span: Span,
        visibility: VisibilityBehavior,
        parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
    ) -> Self {
        let mut builder = SymbolMetadataBuilder::new(KestrelSymbolKind::Struct)
            .with_name(name.clone())
            .with_declaration_span(name.span.clone())
            .with_span(span)
            .with_behavior(Arc::new(visibility));

        if let Some(p) = parent {
            builder = builder.with_parent(Arc::downgrade(&p));
        }

        StructSymbol {
            metadata: builder.build(),
        }
    }

    /// Get the type parameters for this struct.
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

    /// Check if this struct is generic (has type parameters)
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

    /// Get the where clause for this struct.
    ///
    /// Delegates to GenericsBehavior. Returns empty where clause if not yet bound.
    pub fn where_clause(&self) -> WhereClause {
        self.metadata
            .get_behavior::<GenericsBehavior>()
            .map(|g| g.where_clause().clone())
            .unwrap_or_else(WhereClause::new)
    }
}
