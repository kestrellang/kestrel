//! GenericsDataFor query - get generics (type params + where clause) for a symbol

use std::sync::Arc;

use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::WhereClause;
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

#[derive(Debug, Clone)]
pub struct GenericsData {
    pub type_params: Vec<Arc<TypeParameterSymbol>>,
    pub where_clause: WhereClause,
}

/// Get generic type parameters and where clause for a symbol that supports generics.
pub struct GenericsDataFor {
    pub symbol_id: SymbolId,
}

impl Query for GenericsDataFor {
    type Output = Option<GenericsData>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let symbol = model.query(SymbolFor { id: self.symbol_id })?;

        match symbol.metadata().kind() {
            KestrelSymbolKind::Struct => {
                symbol
                    .downcast_arc::<StructSymbol>()
                    .ok()
                    .map(|s| GenericsData {
                        type_params: s.type_parameters(),
                        where_clause: s.where_clause(),
                    })
            }
            KestrelSymbolKind::Function => {
                symbol
                    .downcast_arc::<FunctionSymbol>()
                    .ok()
                    .map(|s| GenericsData {
                        type_params: s.type_parameters(),
                        where_clause: s.where_clause(),
                    })
            }
            KestrelSymbolKind::Protocol => {
                symbol
                    .downcast_arc::<ProtocolSymbol>()
                    .ok()
                    .map(|s| GenericsData {
                        type_params: s.type_parameters(),
                        where_clause: s.where_clause(),
                    })
            }
            KestrelSymbolKind::TypeAlias => {
                symbol
                    .downcast_arc::<TypeAliasSymbol>()
                    .ok()
                    .map(|s| GenericsData {
                        type_params: s.type_parameters(),
                        where_clause: s.where_clause(),
                    })
            }
            _ => None,
        }
    }
}
