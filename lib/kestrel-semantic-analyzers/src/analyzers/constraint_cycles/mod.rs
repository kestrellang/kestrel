//! Analyzer for detecting circular generic constraint dependencies.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Constraint, Ty, TyKind, WhereClause};
use semantic_tree::cycle::{Cycle, CycleDetector};
use semantic_tree::symbol::{Symbol, SymbolId};

use diagnostics::{CircularConstraintError, CycleMember};

pub struct ConstraintCycleAnalyzer {
    generic_symbols: Mutex<Vec<CollectedGeneric>>,
}

struct CollectedGeneric {
    symbol: Arc<dyn Symbol<KestrelLanguage>>,
    type_params: Vec<Arc<TypeParameterSymbol>>,
    where_clause: WhereClause,
}

impl ConstraintCycleAnalyzer { pub fn new() -> Self { Self { generic_symbols: Mutex::new(Vec::new()) } } }
impl Default for ConstraintCycleAnalyzer { fn default() -> Self { Self::new() } }

impl Analyzer for ConstraintCycleAnalyzer {
    fn name(&self) -> &'static str { "constraint_cycles" }

    fn visit_symbol(&mut self, symbol: &Arc<dyn Symbol<KestrelLanguage>>, _ctx: &mut AnalysisContext) {
        let kind = symbol.metadata().kind();
        let symbol_ref: &dyn Symbol<KestrelLanguage> = symbol.as_ref();
        let collected = match kind {
            KestrelSymbolKind::Struct => {
                symbol_ref.as_any().downcast_ref::<StructSymbol>().map(|s| CollectedGeneric {
                    symbol: symbol.clone(),
                    type_params: s.type_parameters().to_vec(),
                    where_clause: s.where_clause().clone(),
                })
            }
            KestrelSymbolKind::Function => {
                symbol_ref.as_any().downcast_ref::<FunctionSymbol>().map(|s| CollectedGeneric {
                    symbol: symbol.clone(),
                    type_params: s.type_parameters().to_vec(),
                    where_clause: s.where_clause().clone(),
                })
            }
            KestrelSymbolKind::Protocol => {
                symbol_ref.as_any().downcast_ref::<ProtocolSymbol>().map(|s| CollectedGeneric {
                    symbol: symbol.clone(),
                    type_params: s.type_parameters().to_vec(),
                    where_clause: s.where_clause().clone(),
                })
            }
            KestrelSymbolKind::TypeAlias => {
                symbol_ref.as_any().downcast_ref::<TypeAliasSymbol>().map(|s| CollectedGeneric {
                    symbol: symbol.clone(),
                    type_params: s.type_parameters().to_vec(),
                    where_clause: s.where_clause().clone(),
                })
            }
            _ => None,
        };

        if let Some(c) = collected {
            if !c.type_params.is_empty() && !c.where_clause.is_empty() {
                self.generic_symbols.lock().unwrap().push(c);
            }
        }
    }

    fn finalize(&mut self, ctx: &mut AnalysisContext) {
        for collected in self.generic_symbols.lock().unwrap().iter() {
            check_constraint_cycles(&collected.type_params, &collected.where_clause, ctx);
        }
    }
}

fn check_constraint_cycles(
    type_params: &[Arc<TypeParameterSymbol>],
    where_clause: &WhereClause,
    ctx: &mut AnalysisContext,
) {
    // Build dependency graph: param_id -> [param_ids it depends on]
    let mut dependencies: HashMap<SymbolId, Vec<SymbolId>> = HashMap::new();
    let param_map: HashMap<SymbolId, &Arc<TypeParameterSymbol>> = type_params
        .iter()
        .map(|p| (p.metadata().id(), p))
        .collect();

    for constraint in &where_clause.constraints {
        if let Constraint::TypeBound { param: Some(param_id), bounds, .. } = constraint {
            for bound in bounds {
                let referenced_params = collect_type_param_references(bound);
                for ref_id in referenced_params {
                    if param_map.contains_key(&ref_id) && ref_id != *param_id {
                        dependencies.entry(*param_id).or_insert_with(Vec::new).push(ref_id);
                    }
                }
            }
        }
    }

    for param in type_params {
        let param_id = param.metadata().id();
        let mut detector: CycleDetector<SymbolId> = CycleDetector::new();
        if let Some(cycle) = detect_dependency_cycle(param_id, &dependencies, &mut detector) {
            let origin = CycleMember { name: param.metadata().name().value.clone(), span: param.metadata().name().span.clone() };
            let cycle_members: Vec<CycleMember> = cycle
                .cycle()
                .iter()
                .skip(1)
                .filter_map(|id| {
                    param_map.get(id).map(|p| CycleMember {
                        name: p.metadata().name().value.clone(),
                        span: p.metadata().name().span.clone(),
                    })
                })
                .collect();
            ctx.report(CircularConstraintError { origin, cycle: cycle_members });
            break;
        }
    }
}

fn detect_dependency_cycle(
    start: SymbolId,
    dependencies: &HashMap<SymbolId, Vec<SymbolId>>,
    detector: &mut CycleDetector<SymbolId>,
) -> Option<Cycle<SymbolId>> {
    if let Err(cycle) = detector.enter(start) { return Some(cycle); }
    if let Some(deps) = dependencies.get(&start) {
        for &dep in deps {
            if let Some(c) = detect_dependency_cycle(dep, dependencies, detector) {
                detector.exit();
                return Some(c);
            }
        }
    }
    detector.exit();
    None
}

fn collect_type_param_references(ty: &Ty) -> Vec<SymbolId> {
    let mut refs = Vec::new();
    collect_type_param_refs_recursive(ty, &mut refs);
    refs
}

fn collect_type_param_refs_recursive(ty: &Ty, refs: &mut Vec<SymbolId>) {
    match ty.kind() {
        TyKind::TypeParameter(param) => refs.push(param.metadata().id()),
        TyKind::Struct { substitutions, .. } | TyKind::Protocol { substitutions, .. } | TyKind::TypeAlias { substitutions, .. } => {
            for sub_ty in substitutions.types() { collect_type_param_refs_recursive(sub_ty, refs); }
        }
        TyKind::Tuple(elements) => { for e in elements { collect_type_param_refs_recursive(e, refs); } }
        TyKind::Array(elem) => collect_type_param_refs_recursive(elem, refs),
        TyKind::Function { params, return_type } => {
            for p in params { collect_type_param_refs_recursive(p, refs); }
            collect_type_param_refs_recursive(return_type, refs);
        }
        _ => {}
    }
}

pub mod diagnostics;

