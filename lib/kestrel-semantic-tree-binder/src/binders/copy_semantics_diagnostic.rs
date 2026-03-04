//! Shared diagnostic for cloneable-field-requires-Cloneable-conformance.
//!
//! Called from both StructBinder and EnumBinder after computing copy semantics.

use std::sync::Arc;

use kestrel_semantic_model::queries::collect_child_types;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::copy_semantics::CopySemantics;
use kestrel_semantic_tree::behavior::copy_semantics::CopySemanticsBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::TyKind;
use semantic_tree::symbol::Symbol;

use crate::declaration_binder::BindingContext;
use crate::diagnostics::CloneableFieldRequiresCloneableConformance;

/// Check if a struct/enum has cloneable children but doesn't conform to Cloneable.
/// Emits CloneableFieldRequiresCloneableConformance diagnostic if so.
pub(crate) fn check_cloneable_field_diagnostic(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    type_kind: &'static str,
    context: &mut BindingContext,
) {
    // Only check if copy semantics is NotCopyable (Rule 4 case)
    let Some(copy_behavior) = symbol.metadata().get_behavior::<CopySemanticsBehavior>() else {
        return;
    };
    if copy_behavior.semantics() != CopySemantics::NotCopyable {
        return;
    }

    // Check if the type already conforms to Cloneable — if so, no diagnostic needed
    let Some(cloneable_id) = context.model.builtin_registry().cloneable_protocol() else {
        return;
    };
    let conformances = symbol.metadata().get_behavior::<ConformancesBehavior>();
    let conforms_to_cloneable = conformances
        .as_ref()
        .map(|c| {
            c.conformances().iter().any(|ty| {
                if let TyKind::Protocol { symbol, .. } = ty.kind() {
                    symbol.metadata().id() == cloneable_id
                } else {
                    false
                }
            })
        })
        .unwrap_or(false);
    if conforms_to_cloneable {
        return;
    }

    // Also skip if the type has `not Copyable` — that's Rule 1, not Rule 4
    let Some(copyable_id) = context.model.builtin_registry().copyable_protocol() else {
        return;
    };
    let has_not_copyable = conformances
        .as_ref()
        .map(|c| c.has_negative_conformance_to(copyable_id))
        .unwrap_or(false);
    if has_not_copyable {
        return;
    }

    // Also skip if any child is non-copyable — that's Rule 2, not Rule 4
    // Uses the same collect_child_types as CopySemanticsFor to stay in sync
    let child_types = collect_child_types(symbol);
    if child_types.iter().any(|ty| !ty.is_copyable()) {
        return;
    }

    // Find first cloneable child for the diagnostic
    for child in symbol.metadata().children().iter() {
        match child.metadata().kind() {
            KestrelSymbolKind::Field => {
                if let Some(typed) = child.metadata().get_behavior::<TypedBehavior>()
                    && typed.ty().is_cloneable()
                {
                    context
                        .diagnostics
                        .throw(CloneableFieldRequiresCloneableConformance {
                            type_span: symbol.metadata().span().clone(),
                            type_name: symbol.metadata().name().value.clone(),
                            field_name: child.metadata().name().value.clone(),
                            field_span: child.metadata().span().clone(),
                            type_kind,
                        });
                    return;
                }
            }
            KestrelSymbolKind::EnumCase => {
                if let Some(callable) = child.metadata().get_behavior::<CallableBehavior>() {
                    if let Some(param) = callable.parameters().iter().find(|p| p.ty.is_cloneable())
                    {
                        context
                            .diagnostics
                            .throw(CloneableFieldRequiresCloneableConformance {
                                type_span: symbol.metadata().span().clone(),
                                type_name: symbol.metadata().name().value.clone(),
                                field_name: param.bind_name.value.clone(),
                                field_span: child.metadata().span().clone(),
                                type_kind,
                            });
                        return;
                    }
                }
            }
            _ => {}
        }
    }
}
