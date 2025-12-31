//! Semantic model for Kestrel compiler
//!
//! The SemanticModel is the primary interface for querying semantic information
//! about a compiled Kestrel program. It owns the symbol tree, syntax mappings,
//! source code, and registries.

use std::collections::HashMap;
use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::behavior::Behavior;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::extension_registry::ExtensionRegistry;
use crate::query::Query;
use crate::registry::SymbolRegistry;
use kestrel_semantic_tree::builtins::BuiltinRegistry;
use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;

/// The semantic model for a Kestrel program.
///
/// This is the primary interface for querying semantic information about a
/// compiled program. It owns the symbol tree, syntax mappings, source code,
/// and registries needed for semantic analysis.
pub struct SemanticModel {
    /// The root symbol of the semantic tree
    root: Arc<dyn Symbol<KestrelLanguage>>,
    /// Maps symbol IDs to their original syntax nodes
    syntax_map: HashMap<SymbolId, SyntaxNode>,
    /// Source code by filename
    sources: HashMap<String, String>,
    /// Symbol registry for O(1) lookup
    registry: SymbolRegistry,
    /// Extension registry for extension lookups
    extension_registry: ExtensionRegistry,
    /// Builtin registry for language feature lookups
    builtin_registry: Arc<BuiltinRegistry>,
}

impl SemanticModel {
    /// Create a new SemanticModel from tree components.
    ///
    /// Called by SemanticBinder after the build phase.
    pub fn new(
        root: Arc<dyn Symbol<KestrelLanguage>>,
        syntax_map: HashMap<SymbolId, SyntaxNode>,
        sources: HashMap<String, String>,
    ) -> Self {
        let registry = SymbolRegistry::new();
        registry.register_tree(&root);

        Self {
            root,
            syntax_map,
            sources,
            registry,
            extension_registry: ExtensionRegistry::new(),
            builtin_registry: Arc::new(BuiltinRegistry::new()),
        }
    }

    /// Create a new SemanticModel with pre-existing registries.
    ///
    /// Used by SemanticBinder to share registries with SemanticDatabase during binding.
    /// The registries are cloned (Arc-cloned) so both can access the same data.
    pub fn with_registries(
        root: Arc<dyn Symbol<KestrelLanguage>>,
        syntax_map: HashMap<SymbolId, SyntaxNode>,
        sources: HashMap<String, String>,
        registry: SymbolRegistry,
        extension_registry: ExtensionRegistry,
        builtin_registry: Arc<BuiltinRegistry>,
    ) -> Self {
        Self {
            root,
            syntax_map,
            sources,
            registry,
            extension_registry,
            builtin_registry,
        }
    }

    /// Decompose this model into its owned components.
    pub fn into_parts(
        self,
    ) -> (
        Arc<dyn Symbol<KestrelLanguage>>,
        HashMap<SymbolId, SyntaxNode>,
        HashMap<String, String>,
        SymbolRegistry,
        ExtensionRegistry,
        Arc<BuiltinRegistry>,
    ) {
        (
            self.root,
            self.syntax_map,
            self.sources,
            self.registry,
            self.extension_registry,
            self.builtin_registry,
        )
    }

    /// Execute a query against this model.
    pub fn query<Q: Query>(&self, query: Q) -> Q::Output {
        query.execute(self)
    }

    /// Get the root symbol.
    pub fn root(&self) -> &Arc<dyn Symbol<KestrelLanguage>> {
        &self.root
    }

    /// Get the syntax node for a symbol.
    pub fn syntax_for(&self, symbol_id: SymbolId) -> Option<&SyntaxNode> {
        self.syntax_map.get(&symbol_id)
    }

    /// Get source code by filename.
    pub fn source(&self, filename: &str) -> Option<&str> {
        self.sources.get(filename).map(|s| s.as_str())
    }

    /// Get the symbol registry.
    ///
    /// Exposed for queries and binding phase to access.
    pub fn registry(&self) -> &SymbolRegistry {
        &self.registry
    }

    /// Get the extension registry.
    ///
    /// Exposed for queries and binding phase to access.
    pub fn extension_registry(&self) -> &ExtensionRegistry {
        &self.extension_registry
    }

    /// Register an extension (called during binding).
    pub fn register_extension(&self, target_id: SymbolId, extension: Arc<ExtensionSymbol>) {
        self.extension_registry.register(target_id, extension);
    }

    /// Get the builtin registry.
    ///
    /// Exposed for queries and binding phase to access.
    pub fn builtin_registry(&self) -> &Arc<BuiltinRegistry> {
        &self.builtin_registry
    }

    /// Debug print the semantic model (symbol hierarchy).
    pub fn print_semantic_model(&self) {
        fn format_behavior(b: &dyn Behavior<KestrelLanguage>) -> String {
            use kestrel_semantic_tree::behavior::callable::CallableBehavior;
            use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
            use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
            use kestrel_semantic_tree::behavior::function_data::FunctionDataBehavior;
            use kestrel_semantic_tree::behavior::member_access::MemberAccessBehavior;
            use kestrel_semantic_tree::behavior::typed::TypedBehavior;
            use kestrel_semantic_tree::behavior::valued::ValueBehavior;
            use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
            use kestrel_semantic_tree::symbol::import::ImportDataBehavior;

            if let Some(vb) = b.downcast_ref::<VisibilityBehavior>() {
                if let Some(vis) = vb.visibility() {
                    return format!("Visibility({})", vis);
                }
                return "Visibility".to_string();
            }

            if let Some(tb) = b.downcast_ref::<TypedBehavior>() {
                return format!("Typed({})", tb.ty());
            }

            if let Some(import_data) = b.downcast_ref::<ImportDataBehavior>() {
                let path = import_data.module_path().join(".");
                let items = import_data.items();
                if items.is_empty() {
                    if let Some(alias) = import_data.alias() {
                        return format!("Import({} as {})", path, alias);
                    }
                    return format!("Import({})", path);
                }
                let item_strs: Vec<String> = items
                    .iter()
                    .map(|i| {
                        if let Some(alias) = &i.alias {
                            format!("{} as {}", i.name, alias)
                        } else {
                            i.name.clone()
                        }
                    })
                    .collect();
                return format!("Import({}.({}))", path, item_strs.join(", "));
            }

            if let Some(callable) = b.downcast_ref::<CallableBehavior>() {
                let params: Vec<String> = callable
                    .parameters()
                    .iter()
                    .map(|p| {
                        let label = p.external_label().unwrap_or("_");
                        format!("{}: {}", label, p.ty)
                    })
                    .collect();
                return format!("Callable(({}) -> {})", params.join(", "), callable.return_type());
            }

            if let Some(fd) = b.downcast_ref::<FunctionDataBehavior>() {
                return format!(
                    "FunctionData(has_body={}, is_static={})",
                    fd.has_body(),
                    fd.is_static()
                );
            }

            if let Some(vb) = b.downcast_ref::<ValueBehavior>() {
                return format!("Valued({})", vb.ty());
            }

            if let Some(cb) = b.downcast_ref::<ConformancesBehavior>() {
                let conformances: Vec<String> = cb.conformances().iter().map(|t| t.to_string()).collect();
                return format!("Conformances({})", conformances.join(", "));
            }

            if let Some(eb) = b.downcast_ref::<ExecutableBehavior>() {
                let stmt_count = eb.body().statements.len();
                let has_yield = eb.body().yield_expr().is_some();
                return format!("Executable(stmts={}, has_yield={})", stmt_count, has_yield);
            }

            if let Some(ma) = b.downcast_ref::<MemberAccessBehavior>() {
                return format!("MemberAccess({})", ma.member_name());
            }

            "Unknown".to_string()
        }

        fn print_symbol(symbol: &Arc<dyn Symbol<KestrelLanguage>>, level: usize) {
            let indent = "  ".repeat(level);
            let metadata = symbol.metadata();

            let behaviors = metadata.behaviors();
            let behaviors_str = if !behaviors.is_empty() {
                let behavior_strings: Vec<String> = behaviors
                    .iter()
                    .map(|b| format_behavior(b.as_ref()))
                    .collect();
                format!(" [{}]", behavior_strings.join(", "))
            } else {
                String::new()
            };

            println!(
                "{}{:?} '{}'{}",
                indent,
                metadata.kind(),
                metadata.name().value,
                behaviors_str
            );

            for child in metadata.children() {
                print_symbol(&child, level + 1);
            }
        }

        let root = self.root();
        let children = root.metadata().children();

        println!("{} top-level symbols\n", children.len());
        for child in children {
            print_symbol(&child, 0);
        }
    }

    /// Debug print a sorted symbol table view of the model.
    pub fn print_model_symbols(&self) {
        fn collect_symbols(
            symbol: &Arc<dyn Symbol<KestrelLanguage>>,
            symbols: &mut Vec<(String, String)>,
        ) {
            let name = symbol.metadata().name().value.clone();
            let kind = format!("{:?}", symbol.metadata().kind());
            symbols.push((name, kind));

            for child in symbol.metadata().children() {
                collect_symbols(&child, symbols);
            }
        }

        let mut symbols = Vec::new();
        for child in self.root().metadata().children() {
            collect_symbols(&child, &mut symbols);
        }

        println!("Symbols:");
        println!("  {} symbols\n", symbols.len());

        symbols.sort_by(|a, b| a.0.cmp(&b.0));

        println!("  {:<30} {:<15}", "Name", "Kind");
        println!("  {}", "-".repeat(45));

        for (name, kind) in symbols {
            println!("  {:<30} {:<15}", name, kind);
        }
    }
}
