//! Semantic tree data structure
//!
//! This module contains the `SemanticTree` struct which represents the root
//! of a semantic tree and provides access to the symbol table and syntax map.

use std::collections::HashMap;
use std::sync::Arc;

use kestrel_semantic_tree::behavior::visibility::{Visibility, VisibilityBehavior};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_span::{Span, Spanned};
use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::symbol::{Symbol, SymbolId, SymbolMetadata, SymbolMetadataBuilder, SymbolTable};

/// Storage for source code by file, keyed by file name
pub type SourceMap = HashMap<String, String>;

/// Storage for syntax nodes by symbol ID, allowing bind phase to access syntax
pub type SyntaxMap = HashMap<SymbolId, SyntaxNode>;

/// Represents the root of a semantic tree
pub struct SemanticTree {
    root: Arc<dyn Symbol<KestrelLanguage>>,
    symbol_table: SymbolTable<KestrelLanguage>,
    /// Maps symbol IDs to their original syntax nodes for the bind phase
    syntax_map: SyntaxMap,
    /// Source code by file name, stored during build phase for use in bind phase
    sources: SourceMap,
}

impl SemanticTree {
    /// Create a new empty semantic tree with a root symbol
    pub fn new() -> Self {
        let root: Arc<dyn Symbol<KestrelLanguage>> = Arc::new(RootSymbol::new(Span::from(0..0)));
        let symbol_table = SymbolTable::new();
        let syntax_map = SyntaxMap::new();
        let sources = SourceMap::new();

        SemanticTree {
            root,
            symbol_table,
            syntax_map,
            sources,
        }
    }

    /// Get the root symbol
    pub fn root(&self) -> &Arc<dyn Symbol<KestrelLanguage>> {
        &self.root
    }

    /// Get the symbol table
    ///
    /// # Deprecated
    /// This method exposes the internal symbol table which uses global name lookup.
    /// For context-aware symbol resolution that considers imports and scope,
    /// use `SemanticDatabase` and `queries::Db::resolve_type_path()` instead.
    #[deprecated(
        since = "0.1.0",
        note = "Use SemanticDatabase for context-aware symbol resolution"
    )]
    pub fn symbol_table(&self) -> &SymbolTable<KestrelLanguage> {
        &self.symbol_table
    }

    /// Get a mutable reference to the symbol table
    pub(crate) fn symbol_table_mut(&mut self) -> &mut SymbolTable<KestrelLanguage> {
        &mut self.symbol_table
    }

    /// Get the syntax map (maps symbol IDs to their syntax nodes)
    pub fn syntax_map(&self) -> &SyntaxMap {
        &self.syntax_map
    }

    /// Get a mutable reference to the syntax map (for build phase)
    pub(crate) fn syntax_map_mut(&mut self) -> &mut SyntaxMap {
        &mut self.syntax_map
    }

    /// Get the source map (maps file names to source code)
    pub fn sources(&self) -> &SourceMap {
        &self.sources
    }

    /// Get a mutable reference to the source map (for build phase)
    pub(crate) fn sources_mut(&mut self) -> &mut SourceMap {
        &mut self.sources
    }

    /// Consume the tree and return its parts
    ///
    /// This is used by the binder to take ownership of the tree's components
    /// when creating the SemanticModel.
    pub fn into_parts(self) -> (Arc<dyn Symbol<KestrelLanguage>>, SyntaxMap, SourceMap) {
        (self.root, self.syntax_map, self.sources)
    }
}

impl Default for SemanticTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Root symbol for the semantic tree
#[derive(Debug)]
struct RootSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
}

impl Symbol<KestrelLanguage> for RootSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl RootSymbol {
    fn new(source_span: Span) -> Self {
        let name = Spanned::new("<root>".to_string(), Span::from(0..0));
        // RootSymbol uses Module kind as it represents the root of the module hierarchy
        let metadata = SymbolMetadataBuilder::new(KestrelSymbolKind::Module)
            .with_name(name)
            .with_declaration_span(Span::from(0..0))
            .with_span(source_span)
            .build();

        RootSymbol { metadata }
    }
}

/// Build module hierarchy from path segments
///
/// For a path like ["Math", "Vector"], this creates:
///   Root -> Math -> Vector
/// and returns the Vector module as the effective root.
///
/// Modules are created with Public visibility and added to the symbol table.
pub(crate) fn build_module_hierarchy(
    root: &Arc<dyn Symbol<KestrelLanguage>>,
    path_segments: &[String],
    table: &mut SymbolTable<KestrelLanguage>,
) -> Arc<dyn Symbol<KestrelLanguage>> {
    use kestrel_semantic_tree::symbol::module::ModuleSymbol;

    let mut current_parent = root.clone();

    for segment in path_segments {
        // Check if module already exists as a child
        let existing_module = current_parent
            .metadata()
            .children()
            .iter()
            .find(|child| {
                child.metadata().kind() == KestrelSymbolKind::Module
                    && child.metadata().name().value == *segment
            })
            .cloned();

        let module_symbol = if let Some(existing) = existing_module {
            // Module already exists, use it
            existing
        } else {
            // Create new module
            let name = Spanned::new(segment.clone(), Span::from(0..segment.len()));
            let span = Span::from(0..segment.len()); // Placeholder span
            let visibility = VisibilityBehavior::new(Some(Visibility::Public), Span::from(0..6), root.clone());

            let module = ModuleSymbol::new(name, span, visibility);
            let module_arc: Arc<dyn Symbol<KestrelLanguage>> = Arc::new(module);

            // Add to parent
            current_parent.metadata().add_child(&module_arc);

            // Add to symbol table
            table.insert(module_arc.clone());

            module_arc
        };

        // Move down the hierarchy
        current_parent = module_symbol;
    }

    current_parent
}
