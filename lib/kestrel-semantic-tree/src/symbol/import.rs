use std::sync::{Arc, RwLock};

use kestrel_span::{Name, Span};
use semantic_tree::behavior::Behavior;
use semantic_tree::symbol::{Symbol, SymbolId, SymbolMetadata, SymbolMetadataBuilder};

use crate::{
    behavior::KestrelBehaviorKind, language::KestrelLanguage, symbol::kind::KestrelSymbolKind,
};

#[derive(Debug)]
pub struct ImportSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
}

impl Symbol<KestrelLanguage> for ImportSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl ImportSymbol {
    /// Create a new ImportSymbol
    pub fn new(name: Name, parent: Arc<dyn Symbol<KestrelLanguage>>, span: Span) -> Self {
        let metadata = SymbolMetadataBuilder::new(KestrelSymbolKind::Import)
            .with_parent(Arc::downgrade(&parent))
            .with_name(name.clone())
            .with_declaration_span(name.span.clone())
            .with_span(span)
            .build();

        ImportSymbol { metadata }
    }
}

/// Import data behavior stores the parsed import information
#[derive(Debug)]
pub struct ImportDataBehavior {
    /// The module path with spans (e.g., [("A", Span::new(0, 0..1)), ("B", Span::new(0, 2..3)), ("C", Span::new(0, 4..5))])
    module_path_segments: Vec<(String, Span)>,
    /// Span of the entire module path
    module_path_span: Span,
    /// Optional alias for the module (e.g., "D" for "import A.B.C as D")
    alias: Option<String>,
    /// Specific items to import (e.g., [("Foo", None), ("Bar", Some("Baz"))])
    /// Empty if importing the entire module.
    /// Uses RwLock to allow setting target_id during bind phase.
    items: RwLock<Vec<ImportItem>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportItem {
    /// The name of the symbol to import
    pub name: String,
    /// Optional alias for this specific import
    pub alias: Option<String>,
    /// Span of the item name in the source
    pub span: Span,
    /// Resolved symbol ID (filled during bind phase)
    pub target_id: Option<SymbolId>,
}

impl Behavior<KestrelLanguage> for ImportDataBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::ImportData
    }
}

impl ImportDataBehavior {
    pub fn new(
        module_path_segments: Vec<(String, Span)>,
        module_path_span: Span,
        alias: Option<String>,
        items: Vec<ImportItem>,
    ) -> Self {
        ImportDataBehavior {
            module_path_segments,
            module_path_span,
            alias,
            items: RwLock::new(items),
        }
    }

    /// Get the module path as a slice of segment names
    pub fn module_path(&self) -> Vec<String> {
        self.module_path_segments
            .iter()
            .map(|(s, _)| s.clone())
            .collect()
    }

    /// Get the module path segments with their spans
    pub fn module_path_segments(&self) -> &[(String, Span)] {
        &self.module_path_segments
    }

    /// Get the span of the entire module path
    pub fn module_path_span(&self) -> &Span {
        &self.module_path_span
    }

    pub fn alias(&self) -> Option<&str> {
        self.alias.as_deref()
    }

    /// Returns a clone of the import items.
    pub fn items(&self) -> Vec<ImportItem> {
        self.items.read().expect("RwLock poisoned").clone()
    }

    /// Set the resolved target_id for an import item by name.
    ///
    /// Returns true if the item was found and updated, false otherwise.
    pub fn set_target_id(&self, name: &str, target_id: SymbolId) -> bool {
        let mut items = self.items.write().expect("RwLock poisoned");
        if let Some(item) = items.iter_mut().find(|i| i.name == name) {
            item.target_id = Some(target_id);
            true
        } else {
            false
        }
    }

    /// Check if all import items have been resolved.
    pub fn all_resolved(&self) -> bool {
        self.items
            .read()
            .expect("RwLock poisoned")
            .iter()
            .all(|item| item.target_id.is_some())
    }
}

impl Clone for ImportDataBehavior {
    fn clone(&self) -> Self {
        ImportDataBehavior {
            module_path_segments: self.module_path_segments.clone(),
            module_path_span: self.module_path_span.clone(),
            alias: self.alias.clone(),
            items: RwLock::new(self.items.read().expect("RwLock poisoned").clone()),
        }
    }
}
