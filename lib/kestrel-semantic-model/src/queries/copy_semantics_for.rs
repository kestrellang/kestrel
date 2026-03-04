//! CopySemanticsFor query — unified copy semantics computation for structs and enums.
//!
//! Computes whether a type is Copyable, Cloneable, or NotCopyable based on its
//! conformances and the types of its children (fields for structs, case payloads for enums).
//!
//! This is a pure query with no diagnostic emission. The "cloneable field requires
//! Cloneable conformance" diagnostic is emitted by the conformance analyzer instead.

use std::cell::RefCell;

use kestrel_semantic_tree::behavior::copy_semantics::CopySemantics;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::TyKind;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::query::Query;
use crate::queries::SymbolFor;

thread_local! {
    static COMPUTING: RefCell<Vec<SymbolId>> = RefCell::new(Vec::new());
}

/// Query: compute the copy semantics for a struct or enum.
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct CopySemanticsFor {
    pub symbol_id: SymbolId,
}

impl Query for CopySemanticsFor {
    type Output = CopySemantics;

    fn execute(self, model: &SemanticModel) -> CopySemantics {
        // Cycle detection: if we're already computing this symbol, return Copyable
        // (optimistic default, matches current behavior where CycleDetector returns early)
        let is_cycle = COMPUTING.with(|c| c.borrow().contains(&self.symbol_id));
        if is_cycle {
            return CopySemantics::Copyable;
        }

        COMPUTING.with(|c| c.borrow_mut().push(self.symbol_id));
        let result = copy_semantics_impl(model, self.symbol_id);
        COMPUTING.with(|c| c.borrow_mut().retain(|id| *id != self.symbol_id));
        result
    }
}

fn copy_semantics_impl(model: &SemanticModel, symbol_id: SymbolId) -> CopySemantics {
    let Some(symbol) = model.query(SymbolFor { id: symbol_id }) else {
        return CopySemantics::Copyable;
    };

    // Check if the Copyable protocol is registered
    let Some(copyable_id) = model.builtin_registry().copyable_protocol() else {
        return CopySemantics::Copyable;
    };

    // Get conformances
    let conformances = symbol.metadata().get_behavior::<ConformancesBehavior>();

    // Check for explicit `not Copyable`
    let has_not_copyable = conformances
        .as_ref()
        .map(|c| c.has_negative_conformance_to(copyable_id))
        .unwrap_or(false);

    // Collect all child types (unified for struct fields and enum case payloads)
    let child_types = collect_child_types(&symbol);

    // Check if any child type is non-copyable
    let has_non_copyable_child = child_types.iter().any(|ty| !ty.is_copyable());

    // Rule 1 & 2: If explicitly not copyable or has non-copyable child → NotCopyable
    if has_not_copyable || has_non_copyable_child {
        return CopySemantics::NotCopyable;
    }

    // Check if type conforms to Cloneable
    let conforms_to_cloneable = model
        .builtin_registry()
        .cloneable_protocol()
        .map(|cloneable_id| {
            conformances
                .as_ref()
                .map(|c| has_conformance_to(c, cloneable_id))
                .unwrap_or(false)
        })
        .unwrap_or(false);

    // Rule 3: If conforms to Cloneable → Cloneable
    if conforms_to_cloneable {
        return CopySemantics::Cloneable;
    }

    // Check if any child type is cloneable
    let has_cloneable_child = child_types.iter().any(|ty| ty.is_cloneable());

    // Rule 4: Has cloneable child but doesn't conform to Cloneable → NotCopyable
    // (The diagnostic for this case is emitted by the conformance analyzer)
    if has_cloneable_child {
        return CopySemantics::NotCopyable;
    }

    // Rule 5: Otherwise → Copyable
    CopySemantics::Copyable
}

/// Collect all types from a symbol's children that affect copy semantics.
/// For structs: field types (via TypedBehavior)
/// For enums: enum case payload types (via CallableBehavior parameters)
pub fn collect_child_types(
    symbol: &std::sync::Arc<dyn semantic_tree::symbol::Symbol<kestrel_semantic_tree::language::KestrelLanguage>>,
) -> Vec<kestrel_semantic_tree::ty::Ty> {
    let mut types = Vec::new();
    for child in symbol.metadata().children().iter() {
        match child.metadata().kind() {
            KestrelSymbolKind::Field => {
                if let Some(typed) = child.metadata().get_behavior::<TypedBehavior>() {
                    types.push(typed.ty().clone());
                }
            }
            KestrelSymbolKind::EnumCase => {
                if let Some(callable) = child.metadata().get_behavior::<CallableBehavior>() {
                    for param in callable.parameters() {
                        types.push(param.ty.clone());
                    }
                }
            }
            _ => {}
        }
    }
    types
}

/// Check if conformances include a specific protocol by symbol ID.
fn has_conformance_to(conformances: &ConformancesBehavior, protocol_id: SymbolId) -> bool {
    conformances.conformances().iter().any(|ty| {
        if let TyKind::Protocol { symbol, .. } = ty.kind() {
            symbol.metadata().id() == protocol_id
        } else {
            false
        }
    })
}
