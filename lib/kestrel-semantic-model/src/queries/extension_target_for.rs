//! ExtensionTargetFor query — resolve the target type of an extension.
//!
//! Fast path: reads ExtensionTargetBehavior if already attached by the binder.
//! Slow path: computes from syntax via the pure type resolver.
//!
//! This query breaks ordering dependencies — a function/getter/setter can
//! query its parent extension's target type even if the extension binder
//! hasn't run yet.

use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_syntax_tree::SyntaxKind;
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;
use crate::type_resolution::resolve_type_from_syntax_node;

/// Resolve the target type of an extension symbol.
///
/// Tries the fast path (reading ExtensionTargetBehavior) first.
/// Falls back to computing from the extension's syntax node.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ExtensionTargetFor {
    pub symbol_id: SymbolId,
}

impl Query for ExtensionTargetFor {
    type Output = Option<kestrel_semantic_tree::ty::Ty>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let symbol = model.query(SymbolFor { id: self.symbol_id })?;

        // Fast path: behavior already attached by binder
        if let Some(etb) = symbol.metadata().get_behavior::<ExtensionTargetBehavior>() {
            return Some(etb.target_type().clone());
        }

        // Slow path: compute from syntax
        let syntax = model.syntax_for(self.symbol_id)?;
        let file_id = symbol.metadata().span().file_id;

        // Find the Ty child node in the extension's syntax
        let ty_node = syntax.children().find(|c| c.kind() == SyntaxKind::Ty)?;
        Some(resolve_type_from_syntax_node(
            model,
            &ty_node,
            self.symbol_id,
            file_id,
        ))
    }
}
