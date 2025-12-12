//! Analyzer for detecting circular type alias dependencies.
//!
//! Detects cycles like:
//! - Direct self-references: `type A = A`
//! - Two-way cycles: `type A = B; type B = A`
//! - Longer chains: `A -> B -> C -> A`

use std::sync::{Arc, Mutex};

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use super::type_alias_cycles::diagnostics::{CircularTypeAliasError, CycleParticipant};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasTypedBehavior;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::cycle::{Cycle, CycleDetector};
use semantic_tree::symbol::{Symbol, SymbolId};

/// Analyzer that detects circular type alias dependencies
pub struct TypeAliasCycleAnalyzer {
    type_aliases: Mutex<Vec<Arc<dyn Symbol<KestrelLanguage>>>>,
}

impl TypeAliasCycleAnalyzer {
    pub fn new() -> Self { Self { type_aliases: Mutex::new(Vec::new()) } }
}

impl Default for TypeAliasCycleAnalyzer { fn default() -> Self { Self::new() } }

impl Analyzer for TypeAliasCycleAnalyzer {
    fn name(&self) -> &'static str { "type_alias_cycles" }

    fn visit_symbol(&mut self, symbol: &Arc<dyn Symbol<KestrelLanguage>>, _ctx: &mut AnalysisContext) {
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
    // Find the typed behavior that has the resolved aliased type
    let behaviors = type_alias.metadata().behaviors();
    let type_alias_typed = behaviors.iter().find_map(|b| {
        if matches!(b.kind(), KestrelBehaviorKind::TypeAliasTyped) {
            b.as_ref().downcast_ref::<TypeAliasTypedBehavior>()
        } else { None }
    });

    // If there's no resolved type, skip (binding likely failed earlier)
    let Some(resolved) = type_alias_typed else { return; };

    // Track visited aliases via CycleDetector
    let mut visited: CycleDetector<SymbolId> = CycleDetector::new();
    let _ = visited.enter(type_alias.metadata().id());

    if let Some(cycle) = follow_type_alias_chain(resolved.resolved_ty(), &mut visited) {
        let origin = CycleParticipant {
            name: type_alias.metadata().name().value.clone(),
            name_span: type_alias.metadata().name().span.clone(),
        };

        let cycle_participants: Vec<CycleParticipant> = cycle
            .cycle()
            .iter()
            .skip(1) // skip the origin which is first in the cycle
            .filter_map(|&id| {
                model.query(kestrel_semantic_model::SymbolFor { id }).map(|s| CycleParticipant {
                    name: s.metadata().name().value.clone(),
                    name_span: s.metadata().name().span.clone(),
                })
            })
            .collect();

        ctx.report(CircularTypeAliasError { origin, cycle: cycle_participants });
    }

    visited.exit();
}

fn follow_type_alias_chain(ty: &Ty, visited: &mut CycleDetector<SymbolId>) -> Option<Cycle<SymbolId>> {
    match ty.kind() {
        TyKind::TypeAlias { symbol, .. } => {
            let alias_id = symbol.metadata().id();
            if let Err(cycle) = visited.enter(alias_id) { return Some(cycle); }

            // Look up the resolved type of this alias and continue following
            let behaviors = symbol.metadata().behaviors();
            let type_alias_typed = behaviors.iter().find_map(|b| {
                if matches!(b.kind(), KestrelBehaviorKind::TypeAliasTyped) {
                    b.as_ref().downcast_ref::<TypeAliasTypedBehavior>()
                } else { None }
            });

            let result = if let Some(resolved) = type_alias_typed {
                follow_type_alias_chain(resolved.resolved_ty(), visited)
            } else { None };
            visited.exit();
            result
        }
        TyKind::Tuple(elements) => {
            for e in elements {
                if let Some(c) = follow_type_alias_chain(e, visited) { return Some(c); }
            }
            None
        }
        TyKind::Function { params, return_type } => {
            for p in params { if let Some(c) = follow_type_alias_chain(p, visited) { return Some(c); } }
            follow_type_alias_chain(return_type, visited)
        }
        _ => None,
    }
}

pub mod diagnostics;
