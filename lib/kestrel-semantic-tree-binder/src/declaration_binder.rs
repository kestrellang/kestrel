//! Declaration binder trait and registry
//!
//! This module defines the `DeclarationBinder` trait for converting syntax nodes to symbols,
//! and the `DeclarationBinderRegistry` which maps syntax kinds to their binders.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use kestrel_semantic_model::SemanticModel;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::cycle::CycleDetector;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::binders::{
    ExtensionBinder, FieldBinder, FunctionBinder, ImportBinder, InitializerBinder, ModuleBinder,
    ProtocolBinder, StructBinder, TerminalBinder, TypeAliasBinder,
};
use crate::maps::SourceMap;

/// Trait for binding declarations.
///
/// During the bind phase, we re-walk the symbol hierarchy and (when available)
/// use the stored syntax node for each symbol to resolve types, conformances,
/// bodies, and other relationships.
pub trait DeclarationBinder {
    /// Binding phase: resolve references and establish relationships
    ///
    /// The syntax node is the node stored in the `SemanticModel` for this symbol,
    /// allowing binders to extract type information directly from syntax during binding.
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
    /// Cycle detector for type alias resolution (uses RefCell for interior mutability)
    pub type_alias_cycle_detector: &'a RefCell<CycleDetector<SymbolId>>,
    /// Source code by file name
    pub sources: &'a SourceMap,
}

impl BindingContext<'_> {
    pub fn source_for_symbol(&self, symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> String {
        let mut current = Some(symbol.clone());

        while let Some(sym) = current {
            if sym.metadata().kind()
                == kestrel_semantic_tree::symbol::kind::KestrelSymbolKind::SourceFile
            {
                let file_name = sym.metadata().name().value.clone();
                return self.sources.get(&file_name).cloned().unwrap_or_default();
            }
            current = sym.metadata().parent();
        }

        String::new()
    }
}

/// Registry mapping SyntaxKind to DeclarationBinder implementations
pub struct DeclarationBinderRegistry {
    binders: HashMap<SyntaxKind, Box<dyn DeclarationBinder>>,
}

impl DeclarationBinderRegistry {
    /// Create a new registry with all resolvers registered
    pub fn new() -> Self {
        let mut binders: HashMap<SyntaxKind, Box<dyn DeclarationBinder>> = HashMap::new();

        // Register declaration resolvers
        binders.insert(SyntaxKind::ModuleDeclaration, Box::new(ModuleBinder));
        binders.insert(SyntaxKind::ImportDeclaration, Box::new(ImportBinder));
        binders.insert(SyntaxKind::TypeAliasDeclaration, Box::new(TypeAliasBinder));
        binders.insert(SyntaxKind::ProtocolDeclaration, Box::new(ProtocolBinder));
        binders.insert(SyntaxKind::StructDeclaration, Box::new(StructBinder));
        binders.insert(SyntaxKind::ExtensionDeclaration, Box::new(ExtensionBinder));
        binders.insert(SyntaxKind::FieldDeclaration, Box::new(FieldBinder));
        binders.insert(SyntaxKind::FunctionDeclaration, Box::new(FunctionBinder));
        binders.insert(
            SyntaxKind::InitializerDeclaration,
            Box::new(InitializerBinder),
        );

        // Register terminal resolvers
        binders.insert(SyntaxKind::Visibility, Box::new(TerminalBinder));
        binders.insert(SyntaxKind::Name, Box::new(TerminalBinder));

        DeclarationBinderRegistry { binders }
    }

    /// Get a resolver for a given SyntaxKind
    pub fn get(&self, kind: SyntaxKind) -> Option<&dyn DeclarationBinder> {
        self.binders.get(&kind).map(|b| b.as_ref())
    }
}

impl Default for DeclarationBinderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
