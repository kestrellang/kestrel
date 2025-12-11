//! ResolveTypePath query - resolve a type path to a Type

use std::sync::Arc;

use kestrel_prelude::primitives;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{FloatBits, IntBits, Ty, TyKind};
use kestrel_span::Span;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::queries::ResolveName;
use crate::query::Query;
use crate::resolution::{SymbolResolution, TypePathResolution};
use crate::visibility;
use crate::SemanticModel;

/// Resolve a type path to a Type.
///
/// Handles:
/// - Primitive types (Int, Bool, String, etc.)
/// - User-defined types via scope resolution
/// - Type parameters
/// - Associated types (including T.Item style)
pub struct ResolveTypePath {
    pub path: Vec<String>,
    pub context: SymbolId,
}

impl Query for ResolveTypePath {
    type Output = TypePathResolution;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        if self.path.is_empty() {
            return TypePathResolution::NotFound {
                segment: String::new(),
                index: 0,
            };
        }

        // Handle built-in primitive types
        if self.path.len() == 1 {
            let segment = &self.path[0];
            if let Some(ty) = resolve_primitive_type(segment, Span::from(0..0)) {
                return TypePathResolution::Resolved(ty);
            }
        }

        let context_symbol = match model.registry().get(self.context) {
            Some(s) => s,
            None => {
                return TypePathResolution::NotFound {
                    segment: self.path[0].clone(),
                    index: 0,
                };
            }
        };

        // First segment: use scope-aware name resolution
        let first = &self.path[0];
        let first_resolution = model.query(ResolveName {
            name: first.clone(),
            context: self.context,
        });

        let mut current_symbol = match first_resolution {
            SymbolResolution::Found(ids) if ids.len() == 1 => match model.registry().get(ids[0]) {
                Some(s) => s,
                None => {
                    return TypePathResolution::NotFound {
                        segment: first.clone(),
                        index: 0,
                    };
                }
            },
            SymbolResolution::Found(ids) => {
                return TypePathResolution::Ambiguous {
                    segment: first.clone(),
                    index: 0,
                    candidates: ids,
                };
            }
            SymbolResolution::Ambiguous(ids) => {
                return TypePathResolution::Ambiguous {
                    segment: first.clone(),
                    index: 0,
                    candidates: ids,
                };
            }
            SymbolResolution::NotFound => {
                return TypePathResolution::NotFound {
                    segment: first.clone(),
                    index: 0,
                };
            }
        };

        // Subsequent segments: search visible children
        for (index, segment) in self.path.iter().enumerate().skip(1) {
            // Special case: if current symbol is a TypeParameter, look up associated types
            // from its protocol bounds (e.g., T.Item where T: Iterator)
            if current_symbol.metadata().kind() == KestrelSymbolKind::TypeParameter {
                if let Some(symbol) = model.registry().get(current_symbol.metadata().id()) {
                    if let Ok(type_param_arc) =
                        symbol.clone().into_any_arc().downcast::<TypeParameterSymbol>()
                    {
                        // Use context (the function/struct where this type is being resolved)
                        // instead of type_param's parent, since the parent may not be set correctly
                        if let Some(result) = resolve_associated_type_from_type_param_with_context(
                            model,
                            &type_param_arc,
                            segment,
                            &self.path[index..],
                            index,
                            self.context,
                        ) {
                            return result;
                        }
                    }
                }
            }

            let matches =
                visibility::find_visible_children_by_name(&current_symbol, segment, &context_symbol);

            match matches.len() {
                0 => {
                    return TypePathResolution::NotFound {
                        segment: segment.clone(),
                        index,
                    };
                }
                1 => {
                    current_symbol = matches.into_iter().next().unwrap();
                }
                _ => {
                    return TypePathResolution::Ambiguous {
                        segment: segment.clone(),
                        index,
                        candidates: matches.iter().map(|s| s.metadata().id()).collect(),
                    };
                }
            }
        }

        // Handle TypeParameterSymbol specially
        if current_symbol.metadata().kind() == KestrelSymbolKind::TypeParameter {
            if let Some(symbol) = model.registry().get(current_symbol.metadata().id()) {
                if let Ok(type_param_arc) = symbol.into_any_arc().downcast::<TypeParameterSymbol>()
                {
                    let span = type_param_arc.metadata().span().clone();
                    let ty = Ty::type_parameter(type_param_arc, span);
                    return TypePathResolution::Resolved(ty);
                }
            }
        }

        // Handle AssociatedTypeSymbol specially
        if current_symbol.metadata().kind() == KestrelSymbolKind::AssociatedType {
            if let Some(symbol) = model.registry().get(current_symbol.metadata().id()) {
                if let Ok(assoc_type_arc) =
                    symbol.into_any_arc().downcast::<AssociatedTypeSymbol>()
                {
                    let span = assoc_type_arc.metadata().span().clone();
                    let ty = Ty::associated_type(assoc_type_arc, span);
                    return TypePathResolution::Resolved(ty);
                }
            }
        }

        // Extract type from TypedBehavior
        let behaviors = current_symbol.metadata().behaviors();
        let typed_behaviors: Vec<_> = behaviors
            .iter()
            .filter_map(|b| {
                if matches!(b.kind(), KestrelBehaviorKind::Typed) {
                    b.as_ref().downcast_ref::<TypedBehavior>()
                } else {
                    None
                }
            })
            .collect();

        let type_alias_behavior = typed_behaviors
            .iter()
            .find(|tb| tb.ty().is_type_alias())
            .copied();

        let typed_behavior = type_alias_behavior.or_else(|| typed_behaviors.first().copied());

        match typed_behavior {
            Some(tb) => TypePathResolution::Resolved(tb.ty().clone()),
            None => TypePathResolution::NotAType {
                symbol_id: current_symbol.metadata().id(),
            },
        }
    }
}

/// Resolve a primitive type name to its semantic type
fn resolve_primitive_type(name: &str, span: Span) -> Option<Ty> {
    match name {
        primitives::INT => Some(Ty::int(IntBits::I64, span)),
        primitives::I8 => Some(Ty::int(IntBits::I8, span)),
        primitives::I16 => Some(Ty::int(IntBits::I16, span)),
        primitives::I32 => Some(Ty::int(IntBits::I32, span)),
        primitives::I64 => Some(Ty::int(IntBits::I64, span)),
        primitives::FLOAT => Some(Ty::float(FloatBits::F64, span)),
        primitives::F32 => Some(Ty::float(FloatBits::F32, span)),
        primitives::F64 => Some(Ty::float(FloatBits::F64, span)),
        primitives::BOOL => Some(Ty::bool(span)),
        primitives::STRING => Some(Ty::string(span)),
        primitives::SELF_TYPE => Some(Ty::self_type(span)),
        _ => None,
    }
}

/// Resolve an associated type from a type parameter's protocol bounds.
///
/// Given a type parameter T and a segment "Item", this looks up the where clause
/// bounds for T (e.g., `where T: Iterator`) and finds the associated type "Item"
/// from those protocol bounds.
fn resolve_associated_type_from_type_param_with_context(
    model: &SemanticModel,
    type_param: &Arc<TypeParameterSymbol>,
    segment: &str,
    remaining_path: &[String],
    _index: usize,
    context_id: SymbolId,
) -> Option<TypePathResolution> {
    // Get the context symbol (the function/struct where this type is being resolved)
    let context = model.registry().get(context_id)?;

    // Get the where clause from the context's GenericsBehavior
    let generics_beh = context.generics_behavior()?;
    let where_clause = generics_beh.where_clause();

    // Get protocol bounds for this type parameter
    let param_id = type_param.metadata().id();
    let bounds = where_clause.bounds_for(param_id);

    if bounds.is_empty() {
        return None;
    }

    // Search protocol bounds for the associated type
    for bound in bounds {
        if let TyKind::Protocol {
            symbol: protocol, ..
        } = bound.kind()
        {
            // Check direct children of protocol
            let protocol_dyn = protocol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
            for child in protocol_dyn.metadata().children() {
                if child.metadata().kind() == KestrelSymbolKind::AssociatedType
                    && child.metadata().name().value == segment
                {
                    // Found it! Create a qualified associated type
                    if let Some(symbol) = model.registry().get(child.metadata().id()) {
                        if let Ok(assoc_type_arc) =
                            symbol.into_any_arc().downcast::<AssociatedTypeSymbol>()
                        {
                            let span = type_param.metadata().span().clone();
                            let container_ty = Ty::type_parameter(type_param.clone(), span.clone());

                            // If there are more segments (e.g., T.Iter.Item), we need to handle
                            // nested associated types - for now just handle one level
                            if remaining_path.len() > 1 {
                                // For nested paths like C.Iter.Item, we need to recursively resolve
                                // First create T.Iter, then look up Item on that
                                let first_assoc_ty = Ty::qualified_associated_type(
                                    assoc_type_arc.clone(),
                                    container_ty.clone(),
                                    span.clone(),
                                );

                                // Now we need to find "Item" in the bounds of "Iter"
                                // Check if the associated type has bounds that are protocols
                                if let Some(result) = resolve_nested_associated_type(
                                    model,
                                    &assoc_type_arc,
                                    first_assoc_ty,
                                    &remaining_path[1..],
                                ) {
                                    return Some(result);
                                }
                            }

                            let ty =
                                Ty::qualified_associated_type(assoc_type_arc, container_ty, span);
                            return Some(TypePathResolution::Resolved(ty));
                        }
                    }
                }
            }

            // Check inherited protocols
            if let Some(SymbolResolution::Found(ids)) =
                find_in_inherited_protocols(&protocol_dyn, segment)
            {
                if let Some(id) = ids.first() {
                    if let Some(symbol) = model.registry().get(*id) {
                        if let Ok(assoc_type_arc) =
                            symbol.into_any_arc().downcast::<AssociatedTypeSymbol>()
                        {
                            let span = type_param.metadata().span().clone();
                            let container_ty = Ty::type_parameter(type_param.clone(), span.clone());
                            let ty =
                                Ty::qualified_associated_type(assoc_type_arc, container_ty, span);
                            return Some(TypePathResolution::Resolved(ty));
                        }
                    }
                }
            }
        }
    }

    None
}

/// Resolve nested associated types (e.g., C.Iter.Item)
///
/// Given an associated type like "Iter" with bounds "Iterator", and remaining path ["Item"],
/// find the "Item" associated type from the Iterator protocol bound.
fn resolve_nested_associated_type(
    model: &SemanticModel,
    assoc_type: &Arc<AssociatedTypeSymbol>,
    container_ty: Ty,
    remaining_path: &[String],
) -> Option<TypePathResolution> {
    if remaining_path.is_empty() {
        return None;
    }

    let segment = &remaining_path[0];

    // Get the bounds of the associated type (e.g., type Iter: Iterator)
    // Associated types use bounds(), not ConformancesBehavior
    let bounds = assoc_type.bounds()?;

    for bound in bounds.iter() {
        if let TyKind::Protocol {
            symbol: protocol, ..
        } = bound.kind()
        {
            let protocol_dyn = protocol.clone() as Arc<dyn Symbol<KestrelLanguage>>;

            // Look for the segment in this protocol
            for child in protocol_dyn.metadata().children() {
                if child.metadata().kind() == KestrelSymbolKind::AssociatedType
                    && child.metadata().name().value == *segment
                {
                    if let Some(symbol) = model.registry().get(child.metadata().id()) {
                        if let Ok(inner_assoc_arc) =
                            symbol.into_any_arc().downcast::<AssociatedTypeSymbol>()
                        {
                            let span = container_ty.span().clone();

                            // If there are still more segments, recurse
                            if remaining_path.len() > 1 {
                                let nested_container = Ty::qualified_associated_type(
                                    inner_assoc_arc.clone(),
                                    container_ty,
                                    span.clone(),
                                );
                                return resolve_nested_associated_type(
                                    model,
                                    &inner_assoc_arc,
                                    nested_container,
                                    &remaining_path[1..],
                                );
                            }

                            let ty =
                                Ty::qualified_associated_type(inner_assoc_arc, container_ty, span);
                            return Some(TypePathResolution::Resolved(ty));
                        }
                    }
                }
            }

            // Check inherited protocols
            if let Some(SymbolResolution::Found(ids)) =
                find_in_inherited_protocols(&protocol_dyn, segment)
            {
                if let Some(id) = ids.first() {
                    if let Some(symbol) = model.registry().get(*id) {
                        if let Ok(inner_assoc_arc) =
                            symbol.into_any_arc().downcast::<AssociatedTypeSymbol>()
                        {
                            let span = container_ty.span().clone();
                            let ty =
                                Ty::qualified_associated_type(inner_assoc_arc, container_ty, span);
                            return Some(TypePathResolution::Resolved(ty));
                        }
                    }
                }
            }
        }
    }

    None
}

/// Search for a name in inherited protocols (for associated type inheritance).
fn find_in_inherited_protocols(
    protocol: &Arc<dyn Symbol<KestrelLanguage>>,
    name: &str,
) -> Option<SymbolResolution> {
    let conformances_beh = protocol.conformances_behavior()?;

    for parent_ty in conformances_beh.conformances() {
        if let TyKind::Protocol {
            symbol: parent_proto,
            ..
        } = parent_ty.kind()
        {
            // Check direct children of parent protocol
            let parent_dyn = parent_proto.clone() as Arc<dyn Symbol<KestrelLanguage>>;
            for child in parent_dyn.metadata().children() {
                if child.metadata().kind() == KestrelSymbolKind::AssociatedType
                    && child.metadata().name().value == name
                {
                    return Some(SymbolResolution::Found(vec![child.metadata().id()]));
                }
            }

            // Recursively check grandparent protocols
            if let Some(result) = find_in_inherited_protocols(&parent_dyn, name) {
                return Some(result);
            }
        }
    }

    None
}
