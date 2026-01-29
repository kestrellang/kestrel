use std::collections::HashMap;
use std::sync::Arc;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use kestrel_semantic_model::{
    AssociatedTypeBindingsForEnum, AssociatedTypeBindingsForStruct, ConformancesForSymbol,
    ExtensionsFor, PropertyRequirement, ProtocolAssociatedTypesWithDefaults,
    ProtocolInitializersWithDefiner, ProtocolMethodsWithDefiner, ProtocolRequiredMethods,
    ProtocolRequiredProperties, SemanticModel, StructFields, SymbolFor,
};
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::callable::{CallableSignature, ReceiverKind, SignatureType};
use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;
use kestrel_semantic_tree::symbol::field::FieldSymbol;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::cycle::CycleDetector;
use semantic_tree::symbol::{Symbol, SymbolId};

mod diagnostics;
use diagnostics::*;

pub struct ConformanceAnalyzer {
    protocols: Vec<(Arc<dyn Symbol<KestrelLanguage>>, Arc<ProtocolSymbol>)>,
    structs: Vec<(Arc<dyn Symbol<KestrelLanguage>>, Arc<StructSymbol>)>,
    enums: Vec<(Arc<dyn Symbol<KestrelLanguage>>, Arc<EnumSymbol>)>,
}

impl ConformanceAnalyzer {
    pub fn new() -> Self {
        Self {
            protocols: Vec::new(),
            structs: Vec::new(),
            enums: Vec::new(),
        }
    }
}

impl Default for ConformanceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ConformanceAnalyzer {
    fn name(&self) -> &'static str {
        "conformance"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        match symbol.metadata().kind() {
            KestrelSymbolKind::Protocol => {
                if let Some(protocol) = get_protocol_arc_from_symbol(symbol) {
                    self.protocols.push((symbol.clone(), protocol));
                }
            },
            KestrelSymbolKind::Struct => {
                let conformances = ctx.model.query(ConformancesForSymbol {
                    symbol_id: symbol.metadata().id(),
                });
                if !conformances.is_empty()
                    && let Ok(struct_sym) = symbol.clone().into_any_arc().downcast::<StructSymbol>()
                {
                    self.structs.push((symbol.clone(), struct_sym));
                }
            },
            KestrelSymbolKind::Enum => {
                let conformances = ctx.model.query(ConformancesForSymbol {
                    symbol_id: symbol.metadata().id(),
                });
                if !conformances.is_empty()
                    && let Ok(enum_sym) = symbol.clone().into_any_arc().downcast::<EnumSymbol>()
                {
                    self.enums.push((symbol.clone(), enum_sym));
                }
            },
            _ => {},
        }
    }

    fn finalize(&mut self, ctx: &mut AnalysisContext) {
        // Circular inheritance + protocol defaults
        for (sym, proto) in &self.protocols {
            check_circular_inheritance(proto, sym, ctx.model, ctx);
            check_protocol_associated_type_defaults(proto, sym, ctx);
        }

        // Also consider structs and enums that gain conformances via extensions
        let mut extra_structs = Vec::new();
        let mut extra_enums = Vec::new();
        for extension in ctx.model.extension_registry().all_extensions() {
            let conformances = ctx.model.query(ConformancesForSymbol {
                symbol_id: extension.metadata().id(),
            });
            if conformances.is_empty() {
                continue;
            }
            if let Some(target_ty) = extension.target_type() {
                match target_ty.kind() {
                    TyKind::Struct { symbol: s, .. } => {
                        let id = s.metadata().id();
                        let already = self.structs.iter().any(|(_, ss)| ss.metadata().id() == id);
                        if !already {
                            extra_structs
                                .push((s.clone() as Arc<dyn Symbol<KestrelLanguage>>, s.clone()));
                        }
                    },
                    TyKind::Enum { symbol: e, .. } => {
                        let id = e.metadata().id();
                        let already = self.enums.iter().any(|(_, es)| es.metadata().id() == id);
                        if !already {
                            extra_enums
                                .push((e.clone() as Arc<dyn Symbol<KestrelLanguage>>, e.clone()));
                        }
                    },
                    _ => {},
                }
            }
        }
        self.structs.extend(extra_structs);
        self.enums.extend(extra_enums);

        // Conformance checks
        for (dyn_sym, struct_sym) in &self.structs {
            check_struct_conformance(struct_sym, dyn_sym, ctx.model, ctx);
        }

        // Conformance checks for enums
        for (dyn_sym, enum_sym) in &self.enums {
            check_enum_conformance(enum_sym, dyn_sym, ctx.model, ctx);
        }

        // Link protocol methods for all structs
        for (dyn_sym, struct_sym) in &self.structs {
            link_protocol_methods_for_struct(struct_sym, dyn_sym, ctx.model, ctx);
        }

        // Link protocol methods for all enums
        for (dyn_sym, enum_sym) in &self.enums {
            link_protocol_methods_for_enum(enum_sym, dyn_sym, ctx.model, ctx);
        }

        // Link protocol initializers for all structs
        for (dyn_sym, struct_sym) in &self.structs {
            link_protocol_initializers_for_struct(struct_sym, dyn_sym, ctx.model, ctx);
        }
    }
}

// Helpers adapted from builder validator/linker

fn get_protocol_arc_from_symbol(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> Option<Arc<ProtocolSymbol>> {
    symbol
        .metadata()
        .get_behavior::<TypedBehavior>()
        .and_then(|tb| match tb.ty().kind() {
            TyKind::Protocol { symbol, .. } => Some(symbol.clone()),
            _ => None,
        })
}

fn check_circular_inheritance(
    protocol: &Arc<ProtocolSymbol>,
    _symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    model: &SemanticModel,
    ctx: &mut AnalysisContext,
) {
    let mut detector: CycleDetector<SymbolId> = CycleDetector::new();
    if let Some(cycle) = check_inheritance_cycle(protocol, model, &mut detector) {
        let span = protocol.metadata().declaration_span().clone();
        let cycle_names: Vec<String> = cycle
            .cycle()
            .iter()
            .filter_map(|&id| {
                model
                    .query(SymbolFor { id })
                    .map(|s| s.metadata().name().value.clone())
            })
            .collect();
        let protocol_name = protocol.metadata().name().value.clone();
        ctx.report(CircularProtocolInheritanceError {
            span,
            protocol_name,
            cycle: cycle_names,
        });
    }
}

fn check_inheritance_cycle(
    protocol: &Arc<ProtocolSymbol>,
    model: &SemanticModel,
    detector: &mut CycleDetector<SymbolId>,
) -> Option<semantic_tree::cycle::Cycle<SymbolId>> {
    let id = protocol.metadata().id();
    if let Err(cycle) = detector.enter(id) {
        return Some(cycle);
    }
    for inherited_ty in model.query(ConformancesForSymbol { symbol_id: id }) {
        if let TyKind::Protocol { symbol, .. } = inherited_ty.kind()
            && let Some(c) = check_inheritance_cycle(symbol, model, detector)
        {
            detector.exit();
            return Some(c);
        }
    }
    detector.exit();
    None
}

fn check_struct_conformance(
    struct_sym: &Arc<StructSymbol>,
    dyn_sym: &Arc<dyn Symbol<KestrelLanguage>>,
    model: &SemanticModel,
    ctx: &mut AnalysisContext,
) {
    let struct_name = &struct_sym.metadata().name().value;
    let struct_id = struct_sym.metadata().id();

    let mut conformances = model.query(ConformancesForSymbol {
        symbol_id: dyn_sym.metadata().id(),
    });
    let extensions = model.query(ExtensionsFor {
        target_id: struct_id,
    });
    for extension in &extensions {
        let ext_confs = model.query(ConformancesForSymbol {
            symbol_id: extension.metadata().id(),
        });
        conformances.extend(ext_confs);
    }
    if conformances.is_empty() {
        return;
    }

    let associated_type_bindings = model.query(AssociatedTypeBindingsForStruct { struct_id });

    // Compute self_type first - this is used for Self substitution in struct method signatures
    let self_type = dyn_sym
        .metadata()
        .get_behavior::<TypedBehavior>()
        .map(|tb| SignatureType::from_ty(tb.ty()))
        .unwrap_or_else(|| SignatureType::Named(vec![struct_name.clone()]));

    let mut all_methods = collect_methods_from_symbol(dyn_sym);
    let extensions = model.query(ExtensionsFor {
        target_id: struct_id,
    });
    for extension in extensions {
        let methods =
            collect_methods_from_symbol(&(extension.clone() as Arc<dyn Symbol<KestrelLanguage>>));
        all_methods.extend(methods);
    }

    // Build struct method map with Self substituted to the concrete type
    let mut self_bindings = HashMap::new();
    self_bindings.insert("Self".to_string(), self_type.clone());

    let struct_method_map: HashMap<MethodLookupKey, (Arc<FunctionSymbol>, SignatureType)> =
        all_methods
            .iter()
            .map(|f| {
                // Substitute Self in the signature and return type
                let substituted_sig = substitute_signature(&f.signature(), &self_bindings);
                let raw_return = SignatureType::from_ty(&f.return_type());
                let substituted_return = substitute_associated_types(&raw_return, &self_bindings);
                (
                    substituted_sig.lookup_key(),
                    (f.clone(), substituted_return),
                )
            })
            .collect();

    for conformance_ty in &conformances {
        let (protocol_symbol, type_param_bindings) = match resolve_protocol_type(conformance_ty) {
            Some(r) => r,
            None => continue,
        };
        let protocol_name = &protocol_symbol.metadata().name().value;

        let protocol_associated_types = model.query(ProtocolAssociatedTypesWithDefaults {
            protocol_id: protocol_symbol.metadata().id(),
        });
        for (type_name, default_type) in &protocol_associated_types {
            if default_type.is_none() && !associated_type_bindings.contains_key(type_name) {
                let span = struct_sym.metadata().declaration_span().clone();
                ctx.report(MissingAssociatedTypeError {
                    span,
                    struct_name: struct_name.clone(),
                    protocol_name: protocol_name.clone(),
                    type_name: type_name.clone(),
                });
            }
        }

        let mut effective_bindings = type_param_bindings;
        for (name, binding) in &associated_type_bindings {
            effective_bindings.insert(name.clone(), binding.clone());
        }
        for (type_name, default_type) in &protocol_associated_types {
            if !effective_bindings.contains_key(type_name)
                && let Some(default) = default_type
            {
                effective_bindings.insert(type_name.clone(), default.clone());
            }
        }
        // Use the already-computed self_type
        effective_bindings.insert("Self".to_string(), self_type.clone());

        let required_methods = model.query(ProtocolRequiredMethods {
            protocol_id: protocol_symbol.metadata().id(),
        });
        for (protocol_sig, method) in &required_methods {
            let method_name = &method.metadata().name().value;
            let raw_return_type = SignatureType::from_ty(&method.return_type());
            let required_return_type =
                substitute_associated_types(&raw_return_type, &effective_bindings);
            let substituted_sig = substitute_signature(protocol_sig, &effective_bindings);
            match struct_method_map.get(&substituted_sig.lookup_key()) {
                None => {
                    let span = struct_sym.metadata().declaration_span().clone();
                    ctx.report(MissingProtocolMethodError {
                        span,
                        struct_name: struct_name.clone(),
                        protocol_name: protocol_name.clone(),
                        method_name: method_name.clone(),
                    });
                },
                Some((_struct_method, struct_return_type)) => {
                    if struct_return_type != &required_return_type {
                        let span = struct_sym.metadata().declaration_span().clone();
                        ctx.report(WrongMethodReturnTypeError {
                            span,
                            method_name: method_name.clone(),
                            protocol_name: protocol_name.clone(),
                            expected_type: format!("{:?}", required_return_type),
                            actual_type: format!("{:?}", struct_return_type),
                        });
                    }
                },
            }
        }

        // Check property requirements
        let required_properties = model.query(ProtocolRequiredProperties {
            protocol_id: protocol_symbol.metadata().id(),
        });
        if !required_properties.is_empty() {
            check_property_requirements(
                struct_sym,
                dyn_sym,
                struct_name,
                protocol_name,
                &required_properties,
                model,
                ctx,
            );
        }
    }
}

use kestrel_semantic_tree::behavior::callable::MethodLookupKey;

fn resolve_protocol_type(ty: &Ty) -> Option<(Arc<ProtocolSymbol>, HashMap<String, SignatureType>)> {
    match ty.kind() {
        TyKind::Protocol {
            symbol,
            substitutions,
        } => {
            let mut type_param_bindings = HashMap::new();
            let type_params = symbol.type_parameters();
            for type_param in type_params.iter() {
                let param_id = type_param.metadata().id();
                let param_name = type_param.metadata().name().value.clone();

                // Prefer explicit substitution unless it's an inferred placeholder (`_`).
                if let Some(sub_ty) = substitutions.get(param_id)
                    && !matches!(sub_ty.kind(), TyKind::Infer)
                {
                    type_param_bindings.insert(param_name, SignatureType::from_ty(sub_ty));
                    continue;
                }

                // Otherwise, apply the type parameter's default if present (e.g. Rhs = Self).
                if let Some(default_ty) = type_param.default() {
                    type_param_bindings.insert(param_name, SignatureType::from_ty(default_ty));
                }
            }
            Some((symbol.clone(), type_param_bindings))
        },
        _ => None,
    }
}

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

// Check protocol associated type default bounds satisfaction
fn check_protocol_associated_type_defaults(
    protocol: &Arc<ProtocolSymbol>,
    _symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    ctx: &mut AnalysisContext,
) {
    let protocol_dyn = protocol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
    for child in protocol_dyn.metadata().children() {
        if child.metadata().kind() == KestrelSymbolKind::AssociatedType
            && let Ok(assoc_type) = child.downcast_arc::<kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol>()
                && let Some(bounds) = assoc_type.bounds()
                    && let Some(default_type) = assoc_type.default_type() {
                        validate_type_satisfies_protocol_bounds(
                            &default_type,
                            &bounds,
                            &assoc_type.metadata().name().value,
                            assoc_type.metadata().span().clone(),
                            ctx,
                        );
                    }
    }
}

fn validate_type_satisfies_protocol_bounds(
    bound_type: &Ty,
    required_bounds: &[Ty],
    type_name: &str,
    span: kestrel_span::Span,
    ctx: &mut AnalysisContext,
) {
    let bound_type_name = match bound_type.kind() {
        TyKind::Struct { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::Protocol { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::TypeAlias { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::Error => return,
        _ => format!("{:?}", bound_type.kind()),
    };

    for required_protocol in required_bounds {
        if matches!(required_protocol.kind(), TyKind::Error) {
            continue;
        }
        if let TyKind::Protocol {
            symbol: required_proto_symbol,
            ..
        } = required_protocol.kind()
        {
            let required_protocol_name = required_proto_symbol.metadata().name().value.clone();
            let conforms = match bound_type.kind() {
                TyKind::Struct { symbol, .. } => {
                    let conformances = ctx.model.query(ConformancesForSymbol {
                        symbol_id: symbol.metadata().id(),
                    });
                    conformances.iter().any(|conf| {
                        if let TyKind::Protocol {
                            symbol: proto_sym, ..
                        } = conf.kind()
                        {
                            proto_sym.metadata().id() == required_proto_symbol.metadata().id()
                        } else {
                            false
                        }
                    })
                },
                TyKind::TypeParameter(_) => true,
                TyKind::Error => true,
                _ => false,
            };
            if !conforms {
                ctx.report(AssociatedTypeConstraintNotSatisfiedError {
                    span,
                    type_name: type_name.to_string(),
                    bound_type: bound_type_name.clone(),
                    required_protocol: required_protocol_name,
                });
                return;
            }
        }
    }
}

fn substitute_associated_types(
    sig_type: &SignatureType,
    bindings: &HashMap<String, SignatureType>,
) -> SignatureType {
    substitute_associated_types_recursive(sig_type, bindings, 0)
}

fn substitute_associated_types_recursive(
    sig_type: &SignatureType,
    bindings: &HashMap<String, SignatureType>,
    depth: usize,
) -> SignatureType {
    // Prevent infinite recursion (handles cycles like A -> B -> A)
    const MAX_DEPTH: usize = 10;
    if depth >= MAX_DEPTH {
        return sig_type.clone();
    }

    match sig_type {
        SignatureType::Named(path) if path.len() == 1 => {
            if let Some(replacement) = bindings.get(&path[0]) {
                // Recursively substitute the replacement to handle chains like Rhs -> Self -> UInt8
                substitute_associated_types_recursive(replacement, bindings, depth + 1)
            } else {
                sig_type.clone()
            }
        },
        SignatureType::Tuple(elements) => SignatureType::Tuple(
            elements
                .iter()
                .map(|e| substitute_associated_types_recursive(e, bindings, depth))
                .collect(),
        ),
        SignatureType::Array(element) => SignatureType::Array(Box::new(
            substitute_associated_types_recursive(element, bindings, depth),
        )),
        SignatureType::Function {
            params,
            return_type,
        } => SignatureType::Function {
            params: params
                .iter()
                .map(|p| substitute_associated_types_recursive(p, bindings, depth))
                .collect(),
            return_type: Box::new(substitute_associated_types_recursive(
                return_type,
                bindings,
                depth,
            )),
        },
        _ => sig_type.clone(),
    }
}

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

// Protocol method linker (adapted from builder)
fn link_protocol_methods_for_struct(
    struct_sym: &Arc<StructSymbol>,
    struct_dyn: &Arc<dyn Symbol<KestrelLanguage>>,
    model: &SemanticModel,
    ctx: &mut AnalysisContext,
) {
    let struct_name = &struct_sym.metadata().name().value;
    let struct_id = struct_sym.metadata().id();

    let mut conformances = model.query(ConformancesForSymbol {
        symbol_id: struct_dyn.metadata().id(),
    });
    let extensions = model.query(ExtensionsFor {
        target_id: struct_id,
    });
    for extension in &extensions {
        let ext_confs = model.query(ConformancesForSymbol {
            symbol_id: extension.metadata().id(),
        });
        conformances.extend(ext_confs);
    }
    if conformances.is_empty() {
        return;
    }

    let mut protocol_methods: Vec<(
        Arc<ProtocolSymbol>,
        Arc<FunctionSymbol>,
        CallableSignature,
        HashMap<String, SignatureType>,
        SignatureType, // conformance_signature
    )> = Vec::new();
    for conformance_ty in &conformances {
        if let Some((conforming_protocol, bindings)) =
            resolve_protocol_type_for_link(conformance_ty, struct_dyn, struct_name, model)
        {
            let conformance_sig = SignatureType::from_ty(conformance_ty);
            let methods = model.query(ProtocolMethodsWithDefiner {
                protocol_id: conforming_protocol.metadata().id(),
            });
            for (defining_protocol, method) in methods {
                let sig = method.signature();
                let substituted_sig = substitute_signature(&sig, &bindings);
                protocol_methods.push((
                    defining_protocol,
                    method,
                    substituted_sig,
                    bindings.clone(),
                    conformance_sig.clone(),
                ));
            }
        }
    }

    let mut all_methods = collect_methods_from_symbol(struct_dyn);
    let extensions = model.query(ExtensionsFor {
        target_id: struct_id,
    });
    for extension in extensions {
        let extension_methods =
            collect_methods_from_symbol(&(extension.clone() as Arc<dyn Symbol<KestrelLanguage>>));
        all_methods.extend(extension_methods);
    }

    for struct_method in &all_methods {
        let struct_sig = struct_method.signature();
        let method_name = &struct_method.metadata().name().value;
        let method_span = struct_method.metadata().declaration_span().clone();

        let mut matches: Vec<(Arc<ProtocolSymbol>, Arc<FunctionSymbol>, SignatureType)> =
            Vec::new();
        for (protocol, protocol_method, substituted_sig, _bindings, conformance_sig) in
            &protocol_methods
        {
            if &struct_sig == substituted_sig {
                let struct_receiver = struct_method
                    .metadata()
                    .get_behavior::<CallableBehavior>()
                    .and_then(|cb| cb.receiver());
                let protocol_receiver = protocol_method
                    .metadata()
                    .get_behavior::<CallableBehavior>()
                    .and_then(|cb| cb.receiver());
                if struct_receiver == protocol_receiver {
                    matches.push((
                        protocol.clone(),
                        protocol_method.clone(),
                        conformance_sig.clone(),
                    ));
                } else {
                    let protocol_name = protocol.metadata().name().value.clone();
                    ctx.report(ProtocolMethodReceiverMismatchError {
                        span: method_span.clone(),
                        method_name: method_name.clone(),
                        protocol_name,
                        expected_receiver: receiver_kind_to_string(&protocol_receiver),
                        actual_receiver: receiver_kind_to_string(&struct_receiver),
                    });
                }
            }
        }

        // Deduplicate matches by protocol method ID.
        // This handles the case where a struct explicitly conforms to both A and B,
        // where B: A. The method `a()` from protocol A will appear twice (once from
        // direct conformance to A, once through B's inheritance), but it's the same method.
        // Note: different instantiations of generic protocols (e.g., Conv[Int8] vs Conv[Int32])
        // have different substituted signatures, so the struct method only matches ONE of them,
        // meaning they won't both appear in `matches` for the same struct method.
        matches.sort_by_key(|(_, method, _)| method.metadata().id().raw());
        matches.dedup_by(|(_, method_a, _), (_, method_b, _)| {
            method_a.metadata().id() == method_b.metadata().id()
        });

        if matches.len() > 1 {
            let protocol_names: Vec<String> = matches
                .iter()
                .map(|(p, _, _)| p.metadata().name().value.clone())
                .collect();
            ctx.report(AmbiguousProtocolMethodError {
                span: method_span,
                method_name: method_name.clone(),
                protocols: protocol_names,
            });
        } else if matches.len() == 1 {
            let (protocol, protocol_method, conformance_sig) = &matches[0];
            let implements =
                kestrel_semantic_tree::behavior::implements::ImplementsBehavior::with_conformance(
                    protocol.metadata().id(),
                    protocol_method.metadata().id(),
                    conformance_sig.clone(),
                );
            struct_method.metadata().add_behavior(implements);
        }
    }
}

fn resolve_protocol_type_for_link(
    ty: &Ty,
    struct_dyn: &Arc<dyn Symbol<KestrelLanguage>>,
    struct_name: &str,
    model: &SemanticModel,
) -> Option<(Arc<ProtocolSymbol>, HashMap<String, SignatureType>)> {
    match ty.kind() {
        TyKind::Protocol {
            symbol,
            substitutions,
        } => {
            let mut bindings = HashMap::new();
            let type_params = symbol.type_parameters();
            for type_param in type_params.iter() {
                let param_id = type_param.metadata().id();
                let param_name = type_param.metadata().name().value.clone();

                // Prefer explicit substitution unless it's an inferred placeholder (`_`).
                if let Some(sub_ty) = substitutions.get(param_id)
                    && !matches!(sub_ty.kind(), TyKind::Infer)
                {
                    bindings.insert(param_name, SignatureType::from_ty(sub_ty));
                    continue;
                }

                // Otherwise, apply the default if present.
                if let Some(default_ty) = type_param.default() {
                    bindings.insert(param_name, SignatureType::from_ty(default_ty));
                }
            }
            let self_type = struct_dyn
                .metadata()
                .get_behavior::<TypedBehavior>()
                .map(|tb| SignatureType::from_ty(tb.ty()))
                .unwrap_or_else(|| SignatureType::Named(vec![struct_name.to_string()]));
            bindings.insert("Self".to_string(), self_type);
            if let Ok(struct_sym) = struct_dyn.clone().into_any_arc().downcast::<StructSymbol>() {
                let assoc_bindings = model.query(AssociatedTypeBindingsForStruct {
                    struct_id: struct_sym.metadata().id(),
                });
                for (name, sig_type) in assoc_bindings {
                    bindings.insert(name, sig_type);
                }
            }
            Some((symbol.clone(), bindings))
        },
        _ => None,
    }
}

fn receiver_kind_to_string(receiver: &Option<ReceiverKind>) -> String {
    match receiver {
        None => "static".to_string(),
        Some(ReceiverKind::Borrowing) => "instance".to_string(),
        Some(ReceiverKind::Mutating) => "mutating".to_string(),
        Some(ReceiverKind::Consuming) => "consuming".to_string(),
        Some(ReceiverKind::Initializing) => "initializing".to_string(),
    }
}

/// Check that a struct provides all required properties from a protocol.
fn check_property_requirements(
    struct_sym: &Arc<StructSymbol>,
    _dyn_sym: &Arc<dyn Symbol<KestrelLanguage>>,
    struct_name: &str,
    protocol_name: &str,
    required_properties: &[PropertyRequirement],
    model: &SemanticModel,
    ctx: &mut AnalysisContext,
) {
    // Collect all fields from the struct and its extensions
    let struct_id = struct_sym.metadata().id();
    let mut all_fields = model.query(StructFields { struct_id });

    // Also check extensions for properties
    let extensions = model.query(ExtensionsFor {
        target_id: struct_id,
    });
    for extension in extensions {
        // Get fields from extension
        for child in extension.metadata().children() {
            if child.metadata().kind() == KestrelSymbolKind::Field
                && let Ok(field) = child.clone().into_any_arc().downcast::<FieldSymbol>()
            {
                let ty = child
                    .metadata()
                    .get_behavior::<TypedBehavior>()
                    .map(|typed| typed.ty().clone())
                    .unwrap_or_else(|| field.field_type().clone());
                all_fields.push(kestrel_semantic_model::StructFieldInfo {
                    field_id: field.metadata().id(),
                    name: field.metadata().name().value.clone(),
                    span: field.metadata().span().clone(),
                    is_mutable: field.is_mutable(),
                    is_computed: field.is_computed(),
                    ty,
                });
            }
        }
    }

    // Build a map of field name -> field info for quick lookup
    let field_map: HashMap<String, &kestrel_semantic_model::StructFieldInfo> =
        all_fields.iter().map(|f| (f.name.clone(), f)).collect();

    for requirement in required_properties {
        match field_map.get(&requirement.name) {
            None => {
                // Missing property
                let span = struct_sym.metadata().declaration_span().clone();
                ctx.report(MissingProtocolPropertyError {
                    span,
                    struct_name: struct_name.to_string(),
                    protocol_name: protocol_name.to_string(),
                    property_name: requirement.name.clone(),
                    property_type: format!("{}", requirement.property_type),
                });
            },
            Some(field_info) => {
                // Check type compatibility
                // TODO: More sophisticated type comparison with substitutions
                let field_type_str = format!("{}", field_info.ty);
                let required_type_str = format!("{}", requirement.property_type);
                if field_type_str != required_type_str {
                    let span = struct_sym.metadata().declaration_span().clone();
                    ctx.report(ProtocolPropertyTypeMismatchError {
                        span,
                        struct_name: struct_name.to_string(),
                        protocol_name: protocol_name.to_string(),
                        property_name: requirement.name.clone(),
                        expected_type: required_type_str,
                        actual_type: field_type_str,
                    });
                    continue;
                }

                // Check setter requirement
                if requirement.has_setter {
                    let has_setter = if field_info.is_computed {
                        // For computed properties, check if it has a setter
                        if let Some(sym) = model.query(SymbolFor {
                            id: field_info.field_id,
                        }) {
                            if let Ok(field) = sym.downcast_arc::<FieldSymbol>() {
                                if let Some(setter_id) = field.setter() {
                                    // Check if setter has a body (is implemented)
                                    if let Some(setter_sym) =
                                        model.query(SymbolFor { id: setter_id })
                                    {
                                        setter_sym
                                            .metadata()
                                            .get_behavior::<ExecutableBehavior>()
                                            .is_some()
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        // For stored properties, `var` (mutable) satisfies { get set }
                        field_info.is_mutable
                    };

                    if !has_setter {
                        let span = struct_sym.metadata().declaration_span().clone();
                        ctx.report(ProtocolPropertyMissingSetterError {
                            span,
                            struct_name: struct_name.to_string(),
                            protocol_name: protocol_name.to_string(),
                            property_name: requirement.name.clone(),
                        });
                    }
                }
            },
        }
    }
}

fn collect_initializers_from_symbol(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> Vec<Arc<InitializerSymbol>> {
    symbol
        .metadata()
        .children()
        .into_iter()
        .filter(|child| child.metadata().kind() == KestrelSymbolKind::Initializer)
        .filter_map(|child| child.into_any_arc().downcast::<InitializerSymbol>().ok())
        .collect()
}

/// Link protocol initializers to struct initializers (attach ImplementsBehavior).
///
/// This is similar to `link_protocol_methods_for_struct` but for initializers.
/// It allows initializers with the same labels but different types to coexist
/// if they implement different protocol requirements.
fn link_protocol_initializers_for_struct(
    struct_sym: &Arc<StructSymbol>,
    struct_dyn: &Arc<dyn Symbol<KestrelLanguage>>,
    model: &SemanticModel,
    ctx: &mut AnalysisContext,
) {
    let struct_name = &struct_sym.metadata().name().value;
    let struct_id = struct_sym.metadata().id();

    let mut conformances = model.query(ConformancesForSymbol {
        symbol_id: struct_dyn.metadata().id(),
    });
    let extensions = model.query(ExtensionsFor {
        target_id: struct_id,
    });
    for extension in &extensions {
        let ext_confs = model.query(ConformancesForSymbol {
            symbol_id: extension.metadata().id(),
        });
        conformances.extend(ext_confs);
    }
    if conformances.is_empty() {
        return;
    }

    // Collect protocol initializer requirements with their defining protocols
    let mut protocol_initializers: Vec<(
        Arc<ProtocolSymbol>,
        Arc<InitializerSymbol>,
        CallableSignature,
        HashMap<String, SignatureType>,
        SignatureType, // conformance_signature
    )> = Vec::new();
    for conformance_ty in &conformances {
        if let Some((conforming_protocol, bindings)) =
            resolve_protocol_type_for_link(conformance_ty, struct_dyn, struct_name, model)
        {
            let conformance_sig = SignatureType::from_ty(conformance_ty);
            let initializers = model.query(ProtocolInitializersWithDefiner {
                protocol_id: conforming_protocol.metadata().id(),
            });
            for (defining_protocol, init) in initializers {
                let sig: CallableSignature = init.signature();
                let substituted_sig = substitute_signature(&sig, &bindings);
                protocol_initializers.push((
                    defining_protocol,
                    init,
                    substituted_sig,
                    bindings.clone(),
                    conformance_sig.clone(),
                ));
            }
        }
    }

    // Collect all initializers from struct and extensions
    let mut all_initializers = collect_initializers_from_symbol(struct_dyn);
    let extensions = model.query(ExtensionsFor {
        target_id: struct_id,
    });
    for extension in extensions {
        let extension_inits = collect_initializers_from_symbol(
            &(extension.clone() as Arc<dyn Symbol<KestrelLanguage>>),
        );
        all_initializers.extend(extension_inits);
    }

    // Match struct initializers to protocol initializers
    for struct_init in &all_initializers {
        let struct_sig = struct_init.signature();
        let init_span = struct_init.metadata().declaration_span().clone();

        let mut matches: Vec<(Arc<ProtocolSymbol>, Arc<InitializerSymbol>, SignatureType)> =
            Vec::new();
        for (protocol, protocol_init, substituted_sig, _bindings, conformance_sig) in
            &protocol_initializers
        {
            if &struct_sig == substituted_sig {
                // Initializers always have "initializing" receiver, so no receiver check needed
                matches.push((
                    protocol.clone(),
                    protocol_init.clone(),
                    conformance_sig.clone(),
                ));
            }
        }

        // Deduplicate matches by protocol initializer ID.
        // This handles the case where a struct explicitly conforms to both A and B,
        // where B: A. The init from protocol A will appear twice but it's the same init.
        // Note: different instantiations of generic protocols (e.g., Conv[Int8] vs Conv[Int32])
        // have different substituted signatures, so the struct init only matches ONE of them.
        matches.sort_by_key(|(_, init, _)| init.metadata().id().raw());
        matches.dedup_by(|(_, init_a, _), (_, init_b, _)| {
            init_a.metadata().id() == init_b.metadata().id()
        });

        if matches.len() > 1 {
            let protocol_names: Vec<String> = matches
                .iter()
                .map(|(p, _, _)| p.metadata().name().value.clone())
                .collect();
            ctx.report(AmbiguousProtocolMethodError {
                span: init_span,
                method_name: "init".to_string(),
                protocols: protocol_names,
            });
        } else if matches.len() == 1 {
            let (protocol, protocol_init, conformance_sig) = &matches[0];
            let implements =
                kestrel_semantic_tree::behavior::implements::ImplementsBehavior::with_conformance(
                    protocol.metadata().id(),
                    protocol_init.metadata().id(),
                    conformance_sig.clone(),
                );
            struct_init.metadata().add_behavior(implements);
        }
    }
}

/// Check enum conformance to protocols.
///
/// Similar to `check_struct_conformance` but for enums.
fn check_enum_conformance(
    enum_sym: &Arc<EnumSymbol>,
    dyn_sym: &Arc<dyn Symbol<KestrelLanguage>>,
    model: &SemanticModel,
    ctx: &mut AnalysisContext,
) {
    let enum_name = &enum_sym.metadata().name().value;
    let enum_id = enum_sym.metadata().id();

    let mut conformances = model.query(ConformancesForSymbol {
        symbol_id: dyn_sym.metadata().id(),
    });
    let extensions = model.query(ExtensionsFor { target_id: enum_id });
    for extension in &extensions {
        let ext_confs = model.query(ConformancesForSymbol {
            symbol_id: extension.metadata().id(),
        });
        conformances.extend(ext_confs);
    }
    if conformances.is_empty() {
        return;
    }

    let associated_type_bindings = model.query(AssociatedTypeBindingsForEnum { enum_id });

    // Compute self_type first - this is used for Self substitution in enum method signatures
    let self_type = dyn_sym
        .metadata()
        .get_behavior::<TypedBehavior>()
        .map(|tb| SignatureType::from_ty(tb.ty()))
        .unwrap_or_else(|| SignatureType::Named(vec![enum_name.clone()]));

    let mut all_methods = collect_methods_from_symbol(dyn_sym);
    let extensions = model.query(ExtensionsFor { target_id: enum_id });
    for extension in extensions {
        let methods =
            collect_methods_from_symbol(&(extension.clone() as Arc<dyn Symbol<KestrelLanguage>>));
        all_methods.extend(methods);
    }

    // Build enum method map with Self substituted to the concrete type
    let mut self_bindings = HashMap::new();
    self_bindings.insert("Self".to_string(), self_type.clone());

    let enum_method_map: HashMap<MethodLookupKey, (Arc<FunctionSymbol>, SignatureType)> =
        all_methods
            .iter()
            .map(|f| {
                // Substitute Self in the signature and return type
                let substituted_sig = substitute_signature(&f.signature(), &self_bindings);
                let raw_return = SignatureType::from_ty(&f.return_type());
                let substituted_return = substitute_associated_types(&raw_return, &self_bindings);
                (
                    substituted_sig.lookup_key(),
                    (f.clone(), substituted_return),
                )
            })
            .collect();

    for conformance_ty in &conformances {
        let (protocol_symbol, type_param_bindings) = match resolve_protocol_type(conformance_ty) {
            Some(r) => r,
            None => continue,
        };
        let protocol_name = &protocol_symbol.metadata().name().value;

        let protocol_associated_types = model.query(ProtocolAssociatedTypesWithDefaults {
            protocol_id: protocol_symbol.metadata().id(),
        });
        for (type_name, default_type) in &protocol_associated_types {
            if default_type.is_none() && !associated_type_bindings.contains_key(type_name) {
                let span = enum_sym.metadata().declaration_span().clone();
                ctx.report(MissingAssociatedTypeError {
                    span,
                    struct_name: enum_name.clone(),
                    protocol_name: protocol_name.clone(),
                    type_name: type_name.clone(),
                });
            }
        }

        let mut effective_bindings = type_param_bindings;
        for (name, binding) in &associated_type_bindings {
            effective_bindings.insert(name.clone(), binding.clone());
        }
        for (type_name, default_type) in &protocol_associated_types {
            if !effective_bindings.contains_key(type_name)
                && let Some(default) = default_type
            {
                effective_bindings.insert(type_name.clone(), default.clone());
            }
        }
        // Use the already-computed self_type
        effective_bindings.insert("Self".to_string(), self_type.clone());

        let required_methods = model.query(ProtocolRequiredMethods {
            protocol_id: protocol_symbol.metadata().id(),
        });
        for (protocol_sig, method) in &required_methods {
            let method_name = &method.metadata().name().value;
            let raw_return_type = SignatureType::from_ty(&method.return_type());
            let required_return_type =
                substitute_associated_types(&raw_return_type, &effective_bindings);
            let substituted_sig = substitute_signature(protocol_sig, &effective_bindings);
            match enum_method_map.get(&substituted_sig.lookup_key()) {
                None => {
                    let span = enum_sym.metadata().declaration_span().clone();
                    ctx.report(MissingProtocolMethodError {
                        span,
                        struct_name: enum_name.clone(),
                        protocol_name: protocol_name.clone(),
                        method_name: method_name.clone(),
                    });
                },
                Some((_enum_method, enum_return_type)) => {
                    if enum_return_type != &required_return_type {
                        let span = enum_sym.metadata().declaration_span().clone();
                        ctx.report(WrongMethodReturnTypeError {
                            span,
                            method_name: method_name.clone(),
                            protocol_name: protocol_name.clone(),
                            expected_type: format!("{:?}", required_return_type),
                            actual_type: format!("{:?}", enum_return_type),
                        });
                    }
                },
            }
        }
    }
}

/// Link protocol methods for an enum (attach ImplementsBehavior).
///
/// Similar to `link_protocol_methods_for_struct` but for enums.
fn link_protocol_methods_for_enum(
    enum_sym: &Arc<EnumSymbol>,
    enum_dyn: &Arc<dyn Symbol<KestrelLanguage>>,
    model: &SemanticModel,
    ctx: &mut AnalysisContext,
) {
    let enum_name = &enum_sym.metadata().name().value;
    let enum_id = enum_sym.metadata().id();

    let mut conformances = model.query(ConformancesForSymbol {
        symbol_id: enum_dyn.metadata().id(),
    });
    let extensions = model.query(ExtensionsFor { target_id: enum_id });
    for extension in &extensions {
        let ext_confs = model.query(ConformancesForSymbol {
            symbol_id: extension.metadata().id(),
        });
        conformances.extend(ext_confs);
    }
    if conformances.is_empty() {
        return;
    }

    let mut protocol_methods: Vec<(
        Arc<ProtocolSymbol>,
        Arc<FunctionSymbol>,
        CallableSignature,
        HashMap<String, SignatureType>,
        SignatureType, // conformance_signature
    )> = Vec::new();
    for conformance_ty in &conformances {
        if let Some((conforming_protocol, bindings)) =
            resolve_protocol_type_for_link_enum(conformance_ty, enum_dyn, enum_name, model)
        {
            let conformance_sig = SignatureType::from_ty(conformance_ty);
            let methods = model.query(ProtocolMethodsWithDefiner {
                protocol_id: conforming_protocol.metadata().id(),
            });
            for (defining_protocol, method) in methods {
                let sig = method.signature();
                let substituted_sig = substitute_signature(&sig, &bindings);
                protocol_methods.push((
                    defining_protocol,
                    method,
                    substituted_sig,
                    bindings.clone(),
                    conformance_sig.clone(),
                ));
            }
        }
    }

    let mut all_methods = collect_methods_from_symbol(enum_dyn);
    let extensions = model.query(ExtensionsFor { target_id: enum_id });
    for extension in extensions {
        let extension_methods =
            collect_methods_from_symbol(&(extension.clone() as Arc<dyn Symbol<KestrelLanguage>>));
        all_methods.extend(extension_methods);
    }

    for enum_method in &all_methods {
        let enum_sig = enum_method.signature();
        let method_name = &enum_method.metadata().name().value;
        let method_span = enum_method.metadata().declaration_span().clone();

        let mut matches: Vec<(Arc<ProtocolSymbol>, Arc<FunctionSymbol>, SignatureType)> =
            Vec::new();
        for (protocol, protocol_method, substituted_sig, _bindings, conformance_sig) in
            &protocol_methods
        {
            if &enum_sig == substituted_sig {
                let enum_receiver = enum_method
                    .metadata()
                    .get_behavior::<CallableBehavior>()
                    .and_then(|cb| cb.receiver());
                let protocol_receiver = protocol_method
                    .metadata()
                    .get_behavior::<CallableBehavior>()
                    .and_then(|cb| cb.receiver());
                if enum_receiver == protocol_receiver {
                    matches.push((
                        protocol.clone(),
                        protocol_method.clone(),
                        conformance_sig.clone(),
                    ));
                } else {
                    let protocol_name = protocol.metadata().name().value.clone();
                    ctx.report(ProtocolMethodReceiverMismatchError {
                        span: method_span.clone(),
                        method_name: method_name.clone(),
                        protocol_name,
                        expected_receiver: receiver_kind_to_string(&protocol_receiver),
                        actual_receiver: receiver_kind_to_string(&enum_receiver),
                    });
                }
            }
        }

        // Deduplicate matches by protocol method ID.
        matches.sort_by_key(|(_, method, _)| method.metadata().id().raw());
        matches.dedup_by(|(_, method_a, _), (_, method_b, _)| {
            method_a.metadata().id() == method_b.metadata().id()
        });

        if matches.len() > 1 {
            let protocol_names: Vec<String> = matches
                .iter()
                .map(|(p, _, _)| p.metadata().name().value.clone())
                .collect();
            ctx.report(AmbiguousProtocolMethodError {
                span: method_span,
                method_name: method_name.clone(),
                protocols: protocol_names,
            });
        } else if matches.len() == 1 {
            let (protocol, protocol_method, conformance_sig) = &matches[0];
            let implements =
                kestrel_semantic_tree::behavior::implements::ImplementsBehavior::with_conformance(
                    protocol.metadata().id(),
                    protocol_method.metadata().id(),
                    conformance_sig.clone(),
                );
            enum_method.metadata().add_behavior(implements);
        }
    }
}

fn resolve_protocol_type_for_link_enum(
    ty: &Ty,
    enum_dyn: &Arc<dyn Symbol<KestrelLanguage>>,
    enum_name: &str,
    model: &SemanticModel,
) -> Option<(Arc<ProtocolSymbol>, HashMap<String, SignatureType>)> {
    match ty.kind() {
        TyKind::Protocol {
            symbol,
            substitutions,
        } => {
            let mut bindings = HashMap::new();
            let type_params = symbol.type_parameters();
            for type_param in type_params.iter() {
                let param_id = type_param.metadata().id();
                let param_name = type_param.metadata().name().value.clone();

                // Prefer explicit substitution unless it's an inferred placeholder (`_`).
                if let Some(sub_ty) = substitutions.get(param_id)
                    && !matches!(sub_ty.kind(), TyKind::Infer)
                {
                    bindings.insert(param_name, SignatureType::from_ty(sub_ty));
                    continue;
                }

                // Otherwise, apply the default if present.
                if let Some(default_ty) = type_param.default() {
                    bindings.insert(param_name, SignatureType::from_ty(default_ty));
                }
            }
            let self_type = enum_dyn
                .metadata()
                .get_behavior::<TypedBehavior>()
                .map(|tb| SignatureType::from_ty(tb.ty()))
                .unwrap_or_else(|| SignatureType::Named(vec![enum_name.to_string()]));
            bindings.insert("Self".to_string(), self_type);
            if let Ok(enum_sym) = enum_dyn.clone().into_any_arc().downcast::<EnumSymbol>() {
                let assoc_bindings = model.query(AssociatedTypeBindingsForEnum {
                    enum_id: enum_sym.metadata().id(),
                });
                for (name, sig_type) in assoc_bindings {
                    bindings.insert(name, sig_type);
                }
            }
            Some((symbol.clone(), bindings))
        },
        _ => None,
    }
}
