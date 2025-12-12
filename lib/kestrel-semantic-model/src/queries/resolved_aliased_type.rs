//! ResolvedAliasedType query - get the resolved type behind a type alias (if bound)

use std::sync::Arc;

use kestrel_semantic_tree::symbol::type_alias::TypeAliasSymbol;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasTypedBehavior;
use kestrel_semantic_tree::ty::Ty;
use semantic_tree::symbol::Symbol;
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Get the resolved type that a type alias refers to.
///
/// Returns `None` if the symbol is not a type alias, or if the type alias
/// has not been bound (i.e., missing `TypeAliasTypedBehavior`).
pub struct ResolvedAliasedType {
    pub type_alias_id: SymbolId,
}

impl Query for ResolvedAliasedType {
    type Output = Option<Ty>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let symbol = model.query(SymbolFor {
            id: self.type_alias_id,
        })?;
        let type_alias: Arc<TypeAliasSymbol> = symbol.downcast_arc().ok()?;
        type_alias
            .metadata()
            .get_behavior::<TypeAliasTypedBehavior>()
            .map(|typed| typed.resolved_ty().clone())
    }
}
