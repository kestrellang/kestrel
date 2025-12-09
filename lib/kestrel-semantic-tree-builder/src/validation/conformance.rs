//! Validator for protocol conformance and inheritance
//!
//! This validator checks:
//! - Circular protocol inheritance (protocol A: B where protocol B: A)
//! - Conforming types implement all required methods
//! - Method signatures match protocol requirements

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_tree::behavior::callable::{CallableSignature, MethodLookupKey, SignatureType};
use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::cycle::CycleDetector;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::database::{Db, SemanticDatabase};
use crate::diagnostics::{
    AssociatedTypeConstraintNotSatisfiedError, CircularProtocolInheritanceError,
    MissingAssociatedTypeError, MissingProtocolMethodError, WrongMethodReturnTypeError,
};
use crate::syntax::get_file_id_for_symbol;
use crate::validation::{SymbolContext, Validator};

/// Validator that checks protocol conformance and inheritance rules
pub struct ConformanceValidator {
    /// Collected protocols during the walk
    protocols: Mutex<Vec<CollectedProtocol>>,
    /// Collected structs during the walk
    structs: Mutex<Vec<CollectedStruct>>,
}

/// Data collected for protocols
struct CollectedProtocol {
    symbol: Arc<dyn Symbol<KestrelLanguage>>,
    protocol: Arc<ProtocolSymbol>,
}

/// Data collected for structs
struct CollectedStruct {
    symbol: Arc<dyn Symbol<KestrelLanguage>>,
    struct_sym: Arc<StructSymbol>,
}

impl ConformanceValidator {
    const NAME: &'static str = "conformance";

    pub fn new() -> Self {
        Self {
            protocols: Mutex::new(Vec::new()),
            structs: Mutex::new(Vec::new()),
        }
    }
}

impl Default for ConformanceValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the resolved conformances from a symbol's ConformancesBehavior
fn get_conformances(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Vec<Ty> {
    symbol
        .conformances_behavior()
        .map(|cb| cb.conformances().to_vec())
        .unwrap_or_default()
}

/// Get the Arc<ProtocolSymbol> from a symbol's TypedBehavior
fn get_protocol_arc_from_symbol(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<Arc<ProtocolSymbol>> {
    symbol.typed_behavior().and_then(|tb| {
        if let TyKind::Protocol { symbol, .. } = tb.ty().kind() {
            Some(symbol.clone())
        } else {
            None
        }
    })
}

impl Validator for ConformanceValidator {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn validate_symbol(&self, ctx: &SymbolContext<'_>) {
        let kind = ctx.symbol.metadata().kind();

        // Collect protocols
        if kind == KestrelSymbolKind::Protocol {
            if let Some(protocol) = get_protocol_arc_from_symbol(ctx.symbol) {
                self.protocols.lock().unwrap().push(CollectedProtocol {
                    symbol: ctx.symbol.clone(),
                    protocol,
                });
            }
        }

        // Collect structs with conformances
        if kind == KestrelSymbolKind::Struct {
            if !get_conformances(ctx.symbol).is_empty() {
                if let Some(struct_sym) = ctx.symbol.clone().into_any_arc().downcast::<StructSymbol>().ok() {
                    self.structs.lock().unwrap().push(CollectedStruct {
                        symbol: ctx.symbol.clone(),
                        struct_sym,
                    });
                }
            }
        }
    }

    fn finalize(&self, db: &SemanticDatabase, diagnostics: &mut DiagnosticContext) {
        // Check protocols for circular inheritance
        for collected in self.protocols.lock().unwrap().iter() {
            check_circular_inheritance(&collected.protocol, &collected.symbol, db, diagnostics);
        }

        // Check protocols for associated type default satisfaction
        for collected in self.protocols.lock().unwrap().iter() {
            check_protocol_associated_type_defaults(&collected.protocol, &collected.symbol, diagnostics);
        }

        // Check structs for protocol conformance
        for collected in self.structs.lock().unwrap().iter() {
            check_struct_conformance(&collected.struct_sym, &collected.symbol, db, diagnostics);
        }

        // Link protocol methods for all structs
        // This happens AFTER all binding is complete, so method signatures are available
        for collected in self.structs.lock().unwrap().iter() {
            crate::resolvers::link_protocol_methods_for_struct(
                &collected.struct_sym,
                &collected.symbol,
                db,
                diagnostics,
            );
        }
    }
}

/// Check if a protocol has circular inheritance
fn check_circular_inheritance(
    protocol: &Arc<ProtocolSymbol>,
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    db: &SemanticDatabase,
    diagnostics: &mut DiagnosticContext,
) {
    let protocol_name = &protocol.metadata().name().value;

    // Use CycleDetector for consistent cycle detection
    let mut detector: CycleDetector<SymbolId> = CycleDetector::new();

    if let Some(cycle) = check_inheritance_cycle(protocol, &mut detector) {
        let span = protocol.metadata().declaration_span().clone();
        let file_id = get_file_id_for_symbol(symbol, diagnostics);

        // Build cycle path names for error message
        let cycle_names: Vec<String> = cycle
            .cycle()
            .iter()
            .filter_map(|&id| {
                db.symbol_by_id(id).map(|s| s.metadata().name().value.clone())
            })
            .collect();

        diagnostics.throw(
            CircularProtocolInheritanceError {
                span,
                protocol_name: protocol_name.to_string(),
                cycle: cycle_names,
            },
            file_id,
        );
    }
}

/// Recursively check for inheritance cycles using CycleDetector
fn check_inheritance_cycle(
    protocol: &Arc<ProtocolSymbol>,
    detector: &mut CycleDetector<SymbolId>,
) -> Option<semantic_tree::cycle::Cycle<SymbolId>> {
    let id = protocol.metadata().id();

    // Try to enter - if it fails, we found a cycle
    // Store the guard and forget it so we can manually call exit()
    let guard = match detector.enter(id) {
        Ok(guard) => guard,
        Err(cycle) => return Some(cycle),
    };
    std::mem::forget(guard);

    // Check all inherited protocols (via ConformancesBehavior)
    let protocol_dyn = protocol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
    for inherited_ty in get_conformances(&protocol_dyn) {
        if let TyKind::Protocol { symbol, .. } = inherited_ty.kind() {
            if let Some(cycle) = check_inheritance_cycle(symbol, detector) {
                detector.exit();
                return Some(cycle);
            }
        }
    }

    detector.exit();
    None
}

/// Check that a struct implements all required methods from its conformances
fn check_struct_conformance(
    struct_sym: &StructSymbol,
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    db: &SemanticDatabase,
    diagnostics: &mut DiagnosticContext,
) {
    let conformances = get_conformances(symbol);

    if conformances.is_empty() {
        return;
    }

    let struct_name = &struct_sym.metadata().name().value;
    let struct_id = struct_sym.metadata().id();
    let file_id = get_file_id_for_symbol(symbol, diagnostics);

    // Collect associated type bindings from the struct (e.g., type Item = Int)
    // We need to downcast from StructSymbol reference to get Arc<StructSymbol>
    let struct_arc = symbol
        .clone()
        .into_any_arc()
        .downcast::<StructSymbol>()
        .ok();
    let associated_type_bindings = struct_arc
        .as_ref()
        .map(|s| collect_associated_type_bindings(s))
        .unwrap_or_default();

    // Collect all methods implemented by the struct
    // Use MethodLookupKey (without return type) for lookup, then validate return type separately
    let struct_methods = collect_methods_from_symbol(symbol);
    let struct_method_map: HashMap<MethodLookupKey, (&Arc<FunctionSymbol>, SignatureType)> = struct_methods
        .iter()
        .map(|f| (f.signature().lookup_key(), (f, SignatureType::from_ty(&f.return_type()))))
        .collect();

    // Check each conformance
    for conformance_ty in &conformances {
        let (protocol_symbol, type_param_bindings) =
            match resolve_protocol_type(conformance_ty, struct_id, db) {
                Some(result) => result,
                None => continue,
            };

        let protocol_name = &protocol_symbol.metadata().name().value;

        // Collect all required associated types from the protocol (including defaults)
        let protocol_associated_types = collect_protocol_associated_types_with_defaults(&protocol_symbol);

        // Check each required associated type is provided by the struct
        // (unless it has a default in the protocol)
        for (type_name, default_type) in &protocol_associated_types {
            if default_type.is_none() && !associated_type_bindings.contains_key(type_name) {
                let span = struct_sym.metadata().declaration_span().clone();
                diagnostics.throw(
                    MissingAssociatedTypeError {
                        span,
                        struct_name: struct_name.clone(),
                        protocol_name: protocol_name.clone(),
                        type_name: type_name.clone(),
                    },
                    file_id,
                );
            }
        }

        // Create effective bindings: type params + struct bindings + protocol defaults + Self
        let mut effective_bindings = type_param_bindings; // Start with protocol type parameter substitutions
        // Add struct's associated type bindings
        for (name, binding) in &associated_type_bindings {
            effective_bindings.insert(name.clone(), binding.clone());
        }
        // Add protocol defaults for missing associated types
        for (type_name, default_type) in &protocol_associated_types {
            if !effective_bindings.contains_key(type_name) {
                if let Some(default) = default_type {
                    effective_bindings.insert(type_name.clone(), default.clone());
                }
            }
        }
        // Add Self -> struct type binding for protocol methods using Self type
        // We need to get the struct's actual type and convert it to SignatureType
        let self_type = symbol
            .typed_behavior()
            .map(|tb| SignatureType::from_ty(tb.ty()))
            .unwrap_or_else(|| SignatureType::Named(vec![struct_name.clone()]));
        effective_bindings.insert("Self".to_string(), self_type);

        // Collect all required methods from the protocol (including inherited)
        let required_methods = collect_all_protocol_methods(&protocol_symbol, db);

        // Check each required method
        for (protocol_sig, method) in &required_methods {
            let method_name = &method.metadata().name().value;
            let raw_return_type = SignatureType::from_ty(&method.return_type());
            // Substitute associated types with their bindings (including defaults)
            let required_return_type = substitute_associated_types(&raw_return_type, &effective_bindings);

            // Substitute associated types in the signature for lookup
            let substituted_sig = substitute_signature(protocol_sig, &effective_bindings);

            // Use lookup key (without return type) to find the method, then validate return type separately
            match struct_method_map.get(&substituted_sig.lookup_key()) {
                None => {
                    let span = struct_sym.metadata().declaration_span().clone();

                    diagnostics.throw(
                        MissingProtocolMethodError {
                            span,
                            struct_name: struct_name.clone(),
                            protocol_name: protocol_name.clone(),
                            method_name: method_name.clone(),
                        },
                        file_id,
                    );
                }
                Some((_struct_method, struct_return_type)) => {
                    if struct_return_type != &required_return_type {
                        let span = struct_sym.metadata().declaration_span().clone();

                        diagnostics.throw(
                            WrongMethodReturnTypeError {
                                span,
                                method_name: method_name.clone(),
                                protocol_name: protocol_name.clone(),
                                expected_type: format!("{:?}", required_return_type),
                                actual_type: format!("{:?}", struct_return_type),
                            },
                            file_id,
                        );
                    }
                }
            }
        }
    }
}

/// Resolve a Ty to a ProtocolSymbol and its substitutions if it's a protocol type
fn resolve_protocol_type(
    ty: &Ty,
    _context: SymbolId,
    _db: &SemanticDatabase,
) -> Option<(Arc<ProtocolSymbol>, HashMap<String, SignatureType>)> {
    match ty.kind() {
        TyKind::Protocol {
            symbol,
            substitutions,
        } => {
            // Build a map of type parameter name -> substituted type
            let mut type_param_bindings = HashMap::new();
            let type_params = symbol.type_parameters();
            for type_param in type_params.iter() {
                let param_id = type_param.metadata().id();
                if let Some(sub_ty) = substitutions.get(param_id) {
                    let param_name = type_param.metadata().name().value.clone();
                    type_param_bindings.insert(param_name, SignatureType::from_ty(sub_ty));
                }
            }
            Some((symbol.clone(), type_param_bindings))
        }
        _ => None,
    }
}

/// Collect all methods from a symbol (struct or protocol)
fn collect_methods_from_symbol(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Vec<Arc<FunctionSymbol>> {
    symbol
        .metadata()
        .children()
        .into_iter()
        .filter(|child| child.metadata().kind() == KestrelSymbolKind::Function)
        .filter_map(|child| child.into_any_arc().downcast::<FunctionSymbol>().ok())
        .collect()
}

/// Collect all associated types from a protocol, including their defaults if present
/// Returns a map from associated type name to optional default type
fn collect_protocol_associated_types_with_defaults(
    protocol: &Arc<ProtocolSymbol>,
) -> HashMap<String, Option<SignatureType>> {
    use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol;

    let protocol_dyn = protocol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
    let mut associated_types = HashMap::new();

    for child in protocol_dyn.metadata().children() {
        if child.metadata().kind() == KestrelSymbolKind::AssociatedType {
            if let Ok(assoc_type) = child.into_any_arc().downcast::<AssociatedTypeSymbol>() {
                let name = assoc_type.metadata().name().value.clone();
                let default_type = assoc_type.default_type().map(|ty| SignatureType::from_ty(&ty));
                associated_types.insert(name, default_type);
            }
        }
    }

    associated_types
}

/// Collect associated type bindings from a struct
/// Returns a map from associated type name to the resolved SignatureType
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

/// Substitute associated type names in a SignatureType using the struct's bindings
fn substitute_associated_types(
    sig_type: &SignatureType,
    bindings: &HashMap<String, SignatureType>,
) -> SignatureType {
    match sig_type {
        SignatureType::Named(path) if path.len() == 1 => {
            // Single-segment path might be an associated type
            if let Some(bound_type) = bindings.get(&path[0]) {
                bound_type.clone()
            } else {
                sig_type.clone()
            }
        }
        SignatureType::Tuple(elements) => {
            SignatureType::Tuple(
                elements
                    .iter()
                    .map(|e| substitute_associated_types(e, bindings))
                    .collect(),
            )
        }
        SignatureType::Array(element) => {
            SignatureType::Array(Box::new(substitute_associated_types(element, bindings)))
        }
        SignatureType::Function { params, return_type } => SignatureType::Function {
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

/// Collect all required methods from a protocol, including inherited protocols
fn collect_all_protocol_methods(
    protocol: &Arc<ProtocolSymbol>,
    db: &SemanticDatabase,
) -> HashMap<CallableSignature, Arc<FunctionSymbol>> {
    let mut methods = HashMap::new();
    let mut visited = HashSet::new();

    collect_protocol_methods_recursive(protocol, db, &mut methods, &mut visited);

    methods
}

/// Recursively collect methods from a protocol and its inherited protocols
fn collect_protocol_methods_recursive(
    protocol: &Arc<ProtocolSymbol>,
    db: &SemanticDatabase,
    methods: &mut HashMap<CallableSignature, Arc<FunctionSymbol>>,
    visited: &mut HashSet<SymbolId>,
) {
    let id = protocol.metadata().id();

    if visited.contains(&id) {
        return;
    }
    visited.insert(id);

    // First, collect methods from inherited protocols
    let protocol_dyn = protocol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
    for inherited_ty in get_conformances(&protocol_dyn) {
        if let Some((inherited_protocol, _)) = resolve_protocol_type(&inherited_ty, id, db) {
            collect_protocol_methods_recursive(&inherited_protocol, db, methods, visited);
        }
    }

    // Then collect methods from this protocol
    for method in collect_methods_from_symbol(&protocol_dyn) {
        let sig = method.signature();
        methods.insert(sig, method);
    }
}

/// Check that associated type defaults in protocols satisfy their bounds
fn check_protocol_associated_type_defaults(
    protocol: &Arc<ProtocolSymbol>,
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    diagnostics: &mut DiagnosticContext,
) {
    let protocol_dyn = protocol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
    let file_id = get_file_id_for_symbol(symbol, diagnostics);

    // Check each associated type in the protocol
    for child in protocol_dyn.metadata().children() {
        if child.metadata().kind() == KestrelSymbolKind::AssociatedType {
            if let Ok(assoc_type) = child.downcast_arc::<AssociatedTypeSymbol>() {
                // Get the bounds (if any)
                if let Some(bounds) = assoc_type.bounds() {
                    // Get the default type (if any)
                    if let Some(default_type) = assoc_type.default_type() {
                        // Validate that the default satisfies the bounds
                        validate_type_satisfies_protocol_bounds(
                            &default_type,
                            &bounds,
                            &assoc_type.metadata().name().value,
                            assoc_type.metadata().span().clone(),
                            file_id,
                            diagnostics,
                        );
                    }
                }
            }
        }
    }
}

/// Helper function to validate that a type satisfies protocol bounds
fn validate_type_satisfies_protocol_bounds(
    bound_type: &Ty,
    required_bounds: &[Ty],
    type_name: &str,
    span: kestrel_span::Span,
    file_id: usize,
    diagnostics: &mut DiagnosticContext,
) {
    // Get the type name for error messages
    let bound_type_name = match bound_type.kind() {
        TyKind::Struct { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::Protocol { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::TypeAlias { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::Error { .. } => return, // Skip error types
        _ => format!("{:?}", bound_type.kind()),
    };

    // For each required protocol bound, check if the type conforms to it
    for required_protocol in required_bounds {
        // Skip error bounds
        if matches!(required_protocol.kind(), TyKind::Error { .. }) {
            continue;
        }

        if let TyKind::Protocol { symbol: required_proto_symbol, .. } = required_protocol.kind() {
            let required_protocol_name = required_proto_symbol.metadata().name().value.clone();

            // Check if the bound type conforms to this protocol
            let conforms = match bound_type.kind() {
                TyKind::Struct { symbol, .. } => {
                    // Get the struct's conformances
                    let conformances = symbol
                        .conformances_behavior()
                        .map(|cb| cb.conformances().to_vec())
                        .unwrap_or_default();

                    // Check if any conformance matches the required protocol (by ID)
                    conformances.iter().any(|conf| {
                        if let TyKind::Protocol { symbol: proto_sym, .. } = conf.kind() {
                            proto_sym.metadata().id() == required_proto_symbol.metadata().id()
                        } else {
                            false
                        }
                    })
                }
                TyKind::TypeParameter(_) => true, // Type parameters might conform through bounds
                TyKind::Error { .. } => true,      // Don't report additional errors
                _ => false,                        // Other types don't have conformances
            };

            if !conforms {
                diagnostics.throw(
                    AssociatedTypeConstraintNotSatisfiedError {
                        span,
                        type_name: type_name.to_string(),
                        bound_type: bound_type_name.clone(),
                        required_protocol: required_protocol_name,
                    },
                    file_id,
                );
                return; // Only report the first violation
            }
        }
    }
}
