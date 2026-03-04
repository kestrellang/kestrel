//! Semantic binder for resolving references
//!
//! This module provides `SemanticBinder` which orchestrates the bind phase
//! of semantic analysis, resolving all references and establishing relationships.

use std::cell::RefCell;
use std::sync::Arc;

use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_model::{ExtensionRegistry, SemanticModel, SymbolRegistry};
use kestrel_semantic_tree::builtins::BuiltinRegistry;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_syntax_tree::SyntaxKind;
use semantic_tree::cycle::CycleDetector;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::declaration_binder::{BindingContext, DeclarationBinderRegistry};
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
        // Pre-pass: Register all @builtin protocol IDs before binding.
        // This ensures builtin protocol lookups (e.g., Copyable for `not Copyable`)
        // work regardless of module traversal order during bind_signatures.
        self.register_builtin_protocols(&self.root.clone());

        // Pass 1: Bind all signatures (behaviors only, no bodies)
        self.bind_signatures(&self.root.clone(), diagnostics);

        // Pass 2: Resolve all bodies (all CallableBehaviors now exist)
        self.bind_bodies(&self.root.clone(), diagnostics);

        // Validation passes (including duplicate detection) run as analyzers after binding

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

        if let Some(sk) = syntax_kind
            && let Some(resolver) = self.binder_registry.get(sk)
            && let Some(syntax_node) = self.syntax_map.get(&symbol.metadata().id())
        {
            let mut ctx = BindingContext {
                model: &self.model,
                diagnostics,
                type_alias_cycle_detector: &self.cycle_detector,

                sources: &self.sources,
            };
            resolver.bind_signature(symbol, syntax_node, &mut ctx);
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

        if let Some(sk) = syntax_kind
            && let Some(resolver) = self.binder_registry.get(sk)
            && let Some(syntax_node) = self.syntax_map.get(&symbol.metadata().id())
        {
            let mut ctx = BindingContext {
                model: &self.model,
                diagnostics,
                type_alias_cycle_detector: &self.cycle_detector,

                sources: &self.sources,
            };
            resolver.bind_body(symbol, syntax_node, &mut ctx);
        }

        // Recursively resolve children's bodies
        for child in symbol.metadata().children() {
            self.bind_bodies(&child, diagnostics);
        }
    }

    /// Pre-pass: Register all `@builtin` protocol IDs in the BuiltinRegistry.
    ///
    /// This walks the symbol tree looking for protocols with `@builtin` attributes
    /// and registers them before `bind_signatures` runs. This eliminates ordering
    /// dependencies — e.g., `not Copyable` works in any module regardless of whether
    /// `std.core.copy.Copyable` has been bound yet.
    ///
    /// Only registration happens here; validation (wrong kind, must-be-marker,
    /// duplicate detection) is handled by `process_builtin_attribute` during
    /// `bind_signatures`.
    fn register_builtin_protocols(&self, symbol: &Arc<dyn Symbol<KestrelLanguage>>) {
        if symbol.metadata().kind() == KestrelSymbolKind::Protocol {
            if let Some(syntax_node) = self.syntax_map.get(&symbol.metadata().id()) {
                let source = Self::source_for_symbol(symbol, &self.sources);
                // Use a scratch diagnostic context to avoid duplicate warnings
                let mut scratch_diagnostics = DiagnosticContext::new();
                let attributes =
                    crate::binders::utils::attributes::resolve_attributes(
                        syntax_node,
                        &source,
                        0, // file_id doesn't matter for attribute parsing
                        &mut scratch_diagnostics,
                    );
                if let crate::binders::utils::attributes::BuiltinParseResult::Success(feature) =
                    crate::binders::utils::attributes::parse_builtin_attribute(
                        &attributes,
                        &source,
                        &mut scratch_diagnostics,
                    )
                {
                    if feature.definition().kind.is_protocol() {
                        let _ = self
                            .builtin_registry
                            .register_protocol(feature, symbol.metadata().id());
                    }
                }
            }
        }

        for child in symbol.metadata().children() {
            self.register_builtin_protocols(&child);
        }
    }

    /// Get source text for a symbol by walking up to its SourceFile ancestor.
    fn source_for_symbol(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        sources: &SourceMap,
    ) -> String {
        let mut current = Some(symbol.clone());
        while let Some(sym) = current {
            if sym.metadata().kind() == KestrelSymbolKind::SourceFile {
                let file_name = sym.metadata().name().value.clone();
                return sources.get(&file_name).cloned().unwrap_or_default();
            }
            current = sym.metadata().parent();
        }
        String::new()
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
}
