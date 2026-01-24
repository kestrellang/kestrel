//! TypeOracle implementation for SemanticModel.
//!
//! This module implements the `TypeOracle` trait from `kestrel-semantic-type-inference`,
//! allowing the type inference solver to query type information from the semantic model.

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::behavior::member_access::MemberAccessBehavior;
use kestrel_semantic_tree::builtins::{BuiltinKind, LanguageFeature};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Substitutions, Ty, TyKind, WhereClause};
use kestrel_semantic_type_inference::{MemberError, MemberResolution, TypeOracle};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::{ConformancesForSymbol, ExtensionsFor, ResolvedAliasedType};

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

        // Handle type parameters - look up member in protocol bounds from where clause
        if let TyKind::TypeParameter(type_param) = receiver_ty.kind() {
            // Get bounds by walking up the parent chain to find where clauses
            let bounds = get_type_parameter_bounds(type_param);

            // If any bound is an error type, the type parameter's constraints couldn't be resolved.
            // Return UnknownType to suppress cascading error messages.
            if bounds.iter().any(|b| matches!(b.kind(), TyKind::Error)) {
                return Err(MemberError::UnknownType);
            }

            for bound in &bounds {
                if let TyKind::Protocol {
                    symbol: proto,
                    substitutions: proto_subs,
                } = bound.kind()
                {
                    // Collect the protocol and all its inherited protocols
                    // E.g., for Comparable, this returns [Comparable, Equatable]
                    let all_protocols = collect_protocols_with_inherited(proto, proto_subs);

                    for (current_proto, current_subs) in &all_protocols {
                        // Check protocol's direct members
                        for child in current_proto.metadata().children() {
                            if child.metadata().name().value == member {
                                let member_id = child.metadata().id();
                                if let Some(callable) =
                                    child.metadata().get_behavior::<CallableBehavior>()
                                {
                                    // Substitute type parameters and Self (Self = the type parameter)
                                    let raw_return_ty =
                                        callable.return_type().apply_substitutions(current_subs);
                                    let returns_self =
                                        matches!(raw_return_ty.kind(), TyKind::SelfType);
                                    let return_ty = raw_return_ty.substitute_self(receiver_ty);
                                    let parameters: Vec<Ty> = callable
                                        .parameters()
                                        .iter()
                                        .map(|p| {
                                            p.ty.apply_substitutions(current_subs)
                                                .substitute_self(receiver_ty)
                                        })
                                        .collect();
                                    return Ok(MemberResolution {
                                        ty: return_ty,
                                        symbol_id: member_id,
                                        substitutions: current_subs.clone(),
                                        parameters,
                                        returns_self,
                                    });
                                }
                            }
                        }

                        // Check extensions on this protocol
                        let current_proto_id = current_proto.metadata().id();
                        let proto_extensions = self.query(ExtensionsFor {
                            target_id: current_proto_id,
                        });

                        for ext in &proto_extensions {
                            for child in ext.metadata().children() {
                                if child.metadata().name().value == member {
                                    let member_id = child.metadata().id();
                                    if let Some(callable) =
                                        child.metadata().get_behavior::<CallableBehavior>()
                                    {
                                        // Substitute type parameters and Self (Self = the type parameter)
                                        let raw_return_ty = callable
                                            .return_type()
                                            .apply_substitutions(current_subs);
                                        let returns_self =
                                            matches!(raw_return_ty.kind(), TyKind::SelfType);
                                        let return_ty = raw_return_ty.substitute_self(receiver_ty);
                                        let parameters: Vec<Ty> = callable
                                            .parameters()
                                            .iter()
                                            .map(|p| {
                                                p.ty.apply_substitutions(current_subs)
                                                    .substitute_self(receiver_ty)
                                            })
                                            .collect();
                                        return Ok(MemberResolution {
                                            ty: return_ty,
                                            symbol_id: member_id,
                                            substitutions: current_subs.clone(),
                                            parameters,
                                            returns_self,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Member not found in any protocol bound
            return Err(MemberError::NotFound {
                receiver_ty: receiver_ty.clone(),
                member: member.to_string(),
            });
        }

        // Handle SelfType - we can't resolve it in this context (no function context available).
        // SelfType should ideally be resolved to a concrete type before reaching the type oracle,
        // but if it wasn't, return UnknownType to suppress cascading error messages.
        if matches!(receiver_ty.kind(), TyKind::SelfType) {
            return Err(MemberError::UnknownType);
        }

        // Get the container symbol and substitutions
        let (container, substitutions) = match get_type_container_with_subs(receiver_ty) {
            Some(c) => c,
            None => {
                return Err(MemberError::NotFound {
                    receiver_ty: receiver_ty.clone(),
                    member: member.to_string(),
                });
            },
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

                // Find in extensions on the type itself
                let extension_member = extensions
                    .iter()
                    .flat_map(|ext| ext.metadata().children())
                    .find(|child| child.metadata().name().value == member);

                match extension_member {
                    Some(m) => m,
                    None => {
                        // Not found in direct extensions - check extensions on conforming protocols
                        // e.g., Int64 conforms to Comparable, and `extend Comparable: Less[Self]`
                        // provides `lessThan` method
                        let protocol_ids = self.protocol_conformance_ids_for_type(receiver_ty);

                        let mut found_member = None;
                        for proto_id in protocol_ids {
                            let proto_extensions = self.query(ExtensionsFor {
                                target_id: proto_id,
                            });

                            // Search extensions on this protocol
                            for ext in &proto_extensions {
                                for child in ext.metadata().children() {
                                    if child.metadata().name().value == member {
                                        found_member = Some(child);
                                        break;
                                    }
                                }
                                if found_member.is_some() {
                                    break;
                                }
                            }
                            if found_member.is_some() {
                                break;
                            }
                        }

                        match found_member {
                            Some(m) => m,
                            None => {
                                return Err(MemberError::NotFound {
                                    receiver_ty: receiver_ty.clone(),
                                    member: member.to_string(),
                                });
                            },
                        }
                    },
                }
            },
        };

        let member_id = member_symbol.metadata().id();
        let member_kind = member_symbol.metadata().kind();

        // Handle static vs instance access
        if is_static {
            // Static access - should be a function with no receiver
            if member_kind == KestrelSymbolKind::Function
                && let Some(callable) = member_symbol.metadata().get_behavior::<CallableBehavior>()
                && callable.is_static()
            {
                // Substitute both type parameters and Self with the receiver type
                let raw_return_ty = callable.return_type().apply_substitutions(&substitutions);
                let returns_self = matches!(raw_return_ty.kind(), TyKind::SelfType);
                let return_ty = raw_return_ty.substitute_self(receiver_ty);
                let parameters: Vec<Ty> = callable
                    .parameters()
                    .iter()
                    .map(|p| {
                        p.ty.apply_substitutions(&substitutions)
                            .substitute_self(receiver_ty)
                    })
                    .collect();
                return Ok(MemberResolution {
                    ty: return_ty,
                    symbol_id: member_id,
                    substitutions,
                    parameters,
                    returns_self,
                });
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
            if behavior.kind() == KestrelBehaviorKind::MemberAccess
                && let Some(access) = behavior.as_ref().downcast_ref::<MemberAccessBehavior>()
            {
                // Substitute type parameters and Self with the receiver type
                let member_ty = access
                    .member_type()
                    .apply_substitutions(&substitutions)
                    .substitute_self(receiver_ty);
                // Resolve any qualified associated types (e.g., String.Output → String)
                let member_ty = resolve_all_associated_types(self, &member_ty);
                return Ok(MemberResolution {
                    ty: member_ty,
                    symbol_id: member_id,
                    substitutions,
                    parameters: vec![],  // field access has no parameters
                    returns_self: false, // field access, not a method call
                });
            }
        }

        // Check for method access
        if member_kind == KestrelSymbolKind::Function
            && let Some(callable) = member_symbol.metadata().get_behavior::<CallableBehavior>()
        {
            // For methods, return the return type and parameter types
            // Substitute both type parameters and Self with the receiver type
            let raw_return_ty = callable.return_type().apply_substitutions(&substitutions);
            let returns_self = matches!(raw_return_ty.kind(), TyKind::SelfType);
            let return_ty = raw_return_ty.substitute_self(receiver_ty);
            // Resolve any qualified associated types (e.g., String.Output → String)
            let return_ty = resolve_all_associated_types(self, &return_ty);
            let parameters: Vec<Ty> = callable
                .parameters()
                .iter()
                .map(|p| {
                    let param_ty =
                        p.ty.apply_substitutions(&substitutions)
                            .substitute_self(receiver_ty);
                    resolve_all_associated_types(self, &param_ty)
                })
                .collect();
            return Ok(MemberResolution {
                ty: return_ty,
                symbol_id: member_id,
                substitutions,
                parameters,
                returns_self,
            });
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

        // Handle never type - the bottom type conforms to any protocol
        if matches!(ty.kind(), TyKind::Never) {
            return true;
        }

        // Handle type parameters - check if any bound matches the protocol
        if let TyKind::TypeParameter(type_param) = ty.kind() {
            let bounds = get_type_parameter_bounds(type_param);
            let mut seed_protocols = Vec::new();
            for bound in bounds {
                if let TyKind::Protocol { symbol, .. } = bound.kind() {
                    let bound_id = symbol.metadata().id();
                    // Check if this bound is exactly the protocol we're looking for
                    if bound_id == protocol_id {
                        return true;
                    }
                    seed_protocols.push(bound_id);
                }
            }
            // Check transitive conformance through protocol extensions on the bounds.
            if !seed_protocols.is_empty() {
                let reachable = collect_protocol_ids_via_extensions(self, seed_protocols);
                return reachable.contains(&protocol_id);
            }
            return false;
        }

        // Expand type aliases before checking conformance.
        // e.g., `Int` is a type alias for `Int64`, so we need to check `Int64`'s conformances.
        let ty = &ty.expand_aliases();

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

        // Handle FFISafe conformance for primitive machine types.
        //
        // This is primarily used by @extern(.C) validation and by the built-in
        // "all fields must conform" rule for FFISafe structs.
        //
        // Note: `Pointer` here refers to the primitive `lang.ptr[T]` type, not the
        // stdlib `std.memory.Pointer[T]` struct (which conforms via an extension).
        if let Some(ffi_safe_id) = self.builtin_protocol(LanguageFeature::FFISafe)
            && protocol_id == ffi_safe_id
        {
            match ty.kind() {
                TyKind::Int(_) | TyKind::Float(_) | TyKind::Bool | TyKind::String => {
                    return true;
                },
                TyKind::Pointer(pointee) => {
                    // A pointer is FFI-safe if the pointee type is FFI-safe.
                    // (If the pointee is an error type, we treat it as conforming
                    // to suppress cascading diagnostics.)
                    return self.conforms_to(pointee, protocol_id);
                },
                _ => {},
            }
        }

        // Handle primitive types - they implicitly conform to their literal protocols
        match ty.kind() {
            TyKind::Int(_) => {
                // Primitive ints implicitly conform to ExpressibleByIntLiteral
                if let Some(lit_protocol_id) =
                    self.builtin_protocol(LanguageFeature::ExpressibleByIntLiteral)
                    && protocol_id == lit_protocol_id
                {
                    return true;
                }
                // Primitives don't conform to any other protocols
                return false;
            },
            TyKind::Float(_) => {
                // Primitive floats implicitly conform to ExpressibleByFloatLiteral
                if let Some(lit_protocol_id) =
                    self.builtin_protocol(LanguageFeature::ExpressibleByFloatLiteral)
                    && protocol_id == lit_protocol_id
                {
                    return true;
                }
                // Also ExpressibleByIntLiteral since floats can be created from int literals
                if let Some(lit_protocol_id) =
                    self.builtin_protocol(LanguageFeature::ExpressibleByIntLiteral)
                    && protocol_id == lit_protocol_id
                {
                    return true;
                }
                // Primitives don't conform to any other protocols
                return false;
            },
            TyKind::Bool => {
                // Primitive bool implicitly conforms to ExpressibleByBoolLiteral
                if let Some(lit_protocol_id) =
                    self.builtin_protocol(LanguageFeature::ExpressibleByBoolLiteral)
                    && protocol_id == lit_protocol_id
                {
                    return true;
                }
                return false;
            },
            TyKind::String => {
                // Primitive string implicitly conforms to ExpressibleByStringLiteral
                if let Some(lit_protocol_id) =
                    self.builtin_protocol(LanguageFeature::ExpressibleByStringLiteral)
                    && protocol_id == lit_protocol_id
                {
                    return true;
                }
                return false;
            },
            _ => {},
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
            if let TyKind::Protocol { symbol, .. } = conformance.kind()
                && symbol.metadata().id() == protocol_id
            {
                return true;
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

        for extension in &applicable_extensions {
            let ext_conformances = self.query(ConformancesForSymbol {
                symbol_id: extension.metadata().id(),
            });

            for conformance in ext_conformances {
                if let TyKind::Protocol { symbol, .. } = conformance.kind()
                    && symbol.metadata().id() == protocol_id
                {
                    return true;
                }
            }
        }

        // Check transitive conformance through protocol extensions.
        // If ty conforms to protocol P, and there's "extend P: Q[...]", then ty conforms to Q[...].
        //
        // Example: Int64 conforms to Comparable, and there's "extend Comparable: Less[Self]"
        // So Int64 transitively conforms to Less[Int64] (Self substituted with Int64)
        if self.check_transitive_conformance(
            ty,
            protocol_id,
            type_symbol_id,
            &applicable_extensions,
        ) {
            return true;
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
                        && let Ok(type_alias) = child.downcast_arc::<TypeAliasSymbol>()
                        && let Some(resolved) = self.query(ResolvedAliasedType {
                            type_alias_id: type_alias.metadata().id(),
                        })
                    {
                        return Some(resolved.apply_substitutions(substitutions));
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
                        && let Some(ty) =
                            resolve_associated_type_from_protocol(proto, assoc_name, proto_subs)
                    {
                        return Some(ty.apply_substitutions(substitutions));
                    }
                }

                // Check extensions for associated type bindings
                // (e.g., `extend Maker: Factory { type Product = Int }`)
                let extensions = self.query(ExtensionsFor {
                    target_id: symbol.metadata().id(),
                });

                let applicable_extensions = filter_applicable_extensions_for_conformance(
                    &extensions,
                    &Some(substitutions.clone()),
                );

                for extension in applicable_extensions {
                    // Look for a type alias in the extension
                    for child in extension.metadata().children() {
                        if child.metadata().kind() == KestrelSymbolKind::TypeAlias
                            && child.metadata().name().value == assoc_name
                            && let Ok(type_alias) = child.downcast_arc::<TypeAliasSymbol>()
                            && let Some(resolved) = self.query(ResolvedAliasedType {
                                type_alias_id: type_alias.metadata().id(),
                            })
                        {
                            return Some(resolved.apply_substitutions(substitutions));
                        }
                    }
                }

                None
            },

            // For protocol types, look for associated type declaration
            TyKind::Protocol {
                symbol,
                substitutions,
            } => resolve_associated_type_from_protocol(symbol, assoc_name, substitutions),

            // For type parameters, look up in bounds from where clause
            TyKind::TypeParameter(type_param) => {
                // Get bounds by walking up the parent chain to find where clauses
                let bounds = get_type_parameter_bounds(type_param);

                for bound in &bounds {
                    if let TyKind::Protocol {
                        symbol,
                        substitutions,
                    } = bound.kind()
                        && let Some(ty) =
                            resolve_associated_type_from_protocol(symbol, assoc_name, substitutions)
                    {
                        return Some(ty);
                    }
                }
                None
            },

            // For associated types themselves, we might need to resolve nested projections
            TyKind::AssociatedType { container, .. } => {
                // First resolve the container's associated type, then look for nested
                if let Some(resolved_container) = container {
                    self.resolve_associated_type(resolved_container, assoc_name)
                } else {
                    None
                }
            },

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

    fn builtin_protocol(
        &self,
        feature: kestrel_semantic_tree::builtins::LanguageFeature,
    ) -> Option<SymbolId> {
        self.builtin_registry().protocol(feature)
    }

    fn protocol_for_method(&self, method_id: SymbolId) -> Option<SymbolId> {
        let feature = self.builtin_registry().method_feature(method_id)?;
        let definition = feature.definition();
        if let BuiltinKind::ProtocolMethod { protocol_feature } = definition.kind {
            self.builtin_registry().protocol(protocol_feature)
        } else {
            None
        }
    }

    fn default_integer_type(&self, span: kestrel_span::Span) -> Ty {
        use kestrel_semantic_tree::builtins::LanguageFeature;
        use kestrel_semantic_tree::ty::IntBits;

        // Try to look up the DefaultIntegerLiteralType type alias
        if let Some(type_alias_id) = self
            .builtin_registry()
            .type_alias(LanguageFeature::DefaultIntegerLiteralType)
            && let Some(resolved) = self.query(ResolvedAliasedType { type_alias_id })
        {
            return resolved;
        }

        // Fall back to default Int64
        Ty::int(IntBits::I64, span)
    }

    fn default_float_type(&self, span: kestrel_span::Span) -> Ty {
        use kestrel_semantic_tree::builtins::LanguageFeature;
        use kestrel_semantic_tree::ty::FloatBits;

        // Try to look up the DefaultFloatLiteralType type alias
        if let Some(type_alias_id) = self
            .builtin_registry()
            .type_alias(LanguageFeature::DefaultFloatLiteralType)
            && let Some(resolved) = self.query(ResolvedAliasedType { type_alias_id })
        {
            return resolved;
        }

        // Fall back to default Float64
        Ty::float(FloatBits::F64, span)
    }

    fn default_string_type(&self, span: kestrel_span::Span) -> Ty {
        use kestrel_semantic_tree::builtins::LanguageFeature;

        // Try to look up the DefaultStringLiteralType type alias
        if let Some(type_alias_id) = self
            .builtin_registry()
            .type_alias(LanguageFeature::DefaultStringLiteralType)
            && let Some(resolved) = self.query(ResolvedAliasedType { type_alias_id })
        {
            return resolved;
        }

        // Fall back to primitive string type
        Ty::string(span)
    }

    fn default_boolean_type(&self, span: kestrel_span::Span) -> Ty {
        use kestrel_semantic_tree::builtins::LanguageFeature;

        // Try to look up the DefaultBooleanLiteralType type alias
        if let Some(type_alias_id) = self
            .builtin_registry()
            .type_alias(LanguageFeature::DefaultBooleanLiteralType)
            && let Some(resolved) = self.query(ResolvedAliasedType { type_alias_id })
        {
            return resolved;
        }

        // Fall back to primitive bool type
        Ty::bool(span)
    }

    fn default_char_type(&self, span: kestrel_span::Span) -> Ty {
        use kestrel_semantic_tree::builtins::LanguageFeature;
        use kestrel_semantic_tree::ty::IntBits;

        // Try to look up the DefaultCharLiteralType type alias
        if let Some(type_alias_id) = self
            .builtin_registry()
            .type_alias(LanguageFeature::DefaultCharLiteralType)
            && let Some(resolved) = self.query(ResolvedAliasedType { type_alias_id })
        {
            return resolved;
        }

        // Fall back to i32
        Ty::int(IntBits::I32, span)
    }

    fn default_array_type(&self, element_ty: Ty, span: kestrel_span::Span) -> Option<Ty> {
        // Delegate to the existing make_array_type method on SemanticModel
        self.make_array_type(element_ty, span)
    }
}

impl SemanticModel {
    /// Get all protocol conformances for a concrete type, including conformances
    /// added via type extensions and transitive conformances from protocol extensions.
    pub fn protocol_conformance_ids_for_type(&self, ty: &Ty) -> Vec<SymbolId> {
        collect_protocol_conformance_ids_for_type(self, ty)
    }

    /// Check for transitive conformance through protocol extensions.
    ///
    /// This is a wrapper that initializes the visited set for cycle detection.
    fn check_transitive_conformance(
        &self,
        concrete_ty: &Ty,
        target_protocol_id: SymbolId,
        type_symbol_id: SymbolId,
        applicable_extensions: &[&Arc<ExtensionSymbol>],
    ) -> bool {
        let mut visited = std::collections::HashSet::new();
        check_transitive_conformance_impl(
            self,
            concrete_ty,
            target_protocol_id,
            type_symbol_id,
            applicable_extensions,
            &mut visited,
        )
    }
}

// ============================================================================
// ContextualOracle: Oracle with function context for extension bound lookup
// ============================================================================

use crate::queries::SymbolFor;

/// Oracle wrapper that has context about the current function being analyzed.
///
/// This allows extension where clause bounds to be discovered when resolving
/// members on type parameters. Without context, the oracle can only find bounds
/// from the type parameter's parent chain (struct/protocol), missing constraints
/// added by extensions like `extend Optional[T]: Equatable where T: Equatable`.
pub struct ContextualOracle<'a> {
    model: &'a SemanticModel,
    context_symbol_id: SymbolId,
}

impl<'a> ContextualOracle<'a> {
    /// Create a new contextual oracle.
    ///
    /// # Arguments
    /// * `model` - The semantic model
    /// * `context_symbol_id` - The function/initializer being analyzed
    pub fn new(model: &'a SemanticModel, context_symbol_id: SymbolId) -> Self {
        Self {
            model,
            context_symbol_id,
        }
    }
}

impl TypeOracle for ContextualOracle<'_> {
    fn resolve_member(
        &self,
        receiver_ty: &Ty,
        member: &str,
        is_static: bool,
    ) -> Result<MemberResolution, MemberError> {
        resolve_member_with_context(
            self.model,
            receiver_ty,
            member,
            is_static,
            Some(self.context_symbol_id),
        )
    }

    fn conforms_to(&self, ty: &Ty, protocol_id: SymbolId) -> bool {
        self.model.conforms_to(ty, protocol_id)
    }

    fn resolve_associated_type(&self, container: &Ty, assoc_name: &str) -> Option<Ty> {
        self.model.resolve_associated_type(container, assoc_name)
    }

    fn expand_type_alias(&self, ty: &Ty) -> Ty {
        self.model.expand_type_alias(ty)
    }

    fn symbol_name(&self, symbol_id: SymbolId) -> Option<String> {
        self.model.symbol_name(symbol_id)
    }

    fn builtin_protocol(&self, feature: LanguageFeature) -> Option<SymbolId> {
        self.model.builtin_protocol(feature)
    }

    fn protocol_for_method(&self, method_id: SymbolId) -> Option<SymbolId> {
        self.model.protocol_for_method(method_id)
    }

    fn default_integer_type(&self, span: kestrel_span::Span) -> Ty {
        self.model.default_integer_type(span)
    }

    fn default_float_type(&self, span: kestrel_span::Span) -> Ty {
        self.model.default_float_type(span)
    }

    fn default_string_type(&self, span: kestrel_span::Span) -> Ty {
        self.model.default_string_type(span)
    }

    fn default_boolean_type(&self, span: kestrel_span::Span) -> Ty {
        self.model.default_boolean_type(span)
    }

    fn default_char_type(&self, span: kestrel_span::Span) -> Ty {
        self.model.default_char_type(span)
    }

    fn default_array_type(&self, element_ty: Ty, span: kestrel_span::Span) -> Option<Ty> {
        self.model.default_array_type(element_ty, span)
    }
}

/// Resolve a member with optional context for extension bound lookup.
///
/// This is the context-aware version of resolve_member. When context is provided,
/// it can find additional bounds from extension where clauses.
fn resolve_member_with_context(
    model: &SemanticModel,
    receiver_ty: &Ty,
    member: &str,
    is_static: bool,
    context: Option<SymbolId>,
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

    // Handle type parameters - look up member in protocol bounds from where clause
    if let TyKind::TypeParameter(type_param) = receiver_ty.kind() {
        // Get bounds by walking up the parent chain to find where clauses
        let mut bounds = get_type_parameter_bounds(type_param);

        // If we have context, also check for extension bounds
        if let Some(ctx_id) = context
            && let Some(ext_bounds) =
                get_extension_bounds_for_param(model, ctx_id, type_param.metadata().id())
        {
            bounds.extend(ext_bounds);
        }

        // If any bound is an error type, the type parameter's constraints couldn't be resolved.
        // Return UnknownType to suppress cascading error messages.
        if bounds.iter().any(|b| matches!(b.kind(), TyKind::Error)) {
            return Err(MemberError::UnknownType);
        }

        for bound in &bounds {
            if let TyKind::Protocol {
                symbol: proto,
                substitutions: proto_subs,
            } = bound.kind()
            {
                // Collect the protocol and all its inherited protocols
                // E.g., for Comparable, this returns [Comparable, Equatable]
                let all_protocols = collect_protocols_with_inherited(proto, proto_subs);

                for (current_proto, current_subs) in &all_protocols {
                    // Check protocol's direct members
                    for child in current_proto.metadata().children() {
                        if child.metadata().name().value == member {
                            let member_id = child.metadata().id();
                            if let Some(callable) =
                                child.metadata().get_behavior::<CallableBehavior>()
                            {
                                // Substitute type parameters and Self (Self = the type parameter)
                                let raw_return_ty =
                                    callable.return_type().apply_substitutions(current_subs);
                                let returns_self = matches!(raw_return_ty.kind(), TyKind::SelfType);
                                let return_ty = raw_return_ty.substitute_self(receiver_ty);
                                let parameters: Vec<Ty> = callable
                                    .parameters()
                                    .iter()
                                    .map(|p| {
                                        p.ty.apply_substitutions(current_subs)
                                            .substitute_self(receiver_ty)
                                    })
                                    .collect();
                                return Ok(MemberResolution {
                                    ty: return_ty,
                                    symbol_id: member_id,
                                    substitutions: current_subs.clone(),
                                    parameters,
                                    returns_self,
                                });
                            }
                        }
                    }

                    // Check extensions on this protocol
                    let current_proto_id = current_proto.metadata().id();
                    let proto_extensions = model.query(ExtensionsFor {
                        target_id: current_proto_id,
                    });

                    for ext in &proto_extensions {
                        for child in ext.metadata().children() {
                            if child.metadata().name().value == member {
                                let member_id = child.metadata().id();
                                if let Some(callable) =
                                    child.metadata().get_behavior::<CallableBehavior>()
                                {
                                    // Substitute type parameters and Self (Self = the type parameter)
                                    let raw_return_ty =
                                        callable.return_type().apply_substitutions(current_subs);
                                    let returns_self =
                                        matches!(raw_return_ty.kind(), TyKind::SelfType);
                                    let return_ty = raw_return_ty.substitute_self(receiver_ty);
                                    let parameters: Vec<Ty> = callable
                                        .parameters()
                                        .iter()
                                        .map(|p| {
                                            p.ty.apply_substitutions(current_subs)
                                                .substitute_self(receiver_ty)
                                        })
                                        .collect();
                                    return Ok(MemberResolution {
                                        ty: return_ty,
                                        symbol_id: member_id,
                                        substitutions: current_subs.clone(),
                                        parameters,
                                        returns_self,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        // Member not found in any protocol bound
        return Err(MemberError::NotFound {
            receiver_ty: receiver_ty.clone(),
            member: member.to_string(),
        });
    }

    // For non-type-parameter types, delegate to the model's resolve_member
    // (which doesn't need context)
    model.resolve_member(receiver_ty, member, is_static)
}

/// Walk up from a function to find if it's inside an extension,
/// and if so, return any additional bounds from the extension's where clause.
fn get_extension_bounds_for_param(
    model: &SemanticModel,
    context_id: SymbolId,
    param_id: SymbolId,
) -> Option<Vec<Ty>> {
    let context = model.query(SymbolFor { id: context_id })?;
    let mut current: Option<Arc<dyn Symbol<KestrelLanguage>>> = Some(context);

    while let Some(sym) = current {
        if sym.metadata().kind() == KestrelSymbolKind::Extension {
            // Found extension - check its where clause for bounds on param_id
            if let Some(ext_target) = sym.metadata().get_behavior::<ExtensionTargetBehavior>() {
                let where_clause = ext_target.where_clause();
                let bounds: Vec<Ty> = where_clause
                    .bounds_for(param_id)
                    .into_iter()
                    .filter(|b| matches!(b.kind(), TyKind::Protocol { .. } | TyKind::Error))
                    .cloned()
                    .collect();
                if !bounds.is_empty() {
                    return Some(bounds);
                }
            }
        }
        current = sym.metadata().parent();
    }
    None
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
        },
        TyKind::Enum {
            symbol,
            substitutions,
        } => {
            let dyn_symbol: Arc<dyn Symbol<KestrelLanguage>> = symbol.clone();
            Some((dyn_symbol, substitutions.clone()))
        },
        TyKind::Protocol {
            symbol,
            substitutions,
        } => {
            // Clone substitutions and populate any missing type parameter defaults.
            // This is necessary for protocols like `Addable[Rhs = Self]` where the
            // Rhs parameter has a default value that must be included in substitutions
            // for method signature resolution to work correctly.
            let mut subs_with_defaults = substitutions.clone();
            for type_param in symbol.type_parameters() {
                let param_id = type_param.metadata().id();
                if !subs_with_defaults.contains(param_id) {
                    if let Some(default_ty) = type_param.default() {
                        subs_with_defaults.insert(param_id, default_ty.clone());
                    }
                }
            }
            let dyn_symbol: Arc<dyn Symbol<KestrelLanguage>> = symbol.clone();
            Some((dyn_symbol, subs_with_defaults))
        },
        TyKind::SelfType => {
            // SelfType is handled in resolve_member() before this function is called.
            // If we reach here, it means SelfType wasn't resolved and we can't proceed.
            None
        },
        _ => None,
    }
}

/// Get the symbol ID for a type (if it has one).
fn get_type_symbol_id(ty: &Ty) -> Option<SymbolId> {
    match ty.kind() {
        TyKind::Struct { symbol, .. } => Some(symbol.metadata().id()),
        TyKind::Enum { symbol, .. } => Some(symbol.metadata().id()),
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
                && let Some(default_ty) = assoc.default_type() {
                    return Some(default_ty.apply_substitutions(substitutions));
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

/// Check for transitive conformance through protocol extensions.
///
/// If the concrete type conforms to protocol P, and there's "extend P: Q[...]",
/// then the concrete type transitively conforms to Q (with appropriate substitutions).
///
/// Example: Int64 conforms to Comparable, and there's "extend Comparable: Less[Self]"
/// So Int64 transitively conforms to Less[Int64] (Self substituted with Int64)
fn check_transitive_conformance_impl(
    model: &SemanticModel,
    _concrete_ty: &Ty,
    target_protocol_id: SymbolId,
    type_symbol_id: SymbolId,
    applicable_extensions: &[&Arc<ExtensionSymbol>],
    visited: &mut std::collections::HashSet<SymbolId>,
) -> bool {
    // Collect all protocols that the type directly conforms to
    // (from direct declarations + extensions on the type)
    let mut all_conformances: Vec<Ty> = model.query(ConformancesForSymbol {
        symbol_id: type_symbol_id,
    });

    // Add conformances from applicable extensions on the type
    for extension in applicable_extensions {
        let ext_conformances = model.query(ConformancesForSymbol {
            symbol_id: extension.metadata().id(),
        });
        all_conformances.extend(ext_conformances);
    }

    // For each protocol the type conforms to, check if that protocol has
    // an extension that adds conformance to our target protocol
    for conformance in &all_conformances {
        if let TyKind::Protocol {
            symbol: conformed_protocol,
            substitutions: _conf_subs,
        } = conformance.kind()
        {
            let conformed_protocol_id = conformed_protocol.metadata().id();

            // Prevent infinite loops
            if !visited.insert(conformed_protocol_id) {
                continue;
            }

            // Get extensions on the conformed protocol (e.g., extensions on Comparable)
            let protocol_extensions = model.query(ExtensionsFor {
                target_id: conformed_protocol_id,
            });

            for ext in &protocol_extensions {
                let ext_conformances = model.query(ConformancesForSymbol {
                    symbol_id: ext.metadata().id(),
                });

                for ext_conf in &ext_conformances {
                    if let TyKind::Protocol {
                        symbol: ext_protocol,
                        substitutions: _ext_subs,
                    } = ext_conf.kind()
                        && ext_protocol.metadata().id() == target_protocol_id
                    {
                        // Found a match on the base protocol ID (e.g., Less)
                        // The protocol extension conformance uses Self to refer
                        // to the extended protocol, which we substitute with concrete_ty
                        //
                        // For "extend Comparable: Less[Self]":
                        // - ext_subs maps Less's Rhs param to Self
                        // - We substitute Self -> concrete_ty
                        // - Result: Less[concrete_ty]
                        //
                        // Since Less[Self] means Less[Self] where Self is the concrete type,
                        // and the protocol_id we're checking is just Less (not Less[SomeType]),
                        // we've already matched by protocol_id - we just need to verify the
                        // type arguments would work out correctly.
                        //
                        // For now, we do a simplified check: if the extension adds conformance
                        // to the target protocol, we accept it. Full generic matching would
                        // require comparing substitutions after Self resolution.
                        return true;
                    }
                }
            }

            // Also check transitively: if the conformed protocol itself has protocol extensions
            // that add conformance to other protocols, those might transitively lead to our target.
            // This handles chains like A: B, B: C, C: D
            for ext in &protocol_extensions {
                let ext_conformances = model.query(ConformancesForSymbol {
                    symbol_id: ext.metadata().id(),
                });

                for ext_conf in &ext_conformances {
                    if let TyKind::Protocol {
                        symbol: intermediate_protocol,
                        ..
                    } = ext_conf.kind()
                    {
                        let intermediate_id = intermediate_protocol.metadata().id();
                        if intermediate_id != target_protocol_id
                            && !visited.contains(&intermediate_id)
                        {
                            // Get extensions on this intermediate protocol and check recursively
                            let intermediate_extensions = model.query(ExtensionsFor {
                                target_id: intermediate_id,
                            });

                            // Check if this intermediate protocol has an extension to our target
                            for int_ext in &intermediate_extensions {
                                let int_ext_conformances = model.query(ConformancesForSymbol {
                                    symbol_id: int_ext.metadata().id(),
                                });

                                for int_conf in &int_ext_conformances {
                                    if let TyKind::Protocol { symbol, .. } = int_conf.kind()
                                        && symbol.metadata().id() == target_protocol_id
                                    {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    false
}

/// Collect protocol IDs reachable via protocol extensions, starting from a seed set.
///
/// This follows chains like `extend A: B` and `extend B: C`, returning `[A, B, C]`.
fn collect_protocol_ids_via_extensions<I>(model: &SemanticModel, seed_protocols: I) -> Vec<SymbolId>
where
    I: IntoIterator<Item = SymbolId>,
{
    let mut ordered = Vec::new();
    let mut seen: HashSet<SymbolId> = HashSet::new();
    let mut queue: VecDeque<SymbolId> = VecDeque::new();

    for id in seed_protocols {
        if seen.insert(id) {
            ordered.push(id);
            queue.push_back(id);
        }
    }

    while let Some(protocol_id) = queue.pop_front() {
        let protocol_extensions = model.query(ExtensionsFor {
            target_id: protocol_id,
        });

        for extension in &protocol_extensions {
            let Some(conformances) = extension.metadata().get_behavior::<ConformancesBehavior>()
            else {
                continue;
            };

            for conf_ty in conformances.conformances() {
                if let TyKind::Protocol { symbol, .. } = conf_ty.kind() {
                    let next_id = symbol.metadata().id();
                    if seen.insert(next_id) {
                        ordered.push(next_id);
                        queue.push_back(next_id);
                    }
                }
            }
        }
    }

    ordered
}

/// Collect protocol conformances for a concrete type, including:
/// - Direct conformances on the type
/// - Conformances from applicable type extensions
/// - Transitive conformances added by protocol extensions
fn collect_protocol_conformance_ids_for_type(model: &SemanticModel, ty: &Ty) -> Vec<SymbolId> {
    // Expand type aliases to their underlying type before checking conformances.
    let ty = ty.expand_aliases();

    let type_symbol_id = match get_type_symbol_id(&ty) {
        Some(id) => id,
        None => return Vec::new(),
    };

    let mut seed_protocols: Vec<SymbolId> = Vec::new();

    // Direct conformances on the type
    let mut conformances = model.query(ConformancesForSymbol {
        symbol_id: type_symbol_id,
    });

    // Conformances added via type extensions
    let actual_subs = get_type_substitutions(&ty);
    let extensions = model.query(ExtensionsFor {
        target_id: type_symbol_id,
    });
    let applicable_extensions =
        filter_applicable_extensions_for_conformance(&extensions, &actual_subs);

    for extension in &applicable_extensions {
        let ext_conformances = model.query(ConformancesForSymbol {
            symbol_id: extension.metadata().id(),
        });
        conformances.extend(ext_conformances);
    }

    for conformance in &conformances {
        if let TyKind::Protocol { symbol, .. } = conformance.kind() {
            seed_protocols.push(symbol.metadata().id());
        }
    }

    collect_protocol_ids_via_extensions(model, seed_protocols)
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
/// Expands type aliases before comparing to handle cases like `lang.i64` vs `Int64`.
fn types_match_for_conformance(a: &Ty, b: &Ty) -> bool {
    // Expand type aliases before comparing
    let a = a.expand_aliases();
    let b = b.expand_aliases();

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
        },

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
        },

        // Type parameters - compare by symbol ID
        (TyKind::TypeParameter(a_param), TyKind::TypeParameter(b_param)) => {
            a_param.metadata().id() == b_param.metadata().id()
        },

        // Error types match anything
        (TyKind::Error, _) | (_, TyKind::Error) => true,

        // Different kinds don't match
        _ => false,
    }
}

/// Collect a protocol and all its inherited protocols (transitive closure).
///
/// For example, if `Comparable: Equatable`, calling this with `Comparable` returns
/// `[Comparable, Equatable]` (with appropriate substitutions).
///
/// This is used when resolving members on type parameters - we need to search
/// all inherited protocols, not just the direct bounds.
fn collect_protocols_with_inherited(
    proto: &Arc<ProtocolSymbol>,
    subs: &Substitutions,
) -> Vec<(Arc<ProtocolSymbol>, Substitutions)> {
    let mut result = Vec::new();
    let mut visited = std::collections::HashSet::new();
    collect_protocols_with_inherited_impl(proto, subs, &mut result, &mut visited);
    result
}

/// Internal helper that recursively collects protocols while tracking visited protocols.
fn collect_protocols_with_inherited_impl(
    proto: &Arc<ProtocolSymbol>,
    subs: &Substitutions,
    result: &mut Vec<(Arc<ProtocolSymbol>, Substitutions)>,
    visited: &mut std::collections::HashSet<SymbolId>,
) {
    // Skip if already visited (handles cycles)
    if !visited.insert(proto.metadata().id()) {
        return;
    }

    // Add this protocol to results
    result.push((proto.clone(), subs.clone()));

    // Get inherited protocols from ConformancesBehavior
    if let Some(conformances) = proto.metadata().get_behavior::<ConformancesBehavior>() {
        for conformance in conformances.conformances() {
            if let TyKind::Protocol {
                symbol: inherited_proto,
                substitutions: inherited_subs,
            } = conformance.kind()
            {
                // Apply parent substitutions to inherited protocol's substitutions.
                // For each type in inherited_subs, apply subs to get the final type.
                let mut combined_subs = Substitutions::new();
                for (param_id, ty) in inherited_subs.iter() {
                    combined_subs.insert(*param_id, subs.apply(ty));
                }
                // Also copy over any subs from parent that aren't in inherited_subs
                for (param_id, ty) in subs.iter() {
                    if !combined_subs.contains(*param_id) {
                        combined_subs.insert(*param_id, ty.clone());
                    }
                }

                // Recursively collect from inherited protocol
                collect_protocols_with_inherited_impl(
                    inherited_proto,
                    &combined_subs,
                    result,
                    visited,
                );
            }
        }
    }
}

/// Get protocol bounds for a type parameter by walking up the parent chain.
///
/// This looks at the type parameter's parent symbols (function, struct, protocol)
/// and extracts bounds from their where clauses.
fn get_type_parameter_bounds(type_param: &Arc<TypeParameterSymbol>) -> Vec<Ty> {
    let param_id = type_param.metadata().id();
    let mut bounds = Vec::new();

    // Walk up from the type parameter's parent to find where clauses
    let mut current: Option<Arc<dyn Symbol<KestrelLanguage>>> = type_param.metadata().parent();

    while let Some(parent) = current {
        if let Some(where_clause) = get_where_clause_from_symbol(parent.as_ref()) {
            // Extract bounds for this parameter from the where clause
            for bound in where_clause.bounds_for(param_id) {
                // Include both Protocol bounds and Error bounds.
                // Error bounds indicate that protocol resolution failed - we include them
                // so the caller can detect and suppress cascading error messages.
                match bound.kind() {
                    TyKind::Protocol { .. } | TyKind::Error => {
                        bounds.push(bound.clone());
                    },
                    _ => {},
                }
            }
        }
        current = parent.metadata().parent();
    }

    bounds
}

/// Resolve all qualified associated types in a type to their concrete definitions.
///
/// For example, if `String` conforms to `Addable` with `type Output = String`, then:
/// - `String.Output` → `String`
///
/// This recursively walks the type structure, resolving any `AssociatedType` variants
/// that have a container (qualified associated types like `T.Item`).
fn resolve_all_associated_types(oracle: &impl TypeOracle, ty: &Ty) -> Ty {
    resolve_all_associated_types_impl(oracle, ty, &mut std::collections::HashSet::new())
}

/// Internal helper with cycle detection via visited TyIds.
fn resolve_all_associated_types_impl(
    oracle: &impl TypeOracle,
    ty: &Ty,
    visited: &mut std::collections::HashSet<kestrel_semantic_tree::ty::TyId>,
) -> Ty {
    use kestrel_semantic_tree::ty::ParamInfo;

    // Cycle detection: if we've already visited this type, return it as-is
    let ty_id = ty.id();
    if !visited.insert(ty_id) {
        return ty.clone();
    }

    let result = match ty.kind() {
        // The key case: qualified associated type (e.g., String.Output)
        TyKind::AssociatedType {
            symbol,
            container: Some(container),
        } => {
            // First resolve any associated types in the container itself
            let resolved_container = resolve_all_associated_types_impl(oracle, container, visited);
            let name = symbol.metadata().name().value.clone();

            // Try to resolve the associated type using the oracle
            if let Some(resolved) = oracle.resolve_associated_type(&resolved_container, &name) {
                // Recursively resolve in case the result also has associated types
                resolve_all_associated_types_impl(oracle, &resolved, visited)
            } else {
                // Can't resolve - return the type with the resolved container
                Ty::qualified_associated_type(symbol.clone(), resolved_container, ty.span().clone())
            }
        },

        // Unqualified associated type (container: None) - leave as-is
        // This shouldn't appear after substitute_self, but handle it gracefully
        TyKind::AssociatedType { .. } => ty.clone(),

        // Compound types - recurse into nested types
        TyKind::Tuple(elements) => {
            let new_elements: Vec<Ty> = elements
                .iter()
                .map(|e| resolve_all_associated_types_impl(oracle, e, visited))
                .collect();
            Ty::tuple(new_elements, ty.span().clone())
        },

        // Note: Array[T] struct types are handled by the Struct case below
        TyKind::Pointer(element) => {
            let new_element = resolve_all_associated_types_impl(oracle, element, visited);
            Ty::pointer(new_element, ty.span().clone())
        },

        TyKind::Function {
            params,
            return_type,
        } => {
            let new_params: Vec<Ty> = params
                .iter()
                .map(|p| resolve_all_associated_types_impl(oracle, p, visited))
                .collect();
            let new_return = resolve_all_associated_types_impl(oracle, return_type, visited);
            Ty::function(new_params, new_return, ty.span().clone())
        },

        TyKind::Struct {
            symbol,
            substitutions,
        } => {
            let mut new_subs = Substitutions::new();
            for (id, sub_ty) in substitutions.iter() {
                new_subs.insert(
                    *id,
                    resolve_all_associated_types_impl(oracle, sub_ty, visited),
                );
            }
            Ty::generic_struct(symbol.clone(), new_subs, ty.span().clone())
        },

        TyKind::Enum {
            symbol,
            substitutions,
        } => {
            let mut new_subs = Substitutions::new();
            for (id, sub_ty) in substitutions.iter() {
                new_subs.insert(
                    *id,
                    resolve_all_associated_types_impl(oracle, sub_ty, visited),
                );
            }
            Ty::generic_enum(symbol.clone(), new_subs, ty.span().clone())
        },

        // Don't recurse into protocol substitutions - protocols may have cyclic inheritance
        // and their substitutions shouldn't contain associated types that need resolution
        TyKind::Protocol { .. } => ty.clone(),

        TyKind::TypeAlias {
            symbol,
            substitutions,
        } => {
            let mut new_subs = Substitutions::new();
            for (id, sub_ty) in substitutions.iter() {
                new_subs.insert(
                    *id,
                    resolve_all_associated_types_impl(oracle, sub_ty, visited),
                );
            }
            Ty::generic_type_alias(symbol.clone(), new_subs, ty.span().clone())
        },

        TyKind::UnresolvedFunction {
            param_info,
            return_type,
        } => {
            let new_return = resolve_all_associated_types_impl(oracle, return_type, visited);
            let new_param_info = match param_info {
                ParamInfo::Unconstrained => ParamInfo::Unconstrained,
                ParamInfo::ImplicitIt { it_type } => ParamInfo::ImplicitIt {
                    it_type: Box::new(resolve_all_associated_types_impl(oracle, it_type, visited)),
                },
                ParamInfo::Explicit { param_types } => ParamInfo::Explicit {
                    param_types: param_types
                        .iter()
                        .map(|p| resolve_all_associated_types_impl(oracle, p, visited))
                        .collect(),
                },
            };
            Ty::unresolved_function(new_param_info, new_return, ty.span().clone())
        },

        // Primitive types and special types - no nested types to resolve
        TyKind::Unit
        | TyKind::Never
        | TyKind::Int(_)
        | TyKind::Float(_)
        | TyKind::Bool
        | TyKind::String
        | TyKind::Error
        | TyKind::SelfType
        | TyKind::Infer
        | TyKind::TypeParameter(_) => ty.clone(),
    };

    // Remove from visited after processing so we can visit the same type again
    // from a different path (this is not truly cycle detection, but depth limiting)
    visited.remove(&ty_id);
    result
}

/// Get the where clause from a symbol that can have one.
///
/// Supports FunctionSymbol, InitializerSymbol, StructSymbol, ProtocolSymbol, and ExtensionSymbol.
fn get_where_clause_from_symbol(symbol: &dyn Symbol<KestrelLanguage>) -> Option<WhereClause> {
    // Try FunctionSymbol
    if let Some(func) = symbol.as_any().downcast_ref::<FunctionSymbol>() {
        return Some(func.where_clause());
    }
    // Try InitializerSymbol
    if let Some(init) = symbol.as_any().downcast_ref::<InitializerSymbol>() {
        return Some(init.where_clause());
    }
    // Try StructSymbol
    if let Some(struc) = symbol.as_any().downcast_ref::<StructSymbol>() {
        return Some(struc.where_clause().clone());
    }
    // Try ProtocolSymbol
    if let Some(proto) = symbol.as_any().downcast_ref::<ProtocolSymbol>() {
        return Some(proto.where_clause().clone());
    }
    // Try ExtensionSymbol
    if let Some(ext) = symbol.as_any().downcast_ref::<ExtensionSymbol>() {
        return Some(ext.where_clause());
    }
    // Try SubscriptSymbol - get where clause from GenericsBehavior
    if symbol.metadata().kind() == KestrelSymbolKind::Subscript {
        if let Some(generics) = symbol.metadata().get_behavior::<GenericsBehavior>() {
            return Some(generics.where_clause().clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    // Tests would require setting up a full SemanticModel, which is complex.
    // The TypeOracle implementation is tested indirectly through the type inference tests.
}
