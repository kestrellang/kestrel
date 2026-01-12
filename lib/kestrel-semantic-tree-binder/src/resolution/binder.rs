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
use kestrel_semantic_tree::builtins::BuiltinRegistry;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxKind;
use semantic_tree::cycle::CycleDetector;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::declaration_binder::{BindingContext, DeclarationBinderRegistry};
use crate::diagnostics::DuplicateFunctionSignatureError;
use crate::maps::{SourceMap, SyntaxMap};

/// Binder for resolving references in a semantic tree
///
/// The binder orchestrates the bind phase, walking all symbols and calling
/// their resolvers to resolve references and establish relationships.
///
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
    /// Shared builtin registry
    builtin_registry: Arc<BuiltinRegistry>,
    /// Semantic model used during binding for resolvers
    model: SemanticModel,
    binder_registry: DeclarationBinderRegistry,
    /// Cycle detector for type alias resolution
    cycle_detector: RefCell<CycleDetector<SymbolId>>,
    /// Cycle detector for copy semantics computation (handles recursive struct types)
    copy_semantics_cycle_detector: RefCell<CycleDetector<SymbolId>>,
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
        let (root, syntax_map, sources, registry, extension_registry, builtin_registry) =
            model.into_parts();

        let model = SemanticModel::with_registries(
            root.clone(),
            syntax_map.clone(),
            sources.clone(),
            registry.clone(),
            extension_registry.clone(),
            builtin_registry.clone(),
        );

        Self {
            root,
            syntax_map,
            sources,
            registry,
            extension_registry,
            builtin_registry,
            model,
            binder_registry: DeclarationBinderRegistry::new(),
            cycle_detector: RefCell::new(CycleDetector::new()),
            copy_semantics_cycle_detector: RefCell::new(CycleDetector::new()),
        }
    }

    /// Run the binding phase and return the semantic model (internal)
    ///
    /// Binding is split into two passes to handle forward references:
    /// - Pass 1: Bind all signatures (attach CallableBehavior, GenericsBehavior, etc.)
    /// - Pass 2: Resolve all bodies (now all CallableBehaviors exist)
    ///
    /// This ensures that when resolving a function body, all functions in the file
    /// (including those declared later) have their CallableBehavior attached.
    fn run_binding(&mut self, diagnostics: &mut DiagnosticContext) -> SemanticModel {
        // Pass 1: Bind all signatures (behaviors only, no bodies)
        self.bind_signatures(&self.root.clone(), diagnostics);

        // Pass 2: Resolve all bodies (all CallableBehaviors now exist)
        self.bind_bodies(&self.root.clone(), diagnostics);

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
            self.builtin_registry.clone(),
        )
    }

    /// Pass 1: Recursively bind signatures (behaviors only, no bodies)
    ///
    /// This attaches CallableBehavior, GenericsBehavior, TypedBehavior, etc.
    /// to all symbols, but does NOT resolve function/initializer bodies.
    fn bind_signatures(
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
                        copy_semantics_cycle_detector: &self.copy_semantics_cycle_detector,
                        sources: &self.sources,
                    };
                    resolver.bind_signature(symbol, syntax_node, &mut ctx);
                }
            }
        }

        // Recursively bind children's signatures
        for child in symbol.metadata().children() {
            self.bind_signatures(&child, diagnostics);
        }
    }

    /// Pass 2: Recursively resolve bodies (all signatures now exist)
    ///
    /// At this point, all CallableBehaviors have been attached, so forward
    /// references to functions declared later in the file can be resolved.
    fn bind_bodies(
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
                        copy_semantics_cycle_detector: &self.copy_semantics_cycle_detector,
                        sources: &self.sources,
                    };
                    resolver.bind_body(symbol, syntax_node, &mut ctx);
                }
            }
        }

        // Recursively resolve children's bodies
        for child in symbol.metadata().children() {
            self.bind_bodies(&child, diagnostics);
        }
    }

    /// Map symbol kind to syntax kind for resolver lookup
    fn symbol_kind_to_syntax_kind(kind: KestrelSymbolKind) -> Option<SyntaxKind> {
        match kind {
            KestrelSymbolKind::AssociatedType => Some(SyntaxKind::TypeAliasDeclaration),
            KestrelSymbolKind::Deinit => Some(SyntaxKind::DeinitDeclaration),
            KestrelSymbolKind::Enum => Some(SyntaxKind::EnumDeclaration),
            KestrelSymbolKind::EnumCase => Some(SyntaxKind::EnumCaseDeclaration),
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
            KestrelSymbolKind::Getter => Some(SyntaxKind::GetterClause),
            KestrelSymbolKind::Setter => Some(SyntaxKind::SetterClause),
            KestrelSymbolKind::Subscript => Some(SyntaxKind::SubscriptDeclaration),
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
                        .map(|f| f.metadata().declaration_span().clone())
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
