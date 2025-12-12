//! Analyzer for detecting circular type alias dependencies.
//!
//! Detects cycles like:
//! - Direct self-references: `type A = A`
//! - Two-way cycles: `type A = B; type B = A`
//! - Longer chains: `A -> B -> C -> A`

use std::sync::{Arc, Mutex};

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use super::type_alias_cycles::diagnostics::{CircularTypeAliasError, CycleParticipant};
use kestrel_semantic_model::{ResolvedAliasedType, SymbolFor};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::cycle::{Cycle, CycleDetector};
use semantic_tree::symbol::{Symbol, SymbolId};

/// Analyzer that detects circular type alias dependencies
pub struct TypeAliasCycleAnalyzer {
    type_aliases: Mutex<Vec<Arc<dyn Symbol<KestrelLanguage>>>>,
}

impl TypeAliasCycleAnalyzer {
    pub fn new() -> Self {
        Self {
            type_aliases: Mutex::new(Vec::new()),
        }
    }
}

impl Default for TypeAliasCycleAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for TypeAliasCycleAnalyzer {
    fn name(&self) -> &'static str {
        "type_alias_cycles"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        _ctx: &mut AnalysisContext,
    ) {
        if symbol.metadata().kind() == KestrelSymbolKind::TypeAlias {
            self.type_aliases.lock().unwrap().push(symbol.clone());
        }
    }

    fn finalize(&mut self, ctx: &mut AnalysisContext) {
        let model = ctx.model;
        for type_alias in self.type_aliases.lock().unwrap().iter() {
            check_type_alias_for_cycles(type_alias, model, ctx);
        }
    }
}

fn check_type_alias_for_cycles(
    type_alias: &Arc<dyn Symbol<KestrelLanguage>>,
    model: &kestrel_semantic_model::SemanticModel,
    ctx: &mut AnalysisContext,
) {
    // If there's no resolved type, skip (binding likely failed earlier)
    let alias_id = type_alias.metadata().id();
    let Some(resolved) = model.query(ResolvedAliasedType {
        type_alias_id: alias_id,
    }) else {
        return;
    };

    // Track visited aliases via CycleDetector
    let mut visited: CycleDetector<SymbolId> = CycleDetector::new();
    let _ = visited.enter(type_alias.metadata().id());

    if let Some(cycle) = follow_type_alias_chain(&resolved, model, &mut visited) {
        let origin = CycleParticipant {
            name: type_alias.metadata().name().value.clone(),
            name_span: type_alias.metadata().name().span.clone(),
        };

        let cycle_participants: Vec<CycleParticipant> = cycle
            .cycle()
            .iter()
            .skip(1) // skip the origin which is first in the cycle
            .filter_map(|&id| {
                model.query(SymbolFor { id }).map(|s| CycleParticipant {
                    name: s.metadata().name().value.clone(),
                    name_span: s.metadata().name().span.clone(),
                })
            })
            .collect();

        ctx.report(CircularTypeAliasError {
            origin,
            cycle: cycle_participants,
        });
    }

    visited.exit();
}

fn follow_type_alias_chain(
    ty: &Ty,
    model: &kestrel_semantic_model::SemanticModel,
    visited: &mut CycleDetector<SymbolId>,
) -> Option<Cycle<SymbolId>> {
    match ty.kind() {
        TyKind::TypeAlias { symbol, .. } => {
            let alias_id = symbol.metadata().id();
            if let Err(cycle) = visited.enter(alias_id) {
                return Some(cycle);
            }

            // Look up the resolved type of this alias and continue following
            let result = model
                .query(ResolvedAliasedType {
                    type_alias_id: alias_id,
                })
                .and_then(|resolved| follow_type_alias_chain(&resolved, model, visited));
            visited.exit();
            result
        }
        TyKind::Tuple(elements) => {
            for e in elements {
                if let Some(c) = follow_type_alias_chain(e, model, visited) {
                    return Some(c);
                }
            }
            None
        }
        TyKind::Function {
            params,
            return_type,
        } => {
            for p in params {
                if let Some(c) = follow_type_alias_chain(p, model, visited) {
                    return Some(c);
                }
            }
            follow_type_alias_chain(return_type, model, visited)
        }
        _ => None,
    }
}

pub mod diagnostics;
