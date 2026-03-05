//! Shared utility for constructing the `Self` type for a symbol's parent type container.

use std::sync::Arc;

use kestrel_semantic_model::ExtensionTargetFor;
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::ty::{Substitutions, Ty, TyKind};
use semantic_tree::symbol::Symbol;

/// Construct the self type for a given type container (struct, enum, protocol, or extension).
///
/// - Struct/Enum: produces a generic type with identity substitutions (each type param maps to itself).
/// - Protocol: produces `Self` (abstract).
/// - Extension: queries the target type; protocol extensions produce `Self`.
/// - Other kinds: returns `None`.
pub fn self_type_for_parent(
    parent: &Arc<dyn Symbol<KestrelLanguage>>,
    model: &kestrel_semantic_model::SemanticModel,
) -> Option<Ty> {
    let span = parent.metadata().span().clone();

    match parent.metadata().kind() {
        KestrelSymbolKind::Struct => {
            let struct_arc = Arc::clone(parent).downcast_arc::<StructSymbol>().ok()?;
            let substitutions = identity_substitutions(parent, &span);
            Some(Ty::generic_struct(struct_arc, substitutions, span))
        },
        KestrelSymbolKind::Enum => {
            let enum_arc = Arc::clone(parent).downcast_arc::<EnumSymbol>().ok()?;
            let substitutions = identity_substitutions(parent, &span);
            Some(Ty::generic_enum(enum_arc, substitutions, span))
        },
        KestrelSymbolKind::Protocol => Some(Ty::self_type(span)),
        KestrelSymbolKind::Extension => {
            let target = model.query(ExtensionTargetFor {
                symbol_id: parent.metadata().id(),
            })?;
            if matches!(target.kind(), TyKind::Protocol { .. }) {
                Some(Ty::self_type(span))
            } else {
                Some(target)
            }
        },
        _ => None,
    }
}

/// Build identity substitutions: each type parameter maps to itself.
fn identity_substitutions(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    span: &kestrel_span::Span,
) -> Substitutions {
    let mut substitutions = Substitutions::new();
    if let Some(generics) = symbol.metadata().get_behavior::<GenericsBehavior>() {
        for param in generics.type_parameters() {
            let param_id = param.metadata().id();
            let param_ty = Ty::type_parameter(param.clone(), span.clone());
            substitutions.insert(param_id, param_ty);
        }
    }
    substitutions
}
