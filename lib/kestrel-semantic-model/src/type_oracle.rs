//! TypeOracle implementation for SemanticModel.
//!
//! This module implements the `TypeOracle` trait from `kestrel-semantic-type-inference`,
//! allowing the type inference solver to query type information from the semantic model.

use std::sync::Arc;

use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::behavior::member_access::MemberAccessBehavior;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::builtins::BuiltinKind;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasSymbol;
use kestrel_semantic_tree::ty::{Substitutions, Ty, TyKind};
use kestrel_semantic_type_inference::{MemberError, MemberResolution, TypeOracle};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::queries::{ConformancesForSymbol, ExtensionsFor, ResolvedAliasedType};
use crate::SemanticModel;

impl TypeOracle for SemanticModel {
    fn resolve_member(
        &self,
        receiver_ty: &Ty,
        member: &str,
        is_static: bool,
    ) -> Result<MemberResolution, MemberError> {
        // Handle inference placeholders
        if matches!(receiver_ty.kind(), TyKind::Infer) {
            return Err(MemberError::UnknownType);
        }

        // Handle error types
        if matches!(receiver_ty.kind(), TyKind::Error) {
            return Err(MemberError::NotFound {
                receiver_ty: receiver_ty.clone(),
                member: member.to_string(),
            });
        }

        // Get the container symbol and substitutions
        let (container, substitutions) = match get_type_container_with_subs(receiver_ty) {
            Some(c) => c,
            None => {
                return Err(MemberError::NotFound {
                    receiver_ty: receiver_ty.clone(),
                    member: member.to_string(),
                })
            }
        };

        // Look for the member in direct children
        let member_symbol = container
            .metadata()
            .children()
            .into_iter()
            .find(|c| c.metadata().name().value == member);

        // If not found in direct children, search extensions
        let member_symbol = match member_symbol {
            Some(m) => m,
            None => {
                let container_id = container.metadata().id();
                let extensions = self.query(ExtensionsFor {
                    target_id: container_id,
                });

                // Find in extensions
                let extension_member = extensions
                    .iter()
                    .flat_map(|ext| ext.metadata().children())
                    .find(|child| child.metadata().name().value == member);

                match extension_member {
                    Some(m) => m,
                    None => {
                        return Err(MemberError::NotFound {
                            receiver_ty: receiver_ty.clone(),
                            member: member.to_string(),
                        })
                    }
                }
            }
        };

        let member_id = member_symbol.metadata().id();
        let member_kind = member_symbol.metadata().kind();

        // Handle static vs instance access
        if is_static {
            // Static access - should be a function with no receiver
            if member_kind == KestrelSymbolKind::Function {
                if let Some(callable) = member_symbol.metadata().get_behavior::<CallableBehavior>()
                {
                    if callable.is_static() {
                        let return_ty = callable.return_type().apply_substitutions(&substitutions);
                        return Ok(MemberResolution {
                            ty: return_ty,
                            symbol_id: member_id,
                            substitutions,
                        });
                    }
                }
            }
            // Static access on non-static member
            return Err(MemberError::NotFound {
                receiver_ty: receiver_ty.clone(),
                member: member.to_string(),
            });
        }

        // Instance access

        // Check for field access via MemberAccessBehavior
        for behavior in member_symbol.metadata().behaviors() {
            if behavior.kind() == KestrelBehaviorKind::MemberAccess {
                if let Some(access) = behavior.as_ref().downcast_ref::<MemberAccessBehavior>() {
                    let mut member_ty = access.member_type().clone();
                    member_ty = member_ty.apply_substitutions(&substitutions);
                    return Ok(MemberResolution {
                        ty: member_ty,
                        symbol_id: member_id,
                        substitutions,
                    });
                }
            }
        }

        // Check for method access
        if member_kind == KestrelSymbolKind::Function {
            if let Some(callable) = member_symbol.metadata().get_behavior::<CallableBehavior>() {
                // For methods, return the function type (parameters -> return)
                let return_ty = callable.return_type().apply_substitutions(&substitutions);
                return Ok(MemberResolution {
                    ty: return_ty,
                    symbol_id: member_id,
                    substitutions,
                });
            }
        }

        // Member exists but is not accessible (e.g., type alias, nested type)
        Err(MemberError::NotFound {
            receiver_ty: receiver_ty.clone(),
            member: member.to_string(),
        })
    }

    fn conforms_to(&self, ty: &Ty, protocol_id: SymbolId) -> bool {
        // Handle inference placeholders - can't check conformance yet
        if matches!(ty.kind(), TyKind::Infer) {
            return false;
        }

        // Handle error types - treat as conforming to suppress cascading errors
        if matches!(ty.kind(), TyKind::Error) {
            return true;
        }

        // Handle tuple types - check if protocol has tuple_conformance_propagation flag
        if let TyKind::Tuple(elements) = ty.kind() {
            // Look up if this protocol has tuple_conformance_propagation flag
            if let Some(feature) = self.builtin_registry().protocol_feature(protocol_id) {
                let definition = feature.definition();
                if let BuiltinKind::Protocol {
                    tuple_conformance_propagation: true,
                    ..
                } = definition.kind
                {
                    // Tuple conforms if all elements conform
                    return elements
                        .iter()
                        .all(|elem| self.conforms_to(elem, protocol_id));
                }
            }
            // Protocol doesn't have the flag or isn't a builtin, tuples don't conform
            return false;
        }

        // Get the type's symbol ID to check conformances
        let type_symbol_id = match get_type_symbol_id(ty) {
            Some(id) => id,
            None => return false,
        };

        // Get all conformances for this type
        let conformances = self.query(ConformancesForSymbol {
            symbol_id: type_symbol_id,
        });

        // Check if any conformance matches the protocol
        for conformance in conformances {
            if let TyKind::Protocol { symbol, .. } = conformance.kind() {
                if symbol.metadata().id() == protocol_id {
                    return true;
                }
            }
        }

        // Also check extensions for conformances
        // Get actual type's substitutions for filtering applicable extensions
        let actual_subs = get_type_substitutions(ty);

        let extensions = self.query(ExtensionsFor {
            target_id: type_symbol_id,
        });

        // Filter to only applicable extensions based on type arguments
        let applicable_extensions =
            filter_applicable_extensions_for_conformance(&extensions, &actual_subs);

        for extension in applicable_extensions {
            let ext_conformances = self.query(ConformancesForSymbol {
                symbol_id: extension.metadata().id(),
            });

            for conformance in ext_conformances {
                if let TyKind::Protocol { symbol, .. } = conformance.kind() {
                    if symbol.metadata().id() == protocol_id {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn resolve_associated_type(&self, container: &Ty, assoc_name: &str) -> Option<Ty> {
        // Handle inference placeholders
        if matches!(container.kind(), TyKind::Infer) {
            return None;
        }

        match container.kind() {
            // For struct types, look for type alias with that name
            TyKind::Struct {
                symbol,
                substitutions,
            } => {
                // Look for a type alias child with the given name
                for child in symbol.metadata().children() {
                    if child.metadata().kind() == KestrelSymbolKind::TypeAlias
                        && child.metadata().name().value == assoc_name
                    {
                        if let Ok(type_alias) = child.downcast_arc::<TypeAliasSymbol>() {
                            if let Some(resolved) = self.query(ResolvedAliasedType {
                                type_alias_id: type_alias.metadata().id(),
                            }) {
                                return Some(resolved.apply_substitutions(substitutions));
                            }
                        }
                    }
                }

                // Also check protocol conformances for associated type defaults
                let conformances = self.query(ConformancesForSymbol {
                    symbol_id: symbol.metadata().id(),
                });

                for conformance in conformances {
                    if let TyKind::Protocol {
                        symbol: proto,
                        substitutions: proto_subs,
                    } = conformance.kind()
                    {
                        if let Some(ty) =
                            resolve_associated_type_from_protocol(proto, assoc_name, proto_subs)
                        {
                            return Some(ty.apply_substitutions(substitutions));
                        }
                    }
                }

                None
            }

            // For protocol types, look for associated type declaration
            TyKind::Protocol {
                symbol,
                substitutions,
            } => resolve_associated_type_from_protocol(symbol, assoc_name, substitutions),

            // For type parameters, look up in bounds
            TyKind::TypeParameter(type_param) => {
                // Get bounds from the type parameter
                if let Some(conformances) =
                    type_param.metadata().get_behavior::<ConformancesBehavior>()
                {
                    for bound in conformances.conformances() {
                        if let TyKind::Protocol {
                            symbol,
                            substitutions,
                        } = bound.kind()
                        {
                            if let Some(ty) = resolve_associated_type_from_protocol(
                                symbol,
                                assoc_name,
                                substitutions,
                            ) {
                                return Some(ty);
                            }
                        }
                    }
                }
                None
            }

            // For associated types themselves, we might need to resolve nested projections
            TyKind::AssociatedType { container, .. } => {
                // First resolve the container's associated type, then look for nested
                if let Some(resolved_container) = container {
                    self.resolve_associated_type(resolved_container, assoc_name)
                } else {
                    None
                }
            }

            _ => None,
        }
    }

    fn expand_type_alias(&self, ty: &Ty) -> Ty {
        ty.expand_aliases()
    }

    fn symbol_name(&self, symbol_id: SymbolId) -> Option<String> {
        use crate::queries::SymbolFor;
        let symbol = self.query(SymbolFor { id: symbol_id })?;
        Some(symbol.metadata().name().value.clone())
    }
}

/// Get the container symbol and substitutions from a type.
fn get_type_container_with_subs(
    ty: &Ty,
) -> Option<(Arc<dyn Symbol<KestrelLanguage>>, Substitutions)> {
    match ty.kind() {
        TyKind::Struct {
            symbol,
            substitutions,
        } => {
            let dyn_symbol: Arc<dyn Symbol<KestrelLanguage>> = symbol.clone();
            Some((dyn_symbol, substitutions.clone()))
        }
        TyKind::Protocol {
            symbol,
            substitutions,
        } => {
            let dyn_symbol: Arc<dyn Symbol<KestrelLanguage>> = symbol.clone();
            Some((dyn_symbol, substitutions.clone()))
        }
        TyKind::SelfType => {
            // SelfType should be resolved to concrete type before member access
            None
        }
        _ => None,
    }
}

/// Get the symbol ID for a type (if it has one).
fn get_type_symbol_id(ty: &Ty) -> Option<SymbolId> {
    match ty.kind() {
        TyKind::Struct { symbol, .. } => Some(symbol.metadata().id()),
        TyKind::Protocol { symbol, .. } => Some(symbol.metadata().id()),
        TyKind::TypeAlias { symbol, .. } => Some(symbol.metadata().id()),
        _ => None,
    }
}

/// Resolve an associated type from a protocol.
fn resolve_associated_type_from_protocol(
    protocol: &Arc<ProtocolSymbol>,
    assoc_name: &str,
    substitutions: &Substitutions,
) -> Option<Ty> {
    // Look for associated type declaration in protocol children
    for child in protocol.metadata().children() {
        if child.metadata().kind() == KestrelSymbolKind::AssociatedType
            && child.metadata().name().value == assoc_name
        {
            // Found the associated type - check for default
            if let Ok(assoc) =
                child.downcast_arc::<kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol>()
            {
                if let Some(default_ty) = assoc.default_type() {
                    return Some(default_ty.apply_substitutions(substitutions));
                }
            }
            // No default - return the associated type itself
            // (caller should handle this based on context)
            return None;
        }
    }

    // Check inherited protocols
    if let Some(conformances) = protocol.metadata().get_behavior::<ConformancesBehavior>() {
        for parent_proto_ty in conformances.conformances() {
            if let TyKind::Protocol {
                symbol: parent,
                substitutions: parent_subs,
            } = parent_proto_ty.kind()
            {
                // Combine substitutions
                let combined_subs = combine_substitutions(substitutions, parent_subs);
                if let Some(ty) =
                    resolve_associated_type_from_protocol(parent, assoc_name, &combined_subs)
                {
                    return Some(ty);
                }
            }
        }
    }

    None
}

/// Combine two substitution maps, with the first taking precedence.
fn combine_substitutions(outer: &Substitutions, inner: &Substitutions) -> Substitutions {
    let mut result = inner.clone();
    for (id, ty) in outer.iter() {
        result.insert(*id, ty.clone());
    }
    result
}

/// Get the substitutions from a type (struct or enum).
fn get_type_substitutions(ty: &Ty) -> Option<Substitutions> {
    match ty.kind() {
        TyKind::Struct { substitutions, .. } => Some(substitutions.clone()),
        TyKind::Enum { substitutions, .. } => Some(substitutions.clone()),
        _ => None,
    }
}

/// Filter extensions to find those applicable to the given type's substitutions.
///
/// This is similar to `filter_applicable_extensions` in members.rs but simplified
/// for the conformance checking use case.
fn filter_applicable_extensions_for_conformance<'a>(
    extensions: &'a [Arc<ExtensionSymbol>],
    actual_subs: &Option<Substitutions>,
) -> Vec<&'a Arc<ExtensionSymbol>> {
    extensions
        .iter()
        .filter(|ext| {
            // Get the extension's target type behavior
            let behaviors = ext.metadata().behaviors();
            let target_behavior = behaviors
                .iter()
                .find(|b| b.kind() == KestrelBehaviorKind::ExtensionTarget);

            let Some(target_behavior) = target_behavior else {
                // No target behavior - include extension (shouldn't happen)
                return true;
            };

            let Some(target_behavior) = target_behavior
                .as_ref()
                .downcast_ref::<ExtensionTargetBehavior>()
            else {
                return true;
            };

            let target_ty = target_behavior.target_type();

            // Get substitutions from extension's target type
            let extension_subs = match target_ty.kind() {
                TyKind::Struct { substitutions, .. } => Some(substitutions),
                TyKind::Enum { substitutions, .. } => Some(substitutions),
                _ => None,
            };

            // If extension has no type arguments, it applies to all instances
            let Some(extension_subs) = extension_subs else {
                return true;
            };

            // If actual type has no substitutions but extension does, check if extension is fully generic
            let Some(actual_subs) = actual_subs else {
                // No actual subs - extension is applicable if it's fully generic
                return extension_subs.iter().all(|(_, ty)| ty.is_type_parameter());
            };

            // Check if extension's type arguments are applicable
            is_extension_applicable_for_conformance(extension_subs, actual_subs)
        })
        .collect()
}

/// Check if an extension's type arguments are applicable to an actual type's substitutions.
///
/// - Type parameters in the extension match any concrete type
/// - Concrete types in the extension must match exactly
fn is_extension_applicable_for_conformance(
    extension_subs: &Substitutions,
    actual_subs: &Substitutions,
) -> bool {
    // Must have same number of type arguments
    if extension_subs.len() != actual_subs.len() {
        return false;
    }

    // If both have no type arguments, they match
    if extension_subs.is_empty() && actual_subs.is_empty() {
        return true;
    }

    // Check each type argument by parameter ID
    for (param_id, ext_ty) in extension_subs.iter() {
        // Get the corresponding actual type for this parameter
        let Some(actual_ty) = actual_subs.get(*param_id) else {
            return false; // Extension has a param that actual doesn't
        };

        if ext_ty.is_type_parameter() {
            // Extension has a type parameter here - matches anything
            continue;
        } else {
            // Extension has a concrete type - must match exactly
            if !types_match_for_conformance(ext_ty, actual_ty) {
                return false;
            }
        }
    }

    true
}

/// Simple type matching for conformance checking.
///
/// Compares types structurally at the top level.
fn types_match_for_conformance(a: &Ty, b: &Ty) -> bool {
    match (a.kind(), b.kind()) {
        // Primitives
        (TyKind::Unit, TyKind::Unit) => true,
        (TyKind::Never, TyKind::Never) => true,
        (TyKind::Bool, TyKind::Bool) => true,
        (TyKind::String, TyKind::String) => true,
        (TyKind::Int(a_bits), TyKind::Int(b_bits)) => a_bits == b_bits,
        (TyKind::Float(a_bits), TyKind::Float(b_bits)) => a_bits == b_bits,

        // Structs - compare by symbol ID and recursively check substitutions
        (
            TyKind::Struct {
                symbol: a_sym,
                substitutions: a_subs,
            },
            TyKind::Struct {
                symbol: b_sym,
                substitutions: b_subs,
            },
        ) => {
            if a_sym.metadata().id() != b_sym.metadata().id() {
                return false;
            }
            // Recursively check substitutions
            for (param_id, a_ty) in a_subs.iter() {
                if let Some(b_ty) = b_subs.get(*param_id) {
                    if !types_match_for_conformance(a_ty, b_ty) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        }

        // Enums - compare by symbol ID and recursively check substitutions
        (
            TyKind::Enum {
                symbol: a_sym,
                substitutions: a_subs,
            },
            TyKind::Enum {
                symbol: b_sym,
                substitutions: b_subs,
            },
        ) => {
            if a_sym.metadata().id() != b_sym.metadata().id() {
                return false;
            }
            for (param_id, a_ty) in a_subs.iter() {
                if let Some(b_ty) = b_subs.get(*param_id) {
                    if !types_match_for_conformance(a_ty, b_ty) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        }

        // Type parameters - compare by symbol ID
        (TyKind::TypeParameter(a_param), TyKind::TypeParameter(b_param)) => {
            a_param.metadata().id() == b_param.metadata().id()
        }

        // Error types match anything
        (TyKind::Error, _) | (_, TyKind::Error) => true,

        // Different kinds don't match
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    // Tests would require setting up a full SemanticModel, which is complex.
    // The TypeOracle implementation is tested indirectly through the type inference tests.
}
