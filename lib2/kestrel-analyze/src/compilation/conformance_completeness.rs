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

use std::collections::{HashMap, HashSet};

use crate::context::CompilationContext;
use crate::diagnostic::*;
use crate::traits::{CompilationCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Conformances, ConformanceItem, CstNode, Name, NodeKind, TypeAnnotation};
use kestrel_hecs::Entity;
use kestrel_name_res::{ExtensionTargetEntity, ExtensionsFor, ResolveTypePath, TypeResolution};

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
        let ConformanceItem::Positive(ast_ty, _) = item else { continue };
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
        let ConformanceItem::Positive(ast_ty, _) = item else { continue };
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
        let Some(name) = cx.query.get::<Name>(child) else { continue };
        let name = &name.0;

        match child_kind {
            Some(NodeKind::Function | NodeKind::Subscript) => {
                // Required method — check if provided
                if !provided.methods.contains(name.as_str()) {
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
            }
            Some(NodeKind::TypeAlias) => {
                // Required associated type — only if no default (no TypeAnnotation)
                let has_default = cx.query.get::<TypeAnnotation>(child).is_some();
                if !has_default && !provided.type_aliases.contains(name.as_str()) {
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
            Some(NodeKind::Field) => {
                // Required property — check if provided with matching type
                if let Some(&field_entity) = provided.fields.get(name.as_str()) {
                    // Compare types by resolving TypeAnnotation on both
                    let proto_ty = cx.query.get::<TypeAnnotation>(child);
                    let impl_ty = cx.query.get::<TypeAnnotation>(field_entity);
                    if let (Some(proto_ann), Some(impl_ann)) = (proto_ty, impl_ty) {
                        let proto_resolved = resolve_type_entity(cx, &proto_ann.0, protocol);
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
                                    message: format!(
                                        "type does not match protocol requirement",
                                    ),
                                    is_primary: true,
                                }],
                                notes: vec![],
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Resolve an AstType to an entity for type comparison.
fn resolve_type_entity(
    cx: &CompilationContext<'_>,
    ast_ty: &kestrel_ast::AstType,
    context: Entity,
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
        _ => None,
    }
}

struct ProvidedMembers {
    methods: HashSet<String>,
    type_aliases: HashSet<String>,
    /// field name → field entity (for type comparison)
    fields: HashMap<String, Entity>,
}

/// Collect all method names and type alias names provided by a type and its extensions.
fn collect_provided_members(cx: &CompilationContext<'_>, type_entity: Entity) -> ProvidedMembers {
    let mut methods = HashSet::new();
    let mut type_aliases = HashSet::new();
    let mut fields = HashMap::new();

    // Direct members
    collect_from_entity(cx, type_entity, &mut methods, &mut type_aliases, &mut fields);

    // Extension members
    let extensions = cx.query.query(ExtensionsFor {
        target: type_entity,
        root: cx.root,
    });
    for ext in &extensions {
        collect_from_entity(cx, *ext, &mut methods, &mut type_aliases, &mut fields);
    }

    ProvidedMembers { methods, type_aliases, fields }
}

fn collect_from_entity(
    cx: &CompilationContext<'_>,
    entity: Entity,
    methods: &mut HashSet<String>,
    type_aliases: &mut HashSet<String>,
    fields: &mut HashMap<String, Entity>,
) {
    for &child in cx.query.children_of(entity) {
        let Some(name) = cx.query.get::<Name>(child) else { continue };
        match cx.query.get::<NodeKind>(child) {
            Some(NodeKind::Function | NodeKind::Subscript | NodeKind::Initializer) => {
                methods.insert(name.0.clone());
            }
            Some(NodeKind::TypeAlias) => {
                // Only count type aliases with a binding (TypeAnnotation = concrete type)
                if cx.query.get::<TypeAnnotation>(child).is_some() {
                    type_aliases.insert(name.0.clone());
                }
            }
            Some(NodeKind::Field) => {
                fields.insert(name.0.clone(), child);
            }
            _ => {}
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
