//! Analyzer for import declarations
//!
//! Validates that:
//! - Module paths resolve to actual modules
//! - Imported items exist in the target module
//! - Imported items are visible from the importing scope
//! - No duplicate imports create name conflicts

use std::collections::HashMap;
use std::sync::Arc;

use kestrel_semantic_model::{ChildByName, IsVisibleFrom, ResolveModulePath, VisibleChildren};
use kestrel_semantic_tree::behavior::NamespaceScopeMarker;
use kestrel_semantic_tree::error::{
    ImportConflictError, SymbolNotFoundInModuleError, SymbolNotVisibleError,
};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::import::ImportDataBehavior;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

#[derive(Default)]
pub struct ImportAnalyzer;

impl ImportAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ImportAnalyzer {
    fn name(&self) -> &'static str {
        "imports"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        if symbol
            .metadata()
            .get_behavior::<NamespaceScopeMarker>()
            .is_none()
        {
            return;
        }

        let scope_id = symbol.metadata().id();

        let import_symbols: Vec<_> = symbol
            .metadata()
            .children()
            .into_iter()
            .filter(|child| matches!(child.metadata().kind(), KestrelSymbolKind::Import))
            .collect();

        for import_symbol in &import_symbols {
            validate_import(import_symbol, scope_id, ctx);
        }

        check_import_conflicts(&import_symbols, symbol, ctx);
    }
}

fn validate_import(
    import_symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    _scope_id: semantic_tree::symbol::SymbolId,
    ctx: &mut AnalysisContext,
) {
    let Some(import_data) = import_symbol
        .metadata()
        .get_behavior::<ImportDataBehavior>()
    else {
        return;
    };

    let import_id = import_symbol.metadata().id();

    // 1. Validate module path resolution
    let result = ctx.model.query(ResolveModulePath {
        path: import_data.module_path().to_vec(),
        context: import_id,
    });
    let Ok(module_id) = result else {
        let mut err = result.err().unwrap();
        // Fix spans in the error using import data
        let segments = import_data.module_path_segments();
        err.path_span = import_data.module_path_span().clone();
        if err.failed_segment_index < segments.len() {
            err.failed_segment_span = segments[err.failed_segment_index].1.clone();
        }
        ctx.report(err);
        return;
    };

    // 2. Validate import items if present: import A.B.C.(D, E)
    if !import_data.items().is_empty() {
        for item in import_data.items() {
            let Some(target_symbol) = ctx.model.query(ChildByName {
                parent: module_id,
                name: item.name.clone(),
            }) else {
                ctx.report(SymbolNotFoundInModuleError {
                    symbol_name: item.name.clone(),
                    module_path: import_data.module_path().to_vec(),
                    symbol_span: item.span.clone(),
                    module_span: import_symbol.metadata().span(),
                });
                continue;
            };

            let target_id = target_symbol.metadata().id();
            if ctx.model.query(IsVisibleFrom {
                target: target_id,
                context: import_id,
            }) {
                continue;
            }

            let (visibility_str, decl_span) = get_visibility_info(&target_symbol);
            ctx.report(SymbolNotVisibleError {
                symbol_name: item.name.clone(),
                visibility: visibility_str,
                import_span: item.span.clone(),
                declaration_span: Some(decl_span),
            });
        }
    }
}

fn check_import_conflicts(
    import_symbols: &[Arc<dyn Symbol<KestrelLanguage>>],
    scope: &Arc<dyn Symbol<KestrelLanguage>>,
    ctx: &mut AnalysisContext,
) {
    let mut name_sources: HashMap<String, Vec<NameSource>> = HashMap::new();

    // Collect specific imports and local declarations
    for child in scope.metadata().children() {
        match child.metadata().kind() {
            KestrelSymbolKind::Import => {
                let Some(import_data) = child.metadata().get_behavior::<ImportDataBehavior>()
                else {
                    continue;
                };
                for item in import_data.items() {
                    let name = item.alias.clone().unwrap_or_else(|| item.name.clone());
                    if let Some(existing_sources) = name_sources.get(&name)
                        && let Some(first) = existing_sources.first()
                    {
                        ctx.report(ImportConflictError {
                            name: name.clone(),
                            import_span: item.span.clone(),
                            existing_span: first.span.clone(),
                            existing_is_import: first.is_import,
                        });
                    }
                    name_sources.entry(name).or_default().push(NameSource {
                        span: item.span.clone(),
                        is_import: true,
                    });
                }
            },
            KestrelSymbolKind::Struct
            | KestrelSymbolKind::Protocol
            | KestrelSymbolKind::TypeAlias
            | KestrelSymbolKind::Function => {
                let name = child.metadata().name().value.clone();
                let name_span = child.metadata().name().span.clone();
                name_sources.entry(name).or_default().push(NameSource {
                    span: name_span,
                    is_import: false,
                });
            },
            _ => {},
        }
    }

    // Check whole-module imports (without alias) against existing names in scope
    for import_symbol in import_symbols {
        let Some(import_data) = import_symbol
            .metadata()
            .get_behavior::<ImportDataBehavior>()
        else {
            continue;
        };

        if !import_data.items().is_empty() || import_data.alias().is_some() {
            continue;
        }

        let import_id = import_symbol.metadata().id();
        let result = ctx.model.query(ResolveModulePath {
            path: import_data.module_path().to_vec(),
            context: import_id,
        });
        let Ok(module_id) = result else { continue };

        for child in ctx.model.query(VisibleChildren {
            parent: module_id,
            context: import_id,
        }) {
            let name = child.metadata().name().value.clone();
            if let Some(sources) = name_sources.get(&name) {
                for source in sources {
                    ctx.report(ImportConflictError {
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

#[derive(Debug, Clone)]
struct NameSource {
    span: kestrel_span::Span,
    is_import: bool,
}

fn get_visibility_info(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> (String, kestrel_span::Span) {
    use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
    match symbol.metadata().get_behavior::<VisibilityBehavior>() {
        Some(vb) => {
            let vis_str = match vb.visibility() {
                Some(v) => v.to_string(),
                None => "internal".to_string(),
            };
            (vis_str, symbol.metadata().span())
        },
        None => ("internal".to_string(), symbol.metadata().span()),
    }
}
