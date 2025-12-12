//! Validator for detecting circular generic constraint dependencies
//!
//! This validator detects cycles in generic where clause constraints.
//! Currently, direct cycles like `T: U, U: T` are already caught by the
//! "non-protocol bound" validation (type parameters can't be used as bounds).
//!
//! This validator detects more subtle cycles that could occur through protocol
//! type parameters, such as:
//! - `func foo[T: Proto[U], U: Proto[T]]()` where the protocol constraints
//!   create circular dependencies during instantiation.
//!
//! The algorithm collects symbols with generics during the walk, builds a
//! dependency graph, and detects cycles in the finalize phase.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Constraint, Ty, TyKind, WhereClause};
use semantic_tree::cycle::CycleDetector;
use semantic_tree::symbol::{Symbol, SymbolId};

use kestrel_semantic_model::SemanticModel;
use crate::diagnostics::{CircularConstraintError, CycleMember};
use crate::syntax::get_file_id_for_symbol;
use crate::validation::{SymbolContext, Validator};

/// Validator that detects circular generic constraint dependencies
pub struct ConstraintCycleValidator {
    /// Collected symbols with generics during the walk
    generic_symbols: Mutex<Vec<CollectedGeneric>>,
}

/// Data collected for symbols with generics
struct CollectedGeneric {
    symbol: Arc<dyn Symbol<KestrelLanguage>>,
    type_params: Vec<Arc<TypeParameterSymbol>>,
    where_clause: WhereClause,
}

impl ConstraintCycleValidator {
    const NAME: &'static str = "constraint_cycles";

    pub fn new() -> Self {
        Self {
            generic_symbols: Mutex::new(Vec::new()),
        }
    }
}

impl Default for ConstraintCycleValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for ConstraintCycleValidator {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn validate_symbol(&self, ctx: &SymbolContext<'_>) {
        let kind = ctx.symbol.metadata().kind();
        let symbol_ref: &dyn Symbol<KestrelLanguage> = ctx.symbol.as_ref();

        // Collect symbols with type parameters
        let collected = match kind {
            KestrelSymbolKind::Struct => {
                symbol_ref.as_any().downcast_ref::<StructSymbol>().map(|s| {
                    CollectedGeneric {
                        symbol: ctx.symbol.clone(),
                        type_params: s.type_parameters().to_vec(),
                        where_clause: s.where_clause().clone(),
                    }
                })
            }
            KestrelSymbolKind::Function => {
                symbol_ref.as_any().downcast_ref::<FunctionSymbol>().map(|s| {
                    CollectedGeneric {
                        symbol: ctx.symbol.clone(),
                        type_params: s.type_parameters().to_vec(),
                        where_clause: s.where_clause().clone(),
                    }
                })
            }
            KestrelSymbolKind::Protocol => {
                symbol_ref.as_any().downcast_ref::<ProtocolSymbol>().map(|s| {
                    CollectedGeneric {
                        symbol: ctx.symbol.clone(),
                        type_params: s.type_parameters().to_vec(),
                        where_clause: s.where_clause().clone(),
                    }
                })
            }
            KestrelSymbolKind::TypeAlias => {
                symbol_ref.as_any().downcast_ref::<TypeAliasSymbol>().map(|s| {
                    CollectedGeneric {
                        symbol: ctx.symbol.clone(),
                        type_params: s.type_parameters().to_vec(),
                        where_clause: s.where_clause().clone(),
                    }
                })
            }
            _ => None,
        };

        if let Some(collected) = collected {
            if !collected.type_params.is_empty() && !collected.where_clause.is_empty() {
                self.generic_symbols.lock().unwrap().push(collected);
            }
        }
    }

    fn finalize(&self, _model: &SemanticModel, diagnostics: &mut DiagnosticContext) {
        // Check each collected symbol for constraint cycles
        for collected in self.generic_symbols.lock().unwrap().iter() {
            check_constraint_cycles(
                &collected.type_params,
                &collected.where_clause,
                &collected.symbol,
                diagnostics,
            );
        }
    }
}

/// Check for cycles in generic constraint dependencies
///
/// Builds a dependency graph where type parameter T depends on type parameter U
/// if T has a bound that references U (e.g., `T: Protocol[U]`).
fn check_constraint_cycles(
    type_params: &[Arc<TypeParameterSymbol>],
    where_clause: &WhereClause,
    container: &Arc<dyn Symbol<KestrelLanguage>>,
    diagnostics: &mut DiagnosticContext,
) {
    let file_id = get_file_id_for_symbol(container, diagnostics);

    // Build dependency graph: param_id -> [param_ids it depends on]
    let mut dependencies: HashMap<SymbolId, Vec<SymbolId>> = HashMap::new();

    // Map from param_id to param symbol for error reporting
    let param_map: HashMap<SymbolId, &Arc<TypeParameterSymbol>> = type_params
        .iter()
        .map(|p| (p.metadata().id(), p))
        .collect();

    for constraint in &where_clause.constraints {
        match constraint {
            Constraint::TypeBound { param: Some(param_id), bounds, .. } => {
                // Find all type parameters referenced in the bounds
                for bound in bounds {
                    let referenced_params = collect_type_param_references(bound);
                    for ref_id in referenced_params {
                        // Only add dependency if it's to another type parameter in this scope
                        if param_map.contains_key(&ref_id) && ref_id != *param_id {
                            dependencies
                                .entry(*param_id)
                                .or_insert_with(Vec::new)
                                .push(ref_id);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Check for cycles using DFS from each type parameter
    for param in type_params {
        let param_id = param.metadata().id();
        let mut detector: CycleDetector<SymbolId> = CycleDetector::new();

        if let Some(cycle) = detect_dependency_cycle(param_id, &dependencies, &mut detector) {
            let origin = CycleMember {
                name: param.metadata().name().value.clone(),
                span: param.metadata().name().span.clone(),
            };

            let cycle_members: Vec<CycleMember> = cycle
                .cycle()
                .iter()
                .skip(1) // Skip the origin
                .filter_map(|id| {
                    param_map.get(id).map(|p| CycleMember {
                        name: p.metadata().name().value.clone(),
                        span: p.metadata().name().span.clone(),
                    })
                })
                .collect();

            diagnostics.throw(
                CircularConstraintError {
                    origin,
                    cycle: cycle_members,
                });

            // Only report one cycle per generic container
            break;
        }
    }
}

/// Detect cycles in the dependency graph using DFS
fn detect_dependency_cycle(
    start: SymbolId,
    dependencies: &HashMap<SymbolId, Vec<SymbolId>>,
    detector: &mut CycleDetector<SymbolId>,
) -> Option<semantic_tree::cycle::Cycle<SymbolId>> {
    if let Err(cycle) = detector.enter(start) {
        return Some(cycle);
    }

    if let Some(deps) = dependencies.get(&start) {
        for &dep in deps {
            if let Some(cycle) = detect_dependency_cycle(dep, dependencies, detector) {
                detector.exit();
                return Some(cycle);
            }
        }
    }

    detector.exit();
    None
}

/// Collect all type parameter SymbolIds referenced in a type
fn collect_type_param_references(ty: &Ty) -> Vec<SymbolId> {
    let mut refs = Vec::new();
    collect_type_param_refs_recursive(ty, &mut refs);
    refs
}

fn collect_type_param_refs_recursive(ty: &Ty, refs: &mut Vec<SymbolId>) {
    match ty.kind() {
        TyKind::TypeParameter(param) => {
            refs.push(param.metadata().id());
        }
        TyKind::Struct { substitutions, .. } => {
            for sub_ty in substitutions.types() {
                collect_type_param_refs_recursive(sub_ty, refs);
            }
        }
        TyKind::Protocol { substitutions, .. } => {
            for sub_ty in substitutions.types() {
                collect_type_param_refs_recursive(sub_ty, refs);
            }
        }
        TyKind::TypeAlias { substitutions, .. } => {
            for sub_ty in substitutions.types() {
                collect_type_param_refs_recursive(sub_ty, refs);
            }
        }
        TyKind::Tuple(elements) => {
            for elem in elements {
                collect_type_param_refs_recursive(elem, refs);
            }
        }
        TyKind::Array(elem) => {
            collect_type_param_refs_recursive(elem, refs);
        }
        TyKind::Function { params, return_type } => {
            for param in params {
                collect_type_param_refs_recursive(param, refs);
            }
            collect_type_param_refs_recursive(return_type, refs);
        }
        // Primitives and other types don't reference type parameters
        _ => {}
    }
}
