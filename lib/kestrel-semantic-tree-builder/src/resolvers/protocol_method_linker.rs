//! Protocol Method Linker
//!
//! This module implements linking between struct methods and protocol methods.
//! During the BIND phase, after conformances are resolved, we match struct methods
//! to protocol methods based on:
//! - Method name
//! - Parameter labels
//! - Parameter types (with Self/associated type substitution)
//! - Return type (with substitution)
//! - Receiver kind (static, mutating, consuming, instance)
//!
//! A method can implement at most ONE protocol method. If a method would satisfy
//! multiple protocol requirements, it's considered ambiguous and an error is reported.

use std::collections::HashMap;
use std::sync::Arc;

use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::callable::{CallableSignature, ReceiverKind, SignatureType};
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::implements::ImplementsBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::symbol::{Symbol, SymbolId};

use kestrel_semantic_model::{ExtensionsFor, SemanticModel};

use crate::diagnostics::{AmbiguousProtocolMethodError, ProtocolMethodReceiverMismatchError};
use crate::syntax::get_file_id_for_symbol;

/// Link struct methods to protocol methods based on signature matching
///
/// This function should be called during the BIND phase, after:
/// - Conformances have been resolved
/// - Method signatures are available
/// - Type substitutions can be performed
pub fn link_protocol_methods_for_struct(
    struct_sym: &Arc<StructSymbol>,
    struct_dyn: &Arc<dyn Symbol<KestrelLanguage>>,
    model: &SemanticModel,
    diagnostics: &mut DiagnosticContext,
) {
    let file_id = get_file_id_for_symbol(struct_dyn, diagnostics);
    let struct_name = &struct_sym.metadata().name().value;
    let struct_id = struct_sym.metadata().id();

    let mut conformances = struct_dyn
        .metadata()
        .get_behavior::<ConformancesBehavior>()
        .map(|cb| cb.conformances().to_vec())
        .unwrap_or_default();

    // Also collect conformances from extensions
    let extensions = model.query(ExtensionsFor {
        target_id: struct_id,
    });
    for extension in &extensions {
        let extension_conformances = extension
            .metadata()
            .get_behavior::<ConformancesBehavior>()
            .map(|cb| cb.conformances().to_vec())
            .unwrap_or_default();
        conformances.extend(extension_conformances);
    }

    if conformances.is_empty() {
        return;
    }

    // Collect all protocol methods with substitutions
    // The tuple contains: (defining_protocol, method, substituted_signature, bindings)
    let mut protocol_methods: Vec<(
        Arc<ProtocolSymbol>,
        Arc<FunctionSymbol>,
        CallableSignature,
        HashMap<String, SignatureType>,
    )> = Vec::new();

    for conformance_ty in &conformances {
        if let Some((_conforming_protocol, bindings)) =
            resolve_protocol_type(conformance_ty, struct_dyn, struct_name)
        {
            // Get all protocol methods (including inherited), each paired with its defining protocol
            let methods = collect_all_protocol_methods(&_conforming_protocol, model);

            for (defining_protocol, method) in methods {
                let sig = method.signature();
                // Substitute with type parameters, associated types, and Self
                let substituted_sig = substitute_signature(&sig, &bindings);
                // Use defining_protocol (not conforming_protocol) so methods are attributed correctly
                protocol_methods.push((
                    defining_protocol,
                    method,
                    substituted_sig,
                    bindings.clone(),
                ));
            }
        }
    }

    // Collect struct methods
    let mut all_methods = collect_methods_from_symbol(struct_dyn);

    // Also collect methods from applicable extensions
    let struct_id = struct_sym.metadata().id();
    let extensions = model.query(ExtensionsFor {
        target_id: struct_id,
    });
    for extension in extensions {
        // TODO: Filter by applicability (check type arguments and where clauses)
        // For now, include all extensions since filtering can cause stack overflow
        let extension_methods =
            collect_methods_from_symbol(&(extension.clone() as Arc<dyn Symbol<KestrelLanguage>>));
        all_methods.extend(extension_methods);
    }

    // For each method (struct or extension), find matching protocol methods
    for struct_method in &all_methods {
        let struct_sig = struct_method.signature();
        let method_name = &struct_method.metadata().name().value;
        let method_span = struct_method.metadata().declaration_span().clone();

        let mut matches: Vec<(Arc<ProtocolSymbol>, Arc<FunctionSymbol>)> = Vec::new();

        // Check against all protocol methods
        for (protocol, protocol_method, substituted_sig, _bindings) in &protocol_methods {
            if signatures_match(&struct_sig, substituted_sig) {
                // Signature matches, now check receiver kind
                let struct_receiver = struct_method
                    .metadata()
                    .get_behavior::<CallableBehavior>()
                    .and_then(|cb| cb.receiver());
                let protocol_receiver = protocol_method
                    .metadata()
                    .get_behavior::<CallableBehavior>()
                    .and_then(|cb| cb.receiver());

                if receivers_match(&struct_receiver, &protocol_receiver) {
                    matches.push((protocol.clone(), protocol_method.clone()));
                } else {
                    // Receiver kind mismatch - report error
                    let protocol_name = protocol.metadata().name().value.clone();
                    diagnostics.throw(ProtocolMethodReceiverMismatchError {
                        span: method_span.clone(),
                        method_name: method_name.clone(),
                        protocol_name,
                        expected_receiver: receiver_kind_to_string(&protocol_receiver),
                        actual_receiver: receiver_kind_to_string(&struct_receiver),
                    });
                }
            }
        }

        // Check for ambiguity
        if matches.len() > 1 {
            let protocol_names: Vec<String> = matches
                .iter()
                .map(|(p, _)| p.metadata().name().value.clone())
                .collect();

            diagnostics.throw(AmbiguousProtocolMethodError {
                span: method_span,
                method_name: method_name.clone(),
                protocols: protocol_names,
            });
        } else if matches.len() == 1 {
            // Exactly one match - add ImplementsBehavior
            let (protocol, protocol_method) = &matches[0];
            let implements_behavior =
                ImplementsBehavior::new(protocol.metadata().id(), protocol_method.metadata().id());

            struct_method.metadata().add_behavior(implements_behavior);
        }
        // If no matches, that's fine - method doesn't implement any protocol method
    }
}

/// Resolve a protocol type to its symbol and type parameter bindings
fn resolve_protocol_type(
    ty: &Ty,
    struct_dyn: &Arc<dyn Symbol<KestrelLanguage>>,
    struct_name: &str,
) -> Option<(Arc<ProtocolSymbol>, HashMap<String, SignatureType>)> {
    match ty.kind() {
        TyKind::Protocol {
            symbol,
            substitutions,
        } => {
            // Build a map of type parameter name -> substituted type
            let mut bindings = HashMap::new();
            let type_params = symbol.type_parameters();
            for type_param in type_params.iter() {
                let param_id = type_param.metadata().id();
                if let Some(sub_ty) = substitutions.get(param_id) {
                    let param_name = type_param.metadata().name().value.clone();
                    bindings.insert(param_name, SignatureType::from_ty(sub_ty));
                }
            }

            // Add Self -> struct type binding
            let self_type = struct_dyn
                .metadata()
                .get_behavior::<TypedBehavior>()
                .map(|tb| SignatureType::from_ty(tb.ty()))
                .unwrap_or_else(|| SignatureType::Named(vec![struct_name.to_string()]));
            bindings.insert("Self".to_string(), self_type);

            // Add associated type bindings from the struct
            if let Ok(struct_sym) = struct_dyn.clone().into_any_arc().downcast::<StructSymbol>() {
                let assoc_bindings = collect_associated_type_bindings(&struct_sym);
                for (name, sig_type) in assoc_bindings {
                    bindings.insert(name, sig_type);
                }
            }

            Some((symbol.clone(), bindings))
        }
        _ => None,
    }
}

/// Collect associated type bindings from a struct
fn collect_associated_type_bindings(
    struct_sym: &Arc<StructSymbol>,
) -> HashMap<String, SignatureType> {
    use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
    use kestrel_semantic_tree::symbol::type_alias::{TypeAliasSymbol, TypeAliasTypedBehavior};

    let struct_dyn = struct_sym.clone() as Arc<dyn Symbol<KestrelLanguage>>;
    let mut bindings = HashMap::new();

    for child in struct_dyn.metadata().children() {
        if child.metadata().kind() == KestrelSymbolKind::TypeAlias {
            if let Ok(type_alias) = child.into_any_arc().downcast::<TypeAliasSymbol>() {
                let name = type_alias.metadata().name().value.clone();

                // Get the resolved type from TypeAliasTypedBehavior
                let resolved_ty = type_alias
                    .metadata()
                    .behaviors()
                    .iter()
                    .find(|b| b.kind() == KestrelBehaviorKind::TypeAliasTyped)
                    .and_then(|b| b.as_ref().downcast_ref::<TypeAliasTypedBehavior>())
                    .map(|tb| SignatureType::from_ty(tb.resolved_ty()));

                if let Some(sig_type) = resolved_ty {
                    bindings.insert(name, sig_type);
                }
            }
        }
    }

    bindings
}

/// Collect all methods from a symbol
fn collect_methods_from_symbol(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> Vec<Arc<FunctionSymbol>> {
    symbol
        .metadata()
        .children()
        .into_iter()
        .filter(|child| child.metadata().kind() == KestrelSymbolKind::Function)
        .filter_map(|child| child.into_any_arc().downcast::<FunctionSymbol>().ok())
        .collect()
}

/// Collect all methods from a protocol, including inherited protocols
/// Returns (defining_protocol, method) pairs to track which protocol defined each method
fn collect_all_protocol_methods(
    protocol: &Arc<ProtocolSymbol>,
    model: &SemanticModel,
) -> Vec<(Arc<ProtocolSymbol>, Arc<FunctionSymbol>)> {
    use std::collections::HashSet;

    let mut methods = Vec::new();
    let mut visited = HashSet::new();

    collect_protocol_methods_recursive(protocol, model, &mut methods, &mut visited);

    methods
}

/// Recursively collect methods from a protocol and its inherited protocols
/// Each method is paired with the protocol that defines it
fn collect_protocol_methods_recursive(
    protocol: &Arc<ProtocolSymbol>,
    model: &SemanticModel,
    methods: &mut Vec<(Arc<ProtocolSymbol>, Arc<FunctionSymbol>)>,
    visited: &mut std::collections::HashSet<SymbolId>,
) {
    let id = protocol.metadata().id();

    if visited.contains(&id) {
        return;
    }
    visited.insert(id);

    // First, collect methods from inherited protocols
    let protocol_dyn = protocol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
    if let Some(conformances_behavior) = protocol_dyn
        .metadata()
        .get_behavior::<ConformancesBehavior>()
    {
        for inherited_ty in conformances_behavior.conformances() {
            if let TyKind::Protocol { symbol, .. } = inherited_ty.kind() {
                collect_protocol_methods_recursive(symbol, model, methods, visited);
            }
        }
    }

    // Then collect methods from this protocol (paired with this protocol)
    for method in collect_methods_from_symbol(&protocol_dyn) {
        methods.push((protocol.clone(), method));
    }
}

/// Check if two signatures match (name, labels, parameter types)
fn signatures_match(sig1: &CallableSignature, sig2: &CallableSignature) -> bool {
    sig1 == sig2
}

/// Check if two receiver kinds are compatible
fn receivers_match(receiver1: &Option<ReceiverKind>, receiver2: &Option<ReceiverKind>) -> bool {
    receiver1 == receiver2
}

/// Convert receiver kind to string for error messages
fn receiver_kind_to_string(receiver: &Option<ReceiverKind>) -> String {
    match receiver {
        None => "static".to_string(),
        Some(ReceiverKind::Borrowing) => "instance".to_string(),
        Some(ReceiverKind::Mutating) => "mutating".to_string(),
        Some(ReceiverKind::Consuming) => "consuming".to_string(),
        Some(ReceiverKind::Initializing) => "initializing".to_string(),
    }
}

/// Substitute associated types in a CallableSignature
fn substitute_signature(
    sig: &CallableSignature,
    bindings: &HashMap<String, SignatureType>,
) -> CallableSignature {
    CallableSignature {
        name: sig.name.clone(),
        labels: sig.labels.clone(),
        param_types: sig
            .param_types
            .iter()
            .map(|t| substitute_associated_types(t, bindings))
            .collect(),
        return_type: substitute_associated_types(&sig.return_type, bindings),
    }
}

/// Substitute associated type names in a SignatureType using the bindings
fn substitute_associated_types(
    sig_type: &SignatureType,
    bindings: &HashMap<String, SignatureType>,
) -> SignatureType {
    match sig_type {
        SignatureType::Named(path) if path.len() == 1 => {
            // Single-segment path might be an associated type or Self
            if let Some(bound_type) = bindings.get(&path[0]) {
                bound_type.clone()
            } else {
                sig_type.clone()
            }
        }
        SignatureType::Tuple(elements) => SignatureType::Tuple(
            elements
                .iter()
                .map(|e| substitute_associated_types(e, bindings))
                .collect(),
        ),
        SignatureType::Array(element) => {
            SignatureType::Array(Box::new(substitute_associated_types(element, bindings)))
        }
        SignatureType::Function {
            params,
            return_type,
        } => SignatureType::Function {
            params: params
                .iter()
                .map(|p| substitute_associated_types(p, bindings))
                .collect(),
            return_type: Box::new(substitute_associated_types(return_type, bindings)),
        },
        // For other types, return as-is
        _ => sig_type.clone(),
    }
}
