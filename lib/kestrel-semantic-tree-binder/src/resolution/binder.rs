//! Semantic binder for resolving references
//!
//! This module provides `SemanticBinder` which orchestrates the bind phase
//! of semantic analysis, resolving all references and establishing relationships.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_model::{ExtensionRegistry, SemanticModel, SymbolRegistry};
use kestrel_semantic_tree::behavior::callable::CallableSignature;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxKind;
use semantic_tree::cycle::CycleDetector;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::diagnostics::DuplicateFunctionSignatureError;
use crate::declaration_binder::{BindingContext, DeclarationBinderRegistry};
use crate::tree::{SourceMap, SyntaxMap};

/// Binder for resolving references in a semantic tree
///
/// The binder orchestrates the bind phase, walking all symbols and calling
/// their resolvers to resolve references and establish relationships.
///
/// # Example
///
/// ```ignore
/// let tree = builder.build();
/// let model = SemanticBinder::bind(tree, &mut diagnostics);
/// ```
pub struct SemanticBinder {
    /// Root symbol from the tree
    root: Arc<dyn Symbol<KestrelLanguage>>,
    /// Syntax map from the tree
    syntax_map: SyntaxMap,
    /// Source map from the tree
    sources: SourceMap,
    /// Shared symbol registry
    registry: SymbolRegistry,
    /// Shared extension registry
    extension_registry: ExtensionRegistry,
    /// Semantic model used during binding for resolvers
    model: SemanticModel,
    binder_registry: DeclarationBinderRegistry,
    cycle_detector: RefCell<CycleDetector<SymbolId>>,
}

impl SemanticBinder {
    /// Bind a semantic tree and return the semantic model
    ///
    /// This is the primary entry point for the binding phase. It consumes the
    /// model, runs all binding passes, and returns a SemanticModel.
    pub fn bind(model: SemanticModel, diagnostics: &mut DiagnosticContext) -> SemanticModel {
        let mut binder = Self::from_model(model);
        binder.run_binding(diagnostics)
    }

    fn from_model(model: SemanticModel) -> Self {
        let (root, syntax_map, sources, registry, extension_registry) = model.into_parts();

        let model = SemanticModel::with_registries(
            root.clone(),
            syntax_map.clone(),
            sources.clone(),
            registry.clone(),
            extension_registry.clone(),
        );

        Self {
            root,
            syntax_map,
            sources,
            registry,
            extension_registry,
            model,
            binder_registry: DeclarationBinderRegistry::new(),
            cycle_detector: RefCell::new(CycleDetector::new()),
        }
    }

    /// Run the binding phase and return the semantic model (internal)
    fn run_binding(&mut self, diagnostics: &mut DiagnosticContext) -> SemanticModel {
        // Walk all symbols and call bind_declaration
        self.bind_symbol(&self.root.clone(), diagnostics);

        // Post-binding pass: detect duplicate function signatures
        self.check_duplicate_signatures(&self.root.clone(), diagnostics);

        // Validation passes migrated to analyzers; run in compiler/test harness after binding

        // Create SemanticModel with the shared registries
        SemanticModel::with_registries(
            self.root.clone(),
            self.syntax_map.clone(),
            self.sources.clone(),
            self.registry.clone(),
            self.extension_registry.clone(),
        )
    }

    /// Recursively bind a symbol and its children
    fn bind_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        diagnostics: &mut DiagnosticContext,
    ) {
        let kind = symbol.metadata().kind();

        // Map symbol kind to syntax kind for resolver lookup
        let syntax_kind = Self::symbol_kind_to_syntax_kind(kind);

        if let Some(sk) = syntax_kind {
            if let Some(resolver) = self.binder_registry.get(sk) {
                if let Some(syntax_node) = self.syntax_map.get(&symbol.metadata().id()) {
                    let mut ctx = BindingContext {
                        model: &self.model,
                        diagnostics,
                        type_alias_cycle_detector: &self.cycle_detector,
                        sources: &self.sources,
                    };
                    resolver.bind_declaration(symbol, syntax_node, &mut ctx);
                }
            }
        }

        // Recursively bind children
        for child in symbol.metadata().children() {
            self.bind_symbol(&child, diagnostics);
        }
    }

    /// Map symbol kind to syntax kind for resolver lookup
    fn symbol_kind_to_syntax_kind(kind: KestrelSymbolKind) -> Option<SyntaxKind> {
        match kind {
            KestrelSymbolKind::AssociatedType => Some(SyntaxKind::TypeAliasDeclaration),
            KestrelSymbolKind::Extension => Some(SyntaxKind::ExtensionDeclaration),
            KestrelSymbolKind::Import => Some(SyntaxKind::ImportDeclaration),
            KestrelSymbolKind::Initializer => Some(SyntaxKind::InitializerDeclaration),
            KestrelSymbolKind::Protocol => Some(SyntaxKind::ProtocolDeclaration),
            KestrelSymbolKind::Struct => Some(SyntaxKind::StructDeclaration),
            KestrelSymbolKind::Field => Some(SyntaxKind::FieldDeclaration),
            KestrelSymbolKind::Function => Some(SyntaxKind::FunctionDeclaration),
            KestrelSymbolKind::Module => Some(SyntaxKind::ModuleDeclaration),
            KestrelSymbolKind::TypeAlias => Some(SyntaxKind::TypeAliasDeclaration),
            KestrelSymbolKind::TypeParameter => Some(SyntaxKind::TypeParameter),
            KestrelSymbolKind::SourceFile => None,
        }
    }

    /// Check for duplicate function signatures within each scope
    fn check_duplicate_signatures(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        diagnostics: &mut DiagnosticContext,
    ) {
        let kind = symbol.metadata().kind();

        // Scopes that can contain functions: Module, Struct, SourceFile
        let is_scope = matches!(
            kind,
            KestrelSymbolKind::Module | KestrelSymbolKind::Struct | KestrelSymbolKind::SourceFile
        );

        if is_scope {
            let mut signatures: HashMap<CallableSignature, Vec<Arc<dyn Symbol<KestrelLanguage>>>> =
                HashMap::new();

            for child in symbol.metadata().children() {
                if child.metadata().kind() == KestrelSymbolKind::Function {
                    if let Some(func_sym) = child.as_ref().downcast_ref::<FunctionSymbol>() {
                        let sig = func_sym.signature();
                        signatures.entry(sig).or_default().push(child.clone());
                    }
                }
            }

            // Report duplicates
            for (sig, funcs) in signatures {
                if funcs.len() > 1 {
                    let first = &funcs[0];
                    let first_span = first.metadata().declaration_span().clone();

                    let duplicate_spans: Vec<Span> = funcs[1..]
                        .iter()
                        .map(|f| {
                            f.metadata().declaration_span().clone()
                        })
                        .collect();

                    diagnostics.throw(DuplicateFunctionSignatureError {
                        signature: sig.display(),
                        first_span,
                        duplicate_spans,
                    });
                }
            }
        }

        // Recursively check children
        for child in symbol.metadata().children() {
            self.check_duplicate_signatures(&child, diagnostics);
        }
    }
}
