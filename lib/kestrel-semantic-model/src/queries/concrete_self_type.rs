//! ConcreteSelfType query - resolve what Self refers to in a context

use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Resolve the concrete type that `Self` refers to in a given context.
///
/// Walks the parent chain looking for Struct/Enum/Extension declarations.
/// Returns `None` for Protocol contexts (where Self is symbolic).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ConcreteSelfType {
    pub context_id: SymbolId,
}

impl Query for ConcreteSelfType {
    type Output = Option<Ty>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let mut current = Some(self.context_id);

        while let Some(id) = current {
            let symbol = model.query(SymbolFor { id })?;

            if symbol.metadata().kind() == KestrelSymbolKind::Extension
                && let Some(target_beh) =
                    symbol.metadata().get_behavior::<ExtensionTargetBehavior>()
            {
                let target_ty = target_beh.target_type();
                if !matches!(target_ty.kind(), TyKind::Protocol { .. } | TyKind::SelfType) {
                    return Some(target_ty.clone());
                }
                return None;
            }

            if matches!(
                symbol.metadata().kind(),
                KestrelSymbolKind::Struct | KestrelSymbolKind::Enum
            ) && let Some(typed) = symbol.metadata().get_behavior::<TypedBehavior>()
            {
                let ty = typed.ty();
                if !matches!(ty.kind(), TyKind::SelfType) {
                    return Some(ty.clone());
                }
            }

            if symbol.metadata().kind() == KestrelSymbolKind::Protocol {
                return None;
            }

            current = symbol.metadata().parent().map(|p| p.metadata().id());
        }

        None
    }
}
