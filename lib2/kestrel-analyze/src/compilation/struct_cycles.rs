//! # Struct Cycle Analyzer
//!
//! Detects circular struct containment that would produce infinite-size types.
//! A struct that (transitively) contains itself by value cannot be represented
//! in memory because its layout would be infinitely recursive.
//!
//! The walk follows struct field types through other structs and tuples, but
//! stops at heap-indirected types (Array, Optional, pointers) and function types.
//! Computed properties are skipped since they don't store values.
//!
//! ## Diagnostics
//!
//! ### E449 -- `self_containing_struct` (Error, Correctness)
//!
//! **Message:** "struct '{name}' contains itself through field '{field_name}'"
//!
//! ### E450 -- `circular_struct_containment` (Error, Correctness)
//!
//! **Message:** "circular struct containment: '{origin}' -> ... -> '{origin}'"

use std::collections::HashSet;

use crate::context::CompilationContext;
use crate::diagnostic::*;
use crate::traits::{CompilationCheck, Describe};
use crate::util;
use kestrel_ast::AstType;
use kestrel_ast_builder::{Callable, Name, NodeKind, TypeAnnotation};
use kestrel_hecs::Entity;
use kestrel_name_res::{ResolveTypePath, TypeResolution};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E449",
        name: "self_containing_struct",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E450",
        name: "circular_struct_containment",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct StructCycleAnalyzer;

impl Describe for StructCycleAnalyzer {
    fn id(&self) -> &'static str {
        "struct_cycles"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl CompilationCheck for StructCycleAnalyzer {
    fn check(&self, cx: &CompilationContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();
        let mut checked = HashSet::new();

        // Collect all structs by walking from the root
        let mut structs = Vec::new();
        collect_structs(cx, cx.root, &mut structs);

        for entity in structs {
            if checked.contains(&entity) {
                continue;
            }

            // DFS from this struct to find cycles
            let mut path = Vec::new();
            let mut active = HashSet::new();
            if let Some(result) = find_cycle(cx, entity, &mut path, &mut active) {
                emit_cycle_diagnostic(cx, &result, &mut diags);
                for &e in &result.cycle {
                    checked.insert(e);
                }
            }
            checked.insert(entity);
        }

        diags
    }
}

/// Recursively collect all Struct entities from the module tree.
fn collect_structs(cx: &CompilationContext<'_>, entity: Entity, out: &mut Vec<Entity>) {
    if cx.query.get::<NodeKind>(entity) == Some(&NodeKind::Struct) {
        out.push(entity);
    }
    for &child in cx.query.children_of(entity) {
        collect_structs(cx, child, out);
    }
}

/// Result of cycle detection: the cycle path (struct entities) and the field that closes it.
struct CycleResult {
    cycle: Vec<Entity>,
    closing_field: Entity,
}

/// DFS to find a cycle starting from `entity`. Returns the cycle path if found.
/// `path` tracks the current DFS stack, `active` tracks entities in the stack.
fn find_cycle(
    cx: &CompilationContext<'_>,
    entity: Entity,
    path: &mut Vec<Entity>,
    active: &mut HashSet<Entity>,
) -> Option<CycleResult> {
    if active.contains(&entity) {
        // Found a cycle — extract the cycle portion from the path
        let cycle_start = path.iter().position(|&e| e == entity).unwrap();
        let mut cycle = path[cycle_start..].to_vec();
        cycle.push(entity); // close the cycle
        // closing_field will be set by the caller
        return Some(CycleResult {
            cycle,
            closing_field: entity,
        });
    }

    active.insert(entity);
    path.push(entity);

    // Walk stored fields of this struct
    for &child in cx.query.children_of(entity) {
        // Only check stored fields (not computed properties, methods, etc.)
        if cx.query.get::<NodeKind>(child) != Some(&NodeKind::Field) {
            continue;
        }
        // Skip computed properties (they have a Callable component for the getter)
        if cx.query.get::<Callable>(child).is_some() {
            continue;
        }

        let Some(ann) = cx.query.get::<TypeAnnotation>(child) else {
            continue;
        };

        // Check if the field's type leads to a cycle
        if let Some(result) = check_type_for_cycle(cx, &ann.0, entity, path, active) {
            // Only set closing_field if it hasn't been set yet (preserve innermost)
            if result.closing_field == result.cycle[0] {
                return Some(CycleResult {
                    closing_field: child,
                    ..result
                });
            }
            return Some(result);
        }
    }

    path.pop();
    active.remove(&entity);
    None
}

/// Check if a type (transitively) contains a struct that's in the active DFS path.
/// Follows struct fields and tuples; stops at heap-indirected types.
fn check_type_for_cycle(
    cx: &CompilationContext<'_>,
    ty: &AstType,
    context: Entity,
    path: &mut Vec<Entity>,
    active: &mut HashSet<Entity>,
) -> Option<CycleResult> {
    match ty {
        AstType::Named { segments, .. } => {
            // Resolve the type name to an entity
            let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
            let result = cx.query.query(ResolveTypePath {
                segments: seg_names,
                context,
                root: cx.root,
            });

            if let TypeResolution::Found(entity) = result {
                if cx.query.get::<NodeKind>(entity) == Some(&NodeKind::Struct) {
                    // Recurse into the struct
                    return find_cycle(cx, entity, path, active);
                }
            }
            None
        },
        AstType::Tuple(elements, _) => {
            // Tuples are inline — recurse into each element
            for elem in elements {
                if let Some(cycle) = check_type_for_cycle(cx, elem, context, path, active) {
                    return Some(cycle);
                }
            }
            None
        },
        // Array, Optional, Dictionary, Result, Function — heap-indirected, stop here
        _ => None,
    }
}

/// Emit a diagnostic for a detected cycle. Primary label on the closing field.
fn emit_cycle_diagnostic(
    cx: &CompilationContext<'_>,
    result: &CycleResult,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let cycle = &result.cycle;
    if cycle.len() <= 1 {
        return;
    }

    let origin = cycle[0];
    let origin_name = cx
        .query
        .get::<Name>(origin)
        .map(|n| n.0.clone())
        .unwrap_or_else(|| "?".into());

    // Primary label points at the field that closes the cycle
    let field_span = util::entity_span(cx.query, result.closing_field);

    if cycle.len() == 2 {
        // Self-cycle: struct contains itself directly
        diags.push(AnalyzeDiagnostic {
            descriptor_id: "E449",
            severity: Severity::Error,
            message: format!("struct '{}' cannot contain itself", origin_name),
            labels: vec![DiagLabel {
                span: field_span,
                message: "self-referencing field".into(),
                is_primary: true,
            }],
            notes: vec!["use an Array or Optional to break the cycle with heap indirection".into()],
        });
    } else {
        // Multi-struct cycle
        let cycle_names: Vec<String> = cycle
            .iter()
            .filter_map(|&e| cx.query.get::<Name>(e).map(|n| n.0.clone()))
            .collect();
        let cycle_str = cycle_names.join(" -> ");

        diags.push(AnalyzeDiagnostic {
            descriptor_id: "E450",
            severity: Severity::Error,
            message: format!("circular struct containment: {}", cycle_str),
            labels: vec![DiagLabel {
                span: field_span,
                message: "cycle closes here".into(),
                is_primary: true,
            }],
            notes: vec!["use an Array or Optional to break the cycle with heap indirection".into()],
        });
    }
}
