//! Analyzer for parent protocol conformance validation
//!
//! Validates that if a struct conforms to protocol B which inherits from A,
//! it must also explicitly declare conformance to A.

use std::sync::Arc;

use kestrel_semantic_model::queries::ProtocolRequiredMethods;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::TyKind;
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

mod diagnostics;
use diagnostics::MissingParentProtocolConformanceError;

/// Analyzer that validates parent protocol conformance for structs.
pub struct ParentProtocolConformanceAnalyzer;

impl ParentProtocolConformanceAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ParentProtocolConformanceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ParentProtocolConformanceAnalyzer {
    fn name(&self) -> &'static str {
        "parent_protocol_conformance"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        // Only validate structs, not protocols
        if symbol.metadata().kind() != KestrelSymbolKind::Struct {
            return;
        }

        let Some(conformances) = symbol.metadata().get_behavior::<ConformancesBehavior>() else {
            return;
        };

        let conformance_list = conformances.conformances();

        // Collect all directly declared protocol IDs for quick lookup
        let declared_protocol_ids: std::collections::HashSet<_> = conformance_list
            .iter()
            .filter_map(|ty| {
                if let TyKind::Protocol { symbol, .. } = ty.kind() {
                    Some(symbol.metadata().id())
                } else {
                    None
                }
            })
            .collect();

        // For each declared conformance, check its parent protocols
        for conformance in conformance_list {
            if let TyKind::Protocol {
                symbol: protocol_symbol,
                ..
            } = conformance.kind()
            {
                // Get the protocol's own conformances (parent protocols)
                if let Some(parent_conformances) = protocol_symbol
                    .metadata()
                    .get_behavior::<ConformancesBehavior>()
                {
                    for parent in parent_conformances.conformances() {
                        if let TyKind::Protocol {
                            symbol: parent_protocol,
                            ..
                        } = parent.kind()
                        {
                            let parent_id = parent_protocol.metadata().id();

                            // Skip if parent protocol has implicit conformance (like Copyable)
                            if let Some(feature) =
                                ctx.model.builtin_registry().protocol_feature(parent_id)
                                && let kestrel_semantic_tree::builtins::BuiltinKind::Protocol {
                                    implicit_conformance: true,
                                    ..
                                } = feature.definition().kind
                            {
                                continue;
                            }

                            // Check if the parent protocol is in our declared conformances
                            if !declared_protocol_ids.contains(&parent_id) {
                                let required_methods = ctx.model.query(ProtocolRequiredMethods {
                                    protocol_id: parent_id,
                                });

                                // Only report error if there are actually methods that need to be implemented
                                if !required_methods.is_empty() {
                                    let child_name =
                                        protocol_symbol.metadata().name().value.clone();
                                    let parent_name =
                                        parent_protocol.metadata().name().value.clone();
                                    let struct_name = symbol.metadata().name().value.clone();

                                    ctx.report(MissingParentProtocolConformanceError {
                                        span: symbol.metadata().span().clone(),
                                        struct_name,
                                        child_protocol: child_name,
                                        parent_protocol: parent_name,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
