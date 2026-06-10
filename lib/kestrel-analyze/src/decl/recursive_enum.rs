//! # Recursive Enum Analyzer
//!
//! Detects recursive enums that reference themselves in case parameters without
//! the `indirect` keyword. Non-indirect enums have value semantics and must have
//! a known size at compile time, which is impossible if the enum recursively
//! contains itself.
//!
//! Walks case payload types transitively through structs, tuples, and other enums.
//! Stops at heap-indirected types (Array, Optional, pointers, functions).
//!
//! ## Diagnostics
//!
//! ### E429 -- `recursive_enum` (Error, Correctness)
//! **Message:** "enum '{name}' is recursive without 'indirect'"

use std::collections::HashSet;

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast::AstType;
use kestrel_ast_builder::{Callable, IsIndirect, NodeKind, TypeAnnotation};
use kestrel_hecs::Entity;
use kestrel_name_res::{ResolveTypePath, TypeResolution};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E429",
    name: "recursive_enum",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct RecursiveEnumAnalyzer;

impl Describe for RecursiveEnumAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::RecursiveEnum
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for RecursiveEnumAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Enum]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let entity = cx.entity;

        // Indirect enums are heap-allocated — recursion is fine
        if cx.query.get::<IsIndirect>(entity).is_some() {
            return vec![];
        }

        // Walk case payloads looking for a path back to this enum
        let mut visited = HashSet::new();
        if let Some(rec) = find_recursive_case(cx, entity, &mut visited) {
            // Direct recursion: primary on enum decl. Indirect: primary on case.
            let primary_entity = if rec.is_direct {
                entity
            } else {
                rec.case_entity
            };
            let span = util::entity_span(cx.query, primary_entity);
            return vec![AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[0].id,
                severity: DESCRIPTORS[0].default_severity,
                message: "recursive enum requires `indirect`".into(),
                labels: vec![DiagLabel {
                    span,
                    message: "recursive enum requires `indirect`".into(),
                    is_primary: true,
                }],
                notes: vec![
                    "add 'indirect' before the enum declaration to allow recursive cases".into(),
                ],
            }];
        }

        vec![]
    }
}

/// Result of recursive case detection.
struct RecursiveCase {
    case_entity: Entity,
    /// True if the case directly references the enum (not through another type)
    is_direct: bool,
}

/// Find the first enum case whose payload transitively references `target_enum`.
fn find_recursive_case(
    cx: &DeclContext<'_>,
    target_enum: Entity,
    visited: &mut HashSet<Entity>,
) -> Option<RecursiveCase> {
    for child in util::children_of_kind(cx.query, target_enum, NodeKind::EnumCase) {
        let Some(callable) = cx.query.get::<Callable>(child) else {
            continue; // valueless case
        };
        for param in &callable.params {
            if let Some(ref ty) = param.ty {
                // Check direct reference first
                if type_references_directly(cx, target_enum, ty, target_enum) {
                    return Some(RecursiveCase {
                        case_entity: child,
                        is_direct: true,
                    });
                }
                // Check transitive reference
                visited.clear();
                visited.insert(target_enum);
                if type_contains(cx, target_enum, ty, target_enum, visited) {
                    return Some(RecursiveCase {
                        case_entity: child,
                        is_direct: false,
                    });
                }
            }
        }
    }
    None
}

/// Check if a type directly names the target enum (no transitive walk).
fn type_references_directly(
    cx: &DeclContext<'_>,
    target_enum: Entity,
    ty: &AstType,
    context: Entity,
) -> bool {
    match ty {
        AstType::Named { segments, .. } => {
            let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
            let resolved = cx.query.query(ResolveTypePath {
                segments: seg_names,
                context,
                root: cx.root,
            });
            matches!(resolved, TypeResolution::Found(e) if e == target_enum)
        },
        AstType::Tuple(elements, _) => elements
            .iter()
            .any(|e| type_references_directly(cx, target_enum, e, context)),
        _ => false,
    }
}

/// Check if `entity` (a struct or enum) transitively contains `target_enum`.
fn entity_contains(
    cx: &DeclContext<'_>,
    target_enum: Entity,
    entity: Entity,
    visited: &mut HashSet<Entity>,
) -> bool {
    if !visited.insert(entity) {
        return false;
    }

    let kind = cx.query.get::<NodeKind>(entity);

    match kind {
        Some(NodeKind::Enum) => {
            // Walk enum case payloads
            for child in util::children_of_kind(cx.query, entity, NodeKind::EnumCase) {
                let Some(callable) = cx.query.get::<Callable>(child) else {
                    continue;
                };
                for param in &callable.params {
                    if let Some(ref ty) = param.ty
                        && type_contains(cx, target_enum, ty, entity, visited)
                    {
                        return true;
                    }
                }
            }
        },
        Some(NodeKind::Struct) => {
            // Walk stored fields
            for child in util::children_of_kind(cx.query, entity, NodeKind::Field) {
                // Skip computed properties (have a Callable for the getter)
                if cx.query.get::<Callable>(child).is_some() {
                    continue;
                }
                let Some(ann) = cx.query.get::<TypeAnnotation>(child) else {
                    continue;
                };
                if type_contains(cx, target_enum, &ann.0, entity, visited) {
                    return true;
                }
            }
        },
        _ => {},
    }

    false
}

/// Check if an AstType transitively contains the target enum entity.
/// Follows Named types through structs/enums, tuples inline.
/// Stops at heap-indirected types (Array, Optional, function, etc.).
fn type_contains(
    cx: &DeclContext<'_>,
    target_enum: Entity,
    ty: &AstType,
    context: Entity,
    visited: &mut HashSet<Entity>,
) -> bool {
    match ty {
        AstType::Named { segments, .. } => {
            let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
            let resolved = cx.query.query(ResolveTypePath {
                segments: seg_names,
                context,
                root: cx.root,
            });

            if let TypeResolution::Found(entity) = resolved {
                // Direct self-reference
                if entity == target_enum {
                    return true;
                }
                // Recurse into structs and non-indirect enums
                let kind = cx.query.get::<NodeKind>(entity);
                match kind {
                    Some(NodeKind::Struct) => {
                        return entity_contains(cx, target_enum, entity, visited);
                    },
                    Some(NodeKind::Enum) => {
                        // Only follow non-indirect enums (indirect = heap allocated)
                        if cx.query.get::<IsIndirect>(entity).is_none() {
                            return entity_contains(cx, target_enum, entity, visited);
                        }
                    },
                    _ => {},
                }
            }
            false
        },
        AstType::Tuple(elements, _) => {
            // Tuples are inline — check each element
            elements
                .iter()
                .any(|elem| type_contains(cx, target_enum, elem, context, visited))
        },
        // Array, Optional, Dictionary, Result, Function — heap-indirected, stop
        _ => false,
    }
}
