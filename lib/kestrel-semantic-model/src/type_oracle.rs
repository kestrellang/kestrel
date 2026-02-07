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
use kestrel_semantic_tree::behavior::computed_member_access::ComputedMemberAccessBehavior;
use kestrel_semantic_tree::behavior::member_access::MemberAccessBehavior;
use kestrel_semantic_tree::builtins::{BuiltinKind, LanguageFeature};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol;
use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Constraint, Substitutions, Ty, TyId, TyKind, WhereClause};
use kestrel_semantic_type_inference::{MemberError, MemberResolution, TypeOracle};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::{ConformancesForSymbol, ExtensionsFor, ResolvedAliasedType, SymbolFor};

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
                // Apply protocol defaults with Self = receiver_ty (the type parameter)
                // This resolves defaults like `Rhs = Self` in `Equal[Rhs = Self]`
                let bound = apply_protocol_defaults(bound.clone(), Some(receiver_ty));
                if let TyKind::Protocol {
                    symbol: proto,
                    substitutions: proto_subs,
                } = bound.kind()
                {
                    // Collect the protocol and all its inherited protocols
                    // E.g., for Comparable, this returns [Comparable, Equatable]
                    let all_protocols = collect_protocols_with_inherited(proto, proto_subs, Some(receiver_ty));

                    for (proto, proto_subs) in &all_protocols {
                        // Check protocol's direct members
                        for child in proto.metadata().children() {
                            if child.metadata().name().value == member {
                                let member_id = child.metadata().id();
                                if let Some(callable) =
                                    child.metadata().get_behavior::<CallableBehavior>()
                                {
                                    // Substitute type parameters and Self (Self = the type parameter)
                                    let raw_return_ty =
                                        callable.return_type().apply_substitutions(proto_subs);
                                    let returns_self =
                                        matches!(raw_return_ty.kind(), TyKind::SelfType);
                                    let return_ty = raw_return_ty.substitute_self(receiver_ty);
                                    let parameters: Vec<Ty> = callable
                                        .parameters()
                                        .iter()
                                        .map(|p| {
                                            p.ty.apply_substitutions(proto_subs)
                                                .substitute_self(receiver_ty)
                                        })
                                        .collect();
                                    let where_constraints = get_where_clause_from_symbol(child.as_ref()).unwrap_or_default();
                                    return Ok(MemberResolution {
                                        ty: return_ty,
                                        symbol_id: member_id,
                                        substitutions: proto_subs.clone(),
                                        parameters,
                                        returns_self,
                                        where_constraints,
                                        required_parameter_count: callable.required_parameter_count(),
                                    });
                                }
                            }
                        }

                        // Check extensions on this protocol
                        let proto_id = proto.metadata().id();
                        let proto_extensions = self.query(ExtensionsFor {
                            target_id: proto_id,
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
                                            .apply_substitutions(proto_subs);
                                        let returns_self =
                                            matches!(raw_return_ty.kind(), TyKind::SelfType);
                                        let return_ty = raw_return_ty.substitute_self(receiver_ty);
                                        let parameters: Vec<Ty> = callable
                                            .parameters()
                                            .iter()
                                            .map(|p| {
                                                p.ty.apply_substitutions(proto_subs)
                                                    .substitute_self(receiver_ty)
                                            })
                                            .collect();
                                        let where_constraints = get_where_clause_from_symbol(child.as_ref()).unwrap_or_default();
                                        return Ok(MemberResolution {
                                            ty: return_ty,
                                            symbol_id: member_id,
                                            substitutions: proto_subs.clone(),
                                            parameters,
                                            returns_self,
                                            where_constraints,
                                            required_parameter_count: callable.required_parameter_count(),
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
        let (container, substitutions) = match get_type_container_with_subs(self, receiver_ty) {
            Some((c, s)) => {
                (c, s)
            },
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
            Some(m) => {
                m
            },
            None => {
                let container_id = container.metadata().id();
                let extensions = self.query(ExtensionsFor {
                    target_id: container_id,
                });

                // Find in extensions on the type itself, filtering by where-clause constraints.
                // Extensions with unsatisfied where clauses (e.g., `where T: Cloneable`)
                // must be excluded so that calls like `iter.cycle()` on non-Cloneable
                // receivers fail at semantic checking instead of monomorphization.
                let extension_member = extensions
                    .iter()
                    .filter(|ext| {
                        // Check if extension's where-clause constraints are satisfied
                        if let Some(target_beh) = ext.metadata().get_behavior::<ExtensionTargetBehavior>() {
                            let where_clause = target_beh.where_clause();
                            if where_clause.constraints().is_empty() {
                                return true;
                            }
                            check_where_clause_satisfied(self, &where_clause, &substitutions)
                        } else {
                            true
                        }
                    })
                    .flat_map(|ext| ext.metadata().children())
                    .find(|child| child.metadata().name().value == member);

                match extension_member {
                    Some(m) => m,
                    None => {
                        // Not found in direct extensions - check conforming protocols and
                        // their extensions (with substitutions).
                        if let Some(resolution) = resolve_member_via_protocol_conformance(
                            self,
                            receiver_ty,
                            member,
                            None,
                        ) {
                            return Ok(resolution);
                        }

                        return Err(MemberError::NotFound {
                            receiver_ty: receiver_ty.clone(),
                            member: member.to_string(),
                        });
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
                // Instantiate method-level type parameters with fresh inference variables
                let mut combined_subs = substitutions.clone();
                if let Some(func_sym) = member_symbol.as_any().downcast_ref::<FunctionSymbol>() {
                    let method_type_params = func_sym.type_parameters();
                    for type_param in method_type_params {
                        let param_id = type_param.metadata().id();
                        let infer_ty = Ty::infer(callable.span().clone());
                        combined_subs.insert(param_id, infer_ty);
                    }
                }

                // Substitute both type parameters and Self with the receiver type
                let raw_return_ty = callable.return_type().apply_substitutions(&combined_subs);
                let returns_self = matches!(raw_return_ty.kind(), TyKind::SelfType);
                let return_ty = raw_return_ty.substitute_self(receiver_ty);
                let parameters: Vec<Ty> = callable
                    .parameters()
                    .iter()
                    .map(|p| {
                        p.ty.apply_substitutions(&combined_subs)
                            .substitute_self(receiver_ty)
                    })
                    .collect();
                let where_constraints = get_where_clause_from_symbol(member_symbol.as_ref()).unwrap_or_default();
                return Ok(MemberResolution {
                    ty: return_ty,
                    symbol_id: member_id,
                    substitutions: combined_subs,
                    parameters,
                    returns_self,
                    where_constraints,
                    required_parameter_count: callable.required_parameter_count(),
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
                    where_constraints: WhereClause::new(), // field access has no where constraints
                    required_parameter_count: 0,
                });
            }
            // Check for computed property access via ComputedMemberAccessBehavior
            if behavior.kind() == KestrelBehaviorKind::ComputedMemberAccess
                && let Some(access) = behavior.as_ref().downcast_ref::<ComputedMemberAccessBehavior>()
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
                    parameters: vec![],  // computed property access has no parameters
                    returns_self: false, // computed property access, not a method call
                    where_constraints: WhereClause::new(), // computed property access has no where constraints
                    required_parameter_count: 0,
                });
            }
        }

        // Check for method access
        if member_kind == KestrelSymbolKind::Function
            && let Some(callable) = member_symbol.metadata().get_behavior::<CallableBehavior>()
        {

            // Create fresh inference variables for method type parameters.
            // For deferred method calls, the binding phase didn't create these.
            // For non-deferred calls, the solver will merge with call-site substitutions,
            // overwriting these with the binding-time inference variables.
            let mut combined_subs = substitutions.clone();
            if let Some(func_sym) = member_symbol.as_any().downcast_ref::<FunctionSymbol>() {
                let method_type_params = func_sym.type_parameters();
                for type_param in method_type_params {
                    let param_id = type_param.metadata().id();
                    let infer_ty = Ty::infer(callable.span().clone());
                    combined_subs.insert(param_id, infer_ty);
                }
            }

            // For methods, return the return type and parameter types
            // Substitute both type parameters and Self with the receiver type
            let raw_return_ty = callable.return_type().apply_substitutions(&combined_subs);
            let returns_self = matches!(raw_return_ty.kind(), TyKind::SelfType);
            let return_ty = raw_return_ty.substitute_self(receiver_ty);
            // Resolve any qualified associated types (e.g., String.Output → String)
            let return_ty = resolve_all_associated_types(self, &return_ty);
            let parameters: Vec<Ty> = callable
                .parameters()
                .iter()
                .map(|p| {
                    let param_ty =
                        p.ty.apply_substitutions(&combined_subs)
                            .substitute_self(receiver_ty);
                    resolve_all_associated_types(self, &param_ty)
                })
                .collect();
            let where_constraints = get_where_clause_from_symbol(member_symbol.as_ref()).unwrap_or_default();
            return Ok(MemberResolution {
                ty: return_ty,
                symbol_id: member_id,
                substitutions: combined_subs,
                parameters,
                returns_self,
                where_constraints,
                required_parameter_count: callable.required_parameter_count(),
            });
        }

        // Member exists but is not accessible (e.g., type alias, nested type)
        Err(MemberError::NotFound {
            receiver_ty: receiver_ty.clone(),
            member: member.to_string(),
        })
    }

    fn resolve_member_with_arity(
        &self,
        receiver_ty: &Ty,
        member: &str,
        is_static: bool,
        argument_count: usize,
    ) -> Result<MemberResolution, MemberError> {
        if let Ok(resolution) = self.resolve_member(receiver_ty, member, is_static)
            && argument_count >= resolution.required_parameter_count
            && argument_count <= resolution.parameters.len()
        {
            return Ok(resolution);
        }

        if matches!(receiver_ty.kind(), TyKind::Infer) {
            return Err(MemberError::UnknownType);
        }

        let (container, substitutions) = match get_type_container_with_subs(self, receiver_ty) {
            Some((c, s)) => (c, s),
            None => {
                if let Some(resolution) = resolve_member_via_protocol_conformance(
                    self,
                    receiver_ty,
                    member,
                    Some(argument_count),
                ) {
                    return Ok(resolution);
                }
                return Err(MemberError::NotFound {
                    receiver_ty: receiver_ty.clone(),
                    member: member.to_string(),
                });
            },
        };

        let mut candidates: Vec<Arc<dyn Symbol<KestrelLanguage>>> = container
            .metadata()
            .children()
            .into_iter()
            .filter(|c| c.metadata().name().value == member)
            .collect();

        let extensions = self.query(ExtensionsFor {
            target_id: container.metadata().id(),
        });
        candidates.extend(
            extensions
                .iter()
                .flat_map(|ext| ext.metadata().children())
                .filter(|c| c.metadata().name().value == member),
        );

        for candidate in candidates {
            if let Some(callable) = candidate.metadata().get_behavior::<CallableBehavior>() {
                if !callable.arity_matches(argument_count) {
                    continue;
                }
                if callable.is_static() != is_static {
                    continue;
                }

                let mut combined_subs = substitutions.clone();
                if let Some(func_sym) = candidate.as_any().downcast_ref::<FunctionSymbol>() {
                    for type_param in func_sym.type_parameters() {
                        let param_id = type_param.metadata().id();
                        combined_subs.insert(param_id, Ty::infer(callable.span().clone()));
                    }
                }

                let raw_return_ty = callable.return_type().apply_substitutions(&combined_subs);
                let returns_self = matches!(raw_return_ty.kind(), TyKind::SelfType);
                let return_ty =
                    resolve_all_associated_types(self, &raw_return_ty.substitute_self(receiver_ty));
                let parameters: Vec<Ty> = callable
                    .parameters()
                    .iter()
                    .map(|p| {
                        resolve_all_associated_types(
                            self,
                            &p.ty
                                .apply_substitutions(&combined_subs)
                                .substitute_self(receiver_ty),
                        )
                    })
                    .collect();
                let where_constraints =
                    get_where_clause_from_symbol(candidate.as_ref()).unwrap_or_default();

                return Ok(MemberResolution {
                    ty: return_ty,
                    symbol_id: candidate.metadata().id(),
                    substitutions: combined_subs,
                    parameters,
                    returns_self,
                    where_constraints,
                    required_parameter_count: callable.required_parameter_count(),
                });
            }
        }

        if let Some(resolution) = resolve_member_via_protocol_conformance(
            self,
            receiver_ty,
            member,
            Some(argument_count),
        ) {
            return Ok(resolution);
        }

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
            if bound_protocols_include(self, &bounds, protocol_id) {
                return true;
            }
            return false;
        }

        // Handle associated types - check bounds on the associated type definition
        if let TyKind::AssociatedType { symbol, container } = ty.kind() {
            // If the associated type can be resolved to a concrete type, defer to that
            let assoc_name = symbol.metadata().name().value.clone();
            if let Some(container) = container
                && let Some(resolved) = self.resolve_associated_type(container, &assoc_name)
            {
                return self.conforms_to(&resolved, protocol_id);
            }

            if let Some(bounds) = symbol.bounds() {
                if bound_protocols_include(self, &bounds, protocol_id) {
                    return true;
                }
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

        // Filter to only applicable extensions based on type arguments and where clauses
        let applicable_extensions =
            filter_applicable_extensions_for_conformance(Some(self), &extensions, &actual_subs);

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
                        let result = resolved.apply_substitutions(substitutions);
                        // Recursively resolve any nested associated types
                        let mut visited = std::collections::HashSet::new();
                        return Some(deeply_resolve_associated_types(self, &result, &mut visited));
                    }
                }

                // Also look for associated type definitions (e.g., `type Item = U` in struct)
                for child in symbol.metadata().children() {
                    if child.metadata().kind() == KestrelSymbolKind::AssociatedType
                        && child.metadata().name().value == assoc_name
                        && let Ok(assoc_type) = child.downcast_arc::<AssociatedTypeSymbol>()
                        && let Some(default_ty) = assoc_type.default_type()
                    {
                        let result = default_ty.apply_substitutions(substitutions);
                        // Recursively resolve any nested associated types
                        let mut visited = std::collections::HashSet::new();
                        return Some(deeply_resolve_associated_types(self, &result, &mut visited));
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
                        let result = ty.apply_substitutions(substitutions);
                        // Recursively resolve any nested associated types
                        let mut visited = std::collections::HashSet::new();
                        return Some(deeply_resolve_associated_types(self, &result, &mut visited));
                    }
                }

                // Check extensions for associated type bindings
                // (e.g., `extend Maker: Factory { type Product = Int }`)
                let extensions = self.query(ExtensionsFor {
                    target_id: symbol.metadata().id(),
                });

                let applicable_extensions = filter_applicable_extensions_for_conformance(
                    Some(self),
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
                            let result = resolved.apply_substitutions(substitutions);
                            // Recursively resolve any nested associated types
                            let mut visited = std::collections::HashSet::new();
                            return Some(deeply_resolve_associated_types(self, &result, &mut visited));
                        }
                    }

                    // Also look for associated type definitions in extensions
                    for child in extension.metadata().children() {
                        if child.metadata().kind() == KestrelSymbolKind::AssociatedType
                            && child.metadata().name().value == assoc_name
                            && let Ok(assoc_type) = child.downcast_arc::<AssociatedTypeSymbol>()
                            && let Some(default_ty) = assoc_type.default_type()
                        {
                            let result = default_ty.apply_substitutions(substitutions);
                            // Recursively resolve any nested associated types
                            let mut visited = std::collections::HashSet::new();
                            return Some(deeply_resolve_associated_types(self, &result, &mut visited));
                        }
                    }
                }

                None
            },

            // For enum types, look for type alias or associated type with that name
            TyKind::Enum {
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
                        let result = resolved.apply_substitutions(substitutions);
                        // Recursively resolve any nested associated types
                        let mut visited = std::collections::HashSet::new();
                        return Some(deeply_resolve_associated_types(self, &result, &mut visited));
                    }
                }

                // Also look for associated type definitions (e.g., `type Item = U` in enum)
                for child in symbol.metadata().children() {
                    if child.metadata().kind() == KestrelSymbolKind::AssociatedType
                        && child.metadata().name().value == assoc_name
                        && let Ok(assoc_type) = child.downcast_arc::<AssociatedTypeSymbol>()
                        && let Some(default_ty) = assoc_type.default_type()
                    {
                        let result = default_ty.apply_substitutions(substitutions);
                        // Recursively resolve any nested associated types
                        let mut visited = std::collections::HashSet::new();
                        return Some(deeply_resolve_associated_types(self, &result, &mut visited));
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
                        let result = ty.apply_substitutions(substitutions);
                        // Recursively resolve any nested associated types
                        let mut visited = std::collections::HashSet::new();
                        return Some(deeply_resolve_associated_types(self, &result, &mut visited));
                    }
                }

                // Check extensions for associated type bindings
                let extensions = self.query(ExtensionsFor {
                    target_id: symbol.metadata().id(),
                });

                let applicable_extensions = filter_applicable_extensions_for_conformance(
                    Some(self),
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
                            let result = resolved.apply_substitutions(substitutions);
                            // Recursively resolve any nested associated types
                            let mut visited = std::collections::HashSet::new();
                            return Some(deeply_resolve_associated_types(self, &result, &mut visited));
                        }
                    }

                    // Also look for associated type definitions in extensions
                    for child in extension.metadata().children() {
                        if child.metadata().kind() == KestrelSymbolKind::AssociatedType
                            && child.metadata().name().value == assoc_name
                            && let Ok(assoc_type) = child.downcast_arc::<AssociatedTypeSymbol>()
                            && let Some(default_ty) = assoc_type.default_type()
                        {
                            let result = default_ty.apply_substitutions(substitutions);
                            // Recursively resolve any nested associated types
                            let mut visited = std::collections::HashSet::new();
                            return Some(deeply_resolve_associated_types(self, &result, &mut visited));
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

            // For nested associated types, resolve step-by-step:
            //   (Container.Assoc).Target
            // We must first resolve `Container.Assoc`, then resolve `Target` on that result.
            TyKind::AssociatedType { symbol, container } => {
                if let Some(base_container) = container {
                    let base_assoc_name = symbol.metadata().name().value.clone();
                    self.resolve_associated_type(base_container, &base_assoc_name)
                        .and_then(|resolved_base| {
                            self.resolve_associated_type(&resolved_base, assoc_name)
                        })
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

    fn check_from_value_conformance(
        &self,
        target_ty: &Ty,
        source_ty: &Ty,
    ) -> Option<(SymbolId, Substitutions)> {
        check_from_value_conformance_impl(self, target_ty, source_ty)
    }
}

impl SemanticModel {
    /// Get all protocol conformances for a concrete type, including conformances
    /// added via type extensions and transitive conformances from protocol extensions.
    pub fn protocol_conformance_ids_for_type(&self, ty: &Ty) -> Vec<SymbolId> {
        collect_protocol_conformance_ids_for_type(self, ty)
    }

    /// Get all protocol conformances for a concrete type as full protocol types.
    ///
    /// This returns protocol types with their type parameter substitutions preserved.
    pub fn protocol_conformances_for_type(&self, ty: &Ty) -> Vec<Ty> {
        collect_protocol_conformances_for_type(self, ty)
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

    fn function_where_clause(&self, function_id: SymbolId) -> WhereClause {
        use kestrel_semantic_tree::symbol::function::FunctionSymbol;

        // Look up the function symbol
        let symbol = match self.query(SymbolFor { id: function_id }) {
            Some(sym) => sym,
            None => return WhereClause::new(),
        };

        // Try to downcast to FunctionSymbol
        let function_symbol = match symbol.downcast_arc::<FunctionSymbol>() {
            Ok(func) => func,
            Err(_) => return WhereClause::new(),
        };

        // Get the where clause from the function
        function_symbol.where_clause()
    }
}

// ============================================================================
// ContextualOracle: Oracle with function context for extension bound lookup
// ============================================================================

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
            None,
        )
    }

    fn resolve_member_with_arity(
        &self,
        receiver_ty: &Ty,
        member: &str,
        is_static: bool,
        argument_count: usize,
    ) -> Result<MemberResolution, MemberError> {
        resolve_member_with_context(
            self.model,
            receiver_ty,
            member,
            is_static,
            Some(self.context_symbol_id),
            Some(argument_count),
        )
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

        // Handle SelfType using context (protocol or protocol extension)
        if matches!(ty.kind(), TyKind::SelfType) {
            let mut seed_protocols = Vec::new();
            seed_protocols.extend(self_protocol_bounds(self.model, self.context_symbol_id));
            if !seed_protocols.is_empty() {
                if seed_protocols.contains(&protocol_id) {
                    return true;
                }
                let reachable = collect_protocol_ids_via_extensions(self.model, seed_protocols);
                return reachable.contains(&protocol_id);
            }
            return false;
        }

        // Handle type parameters - include extension where-clause bounds from context
        if let TyKind::TypeParameter(type_param) = ty.kind() {
            let mut bounds = get_type_parameter_bounds(type_param);
            if let Some(ext_bounds) = get_extension_bounds_for_param(
                self.model,
                self.context_symbol_id,
                type_param.metadata().id(),
            ) {
                bounds.extend(ext_bounds);
            }
            return bound_protocols_include(self.model, &bounds, protocol_id);
        }

        // Handle associated types - include context SelfBound constraints
        if let TyKind::AssociatedType { symbol, container } = ty.kind() {
            let assoc_name = symbol.metadata().name().value.clone();
            if let Some(container) = container
                && let Some(resolved) = self.model.resolve_associated_type(container, &assoc_name)
            {
                return self.conforms_to(&resolved, protocol_id);
            }

            let bounds = get_associated_type_bounds_with_context(
                self.model,
                symbol,
                Some(self.context_symbol_id),
            );
            return bound_protocols_include(self.model, &bounds, protocol_id);
        }

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

    fn check_from_value_conformance(
        &self,
        target_ty: &Ty,
        source_ty: &Ty,
    ) -> Option<(SymbolId, Substitutions)> {
        self.model
            .check_from_value_conformance(target_ty, source_ty)
    }

    fn normalize_with_constraints(&self, ty: &Ty) -> Ty {
        normalize_type_with_context(self.model, ty, self.context_symbol_id)
    }

    fn function_where_clause(&self, function_id: SymbolId) -> WhereClause {
        self.model.function_where_clause(function_id)
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
    expected_arity: Option<usize>,
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

    // Handle SelfType - resolve members from protocol bounds in the current context
    if matches!(receiver_ty.kind(), TyKind::SelfType) {
        let Some(ctx_id) = context else {
            return Err(MemberError::UnknownType);
        };

        let bounds = get_self_type_bounds_with_context(model, ctx_id, receiver_ty.span());

        // If any bound is an error type, the type's constraints couldn't be resolved.
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
                let all_protocols = collect_protocols_with_inherited(proto, proto_subs, Some(receiver_ty));

                for (proto, proto_subs) in &all_protocols {
                    // Check protocol's direct members
                    for child in proto.metadata().children() {
                        if child.metadata().name().value == member {
                            let member_id = child.metadata().id();
                            if let Some(callable) =
                                child.metadata().get_behavior::<CallableBehavior>()
                            {
                                if let Some(expected) = expected_arity
                                    && !callable.arity_matches(expected)
                                {
                                    continue;
                                }
                                // Substitute type parameters and Self (Self = the receiver)
                                let raw_return_ty =
                                    callable.return_type().apply_substitutions(proto_subs);
                                let returns_self = matches!(raw_return_ty.kind(), TyKind::SelfType);
                                let return_ty = raw_return_ty.substitute_self(receiver_ty);
                                let parameters: Vec<Ty> = callable
                                    .parameters()
                                    .iter()
                                    .map(|p| {
                                        p.ty.apply_substitutions(proto_subs)
                                            .substitute_self(receiver_ty)
                                    })
                                    .collect();
                                let where_constraints = get_where_clause_from_symbol(child.as_ref()).unwrap_or_default();
                                return Ok(MemberResolution {
                                    ty: return_ty,
                                    symbol_id: member_id,
                                    substitutions: proto_subs.clone(),
                                    parameters,
                                    returns_self,
                                    where_constraints,
                                    required_parameter_count: callable.required_parameter_count(),
                                });
                            }
                        }
                    }

                    // Check extensions on this protocol
                    let proto_id = proto.metadata().id();
                    let proto_extensions = model.query(ExtensionsFor {
                        target_id: proto_id,
                    });

                    for ext in &proto_extensions {
                        for child in ext.metadata().children() {
                            if child.metadata().name().value == member {
                                let member_id = child.metadata().id();
                                if let Some(callable) =
                                    child.metadata().get_behavior::<CallableBehavior>()
                                {
                                    if let Some(expected) = expected_arity
                                        && !callable.arity_matches(expected)
                                    {
                                        continue;
                                    }
                                    // Substitute type parameters and Self (Self = the receiver)
                                    let raw_return_ty =
                                        callable.return_type().apply_substitutions(proto_subs);
                                    let returns_self =
                                        matches!(raw_return_ty.kind(), TyKind::SelfType);
                                    let return_ty = raw_return_ty.substitute_self(receiver_ty);
                                    let parameters: Vec<Ty> = callable
                                        .parameters()
                                        .iter()
                                        .map(|p| {
                                            p.ty.apply_substitutions(proto_subs)
                                                .substitute_self(receiver_ty)
                                        })
                                        .collect();
                                    let where_constraints = get_where_clause_from_symbol(child.as_ref()).unwrap_or_default();
                                    return Ok(MemberResolution {
                                        ty: return_ty,
                                        symbol_id: member_id,
                                        substitutions: proto_subs.clone(),
                                        parameters,
                                        returns_self,
                                        where_constraints,
                                        required_parameter_count: callable.required_parameter_count(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

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
                let all_protocols = collect_protocols_with_inherited(proto, proto_subs, Some(receiver_ty));

                for (proto, proto_subs) in &all_protocols {
                    // Check protocol's direct members
                    for child in proto.metadata().children() {
                        if child.metadata().name().value == member {
                            let member_id = child.metadata().id();
                            if let Some(callable) =
                                child.metadata().get_behavior::<CallableBehavior>()
                            {
                                if let Some(expected) = expected_arity
                                    && !callable.arity_matches(expected)
                                {
                                    continue;
                                }
                                // Substitute type parameters and Self (Self = the type parameter)
                                let raw_return_ty =
                                    callable.return_type().apply_substitutions(proto_subs);
                                let returns_self = matches!(raw_return_ty.kind(), TyKind::SelfType);
                                let return_ty = raw_return_ty.substitute_self(receiver_ty);
                                let parameters: Vec<Ty> = callable
                                    .parameters()
                                    .iter()
                                    .map(|p| {
                                        p.ty.apply_substitutions(proto_subs)
                                            .substitute_self(receiver_ty)
                                    })
                                    .collect();
                                let where_constraints = get_where_clause_from_symbol(child.as_ref()).unwrap_or_default();
                                return Ok(MemberResolution {
                                    ty: return_ty,
                                    symbol_id: member_id,
                                    substitutions: proto_subs.clone(),
                                    parameters,
                                    returns_self,
                                    where_constraints,
                                    required_parameter_count: callable.required_parameter_count(),
                                });
                            }
                        }
                    }

                    // Check extensions on this protocol
                    let proto_id = proto.metadata().id();
                    let proto_extensions = model.query(ExtensionsFor {
                        target_id: proto_id,
                    });

                    for ext in &proto_extensions {
                        for child in ext.metadata().children() {
                            if child.metadata().name().value == member {
                                let member_id = child.metadata().id();
                                if let Some(callable) =
                                    child.metadata().get_behavior::<CallableBehavior>()
                                {
                                    if let Some(expected) = expected_arity
                                        && !callable.arity_matches(expected)
                                    {
                                        continue;
                                    }
                                    // Substitute type parameters and Self (Self = the type parameter)
                                    let raw_return_ty =
                                        callable.return_type().apply_substitutions(proto_subs);
                                    let returns_self =
                                        matches!(raw_return_ty.kind(), TyKind::SelfType);
                                    let return_ty = raw_return_ty.substitute_self(receiver_ty);
                                    let parameters: Vec<Ty> = callable
                                        .parameters()
                                        .iter()
                                        .map(|p| {
                                            p.ty.apply_substitutions(proto_subs)
                                                .substitute_self(receiver_ty)
                                        })
                                        .collect();
                                    let where_constraints = get_where_clause_from_symbol(child.as_ref()).unwrap_or_default();
                                    return Ok(MemberResolution {
                                        ty: return_ty,
                                        symbol_id: member_id,
                                        substitutions: proto_subs.clone(),
                                        parameters,
                                        returns_self,
                                        where_constraints,
                                        required_parameter_count: callable.required_parameter_count(),
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

    // Handle associated types - look up member in protocol bounds from the associated type definition
    if let TyKind::AssociatedType { symbol: assoc_type, .. } = receiver_ty.kind() {
        let bounds = get_associated_type_bounds_with_context(model, assoc_type, context);

        // If any bound is an error type, the associated type's constraints couldn't be resolved.
        // Return UnknownType to suppress cascading error messages.
        if bounds.iter().any(|b| matches!(b.kind(), TyKind::Error)) {
            return Err(MemberError::UnknownType);
        }

        for bound in &bounds {
            // Apply protocol defaults with Self = receiver_ty (the associated type)
            // This resolves defaults like `Rhs = Self` in `Equal[Rhs = Self]`
            let bound = apply_protocol_defaults(bound.clone(), Some(receiver_ty));
            if let TyKind::Protocol {
                symbol: proto,
                substitutions: proto_subs,
            } = bound.kind()
            {
                // Collect the protocol and all its inherited protocols
                let all_protocols = collect_protocols_with_inherited(proto, proto_subs, Some(receiver_ty));

                for (proto, proto_subs) in &all_protocols {
                    // Check protocol's direct members
                    for child in proto.metadata().children() {
                        if child.metadata().name().value == member {
                            let member_id = child.metadata().id();
                            if let Some(callable) =
                                child.metadata().get_behavior::<CallableBehavior>()
                            {
                                if let Some(expected) = expected_arity
                                    && !callable.arity_matches(expected)
                                {
                                    continue;
                                }
                                // Substitute type parameters and Self (Self = the associated type)
                                let raw_return_ty =
                                    callable.return_type().apply_substitutions(proto_subs);
                                let returns_self = matches!(raw_return_ty.kind(), TyKind::SelfType);
                                let return_ty = raw_return_ty.substitute_self(receiver_ty);
                                let parameters: Vec<Ty> = callable
                                    .parameters()
                                    .iter()
                                    .map(|p| {
                                        p.ty.apply_substitutions(proto_subs)
                                            .substitute_self(receiver_ty)
                                    })
                                    .collect();
                                let where_constraints = get_where_clause_from_symbol(child.as_ref()).unwrap_or_default();
                                return Ok(MemberResolution {
                                    ty: return_ty,
                                    symbol_id: member_id,
                                    substitutions: proto_subs.clone(),
                                    parameters,
                                    returns_self,
                                    where_constraints,
                                    required_parameter_count: callable.required_parameter_count(),
                                });
                            }
                        }
                    }

                    // Check extensions on this protocol
                    let proto_id = proto.metadata().id();
                    let proto_extensions = model.query(ExtensionsFor {
                        target_id: proto_id,
                    });

                    for ext in &proto_extensions {
                        for child in ext.metadata().children() {
                            if child.metadata().name().value == member {
                                let member_id = child.metadata().id();
                                if let Some(callable) =
                                    child.metadata().get_behavior::<CallableBehavior>()
                                {
                                    if let Some(expected) = expected_arity
                                        && !callable.arity_matches(expected)
                                    {
                                        continue;
                                    }
                                    // Substitute type parameters and Self (Self = the associated type)
                                    let raw_return_ty =
                                        callable.return_type().apply_substitutions(proto_subs);
                                    let returns_self =
                                        matches!(raw_return_ty.kind(), TyKind::SelfType);
                                    let return_ty = raw_return_ty.substitute_self(receiver_ty);
                                    let parameters: Vec<Ty> = callable
                                        .parameters()
                                        .iter()
                                        .map(|p| {
                                            p.ty.apply_substitutions(proto_subs)
                                                .substitute_self(receiver_ty)
                                        })
                                        .collect();
                                    let where_constraints = get_where_clause_from_symbol(child.as_ref()).unwrap_or_default();
                                    return Ok(MemberResolution {
                                        ty: return_ty,
                                        symbol_id: member_id,
                                        substitutions: proto_subs.clone(),
                                        parameters,
                                        returns_self,
                                        where_constraints,
                                        required_parameter_count: callable.required_parameter_count(),
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
    if let Some(argument_count) = expected_arity {
        model.resolve_member_with_arity(receiver_ty, member, is_static, argument_count)
    } else {
        model.resolve_member(receiver_ty, member, is_static)
    }
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
    model: &SemanticModel,
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
                if !subs_with_defaults.contains(param_id)
                    && let Some(default_ty) = type_param.default()
                {
                    subs_with_defaults.insert(param_id, default_ty.clone());
                }
            }
            let dyn_symbol: Arc<dyn Symbol<KestrelLanguage>> = symbol.clone();
            Some((dyn_symbol, subs_with_defaults))
        },
        TyKind::TypeAlias {
            symbol,
            substitutions,
        } => {
            // Resolve type alias to its underlying type and recursively get container
            let alias_id = symbol.metadata().id();
            if let Some(resolved) = model.query(ResolvedAliasedType { type_alias_id: alias_id }) {
                let resolved_with_subs = resolved.apply_substitutions(substitutions);
                get_type_container_with_subs(model, &resolved_with_subs)
            } else {
                None
            }
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

/// Deeply resolve associated types recursively in a type.
///
/// This function takes a type that may contain associated types and recursively
/// resolves them until no more associated types remain. For example:
/// - `TakeIterator[ArrayIterator[Int64]].Item` ->
///   `ArrayIterator[Int64].Item` ->
///   `Int64`
///
/// Uses a visited set to detect and handle cycles.
fn deeply_resolve_associated_types(
    oracle: &dyn TypeOracle,
    ty: &Ty,
    visited: &mut std::collections::HashSet<TyId>,
) -> Ty {
    // Check for cycles
    if !visited.insert(ty.id()) {
        return ty.clone();
    }

    let result = match ty.kind() {
        // Associated type with container - resolve the container first, then the associated type,
        // then recurse on the result
        TyKind::AssociatedType { symbol, container } => {
            if let Some(container_ty) = container {
                // First, recursively resolve associated types in the container
                let resolved_container = deeply_resolve_associated_types(oracle, container_ty, visited);

                // Expand type aliases on the resolved container
                let expanded_container = oracle.expand_type_alias(&resolved_container);

                // Try to resolve the associated type on the expanded container
                if let Some(resolved_assoc) = oracle.resolve_associated_type(&expanded_container, &symbol.metadata().name().value) {
                    // Recursively resolve any nested associated types in the result
                    deeply_resolve_associated_types(oracle, &resolved_assoc, visited)
                } else {
                    // Could not resolve - return the type with the resolved container
                    Ty::qualified_associated_type(symbol.clone(), expanded_container, ty.span().clone())
                }
            } else {
                // No container - return as-is
                ty.clone()
            }
        },

        // Struct/Enum/Protocol with substitutions - recursively resolve associated types in substitutions
        TyKind::Struct { symbol, substitutions } => {
            let mut new_subs = Substitutions::new();
            let mut changed = false;
            for (param_id, sub_ty) in substitutions.iter() {
                let resolved_sub = deeply_resolve_associated_types(oracle, sub_ty, visited);
                if resolved_sub.id() != sub_ty.id() {
                    changed = true;
                }
                new_subs.insert(*param_id, resolved_sub);
            }
            if changed {
                Ty::generic_struct(symbol.clone(), new_subs, ty.span().clone())
            } else {
                ty.clone()
            }
        },

        TyKind::Enum { symbol, substitutions } => {
            let mut new_subs = Substitutions::new();
            let mut changed = false;
            for (param_id, sub_ty) in substitutions.iter() {
                let resolved_sub = deeply_resolve_associated_types(oracle, sub_ty, visited);
                if resolved_sub.id() != sub_ty.id() {
                    changed = true;
                }
                new_subs.insert(*param_id, resolved_sub);
            }
            if changed {
                Ty::generic_enum(symbol.clone(), new_subs, ty.span().clone())
            } else {
                ty.clone()
            }
        },

        TyKind::Protocol { symbol, substitutions } => {
            let mut new_subs = Substitutions::new();
            let mut changed = false;
            for (param_id, sub_ty) in substitutions.iter() {
                let resolved_sub = deeply_resolve_associated_types(oracle, sub_ty, visited);
                if resolved_sub.id() != sub_ty.id() {
                    changed = true;
                }
                new_subs.insert(*param_id, resolved_sub);
            }
            if changed {
                Ty::generic_protocol(symbol.clone(), new_subs, ty.span().clone())
            } else {
                ty.clone()
            }
        },

        TyKind::TypeAlias { symbol, substitutions } => {
            let mut new_subs = Substitutions::new();
            let mut changed = false;
            for (param_id, sub_ty) in substitutions.iter() {
                let resolved_sub = deeply_resolve_associated_types(oracle, sub_ty, visited);
                if resolved_sub.id() != sub_ty.id() {
                    changed = true;
                }
                new_subs.insert(*param_id, resolved_sub);
            }
            if changed {
                Ty::generic_type_alias(symbol.clone(), new_subs, ty.span().clone())
            } else {
                ty.clone()
            }
        },

        // Function types - recursively resolve associated types in parameters and return type
        TyKind::Function { params, return_type } => {
            let mut changed = false;
            let mut new_params = Vec::new();
            for param in params {
                let resolved_param = deeply_resolve_associated_types(oracle, param, visited);
                if resolved_param.id() != param.id() {
                    changed = true;
                }
                new_params.push(resolved_param);
            }
            let resolved_return = deeply_resolve_associated_types(oracle, return_type, visited);
            if resolved_return.id() != return_type.id() {
                changed = true;
            }
            if changed {
                Ty::function(new_params, resolved_return, ty.span().clone())
            } else {
                ty.clone()
            }
        },

        // Other types - return as-is
        _ => ty.clone(),
    };

    visited.remove(&ty.id());
    result
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
        filter_applicable_extensions_for_conformance(Some(model), &extensions, &actual_subs);

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

/// Collect protocol conformances for a concrete type, including substitutions.
///
/// This returns protocol types (not just IDs), applying the concrete type's
/// substitutions and any protocol extension conformances (with `Self` replaced
/// by the concrete type).
fn collect_protocol_conformances_for_type(model: &SemanticModel, ty: &Ty) -> Vec<Ty> {
    let ty = ty.expand_aliases();
    let type_symbol_id = match get_type_symbol_id(&ty) {
        Some(id) => id,
        None => return Vec::new(),
    };

    let actual_subs = get_type_substitutions(&ty).unwrap_or_default();
    let actual_subs_opt = Some(actual_subs.clone());

    let mut result = Vec::new();
    let mut queue: VecDeque<Ty> = VecDeque::new();
    let mut seen: HashSet<String> = HashSet::new();

    let mut push_conf = |conf: Ty, result: &mut Vec<Ty>, queue: &mut VecDeque<Ty>| {
        let key = conf.to_string();
        if seen.insert(key) {
            result.push(conf.clone());
            queue.push_back(conf);
        }
    };

    // Direct conformances on the type
    let conformances = model.query(ConformancesForSymbol {
        symbol_id: type_symbol_id,
    });
    for conformance in conformances {
        // Apply default type parameters first, substituting Self with concrete type
        let conformance = apply_protocol_defaults(conformance, Some(&ty));
        let applied = if actual_subs.is_empty() {
            conformance.clone()
        } else {
            conformance.apply_substitutions(&actual_subs)
        };
        let applied = applied.substitute_self(&ty);
        push_conf(applied, &mut result, &mut queue);
    }

    // Conformances added via type extensions
    let extensions = model.query(ExtensionsFor {
        target_id: type_symbol_id,
    });
    let applicable_extensions = filter_applicable_extensions_for_conformance(
        Some(model),
        &extensions,
        &actual_subs_opt,
    );

    for extension in &applicable_extensions {
        let ext_conformances = model.query(ConformancesForSymbol {
            symbol_id: extension.metadata().id(),
        });
        for conformance in ext_conformances {
            // Apply default type parameters first, substituting Self with concrete type
            let conformance = apply_protocol_defaults(conformance, Some(&ty));
            let applied = if actual_subs.is_empty() {
                conformance.clone()
            } else {
                conformance.apply_substitutions(&actual_subs)
            };
            let applied = applied.substitute_self(&ty);
            push_conf(applied, &mut result, &mut queue);
        }
    }

    // Expand through protocol inheritance and protocol extensions
    while let Some(conf) = queue.pop_front() {
        let TyKind::Protocol {
            symbol: proto,
            substitutions: proto_subs,
        } = conf.kind()
        else {
            continue;
        };

        // Include inherited protocols
        // When collecting inherited protocols, pass the concrete type so that default
        // type parameters with `Self` can be substituted with the concrete type.
        for (proto, proto_subs) in collect_protocols_with_inherited(proto, proto_subs, Some(&ty)) {
            let inherited_ty = Ty::generic_protocol(
                proto.clone(),
                proto_subs.clone(),
                conf.span().clone(),
            );
            push_conf(inherited_ty, &mut result, &mut queue);
        }

        // Protocol extension conformances (e.g., extend Equatable: Equal[Self])
        let proto_extensions = model.query(ExtensionsFor {
            target_id: proto.metadata().id(),
        });
        for ext in &proto_extensions {
            let Some(conformances) = ext.metadata().get_behavior::<ConformancesBehavior>() else {
                continue;
            };
            for ext_conf in conformances.conformances() {
                // Apply default type parameters first, substituting Self with concrete type
                let ext_conf = apply_protocol_defaults(ext_conf.clone(), Some(&ty));
                // Apply protocol substitutions (for protocol type params), then Self -> concrete
                let applied = if proto_subs.is_empty() {
                    ext_conf.clone()
                } else {
                    ext_conf.apply_substitutions(proto_subs)
                };
                let applied = applied.substitute_self(&ty);
                push_conf(applied, &mut result, &mut queue);
            }
        }
    }

    result
}

fn resolve_member_via_protocol_conformance(
    model: &SemanticModel,
    receiver_ty: &Ty,
    member: &str,
    expected_arity: Option<usize>,
) -> Option<MemberResolution> {
    let conformances = collect_protocol_conformances_for_type(model, receiver_ty);

    for conformance in &conformances {
        let TyKind::Protocol {
            symbol: proto,
            substitutions: proto_subs,
        } = conformance.kind()
        else {
            continue;
        };

        // NOTE: conformances already include all inherited protocols with substitutions
        // applied by collect_protocol_conformances_for_type, so we don't need to call
        // collect_protocols_with_inherited again.
        // Check protocol's direct members
        for child in proto.metadata().children() {
            if child.metadata().name().value == member {
                let member_id = child.metadata().id();
                if let Some(callable) = child.metadata().get_behavior::<CallableBehavior>() {
                    if let Some(expected) = expected_arity
                        && !callable.arity_matches(expected)
                    {
                        continue;
                    }

                    // Create fresh inference variables for method type parameters.
                    // For deferred method calls, the binding phase didn't create these.
                    // For non-deferred calls, the solver will merge with call-site substitutions.
                    let mut combined_subs = proto_subs.clone();
                    if let Some(func_sym) = child.as_any().downcast_ref::<FunctionSymbol>() {
                        for type_param in func_sym.type_parameters() {
                            let param_id = type_param.metadata().id();
                            let infer_ty = Ty::infer(callable.span().clone());
                            combined_subs.insert(param_id, infer_ty);
                        }
                    }

                    let raw_return_ty = callable.return_type().apply_substitutions(&combined_subs);
                    let returns_self = matches!(raw_return_ty.kind(), TyKind::SelfType);
                    let return_ty_before_resolve = raw_return_ty.substitute_self(receiver_ty);
                    let mut visited = std::collections::HashSet::new();
                    let return_ty = deeply_resolve_associated_types(
                        model,
                        &return_ty_before_resolve,
                        &mut visited,
                    );

                    let parameters: Vec<Ty> = callable
                        .parameters()
                        .iter()
                        .map(|p| {
                            let after_subs = p.ty.apply_substitutions(&combined_subs);
                            let after_self = after_subs.substitute_self(receiver_ty);
                            let mut visited = std::collections::HashSet::new();
                            deeply_resolve_associated_types(model, &after_self, &mut visited)
                        })
                        .collect();

                    let where_constraints =
                        get_where_clause_from_symbol(child.as_ref()).unwrap_or_default();
                    return Some(MemberResolution {
                        ty: return_ty,
                        symbol_id: member_id,
                        substitutions: combined_subs,
                        parameters,
                        returns_self,
                        where_constraints,
                        required_parameter_count: callable.required_parameter_count(),
                    });
                }
            }
        }

        // Check extensions on this protocol
        let proto_id = proto.metadata().id();
        let proto_extensions = model.query(ExtensionsFor {
            target_id: proto_id,
        });

        for ext in &proto_extensions {
            // Check if this protocol extension's where-clause constraints are satisfied.
            // For example, `extend Iterator where Self: Cloneable { func cycle() ... }`
            // should only be visible when receiver_ty conforms to Cloneable.
            if let Some(ext_target) = ext.metadata().get_behavior::<ExtensionTargetBehavior>() {
                let ext_where = ext_target.where_clause();
                let mut ext_applicable = true;
                for constraint in ext_where.constraints() {
                    match constraint {
                        Constraint::SelfBound { associated_type_path, bounds, .. } => {
                            if associated_type_path.is_empty() {
                                // Self: Protocol - check receiver conforms
                                for bound in bounds {
                                    if let TyKind::Protocol { symbol: bound_proto, .. } = bound.kind() {
                                        if !model.conforms_to(receiver_ty, bound_proto.metadata().id()) {
                                            ext_applicable = false;
                                            break;
                                        }
                                    }
                                }
                            }
                            // TODO: Self.Item: Protocol checks
                        }
                        Constraint::TypeBound { param: Some(param_id), bounds, .. } => {
                            // Check type-parameter bounds against substitutions
                            if let Some(actual_ty) = proto_subs.get(*param_id) {
                                for bound in bounds {
                                    if let TyKind::Protocol { symbol: bound_proto, .. } = bound.kind() {
                                        if !model.conforms_to(actual_ty, bound_proto.metadata().id()) {
                                            ext_applicable = false;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    if !ext_applicable {
                        break;
                    }
                }
                if !ext_applicable {
                    continue;
                }
            }

            for child in ext.metadata().children() {
                if child.metadata().name().value == member {
                    let member_id = child.metadata().id();
                    if let Some(callable) = child.metadata().get_behavior::<CallableBehavior>() {
                        if let Some(expected) = expected_arity
                            && !callable.arity_matches(expected)
                        {
                            continue;
                        }

                        // Check if this extension adds protocol conformances
                        // e.g., `extend Comparable: NotEqual[Self]` means methods in the extension
                        // might be fulfilling requirements from NotEqual, not Comparable
                        let mut effective_subs = proto_subs.clone();
                        if let Some(ext_conformances) =
                            ext.metadata().get_behavior::<ConformancesBehavior>()
                        {
                            for ext_conf_ty in ext_conformances.conformances() {
                                let TyKind::Protocol { symbol: ext_proto, .. } = ext_conf_ty.kind() else {
                                    continue;
                                };
                                let method_in_protocol = ext_proto
                                    .metadata()
                                    .children()
                                    .iter()
                                    .any(|c| c.metadata().name().value == member);
                                if !method_in_protocol {
                                    continue;
                                }

                                // Find the corresponding conformance from our receiver type
                                // that matches this protocol.
                                for candidate_conf in &conformances {
                                    if let TyKind::Protocol {
                                        symbol: candidate_proto,
                                        substitutions: candidate_subs,
                                    } = candidate_conf.kind()
                                        && candidate_proto.metadata().id() == ext_proto.metadata().id()
                                    {
                                        effective_subs = candidate_subs.clone();
                                        break;
                                    }
                                }
                            }
                        }

                        // Create fresh inference variables for method type parameters.
                        // For deferred method calls, the binding phase didn't create these.
                        // For non-deferred calls, the solver will merge with call-site substitutions.
                        let mut combined_subs = effective_subs;
                        if let Some(func_sym) = child.as_any().downcast_ref::<FunctionSymbol>() {
                            for type_param in func_sym.type_parameters() {
                                let param_id = type_param.metadata().id();
                                let infer_ty = Ty::infer(callable.span().clone());
                                combined_subs.insert(param_id, infer_ty);
                            }
                        }

                        let raw_return_ty = callable.return_type().apply_substitutions(&combined_subs);
                        let returns_self = matches!(raw_return_ty.kind(), TyKind::SelfType);
                        let return_ty_before_resolve = raw_return_ty.substitute_self(receiver_ty);
                        let mut visited = std::collections::HashSet::new();
                        let return_ty = deeply_resolve_associated_types(
                            model,
                            &return_ty_before_resolve,
                            &mut visited,
                        );

                        let parameters: Vec<Ty> = callable
                            .parameters()
                            .iter()
                            .map(|p| {
                                let after_subs = p.ty.apply_substitutions(&combined_subs);
                                let after_self = after_subs.substitute_self(receiver_ty);
                                let mut visited = std::collections::HashSet::new();
                                deeply_resolve_associated_types(model, &after_self, &mut visited)
                            })
                            .collect();

                        let where_constraints =
                            get_where_clause_from_symbol(child.as_ref()).unwrap_or_default();
                        return Some(MemberResolution {
                            ty: return_ty,
                            symbol_id: member_id,
                            substitutions: combined_subs,
                            parameters,
                            returns_self,
                            where_constraints,
                            required_parameter_count: callable.required_parameter_count(),
                        });
                    }
                }
            }
        }
    }

    None
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
///
/// When `model` is provided, also checks that where clause constraints are satisfied.
/// This is necessary for conditional conformances like `extend Pointer: FFISafe where T: FFISafe`.
fn filter_applicable_extensions_for_conformance<'a>(
    model: Option<&SemanticModel>,
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
                // Still need to check where clause even without type arguments
                if let Some(model) = model
                    && let Some(actual_subs) = actual_subs
                {
                    return check_where_clause_satisfied(
                        model,
                        target_behavior.where_clause(),
                        actual_subs,
                    );
                }
                return true;
            };

            // If actual type has no substitutions but extension does, check if extension is fully generic
            let Some(actual_subs) = actual_subs else {
                // No actual subs - extension is applicable if it's fully generic
                return extension_subs.iter().all(|(_, ty)| ty.is_type_parameter());
            };

            // Check if extension's type arguments are applicable
            if !is_extension_applicable_for_conformance(extension_subs, actual_subs) {
                return false;
            }

            // Check where clause constraints (for conditional conformances)
            if let Some(model) = model
                && !check_where_clause_satisfied(model, target_behavior.where_clause(), actual_subs)
            {
                return false;
            }

            true
        })
        .collect()
}

/// Check if a where clause is satisfied given the actual type substitutions.
///
/// For each `TypeBound` constraint like `T: FFISafe`, looks up the actual type
/// for T and checks if it conforms to FFISafe.
fn check_where_clause_satisfied(
    model: &SemanticModel,
    where_clause: &WhereClause,
    actual_subs: &Substitutions,
) -> bool {
    use kestrel_semantic_tree::ty::Constraint;

    for constraint in where_clause.constraints() {
        // Only process TypeBound constraints - other types (NegativeBound,
        // InheritedAssociatedTypeBound, TypeEquality, SelfBound) are not
        // relevant for basic conformance filtering
        let Constraint::TypeBound {
            param: Some(param_id),
            bounds,
            ..
        } = constraint
        else {
            continue;
        };

        // Get the actual type for this parameter
        let Some(actual_ty) = actual_subs.get(*param_id) else {
            // No substitution for this param - might be a constraint on
            // a different parameter or inherited constraint. Skip it.
            continue;
        };

        // Check each bound
        for bound in bounds {
            if let TyKind::Protocol { symbol, .. } = bound.kind()
                && !model.conforms_to(actual_ty, symbol.metadata().id())
            {
                return false;
            }
        }
    }

    true
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
    receiver_ty: Option<&Ty>,
) -> Vec<(Arc<ProtocolSymbol>, Substitutions)> {
    let mut result = Vec::new();
    let mut visited = std::collections::HashSet::new();
    collect_protocols_with_inherited_impl(proto, subs, receiver_ty, &mut result, &mut visited);
    result
}

/// Internal helper that recursively collects protocols while tracking visited protocols.
fn collect_protocols_with_inherited_impl(
    proto: &Arc<ProtocolSymbol>,
    subs: &Substitutions,
    receiver_ty: Option<&Ty>,
    result: &mut Vec<(Arc<ProtocolSymbol>, Substitutions)>,
    visited: &mut std::collections::HashSet<SymbolId>,
) {
    // Skip if already visited (handles cycles)
    if !visited.insert(proto.metadata().id()) {
        return;
    }

    // Clone substitutions and populate any missing type parameter defaults.
    // This is necessary for protocols like `NotEqual[Rhs = Self]` where the
    // Rhs parameter has a default value that must be included in substitutions
    // for method signature resolution to work correctly.
    let mut subs_with_defaults = subs.clone();
    for type_param in proto.type_parameters() {
        let param_id = type_param.metadata().id();
        if !subs_with_defaults.contains(param_id)
            && let Some(default_ty) = type_param.default()
        {
            // If the default is Self and we have a receiver type (e.g., resolving on an associated type),
            // substitute Self with the receiver type. Otherwise, keep the default as-is.
            let substituted_default = if matches!(default_ty.kind(), TyKind::SelfType) {
                if let Some(recv_ty) = receiver_ty {
                    recv_ty.clone()
                } else {
                    default_ty.clone()
                }
            } else {
                default_ty.clone()
            };
            subs_with_defaults.insert(param_id, substituted_default);
        }
    }

    // Add this protocol to results
    result.push((proto.clone(), subs_with_defaults.clone()));

    // Get inherited protocols from ConformancesBehavior
    if let Some(conformances) = proto.metadata().get_behavior::<ConformancesBehavior>() {
        for conformance in conformances.conformances() {
            if let TyKind::Protocol {
                symbol: inherited_proto,
                substitutions: inherited_subs,
            } = conformance.kind()
            {
                // Apply parent substitutions to inherited protocol's substitutions.
                // For each type in inherited_subs, apply subs_with_defaults to get the final type.
                let mut combined_subs = Substitutions::new();
                for (param_id, ty) in inherited_subs.iter() {
                    combined_subs.insert(*param_id, subs_with_defaults.apply(ty));
                }

                // Add defaults for any inherited protocol type parameters not in inherited_subs.
                // This ensures that if a protocol like `NotEqual[Rhs = Self]` is inherited as
                // just `NotEqual` (without explicit type arguments), we still get `{Rhs → Self}`.
                for type_param in inherited_proto.type_parameters() {
                    let param_id = type_param.metadata().id();
                    if !combined_subs.contains(param_id) {
                        if let Some(default_ty) = type_param.default() {
                            // Apply parent substitutions to the default type.
                            // This handles cases like `Self` which needs to be resolved in parent context.
                            let mut substituted_default = subs_with_defaults.apply(&default_ty);
                            // If after applying parent substitutions we still have Self, and we have a receiver type,
                            // substitute Self with the receiver type.
                            if matches!(substituted_default.kind(), TyKind::SelfType) {
                                if let Some(recv_ty) = receiver_ty {
                                    substituted_default = recv_ty.clone();
                                }
                            }
                            combined_subs.insert(param_id, substituted_default);
                        }
                    }
                }

                // Also copy over any subs from parent that aren't in inherited_subs.
                // This propagates parent protocol type parameters to inherited protocols.
                for (param_id, ty) in subs_with_defaults.iter() {
                    if !combined_subs.contains(*param_id) {
                        combined_subs.insert(*param_id, ty.clone());
                    }
                }

                // Recursively collect from inherited protocol
                collect_protocols_with_inherited_impl(
                    inherited_proto,
                    &combined_subs,
                    receiver_ty,
                    result,
                    visited,
                );
            }
        }
    }
}

/// Apply default type parameter values to a protocol type.
///
/// This fills in any missing type parameters with their default values.
/// For example, `NotEqual` (with default `Rhs = Self`) becomes `NotEqual[Rhs = Self]`.
///
/// The `self_ty` parameter is used to substitute `Self` in default types that
/// reference it. For example, `Equal[Rhs = Self]` where Self is `Int` becomes
/// `Equal[Rhs = Int]`.
fn apply_protocol_defaults(ty: Ty, self_ty: Option<&Ty>) -> Ty {
    let TyKind::Protocol {
        symbol,
        substitutions,
    } = ty.kind()
    else {
        return ty;
    };

    let type_params = symbol.type_parameters();
    if type_params.is_empty() {
        return ty;
    }

    let mut new_subs = substitutions.clone();
    let mut changed = false;

    for param in &type_params {
        let param_id = param.metadata().id();
        if !new_subs.contains(param_id) {
            if let Some(default_ty) = param.default() {
                // Substitute Self in the default type if we have a concrete type
                let resolved_default = if let Some(concrete) = self_ty {
                    default_ty.substitute_self(concrete)
                } else {
                    default_ty.clone()
                };
                new_subs.insert(param_id, resolved_default);
                changed = true;
            }
        }
    }

    if changed {
        Ty::generic_protocol(symbol.clone(), new_subs, ty.span().clone())
    } else {
        ty
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

/// Check if protocol bounds (including transitive protocol extensions) include the target.
fn bound_protocols_include(
    model: &SemanticModel,
    bounds: &[Ty],
    protocol_id: SymbolId,
) -> bool {
    let mut seed_protocols = Vec::new();
    for bound in bounds {
        if let TyKind::Protocol { symbol, .. } = bound.kind() {
            let bound_id = symbol.metadata().id();
            if bound_id == protocol_id {
                return true;
            }
            seed_protocols.push(bound_id);
        }
    }

    if seed_protocols.is_empty() {
        return false;
    }

    let reachable = collect_protocol_ids_via_extensions(model, seed_protocols);
    reachable.contains(&protocol_id)
}

/// Get protocol bounds for an associated type, including context SelfBound constraints.
fn get_associated_type_bounds_with_context(
    model: &SemanticModel,
    assoc_type: &Arc<kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol>,
    context: Option<SymbolId>,
) -> Vec<Ty> {
    let mut bounds = Vec::new();

    if let Some(direct_bounds) = assoc_type.bounds() {
        for bound in direct_bounds {
            if matches!(bound.kind(), TyKind::Protocol { .. } | TyKind::Error) {
                bounds.push(bound.clone());
            }
        }
    }

    let Some(context_id) = context else {
        return bounds;
    };

    let assoc_name = assoc_type.metadata().name().value.clone();
    let where_clauses = collect_where_clauses_for_context(model, context_id);
    for wc in where_clauses {
        for constraint in wc.constraints() {
            if let Constraint::SelfBound {
                associated_type_path,
                bounds: self_bounds,
                ..
            } = constraint
            {
                if !associated_type_path.is_empty()
                    && associated_type_path.last() == Some(&assoc_name)
                {
                    for bound in self_bounds {
                        if matches!(bound.kind(), TyKind::Protocol { .. } | TyKind::Error) {
                            bounds.push(bound.clone());
                        }
                    }
                }
            }
            if let Constraint::InheritedAssociatedTypeBound { path, bounds: assoc_bounds, .. } =
                constraint
            {
                if path.split('.').last() == Some(assoc_name.as_str()) {
                    for bound in assoc_bounds {
                        if matches!(bound.kind(), TyKind::Protocol { .. } | TyKind::Error) {
                            bounds.push(bound.clone());
                        }
                    }
                }
            }
            if let Constraint::TypeBound {
                param: None,
                param_name,
                bounds: param_bounds,
                ..
            } = constraint
            {
                if param_name == &assoc_name {
                    for bound in param_bounds {
                        if matches!(bound.kind(), TyKind::Protocol { .. } | TyKind::Error) {
                            bounds.push(bound.clone());
                        }
                    }
                }
            }
        }
    }

    bounds
}

/// Collect protocol bounds that apply to `Self` in the current context.
fn self_protocol_bounds(model: &SemanticModel, context_id: SymbolId) -> Vec<SymbolId> {
    let mut result = Vec::new();

    // Self bounds from where clauses (Self: Protocol)
    let where_clauses = collect_where_clauses_for_context(model, context_id);
    for wc in where_clauses {
        for constraint in wc.constraints() {
            if let Constraint::SelfBound {
                associated_type_path,
                bounds,
                ..
            } = constraint
            {
                if associated_type_path.is_empty() {
                    for bound in bounds {
                        if let TyKind::Protocol { symbol, .. } = bound.kind() {
                            result.push(symbol.metadata().id());
                        }
                    }
                }
            }
        }
    }

    // Also add the enclosing protocol or protocol extension target, if any
    let mut current = Some(context_id);
    while let Some(id) = current {
        let Some(symbol) = model.query(SymbolFor { id }) else {
            break;
        };

        if symbol.metadata().kind() == KestrelSymbolKind::Protocol {
            result.push(symbol.metadata().id());
            break;
        }

        if symbol.metadata().kind() == KestrelSymbolKind::Extension
            && let Some(target_beh) =
                symbol.metadata().get_behavior::<ExtensionTargetBehavior>()
        {
            let target_ty = target_beh.target_type();
            if let TyKind::Protocol { symbol, .. } = target_ty.kind() {
                result.push(symbol.metadata().id());
                break;
            }
        }

        current = symbol.metadata().parent().map(|p| p.metadata().id());
    }

    result
}

/// Get protocol bounds for `Self` in the current context, as concrete protocol types.
fn get_self_type_bounds_with_context(
    model: &SemanticModel,
    context_id: SymbolId,
    span: &kestrel_span::Span,
) -> Vec<Ty> {
    let mut bounds = Vec::new();

    // Self bounds from where clauses (Self: Protocol)
    let where_clauses = collect_where_clauses_for_context(model, context_id);
    for wc in where_clauses {
        for constraint in wc.constraints() {
            if let Constraint::SelfBound {
                associated_type_path,
                bounds: self_bounds,
                ..
            } = constraint
            {
                if associated_type_path.is_empty() {
                    for bound in self_bounds {
                        if matches!(bound.kind(), TyKind::Protocol { .. } | TyKind::Error) {
                            bounds.push(bound.clone());
                        }
                    }
                }
            }
        }
    }

    // Also add the enclosing protocol or protocol extension target, if any
    let mut current = Some(context_id);
    while let Some(id) = current {
        let Some(symbol) = model.query(SymbolFor { id }) else {
            break;
        };

        if symbol.metadata().kind() == KestrelSymbolKind::Protocol
            && let Ok(protocol) = symbol.clone().downcast_arc::<ProtocolSymbol>()
        {
            bounds.push(Ty::protocol(protocol, span.clone()));
            break;
        }

        if symbol.metadata().kind() == KestrelSymbolKind::Extension
            && let Some(target_beh) =
                symbol.metadata().get_behavior::<ExtensionTargetBehavior>()
        {
            let target_ty = target_beh.target_type();
            if matches!(target_ty.kind(), TyKind::Protocol { .. }) {
                bounds.push(target_ty.clone());
                break;
            }
        }

        current = symbol.metadata().parent().map(|p| p.metadata().id());
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
pub fn resolve_all_associated_types(oracle: &impl TypeOracle, ty: &Ty) -> Ty {
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
        | TyKind::TypeParameter(_)
        | TyKind::UnresolvedPath { .. } => ty.clone(),
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
    if symbol.metadata().kind() == KestrelSymbolKind::Subscript
        && let Some(generics) = symbol.metadata().get_behavior::<GenericsBehavior>()
    {
        return Some(generics.where_clause().clone());
    }
    None
}

/// Check if target_ty conforms to FromValue[source_ty].
///
/// This is used by the Promotable constraint to determine if a value can be
/// implicitly wrapped. For example, `Optional[Int]` conforms to `FromValue[Int]`,
/// allowing `let x: Int? = 5` to be automatically promoted.
///
/// Returns the from() method symbol and substitutions if conformance exists.
fn check_from_value_conformance_impl(
    model: &SemanticModel,
    target_ty: &Ty,
    source_ty: &Ty,
) -> Option<(SymbolId, Substitutions)> {
    // Get the FromValueProtocol ID
    let from_value_protocol_id = model.builtin_protocol(LanguageFeature::FromValueProtocol)?;

    // Get the FromValueMethod ID
    let from_value_method_id = model
        .builtin_registry()
        .method(LanguageFeature::FromValueMethod)?;

    // Expand type aliases before checking
    let target_ty = target_ty.expand_aliases();
    let source_ty = source_ty.expand_aliases();

    // Handle special types that shouldn't be promoted
    if matches!(
        target_ty.kind(),
        TyKind::Infer | TyKind::Error | TyKind::Never | TyKind::Unit
    ) {
        return None;
    }
    if matches!(
        source_ty.kind(),
        TyKind::Infer | TyKind::Error | TyKind::Never
    ) {
        return None;
    }

    // Get the target type's symbol ID
    let target_symbol_id = get_type_symbol_id(&target_ty)?;

    // Get all conformances for the target type (direct + extensions)
    let mut conformances = model.query(ConformancesForSymbol {
        symbol_id: target_symbol_id,
    });

    // Also check extensions for conformances
    let actual_subs = get_type_substitutions(&target_ty);
    let extensions = model.query(ExtensionsFor {
        target_id: target_symbol_id,
    });
    let applicable_extensions =
        filter_applicable_extensions_for_conformance(Some(model), &extensions, &actual_subs);

    for extension in &applicable_extensions {
        let ext_conformances = model.query(ConformancesForSymbol {
            symbol_id: extension.metadata().id(),
        });
        conformances.extend(ext_conformances);
    }

    // Look for a FromValue[X] conformance where X matches source_ty
    for conformance in &conformances {
        if let TyKind::Protocol {
            symbol,
            substitutions,
        } = conformance.kind()
        {
            if symbol.metadata().id() != from_value_protocol_id {
                continue;
            }

            // Found a FromValue conformance - check if the Output type matches source_ty
            // FromValue has one type parameter "Output"
            for type_param in symbol.type_parameters() {
                if type_param.metadata().name().value == "Output" {
                    let param_id = type_param.metadata().id();
                    if let Some(output_ty) = substitutions.get(param_id) {
                        // Apply the target type's substitutions to get the concrete Output type
                        let concrete_output = if let Some(target_subs) = &actual_subs {
                            output_ty.apply_substitutions(target_subs)
                        } else {
                            output_ty.clone()
                        };

                        // Check if the concrete Output type matches the source type
                        if types_match_for_conformance(&concrete_output, &source_ty) {
                            // Build the substitutions for calling from()
                            // The from() method needs: Output -> source_ty
                            let mut method_subs = Substitutions::new();
                            method_subs.insert(param_id, source_ty.clone());

                            // Also add the target type's substitutions (e.g., T for Optional[T])
                            if let Some(target_subs) = &actual_subs {
                                for (id, ty) in target_subs.iter() {
                                    if !method_subs.contains(*id) {
                                        method_subs.insert(*id, ty.clone());
                                    }
                                }
                            }

                            return Some((from_value_method_id, method_subs));
                        }
                    }
                }
            }
        }
    }

    None
}

/// Normalize a type using equality constraints from the context.
///
/// This resolves associated types like `I.Item` to their constrained values
/// when there's an equality constraint like `I.Item = (K, V)` in scope.
///
/// Also handles nested associated types like `I.Iter.Item` by:
/// 1. First collecting derived equality constraints from protocol associated types
/// 2. Then applying all equality constraints to normalize the type
fn normalize_type_with_context(model: &SemanticModel, ty: &Ty, context_id: SymbolId) -> Ty {
    // Collect where clauses by walking up the parent chain
    let where_clauses = collect_where_clauses_for_context(model, context_id);

    // Collect explicit equality constraints
    let mut owned_equalities: Vec<(Ty, Ty)> = where_clauses
        .iter()
        .flat_map(|wc| wc.equality_constraints())
        .map(|(l, r)| (l.clone(), r.clone()))
        .collect();

    // Also derive equality constraints from protocol associated type definitions.
    // For example, if we have `I: Iterable` and Iterable defines
    // `type Iter: Iterator where Iter.Item = Item`, we derive `I.Iter.Item = I.Item`.
    derive_protocol_associated_type_constraints(model, &where_clauses, &mut owned_equalities);

    // In protocol/protocol-extension contexts, implicitly qualify unqualified associated
    // types with `Self` so constraints like `Item = (A, B)` apply to `Self.Item`.
    let qualified_ty = if !owned_equalities.is_empty() && !self_protocol_bounds(model, context_id).is_empty() {
        let mut qualified = Vec::with_capacity(owned_equalities.len());
        let self_ty = Ty::self_type(ty.span().clone());
        for (left, right) in owned_equalities.into_iter() {
            qualified.push((
                left.substitute_self(&self_ty),
                right.substitute_self(&self_ty),
            ));
        }
        owned_equalities = qualified;
        // Also qualify the input type so it matches against qualified constraints
        ty.substitute_self(&self_ty)
    } else {
        ty.clone()
    };

    if owned_equalities.is_empty() {
        return qualified_ty;
    }

    let equalities: Vec<(&Ty, &Ty)> = owned_equalities
        .iter()
        .map(|(l, r)| (l, r))
        .collect();

    normalize_type_inner(&qualified_ty, &equalities)
}


/// Derive equality constraints from protocol associated type definitions.
///
/// For associated types like `type Iter: Iterator where Iter.Item = Item`,
/// this derives constraints like `T.Iter.Item = T.Item` for type parameters `T: Iterable`.
///
/// Uses a heuristic: if an associated type has a protocol bound that also has an associated type
/// with the same name as a sibling in the parent protocol, they are assumed equal.
fn derive_protocol_associated_type_constraints(
    _model: &SemanticModel,
    where_clauses: &[WhereClause],
    equalities: &mut Vec<(Ty, Ty)>,
) {
    use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol;
    use kestrel_semantic_tree::ty::Constraint;

    for wc in where_clauses {
        for constraint in wc.constraints() {
            if let Constraint::TypeBound {
                param: Some(param_id),
                bounds,
                ..
            } = constraint
            {
                let Some(param_symbol) = _model.query(SymbolFor { id: *param_id }) else {
                    continue;
                };
                let Ok(type_param) = param_symbol.downcast_arc::<TypeParameterSymbol>() else {
                    continue;
                };
                let param_ty =
                    Ty::type_parameter(type_param.clone(), type_param.metadata().span().clone());

                for bound in bounds {
                    if let TyKind::Protocol {
                        symbol: proto_sym, ..
                    } = bound.kind()
                    {
                        // For each associated type in the protocol (e.g., Iter in Iterable)
                        for child in proto_sym.metadata().children() {
                            if child.metadata().kind() == KestrelSymbolKind::AssociatedType {
                                let Ok(assoc_type) = child.downcast_arc::<AssociatedTypeSymbol>()
                                else {
                                    continue;
                                };

                                // Check if this associated type has protocol bounds (e.g., Iter: Iterator)
                                let assoc_bounds = assoc_type.bounds().unwrap_or_default();
                                for assoc_bound in &assoc_bounds {
                                    if let TyKind::Protocol {
                                        symbol: bound_proto,
                                        ..
                                    } = assoc_bound.kind()
                                    {
                                        // For each associated type in the bound protocol (e.g., Item in Iterator)
                                        for bound_child in bound_proto.metadata().children() {
                                            if bound_child.metadata().kind()
                                                == KestrelSymbolKind::AssociatedType
                                            {
                                                let bound_assoc_name =
                                                    bound_child.metadata().name().value.clone();

                                                // Look for a matching associated type in the parent protocol
                                                // (e.g., Item in Iterable)
                                                for sibling in proto_sym.metadata().children() {
                                                    if sibling.metadata().kind()
                                                        == KestrelSymbolKind::AssociatedType
                                                        && sibling.metadata().name().value
                                                            == bound_assoc_name
                                                        && sibling.metadata().id()
                                                            != assoc_type.metadata().id()
                                                    {
                                                        // Found a match! Create the constraint:
                                                        // T.Assoc.BoundAssocType = T.SiblingAssocType
                                                        // e.g., I.Iter.Item = I.Item

                                                        let Ok(bound_assoc) = bound_child
                                                            .clone()
                                                            .downcast_arc::<AssociatedTypeSymbol>(
                                                            )
                                                        else {
                                                            continue;
                                                        };
                                                        let Ok(sibling_assoc) = sibling
                                                            .downcast_arc::<AssociatedTypeSymbol>(
                                                            )
                                                        else {
                                                            continue;
                                                        };

                                                        // T.Assoc (e.g., I.Iter)
                                                        let t_assoc = Ty::qualified_associated_type(
                                                            assoc_type.clone(),
                                                            param_ty.clone(),
                                                            param_ty.span().clone(),
                                                        );

                                                        // T.Assoc.BoundAssocType (e.g., I.Iter.Item)
                                                        let left = Ty::qualified_associated_type(
                                                            bound_assoc,
                                                            t_assoc,
                                                            param_ty.span().clone(),
                                                        );

                                                        // T.SiblingAssocType (e.g., I.Item)
                                                        let right = Ty::qualified_associated_type(
                                                            sibling_assoc,
                                                            param_ty.clone(),
                                                            param_ty.span().clone(),
                                                        );

                                                        equalities.push((left, right));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Collect all where clauses from the context by walking up the parent chain.
fn collect_where_clauses_for_context(
    model: &SemanticModel,
    context_id: SymbolId,
) -> Vec<WhereClause> {
    let mut clauses = Vec::new();
    let mut current_id = Some(context_id);

    while let Some(id) = current_id {
        let Some(symbol) = model.query(SymbolFor { id }) else {
            break;
        };

        if let Some(generics_beh) = symbol.metadata().get_behavior::<GenericsBehavior>() {
            let wc = generics_beh.where_clause();
            if !wc.is_empty() {
                clauses.push(wc.clone());
            }
        }

        if let Some(target_beh) = symbol.metadata().get_behavior::<ExtensionTargetBehavior>() {
            let wc = target_beh.where_clause();
            if !wc.is_empty() {
                clauses.push(wc.clone());
            }
        }

        current_id = symbol.metadata().parent().map(|p| p.metadata().id());
    }

    clauses
}

/// Normalize a type using equality constraints (inner implementation).
fn normalize_type_inner(ty: &Ty, equalities: &[(&Ty, &Ty)]) -> Ty {
    let mut current = ty.clone();
    let mut seen = HashSet::new();
    seen.insert(current.to_string());

    let mut changed = true;
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 10; // Safety cap

    while changed && iterations < MAX_ITERATIONS {
        changed = false;
        iterations += 1;

        // Try to apply equality constraints to the whole type
        for (left, right) in equalities {
            let matches_left = equality_types_match(&current, left);
            let matches_right = equality_types_match(&current, right);

            if matches_left || matches_right {
                // We have a match. Prefer the "more concrete" type.
                let next = if equality_is_more_concrete(left, right) {
                    (*left).clone()
                } else {
                    (*right).clone()
                };

                let next_str = next.to_string();
                if next_str != current.to_string() && !seen.contains(&next_str) {
                    current = next;
                    seen.insert(next_str);
                    changed = true;
                    break;
                }
            }
        }

        if changed {
            continue;
        }

        // Try to normalize components
        match current.kind().clone() {
            TyKind::Tuple(elements) => {
                let mut new_elements = Vec::new();
                let mut inner_changed = false;
                for e in elements {
                    let normalized = normalize_type_inner(&e, equalities);
                    if normalized.to_string() != e.to_string() {
                        inner_changed = true;
                    }
                    new_elements.push(normalized);
                }
                if inner_changed {
                    current = Ty::tuple(new_elements, current.span().clone());
                    let current_str = current.to_string();
                    if !seen.contains(&current_str) {
                        seen.insert(current_str);
                        changed = true;
                    }
                }
            },
            TyKind::Pointer(element) => {
                let normalized = normalize_type_inner(&element, equalities);
                if normalized.to_string() != element.to_string() {
                    current = Ty::pointer(normalized, current.span().clone());
                    let current_str = current.to_string();
                    if !seen.contains(&current_str) {
                        seen.insert(current_str);
                        changed = true;
                    }
                }
            },
            TyKind::Function {
                params,
                return_type,
            } => {
                let mut new_params = Vec::new();
                let mut inner_changed = false;
                for p in params {
                    let normalized = normalize_type_inner(&p, equalities);
                    if normalized.to_string() != p.to_string() {
                        inner_changed = true;
                    }
                    new_params.push(normalized);
                }
                let normalized_return = normalize_type_inner(&return_type, equalities);
                if normalized_return.to_string() != return_type.to_string() {
                    inner_changed = true;
                }
                if inner_changed {
                    current = Ty::function(new_params, normalized_return, current.span().clone());
                    let current_str = current.to_string();
                    if !seen.contains(&current_str) {
                        seen.insert(current_str);
                        changed = true;
                    }
                }
            },
            TyKind::Struct {
                symbol,
                substitutions,
            } => {
                let (new_subs, inner_changed) =
                    normalize_substitutions(&substitutions, equalities);
                if inner_changed {
                    current = Ty::generic_struct(symbol, new_subs, current.span().clone());
                    let current_str = current.to_string();
                    if !seen.contains(&current_str) {
                        seen.insert(current_str);
                        changed = true;
                    }
                }
            },
            TyKind::Enum {
                symbol,
                substitutions,
            } => {
                let (new_subs, inner_changed) =
                    normalize_substitutions(&substitutions, equalities);
                if inner_changed {
                    current = Ty::generic_enum(symbol, new_subs, current.span().clone());
                    let current_str = current.to_string();
                    if !seen.contains(&current_str) {
                        seen.insert(current_str);
                        changed = true;
                    }
                }
            },
            TyKind::Protocol {
                symbol,
                substitutions,
            } => {
                let (new_subs, inner_changed) =
                    normalize_substitutions(&substitutions, equalities);
                if inner_changed {
                    current = Ty::generic_protocol(symbol, new_subs, current.span().clone());
                    let current_str = current.to_string();
                    if !seen.contains(&current_str) {
                        seen.insert(current_str);
                        changed = true;
                    }
                }
            },
            TyKind::TypeAlias {
                symbol,
                substitutions,
            } => {
                let (new_subs, inner_changed) =
                    normalize_substitutions(&substitutions, equalities);
                if inner_changed {
                    current = Ty::generic_type_alias(symbol, new_subs, current.span().clone());
                    let current_str = current.to_string();
                    if !seen.contains(&current_str) {
                        seen.insert(current_str);
                        changed = true;
                    }
                }
            },
            TyKind::AssociatedType { symbol, container } if container.is_some() => {
                let cont = container.as_ref().unwrap();
                let normalized_container = normalize_type_inner(cont, equalities);
                if normalized_container.to_string() != cont.to_string() {
                    current = Ty::qualified_associated_type(
                        symbol.clone(),
                        normalized_container,
                        current.span().clone(),
                    );
                    let current_str = current.to_string();
                    if !seen.contains(&current_str) {
                        seen.insert(current_str);
                        changed = true;
                    }
                }
            },
            _ => {},
        }
    }

    current
}

fn normalize_substitutions(
    substitutions: &Substitutions,
    equalities: &[(&Ty, &Ty)],
) -> (Substitutions, bool) {
    let mut new_subs = Substitutions::new();
    let mut inner_changed = false;

    for (param_id, ty) in substitutions.iter() {
        let normalized = normalize_type_inner(ty, equalities);
        if normalized.to_string() != ty.to_string() {
            inner_changed = true;
        }
        new_subs.insert(*param_id, normalized);
    }

    (new_subs, inner_changed)
}

/// Check if two types match for equality constraint purposes.
fn equality_types_match(a: &Ty, b: &Ty) -> bool {
    match (a.kind(), b.kind()) {
        (TyKind::TypeParameter(a_param), TyKind::TypeParameter(b_param)) => {
            a_param.metadata().id() == b_param.metadata().id()
        },
        (
            TyKind::AssociatedType {
                symbol: a_sym,
                container: a_cont,
            },
            TyKind::AssociatedType {
                symbol: b_sym,
                container: b_cont,
            },
        ) => {
            if a_sym.metadata().id() != b_sym.metadata().id() {
                return false;
            }
            match (a_cont, b_cont) {
                (Some(a_c), Some(b_c)) => equality_types_match(a_c, b_c),
                (None, None) => true,
                _ => false,
            }
        },
        // If one is a type param/associated type and other isn't, they don't match
        (TyKind::TypeParameter(_), _) | (_, TyKind::TypeParameter(_)) => false,
        (TyKind::AssociatedType { .. }, _) | (_, TyKind::AssociatedType { .. }) => false,

        _ => a.is_assignable_to(b) && b.is_assignable_to(a),
    }
}

/// Determine which type is more concrete for equality constraints.
fn equality_is_more_concrete(a: &Ty, b: &Ty) -> bool {
    // Avoid normalizing a type parameter to an associated type that depends on itself
    // (e.g., T.Output = T). Prefer the type parameter in that case to prevent
    // recursive expansion like T.Output.Output...
    if let (TyKind::AssociatedType { container: Some(cont), .. }, TyKind::TypeParameter(param)) =
        (a.kind(), b.kind())
    {
        if type_contains_param(cont, param.metadata().id()) {
            return false;
        }
    }
    if let (TyKind::TypeParameter(param), TyKind::AssociatedType { container: Some(cont), .. }) =
        (a.kind(), b.kind())
    {
        if type_contains_param(cont, param.metadata().id()) {
            return true;
        }
    }

    let a_score = equality_type_score(a);
    let b_score = equality_type_score(b);
    if a_score != b_score {
        a_score > b_score
    } else {
        // Tie-breaker: use Display string
        a.to_string() < b.to_string()
    }
}

/// Score a type for concreteness (higher = more concrete).
fn equality_type_score(ty: &Ty) -> i32 {
    match ty.kind() {
        TyKind::TypeParameter(_) => 0,
        TyKind::AssociatedType { .. } => 1,
        TyKind::SelfType => 2,
        TyKind::Protocol { .. } => 3,
        TyKind::Struct { .. } | TyKind::Enum { .. } | TyKind::TypeAlias { .. } => 4,
        TyKind::Int(_) | TyKind::Float(_) | TyKind::Bool | TyKind::String | TyKind::Unit => 5,
        TyKind::Tuple(_) | TyKind::Function { .. } | TyKind::Pointer(_) => 4,
        _ => -1, // Error, Never, Infer, UnresolvedFunction, UnresolvedPath
    }
}

/// Check whether a type contains a specific type parameter (recursively).
fn type_contains_param(ty: &Ty, param_id: SymbolId) -> bool {
    use kestrel_semantic_tree::ty::TyId;

    fn inner(ty: &Ty, param_id: SymbolId, visited: &mut HashSet<TyId>) -> bool {
        let ty_id = ty.id();
        if !visited.insert(ty_id) {
            return false;
        }

        match ty.kind() {
            TyKind::TypeParameter(tp) => tp.metadata().id() == param_id,
            TyKind::AssociatedType {
                container: Some(cont),
                ..
            } => inner(cont, param_id, visited),
            TyKind::Tuple(elements) => elements
                .iter()
                .any(|elem| inner(elem, param_id, visited)),
            TyKind::Pointer(element) => inner(element, param_id, visited),
            TyKind::Function {
                params,
                return_type,
            } => params
                .iter()
                .any(|p| inner(p, param_id, visited))
                || inner(return_type, param_id, visited),
            TyKind::Struct { substitutions, .. }
            | TyKind::Enum { substitutions, .. }
            | TyKind::Protocol { substitutions, .. }
            | TyKind::TypeAlias { substitutions, .. } => substitutions
                .iter()
                .any(|(_, sub_ty)| inner(sub_ty, param_id, visited)),
            _ => false,
        }
    }

    inner(ty, param_id, &mut HashSet::new())
}

#[cfg(test)]
mod tests {
    // Tests would require setting up a full SemanticModel, which is complex.
    // The TypeOracle implementation is tested indirectly through the type inference tests.
}
