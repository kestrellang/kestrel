//! Analyzer for extension method conflicts.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_span::Span;
use semantic_tree::symbol::{Symbol, SymbolId};

use diagnostics::{DuplicateExtensionMethodError, StructExtensionMethodConflictError};

pub struct ExtensionConflictAnalyzer {
    extensions_by_target: Mutex<HashMap<(SymbolId, usize), Vec<CollectedExtension>>>,
    struct_methods: Mutex<HashMap<SymbolId, Vec<(String, Span)>>>,
}

struct CollectedExtension {
    extension_id: SymbolId,
    extension_span: Span,
    methods: Vec<(String, Span)>,
}

impl ExtensionConflictAnalyzer { pub fn new() -> Self { Self { extensions_by_target: Mutex::new(HashMap::new()), struct_methods: Mutex::new(HashMap::new()) } } }
impl Default for ExtensionConflictAnalyzer { fn default() -> Self { Self::new() } }

impl Analyzer for ExtensionConflictAnalyzer {
    fn name(&self) -> &'static str { "extension_conflict" }

    fn visit_symbol(&mut self, symbol: &Arc<dyn Symbol<KestrelLanguage>>, _ctx: &mut AnalysisContext) {
        if symbol.metadata().kind() != KestrelSymbolKind::Extension { return; }
        let Ok(extension) = symbol.clone().downcast_arc::<ExtensionSymbol>() else { return; };
        let Some(target_beh) = extension.extension_target_behavior() else { return; };
        let target_ty = target_beh.target_type();
        let (target_id, struct_symbol, substitutions) = match target_ty.kind() {
            kestrel_semantic_tree::ty::TyKind::Struct { symbol, substitutions, .. } => (symbol.metadata().id(), symbol.clone(), substitutions),
            _ => return,
        };

        let specificity = substitutions.types().filter(|ty| !ty.is_type_parameter()).count();

        let methods: Vec<(String, Span)> = extension
            .metadata()
            .children()
            .into_iter()
            .filter(|c| c.metadata().kind() == KestrelSymbolKind::Function)
            .map(|c| (c.metadata().name().value.clone(), c.metadata().name().span.clone()))
            .collect();

        {
            let mut struct_methods = self.struct_methods.lock().unwrap();
            if !struct_methods.contains_key(&target_id) {
                let methods: Vec<(String, Span)> = struct_symbol
                    .metadata()
                    .children()
                    .into_iter()
                    .filter(|c| c.metadata().kind() == KestrelSymbolKind::Function)
                    .map(|c| (c.metadata().name().value.clone(), c.metadata().name().span.clone()))
                    .collect();
                struct_methods.insert(target_id, methods);
            }
        }

        let collected = CollectedExtension { extension_id: extension.metadata().id(), extension_span: extension.metadata().span().clone(), methods };
        self.extensions_by_target
            .lock().unwrap()
            .entry((target_id, specificity))
            .or_default()
            .push(collected);
    }

    fn finalize(&mut self, ctx: &mut AnalysisContext) {
        let extensions_by_target = self.extensions_by_target.lock().unwrap();
        let struct_methods = self.struct_methods.lock().unwrap();

        for ((target_id, _specificity), extensions) in extensions_by_target.iter() {
            if let Some(struct_method_list) = struct_methods.get(target_id) {
                let struct_method_names: HashMap<&str, &Span> = struct_method_list
                    .iter()
                    .map(|(name, span)| (name.as_str(), span))
                    .collect();
                for ext in extensions {
                    for (method_name, ext_method_span) in &ext.methods {
                        if let Some(&struct_method_span) = struct_method_names.get(method_name.as_str()) {
                            let error = StructExtensionMethodConflictError {
                                method_name: method_name.clone(),
                                struct_method_span: struct_method_span.clone(),
                                extension_method_span: ext_method_span.clone(),
                            };
                            ctx.report(error);
                        }
                    }
                }
            }

            if extensions.len() <= 1 { continue; }

            let mut method_locations: HashMap<String, Vec<(SymbolId, Span)>> = HashMap::new();
            for ext in extensions {
                for (method_name, method_span) in &ext.methods {
                    method_locations.entry(method_name.clone()).or_default().push((ext.extension_id, method_span.clone()));
                }
            }

            for (method_name, locations) in method_locations {
                if locations.len() > 1 {
                    let error = DuplicateExtensionMethodError { method_name, locations: locations.into_iter().map(|(_, span)| span).collect() };
                    ctx.report(error);
                }
            }
        }
    }
}

pub mod diagnostics;
