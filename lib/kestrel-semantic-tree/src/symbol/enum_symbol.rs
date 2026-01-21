use std::sync::Arc;

use kestrel_span::{Name, Span};
use semantic_tree::symbol::{Symbol, SymbolMetadata, SymbolMetadataBuilder};

use crate::{
    behavior::generics::GenericsBehavior, behavior::visibility::VisibilityBehavior,
    language::KestrelLanguage, symbol::kind::KestrelSymbolKind,
    symbol::type_parameter::TypeParameterSymbol, ty::WhereClause,
};

use super::enum_case::EnumCaseSymbol;

/// Represents an enum declaration in the semantic tree.
///
/// Enums are sum types with named cases, each of which may have associated values.
///
/// # Type Resolution
///
/// During build phase, basic symbol information is captured.
/// During bind phase, `GenericsBehavior` is added with resolved type parameters and where clause.
/// Query methods like `type_parameters()` and `where_clause()` delegate to this behavior.
#[derive(Debug)]
pub struct EnumSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
    is_indirect: bool,
}

impl Symbol<KestrelLanguage> for EnumSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl EnumSymbol {
    /// Create a new EnumSymbol with a name, span, visibility, indirect flag, and optional parent
    pub fn new(
        name: Name,
        span: Span,
        visibility: VisibilityBehavior,
        is_indirect: bool,
        parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
    ) -> Self {
        let mut builder = SymbolMetadataBuilder::new(KestrelSymbolKind::Enum)
            .with_name(name.clone())
            .with_declaration_span(name.span.clone())
            .with_span(span)
            .with_behavior(Arc::new(visibility));

        if let Some(p) = parent {
            builder = builder.with_parent(Arc::downgrade(&p));
        }

        EnumSymbol {
            metadata: builder.build(),
            is_indirect,
        }
    }

    /// Check if this enum is indirect (boxed representation for recursive types)
    pub fn is_indirect(&self) -> bool {
        self.is_indirect
    }

    /// Get all enum cases as children
    pub fn cases(&self) -> Vec<Arc<EnumCaseSymbol>> {
        self.metadata
            .children()
            .into_iter()
            .filter_map(|c| {
                if c.metadata().kind() == KestrelSymbolKind::EnumCase {
                    c.downcast_arc::<EnumCaseSymbol>().ok()
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get the type parameters for this enum.
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

    /// Check if this enum is generic (has type parameters)
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

    /// Get the where clause for this enum.
    ///
    /// Delegates to GenericsBehavior. Returns empty where clause if not yet bound.
    pub fn where_clause(&self) -> WhereClause {
        self.metadata
            .get_behavior::<GenericsBehavior>()
            .map(|g| g.where_clause().clone())
            .unwrap_or_default()
    }
}
