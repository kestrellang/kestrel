//! Semantic binder for resolving references
//!
//! This module provides `SemanticBinder` which orchestrates the bind phase
//! of semantic analysis, resolving all references and establishing relationships.

use std::cell::RefCell;
use std::sync::Arc;

use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_model::{ExtensionRegistry, SemanticModel, SymbolRegistry};
use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::builtins::BuiltinRegistry;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;
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
        // Pre-pass: Register ALL @builtin symbol IDs before binding.
        // This ensures builtin lookups (e.g., Copyable for `not Copyable`)
        // work regardless of module traversal order during bind_signatures.
        // Registration only needs symbol metadata + syntax (@builtin attribute) + source text.
        // Validation (wrong kind, duplicate, must_be_marker, signature checks) stays in
        // each binder's `process_builtin_attribute` during `bind_signature`.
        self.register_all_builtins(&self.root.clone());

        // Pass 1: Bind all signatures (behaviors only, no bodies)
        self.bind_signatures(&self.root.clone(), diagnostics);

        // Pass 1.5: Register extensions in ExtensionRegistry.
        // This reads ExtensionTargetBehavior (attached during bind_signature) and
        // writes to ExtensionRegistry. Deferred here so bind_signature is read-only
        // w.r.t. shared state (ExtensionRegistry). Extension queries (ExtensionsFor)
        // are only used during body resolution, analysis, and lowering.
        self.register_extensions(&self.root.clone());

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

    /// Pre-pass: Register ALL `@builtin` symbol IDs in the BuiltinRegistry.
    ///
    /// This walks the symbol tree looking for symbols with `@builtin` attributes
    /// and registers them before `bind_signatures` runs. This eliminates ordering
    /// dependencies — e.g., `not Copyable` works in any module regardless of whether
    /// `std.core.copy.Copyable` has been bound yet.
    ///
    /// Only registration happens here; validation (wrong kind, must-be-marker,
    /// duplicate detection) is handled by `process_builtin_attribute` during
    /// `bind_signatures`.
    fn register_all_builtins(&self, symbol: &Arc<dyn Symbol<KestrelLanguage>>) {
        let kind = symbol.metadata().kind();
        let is_registrable = matches!(
            kind,
            KestrelSymbolKind::Protocol
                | KestrelSymbolKind::Struct
                | KestrelSymbolKind::Enum
                | KestrelSymbolKind::Function
                | KestrelSymbolKind::TypeAlias
        );

        if is_registrable {
            if let Some(syntax_node) = self.syntax_map.get(&symbol.metadata().id()) {
                let source = Self::source_for_symbol(symbol, &self.sources);
                // Use a scratch diagnostic context to avoid duplicate warnings
                let mut scratch_diagnostics = DiagnosticContext::new();
                let attributes = crate::binders::utils::attributes::resolve_attributes(
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
                    let symbol_id = symbol.metadata().id();
                    let def_kind = &feature.definition().kind;

                    // Register based on the feature's expected kind, matching against
                    // the actual symbol kind. Kind mismatches are silently skipped —
                    // validation catches them later in bind_signature.
                    match kind {
                        KestrelSymbolKind::Protocol if def_kind.is_protocol() => {
                            let _ = self.builtin_registry.register_protocol(feature, symbol_id);
                        },
                        KestrelSymbolKind::Struct if def_kind.is_struct() => {
                            let _ = self.builtin_registry.register_struct(feature, symbol_id);
                        },
                        KestrelSymbolKind::Enum if def_kind.is_enum() => {
                            let _ = self.builtin_registry.register_enum(feature, symbol_id);
                        },
                        KestrelSymbolKind::Function if def_kind.is_function() => {
                            let _ = self.builtin_registry.register_function(feature, symbol_id);
                        },
                        KestrelSymbolKind::Function if def_kind.is_protocol_method() => {
                            let _ = self.builtin_registry.register_method(feature, symbol_id);
                        },
                        KestrelSymbolKind::TypeAlias if def_kind.is_type_alias() => {
                            let _ = self
                                .builtin_registry
                                .register_type_alias(feature, symbol_id);
                        },
                        _ => {
                            // Kind mismatch — silently skip.
                            // Validation in bind_signature will emit BuiltinWrongKindError.
                        },
                    }
                }
            }
        }

        for child in symbol.metadata().children() {
            self.register_all_builtins(&child);
        }
    }

    /// Pass 1.5: Register extensions in ExtensionRegistry.
    ///
    /// Walks the tree looking for extension symbols. For each one that has an
    /// ExtensionTargetBehavior (attached during bind_signature), extracts the
    /// target symbol ID and registers the extension.
    fn register_extensions(&self, symbol: &Arc<dyn Symbol<KestrelLanguage>>) {
        if symbol.metadata().kind() == KestrelSymbolKind::Extension {
            if let Some(target_beh) = symbol.metadata().get_behavior::<ExtensionTargetBehavior>() {
                let target_id = match target_beh.target_type().kind() {
                    kestrel_semantic_tree::ty::TyKind::Struct { symbol: s, .. } => {
                        Some(s.metadata().id())
                    },
                    kestrel_semantic_tree::ty::TyKind::Enum { symbol: s, .. } => {
                        Some(s.metadata().id())
                    },
                    kestrel_semantic_tree::ty::TyKind::Protocol { symbol: s, .. } => {
                        Some(s.metadata().id())
                    },
                    _ => None,
                };

                if let Some(target_id) = target_id {
                    if let Ok(ext) = symbol.clone().downcast_arc::<ExtensionSymbol>() {
                        self.model.register_extension(target_id, ext);
                    }
                }
            }
        }

        for child in symbol.metadata().children() {
            self.register_extensions(&child);
        }
    }

    /// Get source text for a symbol by walking up to its SourceFile ancestor.
    fn source_for_symbol(symbol: &Arc<dyn Symbol<KestrelLanguage>>, sources: &SourceMap) -> String {
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
