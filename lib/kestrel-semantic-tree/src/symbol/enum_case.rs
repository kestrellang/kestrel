use std::sync::Arc;

use kestrel_span::{Name, Span};
use semantic_tree::symbol::{Symbol, SymbolMetadata, SymbolMetadataBuilder};

use crate::{language::KestrelLanguage, symbol::kind::KestrelSymbolKind};

use super::enum_symbol::EnumSymbol;

/// Represents an enum case within an enum declaration.
///
/// Enum cases can optionally have associated values (parameters), which are
/// represented using `CallableBehavior` attached during the bind phase.
///
/// # Examples
///
/// ```kestrel
/// enum Color {
///     case Red              // Simple case, no associated values
///     case RGB(r: I32, g: I32, b: I32)  // Case with associated values
/// }
/// ```
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
    /// Create a new EnumCaseSymbol with a name, span, and optional parent
    pub fn new(
        name: Name,
        span: Span,
        parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
    ) -> Self {
        let mut builder = SymbolMetadataBuilder::new(KestrelSymbolKind::EnumCase)
            .with_name(name.clone())
            .with_declaration_span(name.span.clone())
            .with_span(span);

        if let Some(p) = parent {
            builder = builder.with_parent(Arc::downgrade(&p));
        }

        EnumCaseSymbol {
            metadata: builder.build(),
        }
    }

    /// Check if this case has associated values.
    ///
    /// This is determined by the presence of `CallableBehavior`, which is
    /// added during the bind phase for cases with parameter lists.
    pub fn has_associated_values(&self) -> bool {
        use crate::behavior::callable::CallableBehavior;
        self.metadata.get_behavior::<CallableBehavior>().is_some()
    }

    /// Get the parent enum of this case
    pub fn parent_enum(&self) -> Option<Arc<EnumSymbol>> {
        self.metadata
            .parent()
            .and_then(|arc_dyn| arc_dyn.downcast_arc::<EnumSymbol>().ok())
    }
}
