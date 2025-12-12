use std::sync::Arc;

use kestrel_parser::import::ImportDeclaration;
use kestrel_semantic_model::{ResolveModulePath, SymbolFor};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::import::{ImportDataBehavior, ImportItem, ImportSymbol};
use kestrel_span::{Span, Spanned};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::resolver::{BindingContext, Resolver};
use kestrel_syntax_tree::utils::get_node_span;

/// Resolver for import declarations
pub struct ImportResolver;

impl Resolver for ImportResolver {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        _root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        let parent = parent?;

        // Wrap syntax node in ImportDeclaration helper
        let import_decl = ImportDeclaration {
            syntax: syntax.clone(),
            span: get_node_span(syntax, source),
        };

        // Extract module path with spans
        let module_path_node = import_decl.path();
        let module_path_segments = module_path_node.segments_with_spans();
        let module_path_span = module_path_node.span();

        // Extract alias
        let alias = import_decl.alias();

        // Extract import items
        let items = extract_import_items(&import_decl, source);

        // Create import symbol name
        let import_name = if let Some(ref alias) = alias {
            alias.clone()
        } else {
            module_path_segments
                .iter()
                .map(|(s, _)| s.as_str())
                .collect::<Vec<_>>()
                .join(".")
        };

        // NOTE: Span may be incorrect due to rowan position calculation issue
        // when lexer skips whitespace/comments. See utils::get_node_span for details.
        let span = get_node_span(syntax, source);
        let name = Spanned::new(import_name, span.clone());

        // Create import symbol
        let import_symbol = ImportSymbol::new(name, parent.clone(), span);
        let import_arc: Arc<dyn Symbol<KestrelLanguage>> = Arc::new(import_symbol);

        // Store import data in behavior for bind phase
        let import_data =
            ImportDataBehavior::new(module_path_segments, module_path_span, alias, items);
        import_arc.metadata().add_behavior(import_data);

        // Add to parent
        parent.metadata().add_child(&import_arc);

        Some(import_arc)
    }

    fn bind_declaration(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        _syntax: &SyntaxNode,
        ctx: &mut BindingContext,
    ) {
        // Get import data from behavior
        let import_data = match symbol.metadata().get_behavior::<ImportDataBehavior>() {
            Some(data) => data,
            None => {
                eprintln!("Warning: ImportSymbol missing ImportDataBehavior");
                return;
            }
        };

        let import_id = symbol.metadata().id();

        // Resolve module path using query (validation happens in ImportValidationPass)
        let module_id = match ctx.model.query(ResolveModulePath {
            path: import_data.module_path().to_vec(),
            context: import_id,
        }) {
            Ok(id) => id,
            Err(_) => {
                // Error will be reported by ImportValidationPass
                return;
            }
        };

        // Get the module symbol to resolve import items
        let module_symbol = match ctx.model.query(SymbolFor { id: module_id }) {
            Some(s) => s,
            None => return,
        };

        // Resolve and record target_ids for import items
        // Validation happens in ImportValidationPass
        if !import_data.items().is_empty() {
            // import A.B.C.(D, E)
            for item in import_data.items() {
                // Find the symbol in the module's visible children
                let target = module_symbol
                    .metadata()
                    .visible_children()
                    .into_iter()
                    .find(|child| child.metadata().name().value == item.name);

                if let Some(target_symbol) = target {
                    let target_id = target_symbol.metadata().id();
                    // Record the resolved target (validation will check visibility)
                    import_data.set_target_id(&item.name, target_id);
                }
                // Error reporting happens in ImportValidationPass
            }
        }
        // Note: Whole-module import conflicts are validated in ImportValidationPass
    }

    fn is_terminal(&self) -> bool {
        true // Don't walk children of import declarations
    }
}

/// Extract import items from import declaration
fn extract_import_items(import_decl: &ImportDeclaration, _source: &str) -> Vec<ImportItem> {
    import_decl
        .items()
        .into_iter()
        .filter_map(|item_node| {
            // Get the name (first identifier) and its span
            let (name, span) = item_node.children_with_tokens().find_map(|elem| {
                elem.as_token()
                    .filter(|t| t.kind() == SyntaxKind::Identifier)
                    .map(|t| {
                        let range = t.text_range();
                        let span: Span = Span::from(range.start().into()..range.end().into());
                        (t.text().to_string(), span)
                    })
            })?;

            // Check for alias (identifier after "as" keyword)
            let mut found_as = false;
            let alias = item_node.children_with_tokens().find_map(|elem| {
                if let Some(token) = elem.as_token() {
                    if found_as && token.kind() == SyntaxKind::Identifier {
                        return Some(token.text().to_string());
                    }
                    if token.kind() == SyntaxKind::As {
                        found_as = true;
                    }
                }
                None
            });

            Some(ImportItem {
                name,
                alias,
                span,
                target_id: None, // Filled during bind phase
            })
        })
        .collect()
}
