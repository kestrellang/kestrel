use std::sync::Arc;

use kestrel_span::{Name, Span};
use semantic_tree::symbol::{Symbol, SymbolId, SymbolMetadata, SymbolMetadataBuilder};

use crate::{
    behavior::markers::AccessorMarker, language::KestrelLanguage, symbol::kind::KestrelSymbolKind,
};

/// Represents a synthetic getter for a computed property.
///
/// Getters are created for computed properties and provide read access
/// to a property value through a getter function.
#[derive(Debug)]
pub struct GetterSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
}

impl Symbol<KestrelLanguage> for GetterSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl GetterSymbol {
    /// Create a new GetterSymbol
    ///
    /// # Arguments
    /// * `id` - The unique symbol ID for this getter
    /// * `parent` - The parent field symbol
    /// * `field_name` - The name of the field this getter is for
    /// * `name_span` - The span of the getter name in source
    /// * `full_span` - The full span of the getter declaration
    pub fn new(
        id: SymbolId,
        parent: &Arc<dyn Symbol<KestrelLanguage>>,
        field_name: &str,
        name_span: Span,
        full_span: Span,
    ) -> Self {
        let synthetic_name = format!("get:{}", field_name);
        let name = Name::new(synthetic_name, name_span.clone());

        let builder = SymbolMetadataBuilder::new(KestrelSymbolKind::Getter)
            .with_id(id)
            .with_name(name)
            .with_declaration_span(name_span)
            .with_span(full_span)
            .with_parent(Arc::downgrade(parent))
            .with_behavior(Arc::new(AccessorMarker));

        GetterSymbol {
            metadata: builder.build(),
        }
    }
}
