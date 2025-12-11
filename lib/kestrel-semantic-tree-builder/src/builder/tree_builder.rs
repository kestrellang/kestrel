//! Semantic tree builder
//!
//! This module provides `SemanticTreeBuilder` which constructs semantic trees
//! from syntax trees during the build phase.

use std::sync::Arc;

use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::source_file::SourceFileSymbol;
use kestrel_span::{Span, Spanned};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::{Symbol, SymbolTable};

use crate::resolver::ResolverRegistry;
use crate::tree::{build_module_hierarchy, SemanticTree};

use super::ModuleValidator;

/// Builder for constructing semantic trees from syntax
///
/// The builder owns a `SemanticTree` and provides methods to add source files
/// to it. After all files are added, call `build()` to get the finished tree.
///
/// # Example
///
/// ```ignore
/// let mut builder = SemanticTreeBuilder::new();
/// builder.add_file("math.kes", &syntax1, &source1, &mut diagnostics, 0);
/// builder.add_file("vector.kes", &syntax2, &source2, &mut diagnostics, 1);
/// let tree = builder.build();
/// ```
pub struct SemanticTreeBuilder {
    tree: SemanticTree,
    resolver_registry: ResolverRegistry,
}

impl SemanticTreeBuilder {
    /// Create a new builder with an empty semantic tree
    pub fn new() -> Self {
        Self {
            tree: SemanticTree::new(),
            resolver_registry: ResolverRegistry::new(),
        }
    }

    /// Add a source file to the semantic tree
    ///
    /// This processes a single source file and adds its symbols to the tree.
    /// The file's declarations are placed under a SourceFile symbol within
    /// the module hierarchy.
    ///
    /// If module validation fails, diagnostics are emitted but processing
    /// continues with declarations placed directly under the root.
    pub fn add_file(
        &mut self,
        file_name: &str,
        syntax: &SyntaxNode,
        source: &str,
        diagnostics: &mut DiagnosticContext,
        file_id: usize,
    ) {
        let root = self.tree.root().clone();

        // Step 1: Validate and extract module declaration
        let mut validator = ModuleValidator::new(syntax, diagnostics, file_id);
        let module_decl = validator.validate();

        // Step 2: Build/find module hierarchy
        let parent_module = if let Some(decl) = module_decl {
            build_module_hierarchy(&root, &decl.path, self.tree.symbol_table_mut())
        } else {
            root.clone()
        };

        // Step 3: Create a SourceFile symbol under the module
        let file_name_spanned = Spanned::new(file_name.to_string(), Span::from(0..file_name.len()));
        let source_file_symbol: Arc<dyn Symbol<KestrelLanguage>> =
            Arc::new(SourceFileSymbol::new(file_name_spanned, Span::from(0..source.len())));

        parent_module.metadata().add_child(&source_file_symbol);
        self.tree.symbol_table_mut().insert(source_file_symbol.clone());

        // Step 4: Process all top-level declarations
        let mut created_symbols = Vec::new();
        for child in syntax.children() {
            if child.kind() == SyntaxKind::ModuleDeclaration {
                continue;
            }

            if let Some(symbol) = self.walk_node(
                &child,
                source,
                Some(&source_file_symbol),
                &root,
            ) {
                created_symbols.push(symbol);
            }
        }

        // Add all created symbols to the symbol table
        for symbol in created_symbols {
            Self::add_symbol_to_table(&symbol, self.tree.symbol_table_mut());
        }

        // Step 5: Store source code for the bind phase
        self.tree.sources_mut().insert(file_name.to_string(), source.to_string());
    }

    /// Finalize and return the built semantic tree
    pub fn build(self) -> SemanticTree {
        self.tree
    }

    /// Get a reference to the tree being built (for inspection)
    pub fn tree(&self) -> &SemanticTree {
        &self.tree
    }

    /// Walk a syntax node and build symbols using the resolver registry
    fn walk_node(
        &mut self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        // Look up resolver for this syntax kind
        if let Some(resolver) = self.resolver_registry.get(syntax.kind()) {
            // Resolver creates symbol and adds to parent
            if let Some(symbol) = resolver.build_declaration(syntax, source, parent, root) {
                // Store the syntax node for the bind phase
                self.tree
                    .syntax_map_mut()
                    .insert(symbol.metadata().id(), syntax.clone());

                // Check if terminal - if so, don't walk children
                if !resolver.is_terminal() {
                    for child in syntax.children() {
                        self.walk_node(&child, source, Some(&symbol), root);
                    }
                }
                return Some(symbol);
            }
        }

        // No resolver found - walk children anyway (e.g., ClassBody)
        for child in syntax.children() {
            self.walk_node(&child, source, parent, root);
        }

        None
    }

    /// Recursively add a symbol and all its children to the symbol table
    fn add_symbol_to_table(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        table: &mut SymbolTable<KestrelLanguage>,
    ) {
        table.insert(symbol.clone());

        for child in symbol.metadata().children() {
            Self::add_symbol_to_table(&child, table);
        }
    }
}

impl Default for SemanticTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
