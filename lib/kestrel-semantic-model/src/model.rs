//! Semantic model for Kestrel compiler
//!
//! The SemanticModel is the primary interface for querying semantic information
//! about a compiled Kestrel program. It owns the symbol tree, syntax mappings,
//! source code, and registries.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use kestrel_semantic_tree::builtins::{BuiltinRegistry, LanguageFeature};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::ty::{Substitutions, Ty};
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::behavior::Behavior;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::extension_registry::ExtensionRegistry;
use crate::query::Query;
use crate::registry::SymbolRegistry;

/// Type-erased per-query-type cache.
///
/// Each query type `Q` gets a `HashMap<Q, Q::Output>` behind the type erasure.
/// Uses `RefCell` for interior mutability since queries may recurse (a query's
/// `execute` may call `model.query()` for other queries).
struct QueryCache {
    stores: RefCell<HashMap<TypeId, Box<dyn Any>>>,
}

impl QueryCache {
    fn new() -> Self {
        Self {
            stores: RefCell::new(HashMap::new()),
        }
    }
}

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
    /// Memoization cache for query results
    cache: QueryCache,
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
            cache: QueryCache::new(),
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
            cache: QueryCache::new(),
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
    ///
    /// Results are memoized: repeated calls with the same query return cached results.
    /// Uses a three-phase borrow pattern for re-entrancy safety (queries may call
    /// other queries during execution).
    pub fn query<Q: Query>(&self, query: Q) -> Q::Output {
        // Phase 1: check cache (borrow, then release)
        {
            let stores = self.cache.stores.borrow();
            if let Some(store) = stores.get(&TypeId::of::<Q>()) {
                if let Some(result) = store
                    .downcast_ref::<HashMap<Q, Q::Output>>()
                    .and_then(|map| map.get(&query))
                {
                    return result.clone();
                }
            }
        }

        // Phase 2: execute (may recurse into query() — no borrow held)
        let key = query.clone();
        let result = query.execute(self);

        // Phase 3: store result (borrow, then release)
        {
            let mut stores = self.cache.stores.borrow_mut();
            stores
                .entry(TypeId::of::<Q>())
                .or_insert_with(|| Box::new(HashMap::<Q, Q::Output>::new()))
                .downcast_mut::<HashMap<Q, Q::Output>>()
                .unwrap()
                .insert(key, result.clone());
        }

        result
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
    ///
    /// Invalidates the query cache since extension registration changes
    /// the results of queries that depend on extensions (e.g. ExtensionsFor,
    /// AllConformancesFor, AllMethodsFor).
    pub fn register_extension(&self, target_id: SymbolId, extension: Arc<ExtensionSymbol>) {
        self.extension_registry.register(target_id, extension);
        self.cache.stores.borrow_mut().clear();
    }

    /// Invalidate the query cache.
    ///
    /// Call this after mutating any model state (e.g. registering symbols)
    /// that queries may depend on.
    pub fn invalidate_cache(&self) {
        self.cache.stores.borrow_mut().clear();
    }

    /// Get the builtin registry.
    ///
    /// Exposed for queries and binding phase to access.
    pub fn builtin_registry(&self) -> &Arc<BuiltinRegistry> {
        &self.builtin_registry
    }

    /// Create an Array[T] struct type given an element type.
    ///
    /// This is used to create array types for array literals instead of using TyKind::Array.
    /// Returns None if the Array struct builtin is not registered (stdlib not loaded).
    pub fn make_array_type(&self, element_ty: Ty, span: Span) -> Option<Ty> {
        // Look up the Array struct symbol
        let symbol_id = self
            .builtin_registry
            .builtin_struct(LanguageFeature::ArrayStruct)?;
        let symbol = self.registry.get(symbol_id)?;

        // Downcast to StructSymbol
        let struct_symbol: Arc<StructSymbol> = symbol.into_any_arc().downcast().ok()?;

        // Get the T type parameter
        let type_params = struct_symbol.type_parameters();
        let t_param = type_params.first()?;

        // Create substitutions: T -> element_ty
        let mut substitutions = Substitutions::new();
        substitutions.insert(t_param.metadata().id(), element_ty);

        Some(Ty::generic_struct(struct_symbol, substitutions, span))
    }

    /// Create a Slice[T] struct type given an element type.
    ///
    /// This is used to create slice types for array pattern rest bindings.
    /// Returns None if the Slice struct builtin is not registered (stdlib not loaded).
    pub fn make_slice_type(&self, element_ty: Ty, span: Span) -> Option<Ty> {
        // Look up the Slice struct symbol
        let symbol_id = self
            .builtin_registry
            .builtin_struct(LanguageFeature::SliceStruct)?;
        let symbol = self.registry.get(symbol_id)?;

        // Downcast to StructSymbol
        let struct_symbol: Arc<StructSymbol> = symbol.into_any_arc().downcast().ok()?;

        // Get the T type parameter
        let type_params = struct_symbol.type_parameters();
        let t_param = type_params.first()?;

        // Create substitutions: T -> element_ty
        let mut substitutions = Substitutions::new();
        substitutions.insert(t_param.metadata().id(), element_ty);

        Some(Ty::generic_struct(struct_symbol, substitutions, span))
    }

    /// Debug print the semantic model (symbol hierarchy).
    ///
    /// If `full` is true, shows complete details including function body statements.
    pub fn print_semantic_model(&self, full: bool) {
        use kestrel_semantic_tree::behavior::callable::CallableBehavior;
        use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
        use kestrel_semantic_tree::behavior::executable::{
            ExecutableBehavior, ResolvedExecutableBehavior,
        };
        use kestrel_semantic_tree::behavior::function_data::FunctionDataBehavior;
        use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
        use kestrel_semantic_tree::behavior::member_access::MemberAccessBehavior;
        use kestrel_semantic_tree::behavior::typed::TypedBehavior;
        use kestrel_semantic_tree::behavior::valued::ValueBehavior;
        use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
        use kestrel_semantic_tree::symbol::import::ImportDataBehavior;

        /// Format a behavior for inline display (in the [...] brackets)
        fn format_behavior_inline(b: &dyn Behavior<KestrelLanguage>) -> Option<String> {
            if let Some(vb) = b.downcast_ref::<VisibilityBehavior>() {
                if let Some(vis) = vb.visibility() {
                    return Some(format!("Visibility({})", vis));
                }
                return Some("Visibility".to_string());
            }

            if let Some(tb) = b.downcast_ref::<TypedBehavior>() {
                return Some(format!("Typed({})", tb.ty()));
            }

            if let Some(import_data) = b.downcast_ref::<ImportDataBehavior>() {
                let path = import_data.module_path().join(".");
                let items = import_data.items();
                if items.is_empty() {
                    if let Some(alias) = import_data.alias() {
                        return Some(format!("Import({} as {})", path, alias));
                    }
                    return Some(format!("Import({})", path));
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
                return Some(format!("Import({}.({}))", path, item_strs.join(", ")));
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
                return Some(format!(
                    "Callable(({}) -> {})",
                    params.join(", "),
                    callable.return_type()
                ));
            }

            if let Some(fd) = b.downcast_ref::<FunctionDataBehavior>() {
                return Some(format!("FunctionData(has_body={})", fd.has_body()));
            }

            if let Some(vb) = b.downcast_ref::<ValueBehavior>() {
                return Some(format!("Valued({})", vb.ty()));
            }

            if let Some(cb) = b.downcast_ref::<ConformancesBehavior>() {
                let conformances: Vec<String> =
                    cb.conformances().iter().map(|t| t.to_string()).collect();
                return Some(format!("Conformances({})", conformances.join(", ")));
            }

            if let Some(gb) = b.downcast_ref::<GenericsBehavior>() {
                let params: Vec<String> = gb
                    .type_parameters()
                    .iter()
                    .map(|tp| tp.metadata().name().value.clone())
                    .collect();
                if !params.is_empty() {
                    return Some(format!("Generics[{}]", params.join(", ")));
                }
                return None;
            }

            // For ExecutableBehavior, return summary in non-full mode (handled separately in full mode)
            if let Some(eb) = b.downcast_ref::<ExecutableBehavior>() {
                let stmt_count = eb.body().statements.len();
                let has_yield = eb.body().yield_expr().is_some();
                return Some(format!(
                    "Executable(stmts={}, has_yield={})",
                    stmt_count, has_yield
                ));
            }

            if let Some(eb) = b.downcast_ref::<ResolvedExecutableBehavior>() {
                let stmt_count = eb.body().statements.len();
                let has_yield = eb.body().yield_expr().is_some();
                return Some(format!(
                    "ResolvedExecutable(stmts={}, has_yield={})",
                    stmt_count, has_yield
                ));
            }

            if let Some(ma) = b.downcast_ref::<MemberAccessBehavior>() {
                return Some(format!("MemberAccess({})", ma.member_name()));
            }

            None
        }

        fn print_symbol(symbol: &Arc<dyn Symbol<KestrelLanguage>>, level: usize, full: bool) {
            let indent = "  ".repeat(level);
            let metadata = symbol.metadata();

            let behaviors = metadata.behaviors();

            // Collect all behaviors for inline display
            let inline_behaviors: Vec<String> = behaviors
                .iter()
                .filter_map(|b| format_behavior_inline(b.as_ref()))
                .collect();

            // In full mode, also extract the executable body for expanded display
            // Returns (body, is_resolved) tuple
            let executable_body: Option<(_, bool)> = if full {
                // Prefer ResolvedExecutable over Executable if both exist
                let resolved = behaviors.iter().find_map(|b| {
                    b.downcast_ref::<ResolvedExecutableBehavior>()
                        .map(|eb| (eb.body().clone(), true))
                });
                if resolved.is_some() {
                    resolved
                } else {
                    behaviors.iter().find_map(|b| {
                        b.downcast_ref::<ExecutableBehavior>()
                            .map(|eb| (eb.body().clone(), false))
                    })
                }
            } else {
                None
            };

            let behaviors_str = if !inline_behaviors.is_empty() {
                format!(" [{}]", inline_behaviors.join(", "))
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

            // In full mode, print the executable body as an indented block
            if let Some((body, is_resolved)) = executable_body {
                use kestrel_semantic_tree::expr::Expression;
                use kestrel_semantic_tree::expr::IfCondition;
                use kestrel_semantic_tree::pattern::{Mutability, PatternKind};
                use kestrel_semantic_tree::stmt::{Statement, StatementKind};

                /// Format an expression with types shown
                fn format_expr_with_type(expr: &Expression) -> String {
                    expr.debug_compact()
                }

                /// Format a statement with types shown
                fn format_stmt_with_type(stmt: &Statement) -> String {
                    match &stmt.kind {
                        StatementKind::Binding { pattern, value } => {
                            let keyword = match &pattern.kind {
                                PatternKind::Local { mutability, .. } => {
                                    if *mutability == Mutability::Mutable {
                                        "var"
                                    } else {
                                        "let"
                                    }
                                },
                                PatternKind::At { mutability, .. } => {
                                    if *mutability == Mutability::Mutable {
                                        "var"
                                    } else {
                                        "let"
                                    }
                                },
                                _ => "let",
                            };
                            let name = pattern.name().unwrap_or("<error>");
                            let ty = &pattern.ty;
                            let value_str = value
                                .as_ref()
                                .map(|v| format!(" = {}", format_expr_with_type(v)))
                                .unwrap_or_default();
                            format!("{} {}: {}{};", keyword, name, ty, value_str)
                        },
                        StatementKind::Expr(expr) => {
                            format!("{};", format_expr_with_type(expr))
                        },
                        StatementKind::GuardLet { conditions, .. } => {
                            let conds: Vec<_> = conditions
                                .iter()
                                .map(|c| match c {
                                    IfCondition::Let { pattern, value, .. } => {
                                        let name = pattern.name().unwrap_or("<pattern>");
                                        format!(
                                            "let {}: {} = {}",
                                            name,
                                            pattern.ty,
                                            format_expr_with_type(value)
                                        )
                                    },
                                    IfCondition::Expr(e) => format_expr_with_type(e),
                                })
                                .collect();
                            format!("guard {} else {{ ... }}", conds.join(", "))
                        },
                        StatementKind::Deinit { name, .. } => {
                            format!("deinit {};", name)
                        },
                    }
                }

                let body_indent = "  ".repeat(level + 1);
                let label = if is_resolved {
                    "ResolvedExecutable"
                } else {
                    "Executable"
                };
                println!("{}{} {{", body_indent, label);
                let stmt_indent = "  ".repeat(level + 2);
                for stmt in &body.statements {
                    println!("{}{}", stmt_indent, format_stmt_with_type(stmt));
                }
                if let Some(yield_expr) = body.yield_expr() {
                    println!("{}-> {}", stmt_indent, format_expr_with_type(yield_expr));
                }
                println!("{}}}", body_indent);
            }

            for child in metadata.children() {
                print_symbol(&child, level + 1, full);
            }
        }

        let root = self.root();
        let children = root.metadata().children();

        println!("{} top-level symbols\n", children.len());
        for child in children {
            print_symbol(&child, 0, full);
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
