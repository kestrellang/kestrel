//! Analyzer for extension method conflicts.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use kestrel_semantic_model::{ExtensionMethods, StructMethods};
use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::{Substitutions, WhereClause};
use kestrel_span::Span;
use semantic_tree::symbol::{Symbol, SymbolId};

use diagnostics::{DuplicateExtensionMethodError, StructExtensionMethodConflictError};

pub struct ExtensionConflictAnalyzer {
    extensions_by_target: Mutex<HashMap<SymbolId, Vec<CollectedExtension>>>,
    struct_methods: Mutex<HashMap<SymbolId, Vec<(String, Span)>>>,
}

struct CollectedExtension {
    extension_id: SymbolId,
    #[allow(dead_code)]
    extension_span: Span,
    methods: Vec<(String, Span)>,
    substitutions: Substitutions,
    #[allow(dead_code)]
    where_clause: WhereClause,
}

impl ExtensionConflictAnalyzer {
    pub fn new() -> Self {
        Self {
            extensions_by_target: Mutex::new(HashMap::new()),
            struct_methods: Mutex::new(HashMap::new()),
        }
    }
}
impl Default for ExtensionConflictAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ExtensionConflictAnalyzer {
    fn name(&self) -> &'static str {
        "extension_conflict"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        if symbol.metadata().kind() != KestrelSymbolKind::Extension {
            return;
        }
        let Ok(extension) = symbol.clone().downcast_arc::<ExtensionSymbol>() else {
            return;
        };
        let Some(target_beh) = extension
            .metadata()
            .get_behavior::<ExtensionTargetBehavior>()
        else {
            return;
        };
        let target_ty = target_beh.target_type();
        let (target_id, substitutions) = match target_ty.kind() {
            kestrel_semantic_tree::ty::TyKind::Struct {
                symbol,
                substitutions,
                ..
            } => (symbol.metadata().id(), substitutions),
            _ => return,
        };

        let extension_id = extension.metadata().id();
        let methods = ctx.model.query(ExtensionMethods { extension_id });

        {
            let mut struct_methods = self.struct_methods.lock().unwrap();
            if !struct_methods.contains_key(&target_id) {
                struct_methods.insert(
                    target_id,
                    ctx.model.query(StructMethods {
                        struct_id: target_id,
                    }),
                );
            }
        }

        let collected = CollectedExtension {
            extension_id,
            extension_span: extension.metadata().span().clone(),
            methods,
            substitutions: substitutions.clone(),
            where_clause: target_beh.where_clause().clone(),
        };
        self.extensions_by_target
            .lock()
            .unwrap()
            .entry(target_id)
            .or_default()
            .push(collected);
    }

    fn finalize(&mut self, ctx: &mut AnalysisContext) {
        let extensions_by_target = self.extensions_by_target.lock().unwrap();
        let struct_methods = self.struct_methods.lock().unwrap();

        for (target_id, extensions) in extensions_by_target.iter() {
            if let Some(struct_method_list) = struct_methods.get(target_id) {
                let struct_method_names: HashMap<&str, &Span> = struct_method_list
                    .iter()
                    .map(|(name, span)| (name.as_str(), span))
                    .collect();
                for ext in extensions {
                    for (method_name, ext_method_span) in &ext.methods {
                        if let Some(&struct_method_span) =
                            struct_method_names.get(method_name.as_str())
                        {
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

            if extensions.len() <= 1 {
                continue;
            }

            // Check for conflicts between extensions
            for i in 0..extensions.len() {
                for j in i + 1..extensions.len() {
                    let ext1 = &extensions[i];
                    let ext2 = &extensions[j];

                    // Find common methods
                    let mut common_methods = Vec::new();
                    for (name1, span1) in &ext1.methods {
                        for (name2, span2) in &ext2.methods {
                            if name1 == name2 {
                                common_methods.push((name1.clone(), span1.clone(), span2.clone()));
                            }
                        }
                    }

                    if common_methods.is_empty() {
                        continue;
                    }

                    // They share methods - check if they overlap ambiguously
                    if ext1.substitutions.overlaps_with(&ext2.substitutions) {
                        let ext1_spec_of_ext2 =
                            ext1.substitutions.is_specialization_of(&ext2.substitutions);
                        let ext2_spec_of_ext1 =
                            ext2.substitutions.is_specialization_of(&ext1.substitutions);

                        let ambiguous = if ext1_spec_of_ext2 && ext2_spec_of_ext1 {
                            // Identical - definitely ambiguous
                            true
                        } else if ext1_spec_of_ext2 || ext2_spec_of_ext1 {
                            // One is strictly more specific - allowed
                            false
                        } else {
                            // Overlap but no clear winner
                            true
                        };

                        if ambiguous {
                            for (method_name, span1, span2) in common_methods {
                                let error = DuplicateExtensionMethodError {
                                    method_name,
                                    locations: vec![span1, span2],
                                };
                                ctx.report(error);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub mod diagnostics;
