//! ProtocolRequiredProperties query - collect required properties for a protocol (including inherited)

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::field::FieldSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Information about a protocol property requirement.
#[derive(Debug, Clone)]
pub struct PropertyRequirement {
    /// The name of the required property
    pub name: String,
    /// The type of the property
    pub property_type: Ty,
    /// Whether the property requires a getter (always true for computed properties)
    pub has_getter: bool,
    /// Whether the property requires a setter ({ get set } vs { get })
    pub has_setter: bool,
    /// Whether this is a static property
    pub is_static: bool,
    /// The FieldSymbol ID for error reporting
    pub field_id: SymbolId,
}

/// Get all properties required by a protocol, including inherited protocol properties.
///
/// If a protocol defines a property with the same name as an inherited property,
/// the protocol's property overrides the inherited property.
pub struct ProtocolRequiredProperties {
    pub protocol_id: SymbolId,
}

impl Query for ProtocolRequiredProperties {
    type Output = Vec<PropertyRequirement>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let symbol = match model.query(SymbolFor {
            id: self.protocol_id,
        }) {
            Some(s) => s,
            None => return Vec::new(),
        };
        if symbol.metadata().kind() != KestrelSymbolKind::Protocol {
            return Vec::new();
        }
        let Ok(protocol) = symbol.downcast_arc::<ProtocolSymbol>() else {
            return Vec::new();
        };

        let mut properties: HashMap<String, PropertyRequirement> = HashMap::new();
        let mut visited: HashSet<SymbolId> = HashSet::new();
        collect_protocol_properties_recursive(&protocol, model, &mut properties, &mut visited);
        properties.into_values().collect()
    }
}

fn collect_protocol_properties_recursive(
    protocol: &Arc<ProtocolSymbol>,
    model: &SemanticModel,
    properties: &mut HashMap<String, PropertyRequirement>,
    visited: &mut HashSet<SymbolId>,
) {
    let id = protocol.metadata().id();
    if visited.contains(&id) {
        return;
    }
    visited.insert(id);

    let protocol_dyn: Arc<dyn Symbol<KestrelLanguage>> = protocol.clone();
    if let Some(conformances) = protocol_dyn
        .metadata()
        .get_behavior::<ConformancesBehavior>()
    {
        for inherited_ty in conformances.conformances() {
            if let TyKind::Protocol { symbol, .. } = inherited_ty.kind() {
                collect_protocol_properties_recursive(symbol, model, properties, visited);
            }
        }
    }

    for requirement in collect_property_requirements_from_symbol(&protocol_dyn, model) {
        properties.insert(requirement.name.clone(), requirement);
    }
}

fn collect_property_requirements_from_symbol(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    model: &SemanticModel,
) -> Vec<PropertyRequirement> {
    symbol
        .metadata()
        .children()
        .into_iter()
        .filter(|child| child.metadata().kind() == KestrelSymbolKind::Field)
        .filter_map(|child| {
            let field: Arc<FieldSymbol> = child.clone().into_any_arc().downcast().ok()?;

            // Only computed properties can be protocol requirements
            if !field.is_computed() {
                return None;
            }

            // Check if this is a requirement (getter/setter without body) or an implementation
            let getter_id = field.getter()?;
            let getter_sym = model.query(SymbolFor { id: getter_id })?;

            // If the getter has ExecutableBehavior, it has a body and is a default implementation,
            // not a requirement
            if getter_sym
                .metadata()
                .get_behavior::<ExecutableBehavior>()
                .is_some()
            {
                return None;
            }

            // Check if there's a setter requirement
            let has_setter = if let Some(setter_id) = field.setter() {
                // If setter exists but has no body, it's required
                if let Some(setter_sym) = model.query(SymbolFor { id: setter_id }) {
                    setter_sym
                        .metadata()
                        .get_behavior::<ExecutableBehavior>()
                        .is_none()
                } else {
                    false
                }
            } else {
                false
            };

            // Get the resolved type from TypedBehavior (set by binder)
            // Fall back to field_type if no behavior found
            let field_dyn: Arc<dyn Symbol<KestrelLanguage>> = field.clone();
            let property_type = field_dyn
                .metadata()
                .get_behavior::<TypedBehavior>()
                .map(|typed| typed.ty().clone())
                .unwrap_or_else(|| field.field_type().clone());

            Some(PropertyRequirement {
                name: field.metadata().name().value.clone(),
                property_type,
                has_getter: true, // Always true for computed property requirements
                has_setter,
                is_static: field.is_static(),
                field_id: field.metadata().id(),
            })
        })
        .collect()
}
