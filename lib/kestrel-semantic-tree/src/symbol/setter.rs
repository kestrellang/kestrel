use std::sync::Arc;

use kestrel_span::{Name, Span};
use semantic_tree::symbol::{Symbol, SymbolId, SymbolMetadata, SymbolMetadataBuilder};

use crate::{
    behavior::markers::AccessorMarker, language::KestrelLanguage, symbol::kind::KestrelSymbolKind,
};

/// Represents a synthetic setter for a computed property.
///
/// Setters are created for computed properties that have a `set` block.
/// They have a synthetic name like "set:fieldName" to distinguish them
/// from regular methods while allowing lookup by the field they set.
#[derive(Debug)]
pub struct SetterSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
}

impl Symbol<KestrelLanguage> for SetterSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl SetterSymbol {
    /// Create a new SetterSymbol
    ///
    /// # Arguments
    /// * `id` - The unique symbol ID for this setter
    /// * `parent` - The parent field symbol
    /// * `field_name` - The name of the field this setter is for
    /// * `name_span` - The span of the setter's name in source
    /// * `full_span` - The full span of the setter declaration
    pub fn new(
        id: SymbolId,
        parent: &Arc<dyn Symbol<KestrelLanguage>>,
        field_name: &str,
        name_span: Span,
        full_span: Span,
    ) -> Self {
        let synthetic_name = format!("set:{}", field_name);
        let name = Name::new(synthetic_name, name_span.clone());

        let metadata = SymbolMetadataBuilder::new(KestrelSymbolKind::Setter)
            .with_id(id)
            .with_name(name)
            .with_declaration_span(name_span)
            .with_span(full_span)
            .with_parent(Arc::downgrade(parent))
            .with_behavior(Arc::new(AccessorMarker))
            .build();

        SetterSymbol { metadata }
    }
}
