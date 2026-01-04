use std::sync::Arc;

use kestrel_semantic_model::{ResolveModulePath, SymbolFor};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::import::ImportDataBehavior;
use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::symbol::Symbol;

use crate::declaration_binder::{BindingContext, DeclarationBinder};

/// Binder for import declarations
pub struct ImportBinder;

impl DeclarationBinder for ImportBinder {
    fn bind_signature(
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
