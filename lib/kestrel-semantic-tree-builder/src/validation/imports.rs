//! Validator for import declarations
//!
//! This validator validates that:
//! - Module paths resolve to actual modules
//! - Imported items exist in the target module
//! - Imported items are visible from the importing scope
//! - No duplicate imports create name conflicts
//!
//! Note: The target_id resolution happens in ImportResolver.bind_declaration(),
//! but all validation (error checking) happens here to maintain separation of concerns.

use std::collections::HashMap;
use std::sync::Arc;

use kestrel_semantic_tree::error::*;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::import::ImportDataBehavior;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::Symbol;

use crate::database::Db;
use crate::syntax::get_file_id_for_symbol;
use crate::validation::{SymbolContext, Validator};

/// Validator for import declarations
pub struct ImportValidator;

impl ImportValidator {
    const NAME: &'static str = "imports";

    pub fn new() -> Self {
        Self
    }
}

impl Default for ImportValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for ImportValidator {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn validate_symbol(&self, ctx: &SymbolContext<'_>) {
        let kind = ctx.symbol.metadata().kind();

        // Check if this is a scope that contains imports (Module or SourceFile)
        if matches!(
            kind,
            KestrelSymbolKind::Module | KestrelSymbolKind::SourceFile
        ) {
            validate_imports_in_scope(ctx);
        }
    }
}

/// Validate all imports in a given scope
fn validate_imports_in_scope(ctx: &SymbolContext<'_>) {
    let scope_id = ctx.symbol.metadata().id();

    // Get all import symbols in this scope
    let import_symbols: Vec<_> = ctx
        .symbol
        .metadata()
        .children()
        .into_iter()
        .filter(|child| matches!(child.metadata().kind(), KestrelSymbolKind::Import))
        .collect();

    // Validate each import
    for import_symbol in &import_symbols {
        validate_import(import_symbol, scope_id, ctx);
    }

    // Check for import conflicts (whole-module imports)
    check_import_conflicts(&import_symbols, ctx);
}

/// Validate a single import symbol
fn validate_import(
    import_symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    _scope_id: semantic_tree::symbol::SymbolId,
    ctx: &SymbolContext<'_>,
) {
    // Get the import data from behavior
    let import_data = match import_symbol.metadata().get_behavior::<ImportDataBehavior>() {
        Some(data) => data,
        None => {
            // Missing import data - this shouldn't happen, but don't crash
            return;
        }
    };

    let import_id = import_symbol.metadata().id();
    let file_id = get_file_id_for_symbol(import_symbol, &mut *ctx.diagnostics().get());

    // 1. Validate module path resolution
    let module_id = match ctx.db.resolve_module_path(
        import_data.module_path().to_vec(),
        import_id,
    ) {
        Ok(id) => id,
        Err(mut err) => {
            // Fix up the spans in the error using the import data
            let segments = import_data.module_path_segments();
            err.path_span = import_data.module_path_span().clone();
            if err.failed_segment_index < segments.len() {
                err.failed_segment_span = segments[err.failed_segment_index].1.clone();
            }
            ctx.diagnostics().get().throw(err);
            return;
        }
    };

    // Verify the module exists
    if ctx.db.symbol_by_id(module_id).is_none() {
        return;
    }

    // 2. Validate import items if present
    if !import_data.items().is_empty() {
        // import A.B.C.(D, E)
        for item in import_data.items() {
            // Find the symbol in the module's visible children using query
            let target = ctx.db.find_child_by_name(module_id, &item.name);

            match target {
                Some(target_symbol) => {
                    let target_id = target_symbol.metadata().id();

                    // Check visibility using query
                    if !ctx.db.is_visible_from(target_id, import_id) {
                        // Get the actual visibility from the target symbol
                        let (visibility_str, _decl_span) = get_visibility_info(&target_symbol);

                        // Point to the target's name identifier, not the whole declaration
                        let declaration_span = Some(target_symbol.metadata().name().span.clone());

                        ctx.diagnostics().get().throw(
                            SymbolNotVisibleError {
                                symbol_name: item.name.clone(),
                                visibility: visibility_str,
                                import_span: item.span.clone(), // Point to the specific item
                                declaration_span,
                            });
                    }
                }
                None => {
                    ctx.diagnostics().get().throw(
                        SymbolNotFoundInModuleError {
                            symbol_name: item.name.clone(),
                            module_path: import_data.module_path().to_vec(),
                            symbol_span: item.span.clone(), // Point to the specific item
                            module_span: import_symbol.metadata().span(),
                        });
                }
            }
        }
    }
    // Note: Whole-module import conflicts are checked in check_import_conflicts()
}

/// Check for conflicts when doing whole-module imports
fn check_import_conflicts(
    import_symbols: &[Arc<dyn Symbol<KestrelLanguage>>],
    ctx: &SymbolContext<'_>,
) {
    // Build a map of all names that are imported or declared
    let mut name_sources: HashMap<String, Vec<NameSource>> = HashMap::new();

    // First, collect all specific imports and local declarations
    for child in ctx.symbol.metadata().children() {
        let file_id = get_file_id_for_symbol(&child, &mut *ctx.diagnostics().get());

        match child.metadata().kind() {
            KestrelSymbolKind::Import => {
                if let Some(import_data) = child.metadata().get_behavior::<ImportDataBehavior>() {
                    // For specific imports, add each item
                    for item in import_data.items() {
                        let name = item.alias.clone().unwrap_or_else(|| item.name.clone());

                        // Check for duplicates
                        if let Some(existing_sources) = name_sources.get(&name) {
                            // Report error for duplicate import
                            if let Some(first) = existing_sources.first() {
                                ctx.diagnostics().get().throw(
                                    ImportConflictError {
                                        name: name.clone(),
                                        import_span: item.span.clone(),
                                        existing_span: first.span.clone(),
                                        existing_is_import: first.is_import,
                                    });
                            }
                        }

                        name_sources
                            .entry(name)
                            .or_insert_with(Vec::new)
                            .push(NameSource {
                                span: item.span.clone(),
                                file_id,
                                is_import: true,
                            });
                    }
                }
            }
            KestrelSymbolKind::Struct
            | KestrelSymbolKind::Protocol
            | KestrelSymbolKind::TypeAlias
            | KestrelSymbolKind::Function => {
                // Local declaration
                let name = child.metadata().name().value.clone();
                let name_span = child.metadata().name().span.clone();
                name_sources
                    .entry(name)
                    .or_insert_with(Vec::new)
                    .push(NameSource {
                        span: name_span,
                        file_id,
                        is_import: false,
                    });
            }
            _ => {}
        }
    }

    // Now check whole-module imports for conflicts
    for import_symbol in import_symbols {
        let import_data = match import_symbol.metadata().get_behavior::<ImportDataBehavior>() {
            Some(data) => data,
            None => continue,
        };

        // Only check whole-module imports without alias
        if !import_data.items().is_empty() || import_data.alias().is_some() {
            continue;
        }

        let import_id = import_symbol.metadata().id();
        let import_file_id =
            get_file_id_for_symbol(import_symbol, &mut *ctx.diagnostics().get());

        // Resolve the module
        let module_id = match ctx.db.resolve_module_path(
            import_data.module_path().to_vec(),
            import_id,
        ) {
            Ok(id) => id,
            Err(_) => continue, // Already reported in validate_import
        };

        // Check each visible symbol from the module using query
        for child in ctx.db.visible_children_from(module_id, import_id) {
            let name = child.metadata().name().value.clone();

            // Check if this name conflicts with existing names
            if let Some(sources) = name_sources.get(&name) {
                for source in sources {
                    // Report conflict
                    ctx.diagnostics().get().throw(
                        ImportConflictError {
                            name: name.clone(),
                            import_span: import_symbol.metadata().span(),
                            existing_span: source.span.clone(),
                            existing_is_import: source.is_import,
                        });
                }
            }
        }
    }
}

/// Source of a name in the scope (for conflict detection)
#[derive(Debug, Clone)]
struct NameSource {
    span: kestrel_span::Span,
    #[allow(dead_code)] // Kept for future cross-file diagnostics
    file_id: usize,
    is_import: bool,
}

/// Get visibility information from a symbol for error reporting
fn get_visibility_info(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> (String, Option<kestrel_span::Span>) {
    use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;

    match symbol.visibility_behavior() {
        Some(vb) => {
            let vis_str = match vb.visibility() {
                Some(v) => v.to_string(),
                None => "internal".to_string(), // default
            };
            // Use the symbol's span as declaration location
            (vis_str, Some(symbol.metadata().span()))
        }
        None => ("internal".to_string(), Some(symbol.metadata().span())),
    }
}
