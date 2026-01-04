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
    DeinitBinder, EnumBinder, EnumCaseBinder, ExtensionBinder, FieldBinder, FunctionBinder,
    ImportBinder, InitializerBinder, ModuleBinder, ProtocolBinder, StructBinder, TerminalBinder,
    TypeAliasBinder,
};
use crate::maps::SourceMap;

/// Trait for binding declarations.
///
/// During the bind phase, we re-walk the symbol hierarchy and (when available)
/// use the stored syntax node for each symbol to resolve types, conformances,
/// bodies, and other relationships.
///
/// Binding is split into two passes to handle forward references:
/// - Pass 1 (`bind_signature`): Attach behaviors like CallableBehavior, GenericsBehavior, etc.
/// - Pass 2 (`bind_body`): Resolve function/initializer bodies (requires all signatures to be bound)
///
/// This two-pass approach ensures that when resolving a function body, all functions
/// (including those declared later in the file) have their CallableBehavior attached.
pub trait DeclarationBinder {
    /// Pass 1: Bind the declaration's signature (attach behaviors).
    ///
    /// This is called for all symbols before any bodies are resolved.
    /// Implementations should attach CallableBehavior, GenericsBehavior, TypedBehavior, etc.
    /// but should NOT resolve function/initializer bodies.
    fn bind_signature(
        &self,
        _symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        _syntax: &SyntaxNode,
        _context: &mut BindingContext,
    ) {
        // Default: do nothing
    }

    /// Pass 2: Resolve the declaration's body (if any).
    ///
    /// This is called after all signatures have been bound, ensuring that forward
    /// references to functions declared later in the file can be resolved correctly.
    fn bind_body(
        &self,
        _symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        _syntax: &SyntaxNode,
        _context: &mut BindingContext,
    ) {
        // Default: do nothing (most declarations don't have bodies)
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
    /// Cycle detector for copy semantics computation (to handle recursive types)
    pub copy_semantics_cycle_detector: &'a RefCell<CycleDetector<SymbolId>>,
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

    pub fn file_id_for_symbol(&self, symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> usize {
        let mut current = Some(symbol.clone());

        while let Some(sym) = current {
            if sym.metadata().kind()
                == kestrel_semantic_tree::symbol::kind::KestrelSymbolKind::SourceFile
            {
                let file_name = sym.metadata().name().value.clone();
                return self.diagnostics.get_file_id(&file_name).unwrap_or(0);
            }
            current = sym.metadata().parent();
        }

        0
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
        binders.insert(SyntaxKind::EnumDeclaration, Box::new(EnumBinder));
        binders.insert(SyntaxKind::EnumCaseDeclaration, Box::new(EnumCaseBinder));
        binders.insert(SyntaxKind::FieldDeclaration, Box::new(FieldBinder));
        binders.insert(SyntaxKind::FunctionDeclaration, Box::new(FunctionBinder));
        binders.insert(
            SyntaxKind::InitializerDeclaration,
            Box::new(InitializerBinder),
        );
        binders.insert(SyntaxKind::DeinitDeclaration, Box::new(DeinitBinder));

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
