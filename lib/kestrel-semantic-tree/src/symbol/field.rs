use std::sync::Arc;

use kestrel_span::{Name, Span};
use semantic_tree::symbol::{Symbol, SymbolMetadata, SymbolMetadataBuilder};

use crate::{
    behavior::visibility::VisibilityBehavior, language::KestrelLanguage,
    symbol::kind::KestrelSymbolKind, ty::Ty,
};

#[derive(Debug)]
pub struct FieldSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
    is_static: bool,
    is_mutable: bool,
    field_type: Ty,
}

impl Symbol<KestrelLanguage> for FieldSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        return &self.metadata;
    }
}

impl FieldSymbol {
    /// Create a new FieldSymbol with name, span, visibility, static/mutability modifiers, and type
    pub fn new(
        name: Name,
        span: Span,
        visibility: VisibilityBehavior,
        is_static: bool,
        is_mutable: bool,
        field_type: Ty,
        parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
    ) -> Self {
        let mut builder = SymbolMetadataBuilder::new(KestrelSymbolKind::Field)
            .with_name(name.clone())
            .with_declaration_span(name.span.clone())
            .with_span(span)
            .with_behavior(Arc::new(visibility));

        if let Some(p) = parent {
            builder = builder.with_parent(Arc::downgrade(&p));
        }

        FieldSymbol {
            metadata: builder.build(),
            is_static,
            is_mutable,
            field_type,
        }
    }

    /// Check if this field is static
    pub fn is_static(&self) -> bool {
        self.is_static
    }

    /// Check if this field is mutable (var vs let)
    pub fn is_mutable(&self) -> bool {
        self.is_mutable
    }

    /// Get the field's type
    pub fn field_type(&self) -> &Ty {
        &self.field_type
    }
}
