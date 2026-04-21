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

use std::collections::{HashMap, HashSet};

use crate::context::CompilationContext;
use crate::diagnostic::*;
use crate::traits::{CompilationCheck, Describe};
use crate::util;
use kestrel_ast::AstType;
use kestrel_ast_builder::{
    ConformanceItem, Conformances, Name, NodeKind, QualifiedTarget, TypeAnnotation, WhereClause,
    WhereConstraint,
};
use kestrel_hecs::Entity;
use kestrel_name_res::{
    ConformingProtocols, ExtensionTargetEntity, ExtensionsFor, ResolveTypePath, TypeResolution,
};

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

    // Check struct/enum declarations with direct conformances
    if matches!(kind, Some(NodeKind::Struct | NodeKind::Enum)) {
        check_type_conformances(cx, entity, entity, diags);
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
    let proto_name = util::entity_name(cx.query, protocol);
    let decl_span = util::entity_span(cx.query, decl_entity);

    // Collect all method/type names provided by the type and its extensions
    let provided = collect_provided_members(cx, type_entity);

    // Check each protocol requirement
    for &child in cx.query.children_of(protocol) {
        let child_kind = cx.query.get::<NodeKind>(child);
        let Some(name) = cx.query.get::<Name>(child) else {
            continue;
        };
        let name = &name.0;

        match child_kind {
            Some(NodeKind::Function | NodeKind::Subscript) => {
                // Required method — check if provided
                if let Some(&impl_method) = provided.methods.get(name.as_str()) {
                    // Method exists — check return type matches after associated type substitution
                    check_method_return_type(
                        cx,
                        child,
                        impl_method,
                        type_entity,
                        protocol,
                        name,
                        &proto_name,
                        diags,
                    );
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
            },
            Some(NodeKind::TypeAlias) => {
                // Required associated type — only if no default (no TypeAnnotation)
                let has_default = cx.query.get::<TypeAnnotation>(child).is_some();
                // Skip if the type has a type alias with this name but no binding
                // (E442 already reports "requires a type definition")
                let has_incomplete_alias = has_type_alias_by_name(cx, type_entity, name);
                if !has_default
                    && !has_incomplete_alias
                    && !provided.type_aliases.contains(name.as_str())
                {
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
            },
            Some(NodeKind::Field) => {
                // Required property — check if provided with matching type
                if let Some(&field_entity) = provided.fields.get(name.as_str()) {
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

    // Check where clause constraints on the protocol.
    // e.g., `protocol SortedIterator: Iterator where Iterator.Item: Comparable`
    // When BadIterator conforms to SortedIterator and binds Item = NotComparable,
    // verify NotComparable: Comparable.
    check_where_clause_constraints(cx, type_entity, protocol, decl_entity, &provided, diags);
}

/// Check that the type's associated type bindings satisfy protocol where clause constraints.
fn check_where_clause_constraints(
    cx: &CompilationContext<'_>,
    type_entity: Entity,
    protocol: Entity,
    decl_entity: Entity,
    provided: &ProvidedMembers,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let Some(wc) = cx.query.get::<WhereClause>(protocol) else {
        return;
    };

    let type_name = util::entity_name(cx.query, type_entity);
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

/// Check that an impl method's return type matches the protocol method's,
/// comparing by resolved entity so `Self`, associated-type projections, and
/// fully-qualified paths (`std.num.Int64` vs `Int64`) compare equal when they
/// denote the same type.
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
    let Some(proto_ann) = cx.query.get::<TypeAnnotation>(proto_method) else {
        return;
    };
    let Some(impl_ann) = cx.query.get::<TypeAnnotation>(impl_method) else {
        return;
    };

    // Only check single-segment Named return types on the protocol side.
    // Complex return types (generics, tuples, fn types, etc.) would need full
    // type-param substitution which the entity-resolution path doesn't do yet.
    let AstType::Named { segments, .. } = &proto_ann.0 else {
        return;
    };
    if segments.len() != 1 || !segments[0].type_args.is_empty() {
        return;
    }
    let proto_return_name = &segments[0].name;

    let Some(expected) = resolve_expected_return(cx, proto_return_name, type_entity, protocol)
    else {
        return;
    };
    let Some(actual) =
        resolve_type_entity_with_self(cx, &impl_ann.0, type_entity, Some(type_entity))
    else {
        return;
    };

    if expected != actual {
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

/// Resolve a protocol method's single-segment return-type name to the entity
/// the impl is expected to return. If the name is one of the protocol's
/// associated types, project through the type's binding (falling back to a
/// protocol-provided default). Otherwise resolve as a regular type name
/// (covers `Self`, concrete types, etc.) with `Self → type_entity`.
fn resolve_expected_return(
    cx: &CompilationContext<'_>,
    name: &str,
    type_entity: Entity,
    protocol: Entity,
) -> Option<Entity> {
    // Find the protocol's associated type with this name, if any.
    let assoc_entity = cx.query.children_of(protocol).iter().copied().find(|&c| {
        cx.query.get::<NodeKind>(c) == Some(&NodeKind::TypeAlias)
            && cx.query.get::<Name>(c).is_some_and(|n| n.0 == name)
    });

    if let Some(assoc) = assoc_entity {
        // Associated type: find the impl's binding (direct or via extension).
        if let Some(binding) = find_associated_type_binding(cx, type_entity, name, protocol) {
            return resolve_type_entity_with_self(cx, &binding, type_entity, Some(type_entity));
        }
        // Fall back to the protocol's default binding on the associated type itself.
        if let Some(default) = cx.query.get::<TypeAnnotation>(assoc) {
            return resolve_type_entity_with_self(cx, &default.0, protocol, Some(type_entity));
        }
        // No binding and no default — E455 handles "missing associated type".
        return None;
    }

    // Not an associated type — resolve the name as a regular path in the
    // protocol's context, mapping `Self → type_entity`.
    let synthetic = AstType::Named {
        segments: vec![kestrel_ast::PathSegment {
            name: name.to_string(),
            type_args: vec![],
            span: kestrel_span2::Span::synthetic(0),
        }],
        span: kestrel_span2::Span::synthetic(0),
    };
    resolve_type_entity_with_self(cx, &synthetic, protocol, Some(type_entity))
}

/// Find the impl's binding for `assoc_name` on `type_entity`, searching the
/// type itself and its extensions. Qualified bindings (`type P.Output = …`)
/// must match `protocol`; unqualified bindings are accepted.
fn find_associated_type_binding(
    cx: &CompilationContext<'_>,
    type_entity: Entity,
    assoc_name: &str,
    protocol: Entity,
) -> Option<AstType> {
    let mut search = vec![type_entity];
    let extensions = cx.query.query(ExtensionsFor {
        target: type_entity,
        root: cx.root,
    });
    search.extend(extensions.iter());

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
            let Some(ann) = cx.query.get::<TypeAnnotation>(child) else {
                continue;
            };

            let is_for_this_protocol = match cx.query.get::<QualifiedTarget>(child) {
                Some(target) => match &target.0 {
                    AstType::Named { segments, .. } => {
                        let path = segments.iter().map(|s| s.name.clone()).collect();
                        let context = cx.query.parent_of(child).unwrap_or(cx.root);
                        matches!(
                            cx.query.query(ResolveTypePath {
                                segments: path,
                                context,
                                root: cx.root,
                            }),
                            TypeResolution::Found(e) if e == protocol,
                        )
                    },
                    _ => false,
                },
                None => true,
            };

            if is_for_this_protocol {
                return Some(ann.0.clone());
            }
        }
    }
    None
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
    let kestrel_ast::AstType::Named { segments, .. } = ast_ty else {
        return None;
    };
    let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    match cx.query.query(ResolveTypePath {
        segments: seg_names,
        context,
        root: cx.root,
    }) {
        TypeResolution::Found(entity) => Some(entity),
        TypeResolution::SelfType => self_type,
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
    /// method name → impl entity (for signature comparison)
    methods: HashMap<String, Entity>,
    type_aliases: HashSet<String>,
    /// field name → field entity (for type comparison)
    fields: HashMap<String, Entity>,
}

/// Collect all method names and type alias names provided by a type and its extensions.
fn collect_provided_members(cx: &CompilationContext<'_>, type_entity: Entity) -> ProvidedMembers {
    let mut methods = HashMap::new();
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
    methods: &mut HashMap<String, Entity>,
    type_aliases: &mut HashSet<String>,
    fields: &mut HashMap<String, Entity>,
) {
    for &child in cx.query.children_of(entity) {
        let Some(name) = cx.query.get::<Name>(child) else {
            continue;
        };
        match cx.query.get::<NodeKind>(child) {
            Some(NodeKind::Function | NodeKind::Subscript | NodeKind::Initializer) => {
                methods.insert(name.0.clone(), child);
            },
            Some(NodeKind::TypeAlias) => {
                // Only count type aliases with a binding (TypeAnnotation = concrete type)
                if cx.query.get::<TypeAnnotation>(child).is_some() {
                    type_aliases.insert(name.0.clone());
                }
            },
            Some(NodeKind::Field) => {
                fields.insert(name.0.clone(), child);
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
