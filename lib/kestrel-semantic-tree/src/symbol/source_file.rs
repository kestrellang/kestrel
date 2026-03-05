use std::sync::Arc;

use kestrel_span::{Name, Span};
use semantic_tree::symbol::{Symbol, SymbolMetadata, SymbolMetadataBuilder};

use crate::{
    behavior::markers::{CallableScopeMarker, NamespaceScopeMarker},
    language::KestrelLanguage,
    symbol::kind::KestrelSymbolKind,
};

#[derive(Debug)]
pub struct SourceFileSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
}

impl Symbol<KestrelLanguage> for SourceFileSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl SourceFileSymbol {
    /// Create a new SourceFileSymbol with a file name and span
    pub fn new(name: Name, span: Span, parent: Option<Arc<dyn Symbol<KestrelLanguage>>>) -> Self {
        let mut builder = SymbolMetadataBuilder::new(KestrelSymbolKind::SourceFile)
            .with_name(name.clone())
            .with_declaration_span(name.span.clone())
            .with_span(span)
            .with_behavior(Arc::new(NamespaceScopeMarker))
            .with_behavior(Arc::new(CallableScopeMarker));

        if let Some(p) = parent {
            builder = builder.with_parent(Arc::downgrade(&p));
        }

        SourceFileSymbol {
            metadata: builder.build(),
        }
    }
}
