//! TypeOracle implementation for SemanticModel.
//!
//! This module implements the `TypeOracle` trait from `kestrel-semantic-type-inference`,
//! allowing the type inference solver to query type information from the semantic model.

use std::sync::Arc;

use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::member_access::MemberAccessBehavior;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::language::KestrelLanguage;
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
                if let Some(callable) = member_symbol
                    .metadata()
                    .get_behavior::<CallableBehavior>()
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
            if let Some(callable) = member_symbol
                .metadata()
                .get_behavior::<CallableBehavior>()
            {
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
        let extensions = self.query(ExtensionsFor {
            target_id: type_symbol_id,
        });

        for extension in extensions {
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
                if let Some(conformances) = type_param
                    .metadata()
                    .get_behavior::<ConformancesBehavior>()
                {
                    for bound in conformances.conformances() {
                        if let TyKind::Protocol {
                            symbol,
                            substitutions,
                        } = bound.kind()
                        {
                            if let Some(ty) =
                                resolve_associated_type_from_protocol(symbol, assoc_name, substitutions)
                            {
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
}

/// Get the container symbol and substitutions from a type.
fn get_type_container_with_subs(ty: &Ty) -> Option<(Arc<dyn Symbol<KestrelLanguage>>, Substitutions)> {
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
                if let Some(ty) = resolve_associated_type_from_protocol(parent, assoc_name, &combined_subs)
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

#[cfg(test)]
mod tests {
    // Tests would require setting up a full SemanticModel, which is complex.
    // The TypeOracle implementation is tested indirectly through the type inference tests.
}
