use std::sync::Arc;

use kestrel_span::{Name, Span};
use semantic_tree::symbol::{Symbol, SymbolMetadata, SymbolMetadataBuilder};

use crate::{
    behavior::callable::CallableBehavior, behavior::visibility::VisibilityBehavior,
    language::KestrelLanguage, symbol::kind::KestrelSymbolKind,
};

/// Represents an enum case in the semantic tree.
///
/// Enum cases are the individual variants of an enum type. Each case may optionally
/// have associated values, which are represented via `CallableBehavior`.
///
/// # Associated Values
///
/// Cases with associated values (e.g., `case Some(value: T)`) have a `CallableBehavior`
/// attached during the bind phase. Cases without associated values (e.g., `case None`)
/// do not have this behavior.
#[derive(Debug)]
pub struct EnumCaseSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
}

impl Symbol<KestrelLanguage> for EnumCaseSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl EnumCaseSymbol {
    /// Create a new EnumCaseSymbol with a name, span, visibility, and optional parent
    pub fn new(
        name: Name,
        span: Span,
        visibility: VisibilityBehavior,
        parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
    ) -> Self {
        let mut builder = SymbolMetadataBuilder::new(KestrelSymbolKind::EnumCase)
            .with_name(name.clone())
            .with_declaration_span(name.span.clone())
            .with_span(span)
            .with_behavior(Arc::new(visibility));

        if let Some(p) = parent {
            builder = builder.with_parent(Arc::downgrade(&p));
        }

        EnumCaseSymbol {
            metadata: builder.build(),
        }
    }

    /// Returns true if this case has associated values (has CallableBehavior)
    pub fn has_associated_values(&self) -> bool {
        self.metadata.get_behavior::<CallableBehavior>().is_some()
    }

    /// Get the CallableBehavior if this case has associated values
    pub fn callable_behavior(&self) -> Option<Arc<CallableBehavior>> {
        self.metadata.get_behavior::<CallableBehavior>()
    }
}
