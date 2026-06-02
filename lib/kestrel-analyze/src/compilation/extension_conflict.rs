//! # Extension Conflict Analyzer
//!
//! Detects conflicts between extension methods and the target type's own
//! methods, and between methods in different extensions of the same type.
//!
//! CompilationCheck that walks all extensions, groups by target entity,
//! and checks for method name collisions.
//!
//! ## Diagnostics
//!
//! ### E411 -- `struct_extension_method_conflict` (Error, Correctness)
//! **Message:** "duplicate method '{name}': defined on both the type and an extension"
//!
//! ### E412 -- `duplicate_extension_method` (Error, Correctness)
//! **Message:** "duplicate method '{name}' in extensions of '{type}'"

use std::collections::HashMap;

use crate::context::CompilationContext;
use crate::diagnostic::*;
use crate::traits::{CompilationCheck, Describe};
use crate::util;
use kestrel_ast::AstType;
use kestrel_ast_builder::{Callable, ConformanceItem, Conformances, Name, NodeKind, WhereClause};
use kestrel_hecs::Entity;
use kestrel_name_res::ExtensionTargetEntity;
use kestrel_name_res::conformances::{extract_ast_type_args, resolve_conformance_entity};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E411",
        name: "struct_extension_method_conflict",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E412",
        name: "duplicate_extension_method",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct ExtensionConflictAnalyzer;

impl Describe for ExtensionConflictAnalyzer {
    fn id(&self) -> &'static str {
        "extension_conflict"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl CompilationCheck for ExtensionConflictAnalyzer {
    fn check(&self, cx: &CompilationContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        // Step 1: Collect all extensions and group by target entity
        let mut extensions_by_target: HashMap<Entity, Vec<Entity>> = HashMap::new();
        collect_extensions(cx, cx.root, &mut extensions_by_target);

        // Step 2: For each target, check for method conflicts
        for (target, extensions) in &extensions_by_target {
            // Collect methods on the target type itself
            let target_methods = collect_named_children(cx, *target);

            // Collect methods from each extension, tracking source
            let mut ext_methods: Vec<(String, Entity, Entity)> = Vec::new(); // (name, method, extension)
            for &ext in extensions {
                for (name, method) in collect_named_children(cx, ext) {
                    ext_methods.push((name, method, ext));
                }
            }

            // Check E411: struct/enum method vs extension method
            // Only applies to structs/enums — protocol extension defaults are expected
            let target_name = util::entity_name(cx.query, *target);
            let target_kind = cx.query.get::<NodeKind>(*target);
            let is_concrete_type = matches!(target_kind, Some(NodeKind::Struct | NodeKind::Enum));
            if is_concrete_type {
                for (ext_name, ext_method, _ext) in &ext_methods {
                    if let Some((_, struct_method)) = target_methods
                        .iter()
                        .find(|(n, m)| n == ext_name && same_labels(cx, *m, *ext_method))
                    {
                        diags.push(AnalyzeDiagnostic {
                            descriptor_id: DESCRIPTORS[0].id,
                            severity: DESCRIPTORS[0].default_severity,
                            message: format!(
                                "duplicate method '{}': defined on both '{}' and an extension",
                                ext_name, target_name,
                            ),
                            labels: vec![
                                DiagLabel {
                                    span: util::entity_span(cx.query, *struct_method),
                                    message: "method defined here on type".into(),
                                    is_primary: false,
                                },
                                DiagLabel {
                                    span: util::entity_span(cx.query, *ext_method),
                                    message: "conflicting extension method".into(),
                                    is_primary: true,
                                },
                            ],
                            notes: vec![],
                        });
                    }
                }
            } // is_concrete_type

            // Check E412: extension vs extension method (same specificity)
            // Only for concrete types. Skip if either extension has where clauses
            // (where clauses may differentiate the extensions, preventing real overlap).
            if !is_concrete_type {
                continue;
            }
            for i in 0..ext_methods.len() {
                for j in (i + 1)..ext_methods.len() {
                    let (name_i, method_i, ext_i) = &ext_methods[i];
                    let (name_j, method_j, ext_j) = &ext_methods[j];

                    if name_i != name_j || ext_i == ext_j || !same_labels(cx, *method_i, *method_j)
                    {
                        continue;
                    }

                    // Skip if either extension has where clauses (may not overlap)
                    if has_where_clause(cx, *ext_i) || has_where_clause(cx, *ext_j) {
                        continue;
                    }

                    // Check if extensions have same specificity
                    let spec_i = extension_specificity(cx, *ext_i);
                    let spec_j = extension_specificity(cx, *ext_j);
                    if spec_i != spec_j {
                        continue; // Different specificity — not a conflict
                    }

                    // Distinct parameterized-protocol conformances: the two
                    // methods witness different instantiations of the same
                    // protocol (e.g. `Subtractable[Duration]` vs
                    // `Subtractable[Instant]`). Dispatch goes through the
                    // protocol by argument type, not an ambiguous direct call,
                    // so this is legal — and mirrors multi-conformance declared
                    // on the type body, which this analyzer never flags.
                    if witnesses_distinct_instantiation(cx, *ext_i, *ext_j, name_i) {
                        continue;
                    }

                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[1].id,
                        severity: DESCRIPTORS[1].default_severity,
                        message: format!(
                            "duplicate method '{}' in extensions of '{}'",
                            name_i, target_name,
                        ),
                        labels: vec![
                            DiagLabel {
                                span: util::entity_span(cx.query, *method_i),
                                message: "first definition here".into(),
                                is_primary: false,
                            },
                            DiagLabel {
                                span: util::entity_span(cx.query, *method_j),
                                message: "conflicting definition here".into(),
                                is_primary: true,
                            },
                        ],
                        notes: vec![],
                    });
                }
            }
        }

        diags
    }
}

/// Recursively collect all Extension entities and group by resolved target.
fn collect_extensions(
    cx: &CompilationContext<'_>,
    entity: Entity,
    out: &mut HashMap<Entity, Vec<Entity>>,
) {
    if cx.query.get::<NodeKind>(entity) == Some(&NodeKind::Extension)
        && let Some(target) = cx.query.query(ExtensionTargetEntity {
            extension: entity,
            root: cx.root,
        })
    {
        out.entry(target).or_default().push(entity);
    }
    for &child in cx.query.children_of(entity) {
        collect_extensions(cx, child, out);
    }
}

/// Collect named Function/Subscript children of an entity.
fn collect_named_children(cx: &CompilationContext<'_>, entity: Entity) -> Vec<(String, Entity)> {
    cx.query
        .children_of(entity)
        .iter()
        .filter(|&&child| {
            matches!(
                cx.query.get::<NodeKind>(child),
                Some(NodeKind::Function | NodeKind::Subscript)
            )
        })
        .filter_map(|&child| {
            let name = cx.query.get::<Name>(child)?.0.clone();
            Some((name, child))
        })
        .collect()
}

/// Check if two callable entities have the same parameter labels.
/// Methods with the same name but different labels are overloads, not conflicts.
fn same_labels(cx: &CompilationContext<'_>, a: Entity, b: Entity) -> bool {
    let labels_a: Vec<Option<String>> = cx
        .query
        .get::<Callable>(a)
        .map(|c| c.params.iter().map(|p| p.label.clone()).collect())
        .unwrap_or_default();
    let labels_b: Vec<Option<String>> = cx
        .query
        .get::<Callable>(b)
        .map(|c| c.params.iter().map(|p| p.label.clone()).collect())
        .unwrap_or_default();
    labels_a == labels_b
}

/// Check if an extension has any where clause constraints.
fn has_where_clause(cx: &CompilationContext<'_>, extension: Entity) -> bool {
    cx.query
        .get::<WhereClause>(extension)
        .is_some_and(|wc| !wc.0.is_empty())
}

/// Count concrete (non-type-parameter) type args on an extension target.
fn extension_specificity(cx: &CompilationContext<'_>, extension: Entity) -> usize {
    use kestrel_hir::ty::HirTy;
    use kestrel_hir_lower::LowerExtensionTargetTypeArgs;

    cx.query
        .query(LowerExtensionTargetTypeArgs {
            extension,
            root: cx.root,
        })
        .map(|args| {
            args.iter()
                .filter(|t| !matches!(t, HirTy::Param(..)))
                .count()
        })
        .unwrap_or(0)
}

/// True when `ext_i` and `ext_j` both declare conformance to the *same*
/// protocol with *different* type arguments, and `method` is a requirement of
/// that protocol. In that case the two same-named methods are witnesses to
/// distinct protocol instantiations (e.g. `Subtractable[Duration]` and
/// `Subtractable[Instant]`), dispatched by the protocol — not a true duplicate.
fn witnesses_distinct_instantiation(
    cx: &CompilationContext<'_>,
    ext_i: Entity,
    ext_j: Entity,
    method: &str,
) -> bool {
    let confs_i = extension_conformance_instantiations(cx, ext_i);
    let confs_j = extension_conformance_instantiations(cx, ext_j);
    for (proto_i, args_i) in &confs_i {
        for (proto_j, args_j) in &confs_j {
            // Same protocol entity, different instantiation arguments.
            if proto_i == proto_j && args_i != args_j {
                // Only a non-conflict if `method` is actually a requirement of
                // that protocol — unrelated inherent helpers still conflict.
                if collect_named_children(cx, *proto_i)
                    .iter()
                    .any(|(name, _)| name == method)
                {
                    return true;
                }
            }
        }
    }
    false
}

/// The protocol instantiations an extension declares conformance to, as
/// `(protocol_entity, args_key)` pairs. `args_key` is a structural string of
/// the conformance's type arguments so distinct instantiations compare unequal.
fn extension_conformance_instantiations(
    cx: &CompilationContext<'_>,
    ext: Entity,
) -> Vec<(Entity, String)> {
    let Some(conformances) = cx.query.get::<Conformances>(ext) else {
        return Vec::new();
    };
    conformances
        .0
        .iter()
        .filter_map(|item| {
            let ConformanceItem::Positive(ast_ty, _) = item else {
                return None;
            };
            let proto = resolve_conformance_entity(cx.query, ast_ty, ext, cx.root)?;
            if cx.query.get::<NodeKind>(proto) != Some(&NodeKind::Protocol) {
                return None;
            }
            let args_key = extract_ast_type_args(ast_ty)
                .iter()
                .map(ast_type_key)
                .collect::<Vec<_>>()
                .join(",");
            Some((proto, args_key))
        })
        .collect()
}

/// A structural string key for an `AstType`, ignoring spans, so two type
/// arguments compare equal iff they name the same (possibly generic) type.
fn ast_type_key(ty: &AstType) -> String {
    match ty {
        AstType::Named { segments, .. } => {
            let path = segments
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join(".");
            match segments.last() {
                Some(seg) if !seg.type_args.is_empty() => {
                    let inner = seg
                        .type_args
                        .iter()
                        .map(ast_type_key)
                        .collect::<Vec<_>>()
                        .join(",");
                    format!("{path}[{inner}]")
                }
                _ => path,
            }
        }
        other => format!("{other:?}"),
    }
}
