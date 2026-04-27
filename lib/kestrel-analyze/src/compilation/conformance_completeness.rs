//! # Conformance Completeness Analyzer
//!
//! Checks that types satisfy all requirements of protocols they conform to.
//! When `extend Foo: SomeProtocol`, verifies that Foo (+ extensions) provides
//! all required methods and associated types.
//!
//! ## Diagnostics
//!
//! ### E454 -- `missing_protocol_method` (Error, Correctness)
//! **Message:** "type '{type}' does not implement method '{method}' from protocol '{proto}'"
//!
//! ### E455 -- `missing_associated_type` (Error, Correctness)
//! **Message:** "type '{type}' does not provide associated type '{name}' from protocol '{proto}'"
//!
//! ### E457 -- `where_clause_constraint_not_satisfied` (Error, Correctness)
//! **Message:** "type '{bound}' does not satisfy bound '{proto}' on associated type '{name}'"
//!
//! ### E458 -- `wrong_method_return_type` (Error, Correctness)
//! **Message:** "method '{name}' has wrong return type for protocol '{proto}'"
//!
//! ### E459 -- `wrong_method_receiver_kind` (Error, Correctness)
//! **Message:** "method '{name}' has wrong receiver kind for protocol '{proto}'"
//!
//! ### E460 -- `missing_property_setter` (Error, Correctness)
//! **Message:** "property '{name}' requires a setter to satisfy protocol '{proto}'"
//!
//! ### E462 -- `conflicting_associated_type` (Error, Correctness)
//! **Message:** "conflicting associated type '{name}' inherited by protocol '{proto}'"

use std::collections::{HashMap, HashSet};

use crate::context::CompilationContext;
use crate::diagnostic::*;
use crate::traits::{CompilationCheck, Describe};
use crate::util;
use kestrel_ast::AstType;
use kestrel_ast_builder::{
    Callable, ConformanceItem, Conformances, Name, NodeKind, QualifiedTarget, Settable,
    TypeAnnotation, WhereClause, WhereConstraint,
};
use kestrel_hecs::Entity;
use kestrel_hir_lower::{LowerCallableReturnType, LowerCallableTypes, LowerTypeAnnotation};
use kestrel_name_res::{
    ConformingProtocols, ExtensionTargetEntity, ExtensionsFor, ProtocolAssociatedTypes,
    ProtocolMembers, ResolveTypePath, TypeResolution,
};
use kestrel_type_infer::compare::{AssocBinding, TypeCompareEnv, compare_hir_types};
use kestrel_type_infer::result::ResolvedTy;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E454",
        name: "missing_protocol_method",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E455",
        name: "missing_associated_type",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E456",
        name: "protocol_property_type_mismatch",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E457",
        name: "where_clause_constraint_not_satisfied",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E458",
        name: "wrong_method_return_type",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E459",
        name: "wrong_method_receiver_kind",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E460",
        name: "missing_property_setter",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E462",
        name: "conflicting_associated_type",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E463",
        name: "ambiguous_method_satisfies_multiple_protocols",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct ConformanceCompletenessAnalyzer;

impl Describe for ConformanceCompletenessAnalyzer {
    fn id(&self) -> &'static str {
        "conformance_completeness"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl CompilationCheck for ConformanceCompletenessAnalyzer {
    fn check(&self, cx: &CompilationContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        // Walk all entities to find conformance declarations
        check_entity(cx, cx.root, &mut diags);

        diags
    }
}

fn check_entity(cx: &CompilationContext<'_>, entity: Entity, diags: &mut Vec<AnalyzeDiagnostic>) {
    let kind = cx.query.get::<NodeKind>(entity);

    if kind == Some(&NodeKind::Protocol) {
        check_associated_type_conflicts(cx, entity, diags);
    }

    // Check struct/enum declarations with direct conformances
    if matches!(kind, Some(NodeKind::Struct | NodeKind::Enum)) {
        check_type_conformances(cx, entity, entity, diags);
        check_ambiguous_method_satisfaction(cx, entity, diags);
    }

    // Check extensions that add conformances
    if kind == Some(&NodeKind::Extension) {
        if let Some(target) = cx.query.query(ExtensionTargetEntity {
            extension: entity,
            root: cx.root,
        }) {
            // Only check if this extension declares new conformances
            if let Some(conf) = cx.query.get::<Conformances>(entity) {
                if !conf.0.is_empty() {
                    check_extension_conformances(cx, entity, target, diags);
                }
            }
        }
    }

    for &child in cx.query.children_of(entity) {
        check_entity(cx, child, diags);
    }
}

/// Check that a type satisfies all its directly declared protocol conformances.
fn check_type_conformances(
    cx: &CompilationContext<'_>,
    entity: Entity,
    conforming_entity: Entity,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let Some(conformances) = cx.query.get::<Conformances>(conforming_entity) else {
        return;
    };

    for item in &conformances.0 {
        let ConformanceItem::Positive(ast_ty, _) = item else {
            continue;
        };
        let Some(protocol) = resolve_conformance(cx, ast_ty, conforming_entity) else {
            continue;
        };
        if cx.query.get::<NodeKind>(protocol) != Some(&NodeKind::Protocol) {
            continue;
        }

        check_protocol_requirements(cx, entity, protocol, conforming_entity, diags);
    }
}

/// Check that an extension satisfies the protocol requirements it declares.
fn check_extension_conformances(
    cx: &CompilationContext<'_>,
    extension: Entity,
    target: Entity,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let Some(conformances) = cx.query.get::<Conformances>(extension) else {
        return;
    };

    for item in &conformances.0 {
        let ConformanceItem::Positive(ast_ty, _) = item else {
            continue;
        };
        let Some(protocol) = resolve_conformance(cx, ast_ty, extension) else {
            continue;
        };
        if cx.query.get::<NodeKind>(protocol) != Some(&NodeKind::Protocol) {
            continue;
        }

        check_protocol_requirements(cx, target, protocol, extension, diags);
    }
}

/// Check that `type_entity` satisfies all requirements of `protocol`.
/// `decl_entity` is where the conformance was declared (for span reporting).
fn check_protocol_requirements(
    cx: &CompilationContext<'_>,
    type_entity: Entity,
    protocol: Entity,
    decl_entity: Entity,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let type_name = util::entity_name(cx.query, type_entity);
    let decl_span = util::entity_span(cx.query, decl_entity);

    // Collect all method/type names provided by the type and its extensions
    let provided = collect_provided_members(cx, type_entity);

    let members = cx.query.query(ProtocolMembers {
        protocol,
        root: cx.root,
    });

    // Check callable/property requirements, including inherited ones. Members
    // from protocol extensions are defaults and therefore already satisfy the
    // requirement without an implementation on the conforming type.
    let default_methods: Vec<Entity> = members
        .iter()
        .filter(|member| member.extension.is_some())
        .map(|member| member.entity)
        .collect();

    for member in &members {
        if member.extension.is_some() {
            continue;
        }
        let child = member.entity;
        let child_kind = cx.query.get::<NodeKind>(child);
        let Some(name) = member_lookup_name(cx, child) else {
            continue;
        };
        let proto_name = util::entity_name(cx.query, member.declaring_protocol);

        match child_kind {
            Some(NodeKind::Function | NodeKind::Subscript) => {
                // Required method — look for an overload on the impl side
                // whose signature shape (arity + labels) matches the
                // protocol requirement. A name match with the wrong shape
                // is treated as "not implemented" (lib1 parity: the impl
                // only satisfies the requirement if labels + arity line up).
                let proto_call = cx.query.get::<Callable>(child);
                let candidates = provided.methods.get(name.as_str());
                let sig_match = candidates.and_then(|cands| {
                    cands
                        .iter()
                        .copied()
                        .find(|&c| signatures_match(proto_call, cx.query.get::<Callable>(c)))
                });

                let mut matched_impl: Option<Entity> = None;
                if let Some(impl_method) = sig_match {
                    let impl_call = cx.query.get::<Callable>(impl_method);
                    if !receivers_match(proto_call, impl_call) {
                        let impl_span = util::entity_span(cx.query, impl_method);
                        let expected = match proto_call.and_then(|c| c.receiver.as_ref()) {
                            Some(_) => "instance",
                            None => "static",
                        };
                        diags.push(AnalyzeDiagnostic {
                            descriptor_id: DESCRIPTORS[5].id,
                            severity: DESCRIPTORS[5].default_severity,
                            message: format!(
                                "method '{}' has wrong receiver kind for protocol '{}'",
                                name, proto_name,
                            ),
                            labels: vec![DiagLabel {
                                span: impl_span,
                                message: format!(
                                    "expected {} method to match protocol receiver",
                                    expected,
                                ),
                                is_primary: true,
                            }],
                            notes: vec![],
                        });
                    } else {
                        matched_impl = Some(impl_method);
                    }
                } else if protocol_default_method_matches(cx, &default_methods, child) {
                    // Protocol extension defaults satisfy the requirement.
                    // This covers both `extend P { func req(...) }` and
                    // extension-added conformances like `extend P: Q { ... }`.
                } else {
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[0].id,
                        severity: DESCRIPTORS[0].default_severity,
                        message: format!(
                            "type '{}' does not implement method '{}' from protocol '{}'",
                            type_name, name, proto_name,
                        ),
                        labels: vec![DiagLabel {
                            span: decl_span.clone(),
                            message: format!("missing method '{}'", name),
                            is_primary: true,
                        }],
                        notes: vec![],
                    });
                }

                if let Some(impl_method) = matched_impl {
                    check_method_return_type(
                        cx,
                        child,
                        impl_method,
                        type_entity,
                        member.declaring_protocol,
                        &name,
                        &proto_name,
                        diags,
                    );
                }
            },
            Some(NodeKind::Field) => {
                // Required property — check if provided with matching type
                if let Some(&field_entity) = provided.fields.get(name.as_str()) {
                    // Setter requirement: if the protocol declares `{ get set }`
                    // the impl must also be settable (either a `var` stored
                    // property or a computed property with a `set` accessor).
                    let proto_needs_set = cx.query.get::<Settable>(child).is_some();
                    let impl_has_set = cx.query.get::<Settable>(field_entity).is_some();
                    if proto_needs_set && !impl_has_set {
                        diags.push(AnalyzeDiagnostic {
                            descriptor_id: DESCRIPTORS[6].id,
                            severity: DESCRIPTORS[6].default_severity,
                            message: format!(
                                "property '{}' requires a setter to satisfy protocol '{}'",
                                name, proto_name,
                            ),
                            labels: vec![DiagLabel {
                                span: decl_span.clone(),
                                message: format!("missing setter for '{}'", name),
                                is_primary: true,
                            }],
                            notes: vec![],
                        });
                    }
                    // Compare types by resolving TypeAnnotation on both
                    let proto_ty = cx.query.get::<TypeAnnotation>(child);
                    let impl_ty = cx.query.get::<TypeAnnotation>(field_entity);
                    if let (Some(proto_ann), Some(impl_ann)) = (proto_ty, impl_ty) {
                        // Resolve protocol side with Self → conforming type
                        let proto_resolved = resolve_type_entity_with_self(
                            cx,
                            &proto_ann.0,
                            protocol,
                            Some(type_entity),
                        );
                        let impl_resolved = resolve_type_entity(cx, &impl_ann.0, type_entity);
                        if proto_resolved != impl_resolved || proto_resolved.is_none() {
                            let field_span = util::entity_span(cx.query, field_entity);
                            diags.push(AnalyzeDiagnostic {
                                descriptor_id: DESCRIPTORS[2].id,
                                severity: DESCRIPTORS[2].default_severity,
                                message: format!(
                                    "property '{}' has wrong type for protocol '{}'",
                                    name, proto_name,
                                ),
                                labels: vec![DiagLabel {
                                    span: field_span,
                                    message: format!("type does not match protocol requirement",),
                                    is_primary: true,
                                }],
                                notes: vec![],
                            });
                        }
                    }
                }
            },
            _ => {},
        }
    }

    let associated_types = cx.query.query(ProtocolAssociatedTypes {
        protocol,
        root: cx.root,
    });

    // Check associated type requirements, including inherited ones. Extension
    // aliases are defaults, not obligations.
    for member in &associated_types {
        if member.extension.is_some() {
            continue;
        }
        let child = member.entity;
        let Some(name) = member_lookup_name(cx, child) else {
            continue;
        };
        let proto_name = util::entity_name(cx.query, member.declaring_protocol);

        let has_default = cx.query.get::<TypeAnnotation>(child).is_some()
            || find_protocol_extension_assoc_binding(
                cx,
                protocol,
                member.declaring_protocol,
                &name,
            )
            .is_some();
        // Skip if the type has a type alias with this name but no binding
        // (E442 already reports "requires a type definition")
        let has_incomplete_alias = has_type_alias_by_name(cx, type_entity, &name);
        if !has_default && !has_incomplete_alias && !provided.type_aliases.contains(name.as_str()) {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[1].id,
                severity: DESCRIPTORS[1].default_severity,
                message: format!(
                    "type '{}' does not provide associated type '{}' from protocol '{}'",
                    type_name, name, proto_name,
                ),
                labels: vec![DiagLabel {
                    span: decl_span.clone(),
                    message: format!("missing associated type '{}'", name),
                    is_primary: true,
                }],
                notes: vec![],
            });
        }
    }

    // Check where clause constraints on the protocol.
    // e.g., `protocol SortedIterator: Iterator where Iterator.Item: Comparable`
    // When BadIterator conforms to SortedIterator and binds Item = NotComparable,
    // verify NotComparable: Comparable.
    check_where_clause_constraints(cx, type_entity, protocol, decl_entity, diags);
}

fn check_associated_type_conflicts(
    cx: &CompilationContext<'_>,
    protocol: Entity,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let Some(conformances) = cx.query.get::<Conformances>(protocol) else {
        return;
    };

    let mut by_name: HashMap<String, Vec<(Entity, Entity)>> = HashMap::new();
    for item in &conformances.0 {
        let ConformanceItem::Positive(ast_ty, _) = item else {
            continue;
        };
        let Some(parent_protocol) = resolve_conformance(cx, ast_ty, protocol) else {
            continue;
        };
        if cx.query.get::<NodeKind>(parent_protocol) != Some(&NodeKind::Protocol) {
            continue;
        }

        for (assoc, declaring_protocol) in declared_associated_types(cx, parent_protocol) {
            let Some(name) = member_lookup_name(cx, assoc) else {
                continue;
            };
            let entries = by_name.entry(name).or_default();
            if !entries.iter().any(|(entity, _)| *entity == assoc) {
                entries.push((assoc, declaring_protocol));
            }
        }
    }

    let proto_name = util::entity_name(cx.query, protocol);
    let span = util::entity_span(cx.query, protocol);
    for (name, entries) in by_name {
        if entries.len() < 2 {
            continue;
        }

        let protocols: Vec<String> = entries
            .iter()
            .map(|(_, p)| util::entity_name(cx.query, *p))
            .collect();
        diags.push(AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[7].id,
            severity: DESCRIPTORS[7].default_severity,
            message: format!(
                "conflicting associated type '{}' inherited by protocol '{}'",
                name, proto_name,
            ),
            labels: vec![DiagLabel {
                span: span.clone(),
                message: format!("conflicting associated type '{}'", name),
                is_primary: true,
            }],
            notes: vec![format!(
                "associated type '{}' is declared by protocols {}",
                name,
                protocols.join(", ")
            )],
        });
    }
}

fn declared_associated_types(
    cx: &CompilationContext<'_>,
    protocol: Entity,
) -> Vec<(Entity, Entity)> {
    let mut out = Vec::new();
    let mut seen_protocols = HashSet::new();
    collect_declared_associated_types(cx, protocol, &mut seen_protocols, &mut out);
    out
}

fn collect_declared_associated_types(
    cx: &CompilationContext<'_>,
    protocol: Entity,
    seen_protocols: &mut HashSet<Entity>,
    out: &mut Vec<(Entity, Entity)>,
) {
    if !seen_protocols.insert(protocol) {
        return;
    }

    for &child in cx.query.children_of(protocol) {
        if cx.query.get::<NodeKind>(child) == Some(&NodeKind::TypeAlias)
            && cx.query.get::<QualifiedTarget>(child).is_none()
        {
            out.push((child, protocol));
        }
    }

    let Some(conformances) = cx.query.get::<Conformances>(protocol) else {
        return;
    };
    for item in &conformances.0 {
        let ConformanceItem::Positive(ast_ty, _) = item else {
            continue;
        };
        let Some(parent_protocol) = resolve_conformance(cx, ast_ty, protocol) else {
            continue;
        };
        if cx.query.get::<NodeKind>(parent_protocol) == Some(&NodeKind::Protocol) {
            collect_declared_associated_types(cx, parent_protocol, seen_protocols, out);
        }
    }
}

/// Check that the type's associated type bindings satisfy protocol where clause constraints.
fn check_where_clause_constraints(
    cx: &CompilationContext<'_>,
    type_entity: Entity,
    protocol: Entity,
    decl_entity: Entity,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let Some(wc) = cx.query.get::<WhereClause>(protocol) else {
        return;
    };

    let decl_span = util::entity_span(cx.query, decl_entity);

    for constraint in &wc.0 {
        let WhereConstraint::Bound {
            subject, protocols, ..
        } = constraint
        else {
            continue;
        };

        // We care about associated type bounds like `Iterator.Item: Comparable`.
        // The subject is a multi-segment path where the last segment is the assoc type name.
        let kestrel_ast::AstType::Named { segments, .. } = subject else {
            continue;
        };
        if segments.len() < 2 {
            continue;
        }
        let assoc_type_name = &segments.last().unwrap().name;

        // Find what the conforming type binds this associated type to
        let bound_entity = find_type_alias_binding(cx, type_entity, assoc_type_name);
        let Some(bound_entity) = bound_entity else {
            continue; // No binding found — E455 already handles missing assoc types
        };

        // Get the TypeAnnotation to find what concrete type is bound
        let Some(type_ann) = cx.query.get::<TypeAnnotation>(bound_entity) else {
            continue;
        };

        // Resolve the bound type to an entity
        let Some(bound_type_entity) = resolve_type_entity(cx, &type_ann.0, type_entity) else {
            continue;
        };

        // Check each required protocol constraint
        let bound_conforming = cx.query.query(ConformingProtocols {
            entity: bound_type_entity,
            root: cx.root,
        });

        for req_proto_ast in protocols {
            let Some(req_proto_entity) = resolve_type_entity(cx, req_proto_ast, protocol) else {
                continue;
            };
            if cx.query.get::<NodeKind>(req_proto_entity) != Some(&NodeKind::Protocol) {
                continue;
            }

            if !bound_conforming.contains(&req_proto_entity) {
                let bound_type_name = util::entity_name(cx.query, bound_type_entity);
                let req_proto_name = util::entity_name(cx.query, req_proto_entity);
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[3].id,
                    severity: DESCRIPTORS[3].default_severity,
                    message: format!(
                        "type '{}' does not satisfy bound '{}' on associated type '{}'",
                        bound_type_name, req_proto_name, assoc_type_name,
                    ),
                    labels: vec![DiagLabel {
                        span: decl_span.clone(),
                        message: format!(
                            "'{}' bound to '{}' which does not conform to '{}'",
                            assoc_type_name, bound_type_name, req_proto_name,
                        ),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
            }
        }
    }
}

/// Find the type alias entity that binds an associated type name for a type.
/// Searches the type's direct children and its extensions.
fn find_type_alias_binding(
    cx: &CompilationContext<'_>,
    type_entity: Entity,
    assoc_name: &str,
) -> Option<Entity> {
    // Search direct children
    for &child in cx.query.children_of(type_entity) {
        if cx.query.get::<NodeKind>(child) == Some(&NodeKind::TypeAlias)
            && cx
                .query
                .get::<Name>(child)
                .is_some_and(|n| n.0 == assoc_name)
            && cx.query.get::<TypeAnnotation>(child).is_some()
        {
            return Some(child);
        }
    }

    // Search extensions
    let extensions = cx.query.query(ExtensionsFor {
        target: type_entity,
        root: cx.root,
    });
    for ext in &extensions {
        for &child in cx.query.children_of(*ext) {
            if cx.query.get::<NodeKind>(child) == Some(&NodeKind::TypeAlias)
                && cx
                    .query
                    .get::<Name>(child)
                    .is_some_and(|n| n.0 == assoc_name)
                && cx.query.get::<TypeAnnotation>(child).is_some()
            {
                return Some(child);
            }
        }
    }
    None
}

fn protocol_default_method_matches(
    cx: &CompilationContext<'_>,
    default_methods: &[Entity],
    requirement: Entity,
) -> bool {
    let Some(req_name) = member_lookup_name(cx, requirement) else {
        return false;
    };
    let req_call = cx.query.get::<Callable>(requirement);
    default_methods.iter().any(|&default| {
        member_lookup_name(cx, default).is_some_and(|name| name == req_name)
            && signatures_match(req_call, cx.query.get::<Callable>(default))
            && receivers_match(req_call, cx.query.get::<Callable>(default))
    })
}

fn protocol_closure(cx: &CompilationContext<'_>, protocol: Entity) -> Vec<Entity> {
    let mut protocols = vec![protocol];
    protocols.extend(cx.query.query(ConformingProtocols {
        entity: protocol,
        root: cx.root,
    }));
    protocols
}

fn find_protocol_extension_assoc_binding(
    cx: &CompilationContext<'_>,
    protocol: Entity,
    assoc_protocol: Entity,
    assoc_name: &str,
) -> Option<AstType> {
    for target in protocol_closure(cx, protocol) {
        let extensions = cx.query.query(ExtensionsFor {
            target,
            root: cx.root,
        });
        for ext in extensions {
            for &child in cx.query.children_of(ext) {
                if cx.query.get::<NodeKind>(child) != Some(&NodeKind::TypeAlias) {
                    continue;
                }
                if !cx
                    .query
                    .get::<Name>(child)
                    .is_some_and(|name| name.0 == assoc_name)
                {
                    continue;
                }
                let Some(ann) = cx.query.get::<TypeAnnotation>(child) else {
                    continue;
                };

                let matches_protocol = match cx.query.get::<QualifiedTarget>(child) {
                    Some(_) => resolve_qualified_target(cx, child) == Some(assoc_protocol),
                    None => target == assoc_protocol,
                };
                if matches_protocol {
                    return Some(ann.0.clone());
                }
            }
        }
    }
    None
}

fn resolve_qualified_target(cx: &CompilationContext<'_>, alias: Entity) -> Option<Entity> {
    let target = cx.query.get::<QualifiedTarget>(alias)?;
    let AstType::Named { segments, .. } = &target.0 else {
        return None;
    };
    let path = segments.iter().map(|s| s.name.clone()).collect();
    let context = cx.query.parent_of(alias).unwrap_or(cx.root);
    match cx.query.query(ResolveTypePath {
        segments: path,
        context,
        root: cx.root,
    }) {
        TypeResolution::Found(entity) => Some(entity),
        _ => None,
    }
}

/// Check that an impl method's return type matches the protocol method's.
/// Return annotations are lowered to HIR and compared after substituting the
/// conforming type for `Self` and resolving associated-type bindings.
fn check_method_return_type(
    cx: &CompilationContext<'_>,
    proto_method: Entity,
    impl_method: Entity,
    type_entity: Entity,
    protocol: Entity,
    method_name: &str,
    proto_name: &str,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let expected = cx.query.query(LowerCallableReturnType {
        entity: proto_method,
        root: cx.root,
    });
    let actual = cx.query.query(LowerCallableReturnType {
        entity: impl_method,
        root: cx.root,
    });
    let mut env = type_compare_env_for_conformance(cx, type_entity, protocol);

    // Align method-level type parameters. `func make[U] -> U` on the protocol
    // and impl have distinct `U` entities, so the returns compare unequal
    // without this mapping. Matched positionally; if arities differ the
    // signature analyzer (E457) has already reported that.
    let proto_params: Vec<Entity> = cx
        .query
        .get::<kestrel_ast_builder::TypeParams>(proto_method)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();
    let impl_params: Vec<Entity> = cx
        .query
        .get::<kestrel_ast_builder::TypeParams>(impl_method)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();
    for (&proto_param, &impl_param) in proto_params.iter().zip(impl_params.iter()) {
        env.param_subs
            .push((proto_param, ResolvedTy::Param { entity: impl_param }));
    }

    if !compare_hir_types(cx.query, cx.root, &expected, &actual, &env).is_equal_or_unknown() {
        let impl_span = util::entity_span(cx.query, impl_method);
        diags.push(AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[4].id,
            severity: DESCRIPTORS[4].default_severity,
            message: format!(
                "method '{}' has wrong return type for protocol '{}'",
                method_name, proto_name,
            ),
            labels: vec![DiagLabel {
                span: impl_span,
                message: "wrong return type".into(),
                is_primary: true,
            }],
            notes: vec![],
        });
    }
}

fn type_compare_env_for_conformance(
    cx: &CompilationContext<'_>,
    type_entity: Entity,
    protocol: Entity,
) -> TypeCompareEnv {
    let assoc_bindings = cx
        .query
        .query(ProtocolAssociatedTypes {
            protocol,
            root: cx.root,
        })
        .into_iter()
        .filter_map(|member| {
            let name = member_lookup_name(cx, member.entity)?;
            let ty = if let Some(binding) = find_associated_type_binding_entity(
                cx,
                type_entity,
                &name,
                member.declaring_protocol,
            ) {
                cx.query.query(LowerTypeAnnotation {
                    entity: binding,
                    root: cx.root,
                })?
            } else {
                cx.query.query(LowerTypeAnnotation {
                    entity: member.entity,
                    root: cx.root,
                })?
            };
            Some(AssocBinding {
                assoc: member.entity,
                name,
                ty,
            })
        })
        .collect();

    TypeCompareEnv {
        self_ty: Some(self_type_for_compare(cx, type_entity)),
        assoc_bindings,
        param_subs: Vec::new(),
    }
}

fn self_type_for_compare(cx: &CompilationContext<'_>, type_entity: Entity) -> ResolvedTy {
    let args = cx
        .query
        .get::<kestrel_ast_builder::TypeParams>(type_entity)
        .map(|params| {
            params
                .0
                .iter()
                .map(|&entity| ResolvedTy::Param { entity })
                .collect()
        })
        .unwrap_or_default();
    ResolvedTy::Named {
        entity: type_entity,
        args,
    }
}

/// Find the impl's binding for `assoc_name` on `type_entity`, searching the
/// type itself, its extensions, and — when the associated type is inherited
/// from a parent protocol — any extensions on the parent protocols that
/// bind `<Proto>.<assoc_name> = …`. Qualified bindings must match
/// `protocol`; unqualified bindings are accepted.
///
/// Example: `Optional` conforms to `Equatable`, and
/// `extend Equatable: Equal[Self] { type Equal.Output = Bool }` supplies
/// the `Equal.Output = Bool` binding for every Equatable type.
fn find_associated_type_binding_entity(
    cx: &CompilationContext<'_>,
    type_entity: Entity,
    assoc_name: &str,
    protocol: Entity,
) -> Option<Entity> {
    let mut search = vec![type_entity];
    let extensions = cx.query.query(ExtensionsFor {
        target: type_entity,
        root: cx.root,
    });
    search.extend(extensions.iter());

    // Also walk the conforming type's protocol closure: a binding on an
    // extension of any parent protocol (e.g. Equatable) counts for impls
    // that reach the method through that parent.
    let conformed_protocols = cx.query.query(ConformingProtocols {
        entity: type_entity,
        root: cx.root,
    });
    for proto in &conformed_protocols {
        let proto_exts = cx.query.query(ExtensionsFor {
            target: *proto,
            root: cx.root,
        });
        search.extend(proto_exts.iter());
    }

    // First pass: prefer bindings qualified to this exact protocol —
    // `type Equal.Output = Bool` trumps a sibling `type Output = String`
    // that would otherwise apply to any conformed protocol.
    let mut fallback: Option<Entity> = None;
    for &entity in &search {
        for &child in cx.query.children_of(entity) {
            if cx.query.get::<NodeKind>(child) != Some(&NodeKind::TypeAlias) {
                continue;
            }
            let Some(name) = cx.query.get::<Name>(child) else {
                continue;
            };
            if name.0 != assoc_name {
                continue;
            }
            if cx.query.get::<TypeAnnotation>(child).is_none() {
                continue;
            }

            match cx.query.get::<QualifiedTarget>(child) {
                Some(target) => {
                    let AstType::Named { segments, .. } = &target.0 else {
                        continue;
                    };
                    let path: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
                    let context = cx.query.parent_of(child).unwrap_or(cx.root);
                    let matches = matches!(
                        cx.query.query(ResolveTypePath {
                            segments: path,
                            context,
                            root: cx.root,
                        }),
                        TypeResolution::Found(e) if e == protocol,
                    );
                    if matches {
                        return Some(child);
                    }
                },
                None => {
                    // Unqualified alias. The binding is scoped to whatever
                    // protocol the enclosing scope declares conformance to —
                    // `type Output = T` inside `enum Result: Tryable { … }`
                    // or `extend Optional[T]: Tryable { … }` is
                    // `Tryable.Output`, not `Equal.Output`. Save as a
                    // fallback; a later qualified match wins.
                    if fallback.is_none() && entity_conforms_to(cx, entity, protocol) {
                        fallback = Some(child);
                    }
                },
            }
        }
    }
    fallback
}

/// True if `entity` declares conformance to `protocol` (directly or
/// transitively via refinement). Used to decide whether an unqualified
/// `type Output = …` inside the entity's scope is the binding for
/// `protocol.Output`.
fn entity_conforms_to(cx: &CompilationContext<'_>, entity: Entity, protocol: Entity) -> bool {
    let Some(conformances) = cx.query.get::<Conformances>(entity) else {
        return false;
    };
    let context = cx.query.parent_of(entity).unwrap_or(cx.root);
    let mut seeds: Vec<Entity> = Vec::new();
    for item in &conformances.0 {
        let ConformanceItem::Positive(ast_ty, _) = item else {
            continue;
        };
        let AstType::Named { segments, .. } = ast_ty else {
            continue;
        };
        let path: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
        if let TypeResolution::Found(e) = cx.query.query(ResolveTypePath {
            segments: path,
            context,
            root: cx.root,
        }) {
            if e == protocol {
                return true;
            }
            seeds.push(e);
        }
    }
    let closure = kestrel_name_res::expand_protocol_closure(cx.query, cx.root, seeds);
    closure.contains(&protocol)
}

/// Resolve an AstType to an entity for type comparison.
/// When `self_type` is provided, `Self` resolves to that entity.
fn resolve_type_entity(
    cx: &CompilationContext<'_>,
    ast_ty: &kestrel_ast::AstType,
    context: Entity,
) -> Option<Entity> {
    resolve_type_entity_with_self(cx, ast_ty, context, None)
}

/// Resolve an AstType to an entity, substituting `Self` with `self_type` if provided.
fn resolve_type_entity_with_self(
    cx: &CompilationContext<'_>,
    ast_ty: &kestrel_ast::AstType,
    context: Entity,
    self_type: Option<Entity>,
) -> Option<Entity> {
    let segments: Vec<String> = match ast_ty {
        kestrel_ast::AstType::Named { segments, .. } => {
            segments.iter().map(|s| s.name.clone()).collect()
        },
        kestrel_ast::AstType::Array(_, _) => vec!["Array".into()],
        kestrel_ast::AstType::Dictionary(_, _, _) => vec!["Dictionary".into()],
        kestrel_ast::AstType::Optional(_, _) => vec!["Optional".into()],
        kestrel_ast::AstType::Result { .. } => vec!["Result".into()],
        _ => return None,
    };
    match cx.query.query(ResolveTypePath {
        segments,
        context,
        root: cx.root,
    }) {
        TypeResolution::Found(entity) => Some(entity),
        TypeResolution::SelfType => self_type,
        _ => None,
    }
}

/// Protocol and impl signatures agree on arity and parameter labels.
/// Types are checked separately by `check_method_return_type`; param types
/// aren't compared here because type-param / Self substitution isn't modeled
/// by entity equality.
fn signatures_match(proto: Option<&Callable>, imp: Option<&Callable>) -> bool {
    let (Some(proto), Some(imp)) = (proto, imp) else {
        // If either side lacks a Callable component we can't prove a mismatch —
        // keep parity with the previous behavior and treat as matching.
        return true;
    };
    if proto.params.len() != imp.params.len() {
        return false;
    }
    proto
        .params
        .iter()
        .zip(imp.params.iter())
        .all(|(a, b)| a.label == b.label)
}

/// Both sides are instance methods, or both are static.
fn receivers_match(proto: Option<&Callable>, imp: Option<&Callable>) -> bool {
    match (proto, imp) {
        (Some(p), Some(i)) => p.receiver.is_some() == i.receiver.is_some(),
        _ => true,
    }
}

fn member_lookup_name(cx: &CompilationContext<'_>, entity: Entity) -> Option<String> {
    if let Some(name) = cx.query.get::<Name>(entity) {
        return Some(name.0.clone());
    }
    match cx.query.get::<NodeKind>(entity) {
        Some(NodeKind::Initializer) => Some("init".into()),
        Some(NodeKind::Subscript) => Some("subscript".into()),
        _ => None,
    }
}

/// Check if an entity has a TypeAlias child with the given name (regardless of binding).
fn has_type_alias_by_name(cx: &CompilationContext<'_>, entity: Entity, name: &str) -> bool {
    cx.query.children_of(entity).iter().any(|&child| {
        cx.query.get::<NodeKind>(child) == Some(&NodeKind::TypeAlias)
            && cx.query.get::<Name>(child).is_some_and(|n| n.0 == name)
    })
}

struct ProvidedMembers {
    /// method name → candidate impl entities. A name can map to multiple
    /// overloads that differ by labels/arity; conformance checks must match
    /// each requirement to the overload with the right signature.
    methods: HashMap<String, Vec<Entity>>,
    type_aliases: HashSet<String>,
    /// field name → field entity (for type comparison)
    fields: HashMap<String, Entity>,
}

/// Collect all method names and type alias names provided by a type and its extensions.
fn collect_provided_members(cx: &CompilationContext<'_>, type_entity: Entity) -> ProvidedMembers {
    let mut methods: HashMap<String, Vec<Entity>> = HashMap::new();
    let mut type_aliases = HashSet::new();
    let mut fields = HashMap::new();

    // Direct members
    collect_from_entity(
        cx,
        type_entity,
        &mut methods,
        &mut type_aliases,
        &mut fields,
    );

    // Extension members
    let extensions = cx.query.query(ExtensionsFor {
        target: type_entity,
        root: cx.root,
    });
    for ext in &extensions {
        collect_from_entity(cx, *ext, &mut methods, &mut type_aliases, &mut fields);
    }

    ProvidedMembers {
        methods,
        type_aliases,
        fields,
    }
}

fn collect_from_entity(
    cx: &CompilationContext<'_>,
    entity: Entity,
    methods: &mut HashMap<String, Vec<Entity>>,
    type_aliases: &mut HashSet<String>,
    fields: &mut HashMap<String, Entity>,
) {
    for &child in cx.query.children_of(entity) {
        let name = member_lookup_name(cx, child);
        match cx.query.get::<NodeKind>(child) {
            Some(NodeKind::Function | NodeKind::Subscript | NodeKind::Initializer) => {
                if let Some(name) = name {
                    methods.entry(name).or_default().push(child);
                }
            },
            Some(NodeKind::TypeAlias) => {
                // Only count type aliases with a binding (TypeAnnotation = concrete type)
                if let Some(name) = name {
                    if cx.query.get::<TypeAnnotation>(child).is_some() {
                        type_aliases.insert(name);
                    }
                }
            },
            Some(NodeKind::Field) => {
                if let Some(name) = name {
                    fields.insert(name, child);
                }
            },
            _ => {},
        }
    }
}

/// Resolve a conformance AstType to a protocol entity.
fn resolve_conformance(
    cx: &CompilationContext<'_>,
    ast_ty: &kestrel_ast::AstType,
    context_entity: Entity,
) -> Option<Entity> {
    let kestrel_ast::AstType::Named { segments, .. } = ast_ty else {
        return None;
    };
    let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    let context = cx.query.parent_of(context_entity).unwrap_or(cx.root);
    match cx.query.query(ResolveTypePath {
        segments: seg_names,
        context,
        root: cx.root,
    }) {
        TypeResolution::Found(entity) => Some(entity),
        _ => None,
    }
}

/// Compare each param type of proto_method and impl_method under a conformance
/// env that maps `Self` to the concrete type. Returns true iff all params
/// match (or at least don't disagree). Used to distinguish sibling protocol
/// methods that share a name + labels but differ in parameter types.
fn param_types_match(
    cx: &CompilationContext<'_>,
    type_entity: Entity,
    protocol: Entity,
    proto_method: Entity,
    impl_method: Entity,
) -> bool {
    let proto_params = cx.query.query(LowerCallableTypes {
        entity: proto_method,
        root: cx.root,
    });
    let impl_params = cx.query.query(LowerCallableTypes {
        entity: impl_method,
        root: cx.root,
    });
    let (Some(proto_params), Some(impl_params)) = (proto_params, impl_params) else {
        return true;
    };
    if proto_params.len() != impl_params.len() {
        return false;
    }
    let mut env = type_compare_env_for_conformance(cx, type_entity, protocol);
    let proto_ty_params: Vec<Entity> = cx
        .query
        .get::<kestrel_ast_builder::TypeParams>(proto_method)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();
    let impl_ty_params: Vec<Entity> = cx
        .query
        .get::<kestrel_ast_builder::TypeParams>(impl_method)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();
    for (&p, &i) in proto_ty_params.iter().zip(impl_ty_params.iter()) {
        env.param_subs.push((p, ResolvedTy::Param { entity: i }));
    }
    for (proto_ty, impl_ty) in proto_params.iter().zip(impl_params.iter()) {
        let (Some(proto_ty), Some(impl_ty)) = (proto_ty, impl_ty) else {
            continue;
        };
        if !compare_hir_types(cx.query, cx.root, proto_ty, impl_ty, &env).is_equal_or_unknown() {
            return false;
        }
    }
    true
}

/// True iff there exists a pair (a, b) in `protocols` where neither `a`
/// transitively conforms to `b` nor vice versa. If all pairs are related via
/// refinement/extension, a single impl method covers them all — no ambiguity.
fn has_unrelated_pair(cx: &CompilationContext<'_>, protocols: &[Entity]) -> bool {
    if protocols.len() < 2 {
        return false;
    }
    let closures: Vec<Vec<Entity>> = protocols
        .iter()
        .map(|&p| {
            cx.query.query(ConformingProtocols {
                entity: p,
                root: cx.root,
            })
        })
        .collect();
    for i in 0..protocols.len() {
        for j in (i + 1)..protocols.len() {
            let a = protocols[i];
            let b = protocols[j];
            let a_refines_b = closures[i].contains(&b);
            let b_refines_a = closures[j].contains(&a);
            if !a_refines_b && !b_refines_a {
                return true;
            }
        }
    }
    false
}

/// Check 9 (E463): An impl method that exactly satisfies the signature of
/// method requirements from two or more DIFFERENT conformed protocols is
/// ambiguous — the user must disambiguate (typically by extending each
/// protocol with a qualified impl). Inherited declarations don't count as a
/// second source, so we only walk each protocol's *direct* children.
fn check_ambiguous_method_satisfaction(
    cx: &CompilationContext<'_>,
    type_entity: Entity,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let provided = collect_provided_members(cx, type_entity);
    let conforming = cx.query.query(ConformingProtocols {
        entity: type_entity,
        root: cx.root,
    });
    if conforming.len() < 2 {
        return;
    }

    for (method_name, impl_candidates) in &provided.methods {
        for &impl_method in impl_candidates {
            if cx.query.get::<NodeKind>(impl_method) != Some(&NodeKind::Function) {
                continue;
            }
            let impl_call = cx.query.get::<Callable>(impl_method);

            // Walk direct children of each conformed protocol — a method
            // inherited from a parent protocol won't appear as a child of
            // the refining protocol, so this naturally counts each distinct
            // declaration exactly once.
            let mut matching_protocols: Vec<Entity> = Vec::new();
            for &proto in &conforming {
                for &child in cx.query.children_of(proto) {
                    if cx.query.get::<NodeKind>(child) != Some(&NodeKind::Function) {
                        continue;
                    }
                    let Some(child_name) = member_lookup_name(cx, child) else {
                        continue;
                    };
                    if &child_name != method_name {
                        continue;
                    }
                    let proto_call = cx.query.get::<Callable>(child);
                    if signatures_match(proto_call, impl_call)
                        && receivers_match(proto_call, impl_call)
                        && param_types_match(cx, type_entity, proto, child, impl_method)
                    {
                        matching_protocols.push(proto);
                    }
                }
            }

            matching_protocols.sort_unstable_by_key(|e| e.index());
            matching_protocols.dedup();

            // If every pair of matching protocols is related via refinement
            // or extension (i.e. one conforms to the other), then the impl
            // method satisfies them through a single chain — not ambiguous.
            // Example: `extend Equatable: Equal[Self]` makes Equatable types
            // auto-conform to Equal, so a struct's `equals` serves both.
            if matching_protocols.len() >= 2 && !has_unrelated_pair(cx, &matching_protocols) {
                continue;
            }

            if matching_protocols.len() >= 2 {
                let names: Vec<String> = matching_protocols
                    .iter()
                    .map(|&p| util::entity_name(cx.query, p))
                    .collect();
                let span = util::entity_span(cx.query, impl_method);
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: "E463",
                    severity: Severity::Error,
                    message: format!(
                        "method '{}' is ambiguous: satisfies requirements of multiple protocols ({})",
                        method_name,
                        names.join(", ")
                    ),
                    labels: vec![DiagLabel {
                        span,
                        message: "ambiguous protocol method".into(),
                        is_primary: true,
                    }],
                    notes: vec![
                        "provide distinct implementations via `extend Type: Protocol { ... }` for each protocol".into(),
                    ],
                });
            }
        }
    }
}
