//! ExtensionBoundsForParam query - protocol bounds on a type parameter from extension where clauses

use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Extra protocol bounds on a type parameter from extension where clauses in context.
///
/// Walks the parent chain from context looking for an Extension declaration,
/// then extracts where clause bounds for the given type parameter.
pub struct ExtensionBoundsForParam {
    pub context_id: SymbolId,
    pub param_id: SymbolId,
}

impl Query for ExtensionBoundsForParam {
    type Output = Option<Vec<Ty>>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let context = model.query(SymbolFor { id: self.context_id })?;
        let mut current: Option<std::sync::Arc<dyn semantic_tree::symbol::Symbol<kestrel_semantic_tree::language::KestrelLanguage>>> = Some(context);

        while let Some(sym) = current {
            if sym.metadata().kind() == KestrelSymbolKind::Extension {
                // Found extension - check its where clause for bounds on param_id
                if let Some(ext_target) =
                    sym.metadata().get_behavior::<ExtensionTargetBehavior>()
                {
                    let where_clause = ext_target.where_clause();
                    let bounds: Vec<Ty> = where_clause
                        .bounds_for(self.param_id)
                        .into_iter()
                        .filter(|b| matches!(b.kind(), TyKind::Protocol { .. } | TyKind::Error))
                        .cloned()
                        .collect();
                    if !bounds.is_empty() {
                        return Some(bounds);
                    }
                }
            }
            current = sym.metadata().parent();
        }
        None
    }
}
