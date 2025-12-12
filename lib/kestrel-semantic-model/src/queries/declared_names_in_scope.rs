//! DeclaredNamesInScope query - collect declared child names for a scope

use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_span::Span;
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

#[derive(Debug, Clone)]
pub struct DeclaredName {
    pub name: String,
    pub kind: KestrelSymbolKind,
    pub declaration_span: Span,
}

/// Collect declared child names (direct children only) for a scope symbol.
pub struct DeclaredNamesInScope {
    pub scope_id: SymbolId,
}

impl Query for DeclaredNamesInScope {
    type Output = Vec<DeclaredName>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let scope = match model.query(SymbolFor { id: self.scope_id }) {
            Some(s) => s,
            None => return Vec::new(),
        };

        scope
            .metadata()
            .children()
            .into_iter()
            .map(|child| DeclaredName {
                name: child.metadata().name().value.clone(),
                kind: child.metadata().kind(),
                declaration_span: child.metadata().declaration_span().clone(),
            })
            .collect()
    }
}
