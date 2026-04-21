//! Analyzer for cloneable field conformance
//!
//! Validates that structs/enums with Cloneable fields conform to Cloneable.

use std::sync::Arc;

use kestrel_semantic_model::queries::collect_child_types;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::copy_semantics::{CopySemantics, CopySemanticsBehavior};
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::TyKind;
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

mod diagnostics;
use diagnostics::CloneableFieldRequiresCloneableConformance;

/// Analyzer that validates cloneable field conformance for structs and enums.
pub struct CloneableFieldAnalyzer;

impl CloneableFieldAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CloneableFieldAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for CloneableFieldAnalyzer {
    fn name(&self) -> &'static str {
        "cloneable_field"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        let kind = symbol.metadata().kind();
        if kind != KestrelSymbolKind::Struct && kind != KestrelSymbolKind::Enum {
            return;
        }

        // Only check if copy semantics is NotCopyable (Rule 4 case)
        let Some(copy_behavior) = symbol.metadata().get_behavior::<CopySemanticsBehavior>() else {
            return;
        };
        if copy_behavior.semantics() != CopySemantics::NotCopyable {
            return;
        }

        // Check if the type already conforms to Cloneable — if so, no diagnostic needed
        let Some(cloneable_id) = ctx.model.builtin_registry().cloneable_protocol() else {
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
        let Some(copyable_id) = ctx.model.builtin_registry().copyable_protocol() else {
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
        let child_types = collect_child_types(symbol);
        if child_types.iter().any(|ty| !ty.is_copyable()) {
            return;
        }

        let type_kind = if kind == KestrelSymbolKind::Struct {
            "struct"
        } else {
            "enum"
        };

        // Find first cloneable child for the diagnostic
        for child in symbol.metadata().children().iter() {
            match child.metadata().kind() {
                KestrelSymbolKind::Field => {
                    if let Some(typed) = child.metadata().get_behavior::<TypedBehavior>()
                        && typed.ty().is_cloneable()
                    {
                        ctx.report(CloneableFieldRequiresCloneableConformance {
                            type_span: symbol.metadata().span().clone(),
                            type_name: symbol.metadata().name().value.clone(),
                            field_name: child.metadata().name().value.clone(),
                            field_span: child.metadata().span().clone(),
                            type_kind,
                        });
                        return;
                    }
                },
                KestrelSymbolKind::EnumCase => {
                    if let Some(callable) = child.metadata().get_behavior::<CallableBehavior>() {
                        if let Some(param) =
                            callable.parameters().iter().find(|p| p.ty.is_cloneable())
                        {
                            ctx.report(CloneableFieldRequiresCloneableConformance {
                                type_span: symbol.metadata().span().clone(),
                                type_name: symbol.metadata().name().value.clone(),
                                field_name: param.bind_name.value.clone(),
                                field_span: child.metadata().span().clone(),
                                type_kind,
                            });
                            return;
                        }
                    }
                },
                _ => {},
            }
        }
    }
}
