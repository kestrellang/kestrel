//! ResolveTypePath query - resolve a type path to a Type

use std::sync::Arc;

use kestrel_prelude::lang;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{FloatBits, IntBits, Ty, TyKind};
use kestrel_span::Span;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::{InheritedProtocolMember, ResolveName, SymbolFor, VisibleChildrenByName};
use crate::query::Query;
use crate::resolution::{SymbolResolution, TypePathResolution};

/// Resolve a type path to a Type.
///
/// Handles:
/// - Built-in lang scalar types (lang.i1, lang.i64, lang.f64, lang.str, etc.)
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

        // For "Self" paths, check if it resolves to a type parameter first.
        // This handles protocol extensions where Self is a synthetic type parameter
        // with the target protocol as a bound, allowing Self.Item to work.
        if self.path[0] == "Self" {
            if let Some(result) = try_resolve_self_as_type_param(model, &self.path, self.context) {
                return result;
            }
        }

        // Handle built-in types that don't exist as real symbols.
        //
        // - `Self` (fallback if not a type parameter)
        // - `lang.*` scalar types (i1/i8/.../f16/f32/f64/str)
        if let Some(ty) = resolve_builtin_type_path(&self.path, Span::new(0, 0..0)) {
            return TypePathResolution::Resolved(ty);
        }

        // First segment: use scope-aware name resolution
        let first = &self.path[0];
        let first_resolution = model.query(ResolveName {
            name: first.clone(),
            context: self.context,
        });

        let mut current_symbol = match first_resolution {
            SymbolResolution::Found(ids) if ids.len() == 1 => {
                match model.query(SymbolFor { id: ids[0] }) {
                    Some(s) => s,
                    None => {
                        return TypePathResolution::NotFound {
                            segment: first.clone(),
                            index: 0,
                        };
                    },
                }
            },
            SymbolResolution::Found(ids) => {
                return TypePathResolution::Ambiguous {
                    segment: first.clone(),
                    index: 0,
                    candidates: ids,
                };
            },
            SymbolResolution::Ambiguous(ids) => {
                return TypePathResolution::Ambiguous {
                    segment: first.clone(),
                    index: 0,
                    candidates: ids,
                };
            },
            SymbolResolution::NotFound => {
                return TypePathResolution::NotFound {
                    segment: first.clone(),
                    index: 0,
                };
            },
        };

        // Subsequent segments: search visible children
        for (index, segment) in self.path.iter().enumerate().skip(1) {
            // Special case: if current symbol is a TypeParameter, look up associated types
            // from its protocol bounds (e.g., T.Item where T: Iterator)
            if current_symbol.metadata().kind() == KestrelSymbolKind::TypeParameter
                && let Some(symbol) = model.query(SymbolFor {
                    id: current_symbol.metadata().id(),
                })
                && let Ok(type_param_arc) = symbol
                    .clone()
                    .into_any_arc()
                    .downcast::<TypeParameterSymbol>()
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

            let matches = model.query(VisibleChildrenByName {
                parent: current_symbol.metadata().id(),
                name: segment.clone(),
                context: self.context,
            });

            match matches.len() {
                0 => {
                    return TypePathResolution::NotFound {
                        segment: segment.clone(),
                        index,
                    };
                },
                1 => {
                    current_symbol = matches.into_iter().next().unwrap();
                },
                _ => {
                    return TypePathResolution::Ambiguous {
                        segment: segment.clone(),
                        index,
                        candidates: matches.iter().map(|s| s.metadata().id()).collect(),
                    };
                },
            }
        }

        // Handle TypeParameterSymbol specially
        if current_symbol.metadata().kind() == KestrelSymbolKind::TypeParameter
            && let Some(symbol) = model.query(SymbolFor {
                id: current_symbol.metadata().id(),
            })
            && let Ok(type_param_arc) = symbol.into_any_arc().downcast::<TypeParameterSymbol>()
        {
            let span = type_param_arc.metadata().span().clone();
            let ty = Ty::type_parameter(type_param_arc, span);
            return TypePathResolution::Resolved(ty);
        }

        // Handle AssociatedTypeSymbol specially
        if current_symbol.metadata().kind() == KestrelSymbolKind::AssociatedType
            && let Some(symbol) = model.query(SymbolFor {
                id: current_symbol.metadata().id(),
            })
            && let Ok(assoc_type_arc) = symbol.into_any_arc().downcast::<AssociatedTypeSymbol>()
        {
            let span = assoc_type_arc.metadata().span().clone();
            let ty = Ty::associated_type(assoc_type_arc, span);
            return TypePathResolution::Resolved(ty);
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

/// Resolve a built-in type path (that doesn't exist as a real symbol) to its semantic type.
fn resolve_builtin_type_path(path: &[String], span: Span) -> Option<Ty> {
    if path.len() == 1 && path[0] == "Self" {
        return Some(Ty::self_type(span));
    }

    // Support `lang.i1`, `lang.i64`, `lang.f16`, `lang.str`, etc.
    if path.len() == 2 && path[0] == lang::LANG {
        return resolve_lang_scalar_type(&path[1], span);
    }

    None
}

fn resolve_lang_scalar_type(name: &str, span: Span) -> Option<Ty> {
    match name {
        lang::I1 => Some(Ty::bool(span)),

        lang::I8 => Some(Ty::int(IntBits::I8, span)),
        lang::I16 => Some(Ty::int(IntBits::I16, span)),
        lang::I32 => Some(Ty::int(IntBits::I32, span)),
        lang::I64 => Some(Ty::int(IntBits::I64, span)),

        // Note: unsigned scalar names exist in kestrel_prelude::lang, but Kestrel's semantic
        // Ty currently models "IntBits" (signed) for built-in integers. We can add unsigned
        // semantics later if/when the type system grows an unsigned integer kind.
        lang::F16 => Some(Ty::float(FloatBits::F16, span)),
        lang::F32 => Some(Ty::float(FloatBits::F32, span)),
        lang::F64 => Some(Ty::float(FloatBits::F64, span)),

        lang::STR => Some(Ty::string(span)),
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
    let mut current_id = Some(context_id);
    let param_id = type_param.metadata().id();
    let mut all_bounds = Vec::new();

    // Walk up the symbol hierarchy to collect all protocol bounds for this type parameter
    while let Some(id) = current_id {
        if let Some(symbol) = model.query(SymbolFor { id }) {
            // Check GenericsBehavior
            if let Some(generics_beh) = symbol.metadata().get_behavior::<GenericsBehavior>() {
                all_bounds.extend(
                    generics_beh
                        .where_clause()
                        .bounds_for(param_id)
                        .into_iter()
                        .cloned(),
                );
            }

            // Check ExtensionTargetBehavior
            if let Some(target_beh) = symbol.metadata().get_behavior::<ExtensionTargetBehavior>() {
                all_bounds.extend(
                    target_beh
                        .where_clause()
                        .bounds_for(param_id)
                        .into_iter()
                        .cloned(),
                );
            }

            current_id = symbol.metadata().parent().map(|p| p.metadata().id());
        } else {
            break;
        }
    }

    if all_bounds.is_empty() {
        return None;
    }

    // Search protocol bounds for the associated type
    for bound in all_bounds {
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
                    if let Some(symbol) = model.query(SymbolFor {
                        id: child.metadata().id(),
                    }) && let Ok(assoc_type_arc) =
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

                        let ty = Ty::qualified_associated_type(assoc_type_arc, container_ty, span);
                        return Some(TypePathResolution::Resolved(ty));
                    }
                }
            }

            // Check inherited protocols
            if let Some(member_id) = model.query(InheritedProtocolMember {
                protocol_id: protocol.metadata().id(),
                name: segment.to_string(),
            }) && let Some(symbol) = model.query(SymbolFor { id: member_id })
                && let Ok(assoc_type_arc) = symbol.into_any_arc().downcast::<AssociatedTypeSymbol>()
            {
                let span = type_param.metadata().span().clone();
                let container_ty = Ty::type_parameter(type_param.clone(), span.clone());
                let ty = Ty::qualified_associated_type(assoc_type_arc, container_ty, span);
                return Some(TypePathResolution::Resolved(ty));
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
                    && let Some(symbol) = model.query(SymbolFor {
                        id: child.metadata().id(),
                    })
                    && let Ok(inner_assoc_arc) =
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

                    let ty = Ty::qualified_associated_type(inner_assoc_arc, container_ty, span);
                    return Some(TypePathResolution::Resolved(ty));
                }
            }

            // Check inherited protocols
            if let Some(member_id) = model.query(InheritedProtocolMember {
                protocol_id: protocol.metadata().id(),
                name: segment.to_string(),
            }) && let Some(symbol) = model.query(SymbolFor { id: member_id })
                && let Ok(inner_assoc_arc) =
                    symbol.into_any_arc().downcast::<AssociatedTypeSymbol>()
            {
                let span = container_ty.span().clone();
                let ty = Ty::qualified_associated_type(inner_assoc_arc, container_ty, span);
                return Some(TypePathResolution::Resolved(ty));
            }
        }
    }

    None
}

/// Try to resolve "Self" as a type parameter in the current context.
///
/// This handles protocol extensions where Self is a synthetic type parameter
/// with the target protocol as a bound. Returns Some if Self is found as a
/// type parameter, None otherwise to fall back to builtin SelfType.
/// Try to resolve "Self.Item" style paths in protocol extensions by looking up
/// the associated type from the target protocol.
///
/// For single-segment "Self", returns None to fall back to builtin SelfType.
/// For multi-segment paths like "Self.Item", looks up the associated type
/// from the protocol extension's target protocol.
fn try_resolve_self_as_type_param(
    model: &SemanticModel,
    path: &[String],
    context: SymbolId,
) -> Option<TypePathResolution> {
    // For single-segment "Self", fall back to builtin SelfType
    if path.len() == 1 {
        return None;
    }

    // For multi-segment paths like "Self.Item", try to find the associated type
    // from the protocol extension's target protocol

    // Use ResolveName to check if "Self" resolves to a synthetic type parameter
    let resolution = model.query(ResolveName {
        name: "Self".to_string(),
        context,
    });

    match resolution {
        SymbolResolution::Found(ids) if ids.len() == 1 => {
            let symbol = model.query(SymbolFor { id: ids[0] })?;

            // Only use this if it's a TypeParameter (our synthetic Self)
            if symbol.metadata().kind() != KestrelSymbolKind::TypeParameter {
                return None;
            }

            let type_param = symbol
                .into_any_arc()
                .downcast::<TypeParameterSymbol>()
                .ok()?;

            // "Self.Item" style - use associated type resolution from type param bounds
            let segment = &path[1];
            resolve_associated_type_from_type_param_with_context(
                model,
                &type_param,
                segment,
                &path[1..],
                1,
                context,
            )
        },
        _ => None,
    }
}
