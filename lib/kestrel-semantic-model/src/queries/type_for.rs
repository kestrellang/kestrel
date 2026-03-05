//! TypeFor query — resolve the declared type of a symbol.
//!
//! Fast path: reads TypedBehavior if already attached by the binder.
//! Slow path: computes from syntax via the pure type resolver.
//!
//! This query breaks ordering dependencies between binders — a getter can
//! query its parent field's type even if the field binder hasn't run yet.

use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_syntax_tree::SyntaxKind;
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;
use crate::type_resolution::resolve_type_from_syntax_node;

/// Resolve the declared type of a symbol.
///
/// Tries the fast path (reading TypedBehavior) first.
/// Falls back to computing from the symbol's syntax node.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TypeFor {
    pub symbol_id: SymbolId,
}

impl Query for TypeFor {
    type Output = Option<kestrel_semantic_tree::ty::Ty>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let symbol = model.query(SymbolFor { id: self.symbol_id })?;

        // Fast path: behavior already attached by binder
        if let Some(tb) = symbol.metadata().get_behavior::<TypedBehavior>() {
            return Some(tb.ty().clone());
        }

        // Slow path: compute from syntax
        let syntax = model.syntax_for(self.symbol_id)?;
        let file_id = symbol.metadata().span().file_id;

        // Find the Ty child node in the symbol's syntax
        let ty_node = syntax.children().find(|c| c.kind() == SyntaxKind::Ty)?;
        Some(resolve_type_from_syntax_node(
            model,
            &ty_node,
            self.symbol_id,
            file_id,
        ))
    }
}
