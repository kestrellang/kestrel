use std::collections::HashMap;
use std::sync::Arc;

use kestrel_semantic_model::SymbolFor;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::{
    FlattenedAssociatedType, FlattenedMethod, FlattenedProtocolBehavior, ProtocolSymbol,
};
use kestrel_semantic_tree::ty::TyKind;
use semantic_tree::cycle::{Cycle, CycleDetector};
use semantic_tree::symbol::Symbol;

use crate::diagnostics::{CircularProtocolInheritanceError, InheritedAssociatedTypeConflictError};
use crate::declaration_binder::BindingContext;

/// Flatten a protocol's inheritance hierarchy, collecting all methods and associated types.
pub fn flatten_protocol(
    protocol: &Arc<ProtocolSymbol>,
    ctx: &mut BindingContext,
) -> Option<FlattenedProtocolBehavior> {
    let mut methods: HashMap<String, Vec<FlattenedMethod>> = HashMap::new();
    let mut associated_types: HashMap<String, FlattenedAssociatedType> = HashMap::new();
    let mut cycle_detector = CycleDetector::new();
    let mut visited = std::collections::HashSet::new();
    let mut max_depth = 0;

    match flatten_protocol_recursive(
        protocol,
        &mut methods,
        &mut associated_types,
        &mut cycle_detector,
        &mut visited,
        0,
        &mut max_depth,
        ctx,
    ) {
        Ok(_) => Some(FlattenedProtocolBehavior::new(
            methods,
            associated_types,
            max_depth,
        )),
        Err(cycle) => {
            // Report cycle error
            let cycle_path: Vec<String> = cycle
                .path()
                .iter()
                .filter_map(|id| {
                    ctx.model
                        .query(SymbolFor { id: *id })
                        .map(|s| s.metadata().name().value.clone())
                })
                .collect();

            let error = CircularProtocolInheritanceError {
                protocol_name: protocol.metadata().name().value.clone(),
                span: protocol.metadata().span().clone(),
                cycle: cycle_path,
            };

            ctx.diagnostics.throw(error);
            None
        }
    }
}

fn flatten_protocol_recursive(
    protocol: &Arc<ProtocolSymbol>,
    methods: &mut HashMap<String, Vec<FlattenedMethod>>,
    associated_types: &mut HashMap<String, FlattenedAssociatedType>,
    cycle_detector: &mut CycleDetector<semantic_tree::symbol::SymbolId>,
    visited: &mut std::collections::HashSet<semantic_tree::symbol::SymbolId>,
    depth: usize,
    max_depth: &mut usize,
    ctx: &mut BindingContext,
) -> Result<(), Cycle<semantic_tree::symbol::SymbolId>> {
    let protocol_id = protocol.metadata().id();

    // Skip if already visited (prevents duplicate processing in diamond inheritance)
    if visited.contains(&protocol_id) {
        return Ok(());
    }

    // Enter the node
    cycle_detector.enter(protocol_id)?;

    // Mark as visited
    visited.insert(protocol_id);

    *max_depth = (*max_depth).max(depth);

    let protocol_name = protocol.metadata().name().value.clone();

    // First recurse into inherited protocols (so we get parent definitions first)
    let result =
        if let Some(conformances) = protocol.metadata().get_behavior::<ConformancesBehavior>() {
            let mut res = Ok(());
            for parent_ty in conformances.conformances() {
                if let TyKind::Protocol { symbol: parent, .. } = parent_ty.kind() {
                    if let Err(e) = flatten_protocol_recursive(
                        parent,
                        methods,
                        associated_types,
                        cycle_detector,
                        visited,
                        depth + 1,
                        max_depth,
                        ctx,
                    ) {
                        res = Err(e);
                        break;
                    }
                }
            }
            res
        } else {
            Ok(())
        };

    // If recursion failed, exit and return error
    if result.is_err() {
        cycle_detector.exit();
        return result;
    }

    // Then collect direct members (allowing overrides)
    for child in protocol.metadata().children() {
        match child.metadata().kind() {
            KestrelSymbolKind::Function => {
                let method_name = child.metadata().name().value.clone();
                methods
                    .entry(method_name)
                    .or_default()
                    .push(FlattenedMethod {
                        symbol: child.clone(),
                        source_protocol_name: protocol_name.clone(),
                        definition_span: child.metadata().name().span.clone(),
                    });
            }
            KestrelSymbolKind::AssociatedType => {
                let type_name = child.metadata().name().value.clone();

                // Check for conflict with inherited associated type
                if let Some(existing) = associated_types.get(&type_name) {
                    // Conflict: same associated type name from different protocols
                    let error = InheritedAssociatedTypeConflictError {
                        type_name: type_name.clone(),
                        span: child.metadata().span().clone(),
                        protocol1: existing.source_protocol_name.clone(),
                        protocol2: protocol_name.clone(),
                        definition_span1: existing.definition_span.clone(),
                        definition_span2: child.metadata().name().span.clone(),
                    };
                    ctx.diagnostics.throw(error);
                    continue; // Skip this one, keep existing
                }

                if let Ok(assoc_type) = child.clone().downcast_arc::<AssociatedTypeSymbol>() {
                    associated_types.insert(
                        type_name,
                        FlattenedAssociatedType {
                            symbol: assoc_type,
                            source_protocol_name: protocol_name.clone(),
                            definition_span: child.metadata().name().span.clone(),
                        },
                    );
                }
            }
            _ => {}
        }
    }

    // Manually exit the node
    cycle_detector.exit();

    Ok(())
}
