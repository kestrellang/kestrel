//! Resolver trait and registry
//!
//! This module defines the `Resolver` trait for converting syntax nodes to symbols,
//! and the `ResolverRegistry` which maps syntax kinds to their resolvers.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use kestrel_semantic_model::SemanticModel;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::cycle::CycleDetector;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::resolvers::{
    ExtensionResolver, FieldResolver, FunctionResolver, ImportResolver, InitializerResolver,
    ModuleResolver, ProtocolResolver, StructResolver, TerminalResolver, TypeAliasResolver,
};
use crate::tree::SourceMap;

/// Trait for resolving syntax nodes into semantic symbols
pub trait Resolver {
    /// Build phase: create symbol from syntax node and add to parent
    /// Returns the created symbol for tree walker recursion
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>>;

    /// Binding phase: resolve references and establish relationships
    ///
    /// The syntax node is the same node that was passed to build_declaration,
    /// allowing resolvers to extract type information directly from syntax
    /// during binding rather than storing intermediate Path representations.
    fn bind_declaration(
        &self,
        _symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        _syntax: &SyntaxNode,
        _context: &mut BindingContext,
    ) {
        // Default: do nothing
    }

    /// Whether this node is terminal (stops tree traversal)
    fn is_terminal(&self) -> bool {
        false
    }
}

/// Context for the binding phase
pub struct BindingContext<'a> {
    /// Semantic model for queries
    pub model: &'a SemanticModel,
    /// Diagnostics collector
    pub diagnostics: &'a mut kestrel_reporting::DiagnosticContext,
    /// Current file ID for error reporting
    pub file_id: usize,
    /// Cycle detector for type alias resolution (uses RefCell for interior mutability)
    pub type_alias_cycle_detector: &'a RefCell<CycleDetector<SymbolId>>,
    /// Source code by file name
    pub sources: &'a SourceMap,
}

impl BindingContext<'_> {
    /// Get file_id and source code for a symbol in one call.
    ///
    /// This is the preferred method for resolvers that need both file_id (for diagnostics)
    /// and source code (for span calculation). It performs a single parent-chain traversal.
    ///
    /// Returns (file_id, source) where file_id falls back to self.file_id and source is cloned.
    pub fn get_file_context(&self, symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> (usize, String) {
        match crate::syntax::get_source_file_info(symbol, self.diagnostics) {
            Some(info) => {
                let source = self.sources.get(&info.file_name).cloned().unwrap_or_default();
                (info.file_id, source)
            }
            None => (self.file_id, String::new()),
        }
    }
}

/// Registry mapping SyntaxKind to Resolver implementations
pub struct ResolverRegistry {
    resolvers: HashMap<SyntaxKind, Box<dyn Resolver>>,
}

impl ResolverRegistry {
    /// Create a new registry with all resolvers registered
    pub fn new() -> Self {
        let mut resolvers: HashMap<SyntaxKind, Box<dyn Resolver>> = HashMap::new();

        // Register declaration resolvers
        resolvers.insert(SyntaxKind::ModuleDeclaration, Box::new(ModuleResolver));
        resolvers.insert(SyntaxKind::ImportDeclaration, Box::new(ImportResolver));
        resolvers.insert(SyntaxKind::TypeAliasDeclaration, Box::new(TypeAliasResolver));
        resolvers.insert(SyntaxKind::ProtocolDeclaration, Box::new(ProtocolResolver));
        resolvers.insert(SyntaxKind::StructDeclaration, Box::new(StructResolver));
        resolvers.insert(SyntaxKind::ExtensionDeclaration, Box::new(ExtensionResolver));
        resolvers.insert(SyntaxKind::FieldDeclaration, Box::new(FieldResolver));
        resolvers.insert(SyntaxKind::FunctionDeclaration, Box::new(FunctionResolver));
        resolvers.insert(SyntaxKind::InitializerDeclaration, Box::new(InitializerResolver));

        // Register terminal resolvers
        resolvers.insert(SyntaxKind::Visibility, Box::new(TerminalResolver));
        resolvers.insert(SyntaxKind::Name, Box::new(TerminalResolver));

        ResolverRegistry { resolvers }
    }

    /// Get a resolver for a given SyntaxKind
    pub fn get(&self, kind: SyntaxKind) -> Option<&dyn Resolver> {
        self.resolvers.get(&kind).map(|b| b.as_ref())
    }
}

impl Default for ResolverRegistry {
    fn default() -> Self {
        Self::new()
    }
}
