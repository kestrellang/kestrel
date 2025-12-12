use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use kestrel_semantic_model::{ExtensionsFor, SemanticModel, SymbolFor};
use kestrel_semantic_tree::behavior::callable::{CallableSignature, ReceiverKind, SignatureType};
use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
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
}

impl ConformanceAnalyzer {
    pub fn new() -> Self { Self { protocols: Vec::new(), structs: Vec::new() } }
}

impl Default for ConformanceAnalyzer { fn default() -> Self { Self::new() } }

impl Analyzer for ConformanceAnalyzer {
    fn name(&self) -> &'static str { "conformance" }

    fn visit_symbol(&mut self, symbol: &Arc<dyn Symbol<KestrelLanguage>>, _ctx: &mut AnalysisContext) {
        match symbol.metadata().kind() {
            KestrelSymbolKind::Protocol => {
                if let Some(protocol) = get_protocol_arc_from_symbol(symbol) {
                    self.protocols.push((symbol.clone(), protocol));
                }
            }
            KestrelSymbolKind::Struct => {
                if !get_conformances(symbol).is_empty() {
                    if let Ok(struct_sym) = symbol.clone().into_any_arc().downcast::<StructSymbol>() {
                        self.structs.push((symbol.clone(), struct_sym));
                    }
                }
            }
            _ => {}
        }
    }

    fn finalize(&mut self, ctx: &mut AnalysisContext) {
        // Circular inheritance + protocol defaults
        for (sym, proto) in &self.protocols {
            check_circular_inheritance(proto, sym, ctx.model, ctx);
            check_protocol_associated_type_defaults(proto, sym, ctx);
        }

        // Also consider structs that gain conformances via extensions
        let mut extra_structs = Vec::new();
        for extension in ctx.model.extension_registry().all_extensions() {
            if let Some(cb) = extension.conformances_behavior() {
                if !cb.conformances().is_empty() {
                    if let Some(target_ty) = extension.target_type() {
                        if let TyKind::Struct { symbol: s, .. } = target_ty.kind() {
                            let id = s.metadata().id();
                            let already = self.structs.iter().any(|(_, ss)| ss.metadata().id() == id);
                            if !already { extra_structs.push((s.clone() as Arc<dyn Symbol<KestrelLanguage>>, s.clone())); }
                        }
                    }
                }
            }
        }
        self.structs.extend(extra_structs);

        // Conformance checks
        for (dyn_sym, struct_sym) in &self.structs {
            check_struct_conformance(struct_sym, dyn_sym, ctx.model, ctx);
        }

        // Link protocol methods for all structs
        for (dyn_sym, struct_sym) in &self.structs {
            link_protocol_methods_for_struct(struct_sym, dyn_sym, ctx.model, ctx);
        }
    }
}

// Helpers adapted from builder validator/linker

fn get_conformances(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Vec<Ty> {
    symbol.conformances_behavior().map(|cb| cb.conformances().to_vec()).unwrap_or_default()
}

fn get_protocol_arc_from_symbol(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<Arc<ProtocolSymbol>> {
    symbol.typed_behavior().and_then(|tb| if let TyKind::Protocol { symbol, .. } = tb.ty().kind() { Some(symbol.clone()) } else { None })
}

fn check_circular_inheritance(protocol: &Arc<ProtocolSymbol>, symbol: &Arc<dyn Symbol<KestrelLanguage>>, model: &SemanticModel, ctx: &mut AnalysisContext) {
    let mut detector: CycleDetector<SymbolId> = CycleDetector::new();
    if let Some(cycle) = check_inheritance_cycle(protocol, &mut detector) {
        let span = protocol.metadata().declaration_span().clone();
        let cycle_names: Vec<String> = cycle.cycle().iter().filter_map(|&id| model.query(SymbolFor { id }).map(|s| s.metadata().name().value.clone())).collect();
        let protocol_name = protocol.metadata().name().value.clone();
        ctx.report(CircularProtocolInheritanceError { span, protocol_name, cycle: cycle_names });
    }
}

fn check_inheritance_cycle(protocol: &Arc<ProtocolSymbol>, detector: &mut CycleDetector<SymbolId>) -> Option<semantic_tree::cycle::Cycle<SymbolId>> {
    let id = protocol.metadata().id();
    if let Err(cycle) = detector.enter(id) { return Some(cycle); }
    let protocol_dyn = protocol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
    for inherited_ty in get_conformances(&protocol_dyn) {
        if let TyKind::Protocol { symbol, .. } = inherited_ty.kind() {
            if let Some(c) = check_inheritance_cycle(symbol, detector) { detector.exit(); return Some(c); }
        }
    }
    detector.exit();
    None
}

fn check_struct_conformance(struct_sym: &Arc<StructSymbol>, dyn_sym: &Arc<dyn Symbol<KestrelLanguage>>, model: &SemanticModel, ctx: &mut AnalysisContext) {
    let struct_name = &struct_sym.metadata().name().value;
    let struct_id = struct_sym.metadata().id();

    let mut conformances = get_conformances(dyn_sym);
    let extensions = model.query(ExtensionsFor { target_id: struct_id });
    for extension in &extensions {
        let ext_confs = get_conformances(&(extension.clone() as Arc<dyn Symbol<KestrelLanguage>>));
        conformances.extend(ext_confs);
    }
    if conformances.is_empty() { return; }

    let struct_arc = dyn_sym.clone().into_any_arc().downcast::<StructSymbol>().ok();
    let associated_type_bindings = struct_arc.as_ref().map(|s| collect_associated_type_bindings(s)).unwrap_or_default();

    let mut all_methods = collect_methods_from_symbol(dyn_sym);
    let extensions = model.query(ExtensionsFor { target_id: struct_id });
    for extension in extensions {
        let methods = collect_methods_from_symbol(&(extension.clone() as Arc<dyn Symbol<KestrelLanguage>>));
        all_methods.extend(methods);
    }
    let struct_method_map: HashMap<MethodLookupKey, (Arc<FunctionSymbol>, SignatureType)> = all_methods
        .iter()
        .map(|f| (f.signature().lookup_key(), (f.clone(), SignatureType::from_ty(&f.return_type()))))
        .collect();

    for conformance_ty in &conformances {
        let (protocol_symbol, type_param_bindings) = match resolve_protocol_type(conformance_ty) { Some(r) => r, None => continue };
        let protocol_name = &protocol_symbol.metadata().name().value;

        let protocol_associated_types = collect_protocol_associated_types_with_defaults(&protocol_symbol);
        for (type_name, default_type) in &protocol_associated_types {
            if default_type.is_none() && !associated_type_bindings.contains_key(type_name) {
                let span = struct_sym.metadata().declaration_span().clone();
                ctx.report(MissingAssociatedTypeError { span, struct_name: struct_name.clone(), protocol_name: protocol_name.clone(), type_name: type_name.clone() });
            }
        }

        let mut effective_bindings = type_param_bindings;
        for (name, binding) in &associated_type_bindings { effective_bindings.insert(name.clone(), binding.clone()); }
        for (type_name, default_type) in &protocol_associated_types { if !effective_bindings.contains_key(type_name) { if let Some(default) = default_type { effective_bindings.insert(type_name.clone(), default.clone()); } } }
        let self_type = dyn_sym.typed_behavior().map(|tb| SignatureType::from_ty(tb.ty())).unwrap_or_else(|| SignatureType::Named(vec![struct_name.clone()]));
        effective_bindings.insert("Self".to_string(), self_type);

        let required_methods = collect_all_protocol_methods(&protocol_symbol, model);
        for (protocol_sig, method) in &required_methods {
            let method_name = &method.metadata().name().value;
            let raw_return_type = SignatureType::from_ty(&method.return_type());
            let required_return_type = substitute_associated_types(&raw_return_type, &effective_bindings);
            let substituted_sig = substitute_signature(protocol_sig, &effective_bindings);
            match struct_method_map.get(&substituted_sig.lookup_key()) {
                None => {
                    let span = struct_sym.metadata().declaration_span().clone();
                    ctx.report(MissingProtocolMethodError { span, struct_name: struct_name.clone(), protocol_name: protocol_name.clone(), method_name: method_name.clone() });
                }
                Some((_struct_method, struct_return_type)) => {
                    if struct_return_type != &required_return_type {
                        let span = struct_sym.metadata().declaration_span().clone();
                        ctx.report(WrongMethodReturnTypeError { span, method_name: method_name.clone(), protocol_name: protocol_name.clone(), expected_type: format!("{:?}", required_return_type), actual_type: format!("{:?}", struct_return_type) });
                    }
                }
            }
        }
    }
}

use kestrel_semantic_tree::behavior::callable::MethodLookupKey;

fn resolve_protocol_type(ty: &Ty) -> Option<(Arc<ProtocolSymbol>, HashMap<String, SignatureType>)> {
    match ty.kind() {
        TyKind::Protocol { symbol, substitutions } => {
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

fn collect_methods_from_symbol(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Vec<Arc<FunctionSymbol>> {
    symbol.metadata().children().into_iter()
        .filter(|child| child.metadata().kind() == KestrelSymbolKind::Function)
        .filter_map(|child| child.into_any_arc().downcast::<FunctionSymbol>().ok())
        .collect()
}

fn collect_protocol_associated_types_with_defaults(protocol: &Arc<ProtocolSymbol>) -> HashMap<String, Option<SignatureType>> {
    let protocol_dyn = protocol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
    let mut associated_types = HashMap::new();
    for child in protocol_dyn.metadata().children() {
        if child.metadata().kind() == KestrelSymbolKind::AssociatedType {
            if let Ok(assoc_type) = child.into_any_arc().downcast::<kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol>() {
                let name = assoc_type.metadata().name().value.clone();
                let default_type = assoc_type.default_type().map(|ty| SignatureType::from_ty(&ty));
                associated_types.insert(name, default_type);
            }
        }
    }
    associated_types
}

fn collect_associated_type_bindings(struct_sym: &Arc<StructSymbol>) -> HashMap<String, SignatureType> {
    use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
    use kestrel_semantic_tree::symbol::type_alias::{TypeAliasSymbol, TypeAliasTypedBehavior};
    let struct_dyn = struct_sym.clone() as Arc<dyn Symbol<KestrelLanguage>>;
    let mut bindings = HashMap::new();
    for child in struct_dyn.metadata().children() {
        if child.metadata().kind() == KestrelSymbolKind::TypeAlias {
            if let Ok(type_alias) = child.into_any_arc().downcast::<TypeAliasSymbol>() {
                let name = type_alias.metadata().name().value.clone();
                let resolved_ty = type_alias.metadata().behaviors().iter()
                    .find(|b| b.kind() == KestrelBehaviorKind::TypeAliasTyped)
                    .and_then(|b| b.as_ref().downcast_ref::<TypeAliasTypedBehavior>())
                    .map(|tb| SignatureType::from_ty(tb.resolved_ty()));
                if let Some(sig_type) = resolved_ty { bindings.insert(name, sig_type); }
            }
        }
    }
    bindings
}

// Check protocol associated type default bounds satisfaction
fn check_protocol_associated_type_defaults(protocol: &Arc<ProtocolSymbol>, _symbol: &Arc<dyn Symbol<KestrelLanguage>>, ctx: &mut AnalysisContext) {
    let protocol_dyn = protocol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
    for child in protocol_dyn.metadata().children() {
        if child.metadata().kind() == KestrelSymbolKind::AssociatedType {
            if let Ok(assoc_type) = child.downcast_arc::<kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol>() {
                if let Some(bounds) = assoc_type.bounds() {
                    if let Some(default_type) = assoc_type.default_type() {
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
        TyKind::Error { .. } => return,
        _ => format!("{:?}", bound_type.kind()),
    };

    for required_protocol in required_bounds {
        if matches!(required_protocol.kind(), TyKind::Error { .. }) { continue; }
        if let TyKind::Protocol { symbol: required_proto_symbol, .. } = required_protocol.kind() {
            let required_protocol_name = required_proto_symbol.metadata().name().value.clone();
            let conforms = match bound_type.kind() {
                TyKind::Struct { symbol, .. } => {
                    let conformances = symbol.conformances_behavior().map(|cb| cb.conformances().to_vec()).unwrap_or_default();
                    conformances.iter().any(|conf| if let TyKind::Protocol { symbol: proto_sym, .. } = conf.kind() { proto_sym.metadata().id() == required_proto_symbol.metadata().id() } else { false })
                }
                TyKind::TypeParameter(_) => true,
                TyKind::Error { .. } => true,
                _ => false,
            };
            if !conforms {
                ctx.report(AssociatedTypeConstraintNotSatisfiedError { span, type_name: type_name.to_string(), bound_type: bound_type_name.clone(), required_protocol: required_protocol_name });
                return;
            }
        }
    }
}

fn substitute_associated_types(sig_type: &SignatureType, bindings: &HashMap<String, SignatureType>) -> SignatureType {
    match sig_type {
        SignatureType::Named(path) if path.len() == 1 => bindings.get(&path[0]).cloned().unwrap_or_else(|| sig_type.clone()),
        SignatureType::Tuple(elements) => SignatureType::Tuple(elements.iter().map(|e| substitute_associated_types(e, bindings)).collect()),
        SignatureType::Array(element) => SignatureType::Array(Box::new(substitute_associated_types(element, bindings))),
        SignatureType::Function { params, return_type } => SignatureType::Function {
            params: params.iter().map(|p| substitute_associated_types(p, bindings)).collect(),
            return_type: Box::new(substitute_associated_types(return_type, bindings)),
        },
        _ => sig_type.clone(),
    }
}

fn substitute_signature(sig: &CallableSignature, bindings: &HashMap<String, SignatureType>) -> CallableSignature {
    CallableSignature { name: sig.name.clone(), labels: sig.labels.clone(), param_types: sig.param_types.iter().map(|t| substitute_associated_types(t, bindings)).collect(), return_type: substitute_associated_types(&sig.return_type, bindings) }
}

// Protocol method linker (adapted from builder)
fn link_protocol_methods_for_struct(struct_sym: &Arc<StructSymbol>, struct_dyn: &Arc<dyn Symbol<KestrelLanguage>>, model: &SemanticModel, ctx: &mut AnalysisContext) {
    let struct_name = &struct_sym.metadata().name().value;
    let struct_id = struct_sym.metadata().id();

    let mut conformances = struct_dyn.conformances_behavior().map(|cb| cb.conformances().to_vec()).unwrap_or_default();
    let extensions = model.query(ExtensionsFor { target_id: struct_id });
    for extension in &extensions {
        let ext_confs = extension.conformances_behavior().map(|cb| cb.conformances().to_vec()).unwrap_or_default();
        conformances.extend(ext_confs);
    }
    if conformances.is_empty() { return; }

    let mut protocol_methods: Vec<(Arc<ProtocolSymbol>, Arc<FunctionSymbol>, CallableSignature, HashMap<String, SignatureType>)> = Vec::new();
    for conformance_ty in &conformances {
        if let Some((conforming_protocol, bindings)) = resolve_protocol_type_for_link(conformance_ty, struct_dyn, struct_name) {
            let methods = collect_all_protocol_methods_with_definer(&conforming_protocol, model);
            for (defining_protocol, method) in methods {
                let sig = method.signature();
                let substituted_sig = substitute_signature(&sig, &bindings);
                protocol_methods.push((defining_protocol, method, substituted_sig, bindings.clone()));
            }
        }
    }

    let mut all_methods = collect_methods_from_symbol(struct_dyn);
    let extensions = model.query(ExtensionsFor { target_id: struct_id });
    for extension in extensions {
        let extension_methods = collect_methods_from_symbol(&(extension.clone() as Arc<dyn Symbol<KestrelLanguage>>));
        all_methods.extend(extension_methods);
    }

    for struct_method in &all_methods {
        let struct_sig = struct_method.signature();
        let method_name = &struct_method.metadata().name().value;
        let method_span = struct_method.metadata().declaration_span().clone();

        let mut matches: Vec<(Arc<ProtocolSymbol>, Arc<FunctionSymbol>)> = Vec::new();
        for (protocol, protocol_method, substituted_sig, _bindings) in &protocol_methods {
            if &struct_sig == substituted_sig {
                let struct_receiver = struct_method.callable_behavior().and_then(|cb| cb.receiver());
                let protocol_receiver = protocol_method.callable_behavior().and_then(|cb| cb.receiver());
                if struct_receiver == protocol_receiver {
                    matches.push((protocol.clone(), protocol_method.clone()));
                } else {
                    let protocol_name = protocol.metadata().name().value.clone();
                    ctx.report(ProtocolMethodReceiverMismatchError { span: method_span.clone(), method_name: method_name.clone(), protocol_name, expected_receiver: receiver_kind_to_string(&protocol_receiver), actual_receiver: receiver_kind_to_string(&struct_receiver) });
                }
            }
        }

        if matches.len() > 1 {
            let protocol_names: Vec<String> = matches.iter().map(|(p, _)| p.metadata().name().value.clone()).collect();
            ctx.report(AmbiguousProtocolMethodError { span: method_span, method_name: method_name.clone(), protocols: protocol_names });
        } else if matches.len() == 1 {
            let (protocol, protocol_method) = &matches[0];
            let implements = kestrel_semantic_tree::behavior::implements::ImplementsBehavior::new(protocol.metadata().id(), protocol_method.metadata().id());
            struct_method.metadata().add_behavior(implements);
        }
    }
}

fn resolve_protocol_type_for_link(ty: &Ty, struct_dyn: &Arc<dyn Symbol<KestrelLanguage>>, struct_name: &str) -> Option<(Arc<ProtocolSymbol>, HashMap<String, SignatureType>)> {
    match ty.kind() {
        TyKind::Protocol { symbol, substitutions } => {
            let mut bindings = HashMap::new();
            let type_params = symbol.type_parameters();
            for type_param in type_params.iter() {
                let param_id = type_param.metadata().id();
                if let Some(sub_ty) = substitutions.get(param_id) {
                    let param_name = type_param.metadata().name().value.clone();
                    bindings.insert(param_name, SignatureType::from_ty(sub_ty));
                }
            }
            let self_type = struct_dyn.typed_behavior().map(|tb| SignatureType::from_ty(tb.ty())).unwrap_or_else(|| SignatureType::Named(vec![struct_name.to_string()]));
            bindings.insert("Self".to_string(), self_type);
            if let Ok(struct_sym) = struct_dyn.clone().into_any_arc().downcast::<StructSymbol>() {
                let assoc_bindings = collect_associated_type_bindings(&struct_sym);
                for (name, sig_type) in assoc_bindings { bindings.insert(name, sig_type); }
            }
            Some((symbol.clone(), bindings))
        }
        _ => None,
    }
}

fn collect_all_protocol_methods(protocol: &Arc<ProtocolSymbol>, model: &SemanticModel) -> HashMap<CallableSignature, Arc<FunctionSymbol>> {
    let mut methods = HashMap::new();
    let mut visited = HashSet::new();
    collect_protocol_methods_recursive(protocol, model, &mut methods, &mut visited);
    methods
}

fn collect_protocol_methods_recursive(protocol: &Arc<ProtocolSymbol>, model: &SemanticModel, methods: &mut HashMap<CallableSignature, Arc<FunctionSymbol>>, visited: &mut HashSet<SymbolId>) {
    let id = protocol.metadata().id();
    if visited.contains(&id) { return; }
    visited.insert(id);
    let protocol_dyn = protocol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
    for inherited_ty in get_conformances(&protocol_dyn) {
        if let Some((inherited_protocol, _)) = resolve_protocol_type(&inherited_ty) {
            collect_protocol_methods_recursive(&inherited_protocol, model, methods, visited);
        }
    }
    for method in collect_methods_from_symbol(&protocol_dyn) {
        methods.insert(method.signature(), method);
    }
}

fn collect_all_protocol_methods_with_definer(protocol: &Arc<ProtocolSymbol>, model: &SemanticModel) -> Vec<(Arc<ProtocolSymbol>, Arc<FunctionSymbol>)> {
    let mut methods = Vec::new();
    let mut visited = HashSet::new();
    collect_protocol_methods_recursive_with_definer(protocol, model, &mut methods, &mut visited);
    methods
}

fn collect_protocol_methods_recursive_with_definer(protocol: &Arc<ProtocolSymbol>, model: &SemanticModel, methods: &mut Vec<(Arc<ProtocolSymbol>, Arc<FunctionSymbol>)>, visited: &mut HashSet<SymbolId>) {
    let id = protocol.metadata().id();
    if visited.contains(&id) { return; }
    visited.insert(id);
    let protocol_dyn = protocol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
    if let Some(conformances_behavior) = protocol_dyn.conformances_behavior() {
        for inherited_ty in conformances_behavior.conformances() {
            if let TyKind::Protocol { symbol, .. } = inherited_ty.kind() {
                collect_protocol_methods_recursive_with_definer(symbol, model, methods, visited);
            }
        }
    }
    for method in collect_methods_from_symbol(&protocol_dyn) {
        methods.push((protocol.clone(), method));
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
