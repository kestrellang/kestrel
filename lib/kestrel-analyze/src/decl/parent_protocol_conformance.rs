//! # Parent Protocol Conformance Analyzer
//!
//! Validates that when a struct/enum conforms to a protocol that inherits from
//! another protocol, the type also explicitly conforms to the parent protocol.
//!
//! For example, if protocol B inherits from A, and struct S conforms to B,
//! then S must also declare conformance to A (so that A's methods are provided).
//!
//! Uses `ResolveTypePath` to resolve conformance AstTypes to protocol entities,
//! then checks each protocol's own Conformances (protocol inheritance) to find
//! parent protocols that the struct/enum must also conform to.
//!
//! ## Diagnostics
//!
//! ### E421 -- `missing_parent_protocol_conformance` (Error, Correctness)
//!
//! **Message:** "'{type_name}' conforms to '{child_protocol}' but not its parent '{parent_protocol}'"
//!
//! **Labels:**
//! - Primary: the struct/enum declaration
//!   - Span source: `util::entity_span` on the struct/enum entity
//!   - Message: "missing conformance to '{parent_protocol}'"
//!
//! **Notes:**
//! - "'{child_protocol}' inherits from '{parent_protocol}', so conforming types must also conform to it"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast::ast_type::AstType;
use kestrel_ast_builder::{
    Callable, ConformanceItem, Conformances, Name, NodeKind, TypeAnnotation,
};
use kestrel_hecs::Entity;
use kestrel_name_res::{ExtensionsFor, ProtocolMembers, ResolveTypePath, TypeResolution};
use std::collections::{HashMap, HashSet};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E421",
    name: "missing_parent_protocol_conformance",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ParentProtocolConformanceAnalyzer;

impl Describe for ParentProtocolConformanceAnalyzer {
    fn id(&self) -> &'static str {
        "parent_protocol_conformance"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

/// Resolve an AstType::Named to an entity via ResolveTypePath.
/// Returns None for non-Named types or resolution failures.
fn resolve_conformance_type(
    cx: &DeclContext<'_>,
    context: Entity,
    ast_type: &AstType,
) -> Option<Entity> {
    let AstType::Named { segments, .. } = ast_type else {
        return None;
    };
    let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    let result = cx.query.query(ResolveTypePath {
        segments: seg_names,
        context,
        root: cx.root,
    });
    match result {
        TypeResolution::Found(entity) => Some(entity),
        _ => None,
    }
}

/// Collect the positively conformed protocol entities declared *directly* on
/// `entity` (reads the `Conformances` component only). Does not walk
/// inheritance.
fn collect_positive_conformances(cx: &DeclContext<'_>, entity: Entity) -> Vec<Entity> {
    let Some(conformances) = cx.query.get::<Conformances>(entity) else {
        return vec![];
    };
    conformances
        .0
        .iter()
        .filter_map(|item| {
            let ConformanceItem::Positive(ast_type, _) = item else {
                return None;
            };
            resolve_conformance_type(cx, entity, ast_type)
        })
        .collect()
}

fn collect_explicit_type_conformances(cx: &DeclContext<'_>, entity: Entity) -> HashSet<Entity> {
    let mut out: HashSet<Entity> = collect_positive_conformances(cx, entity)
        .into_iter()
        .collect();
    let extensions = cx.query.query(ExtensionsFor {
        target: entity,
        root: cx.root,
    });
    for ext in extensions {
        out.extend(collect_positive_conformances(cx, ext));
    }
    out
}

struct ProvidedMembers {
    methods: HashMap<String, Vec<Entity>>,
    fields: HashSet<String>,
    type_aliases: HashSet<String>,
}

fn parent_requirements_satisfied(
    cx: &DeclContext<'_>,
    type_entity: Entity,
    parent_protocol: Entity,
) -> bool {
    // Parity with lib1's `ProtocolRequiredMethods`: E421's purpose is to force
    // the user to declare the parent protocol when the parent contributes a
    // methods surface the refining protocol wouldn't otherwise expose. Parents
    // that contribute only associated types (e.g. `protocol Iterator { type
    // Item }`) flow transitively through refinement — no explicit listing
    // needed. Associated-type / field-only parents are checked elsewhere
    // (E457 for where-clause bounds on associated types).
    let provided = collect_provided_members(cx, type_entity);
    let mut saw_method_requirement = false;

    for member in cx.query.query(ProtocolMembers {
        protocol: parent_protocol,
        root: cx.root,
    }) {
        if member.extension.is_some() {
            continue;
        }
        let Some(name) = member_lookup_name(cx, member.entity) else {
            continue;
        };
        if !matches!(
            cx.query.get::<NodeKind>(member.entity),
            Some(NodeKind::Function | NodeKind::Subscript)
        ) {
            continue;
        }
        saw_method_requirement = true;
        let proto_call = cx.query.get::<Callable>(member.entity);
        let Some(candidates) = provided.methods.get(name.as_str()) else {
            return false;
        };
        if !candidates
            .iter()
            .any(|&candidate| signatures_match(proto_call, cx.query.get::<Callable>(candidate)))
        {
            return false;
        }
    }

    saw_method_requirement
}

fn collect_provided_members(cx: &DeclContext<'_>, type_entity: Entity) -> ProvidedMembers {
    let mut provided = ProvidedMembers {
        methods: HashMap::new(),
        fields: HashSet::new(),
        type_aliases: HashSet::new(),
    };
    collect_from_entity(cx, type_entity, &mut provided);

    let extensions = cx.query.query(ExtensionsFor {
        target: type_entity,
        root: cx.root,
    });
    for ext in extensions {
        collect_from_entity(cx, ext, &mut provided);
    }

    provided
}

fn collect_from_entity(cx: &DeclContext<'_>, entity: Entity, provided: &mut ProvidedMembers) {
    for &child in cx.query.children_of(entity) {
        let Some(name) = member_lookup_name(cx, child) else {
            continue;
        };
        match cx.query.get::<NodeKind>(child) {
            Some(NodeKind::Function | NodeKind::Subscript | NodeKind::Initializer) => {
                provided.methods.entry(name).or_default().push(child);
            },
            Some(NodeKind::Field) => {
                provided.fields.insert(name);
            },
            Some(NodeKind::TypeAlias) => {
                if cx.query.get::<TypeAnnotation>(child).is_some() {
                    provided.type_aliases.insert(name);
                }
            },
            _ => {},
        }
    }
}

fn member_lookup_name(cx: &DeclContext<'_>, entity: Entity) -> Option<String> {
    if let Some(name) = cx.query.get::<Name>(entity) {
        return Some(name.0.clone());
    }
    match cx.query.get::<NodeKind>(entity) {
        Some(NodeKind::Initializer) => Some("init".into()),
        Some(NodeKind::Subscript) => Some("subscript".into()),
        _ => None,
    }
}

fn signatures_match(proto: Option<&Callable>, imp: Option<&Callable>) -> bool {
    let (Some(proto), Some(imp)) = (proto, imp) else {
        return true;
    };
    proto.params.len() == imp.params.len()
        && proto
            .params
            .iter()
            .zip(imp.params.iter())
            .all(|(a, b)| a.label == b.label)
}

impl DeclCheck for ParentProtocolConformanceAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Struct, NodeKind::Enum]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let direct_conformances = collect_positive_conformances(cx, cx.entity);
        let explicit_conformances = collect_explicit_type_conformances(cx, cx.entity);
        if explicit_conformances.is_empty() {
            return vec![];
        }

        let type_name = util::entity_name(cx.query, cx.entity);
        let mut diags = Vec::new();

        // For each protocol named on the type declaration, check its immediate
        // parents. Extension-declared conformances participate as explicit
        // parent evidence, but do not themselves trigger E421; extension
        // conformance blocks commonly satisfy inherited requirements locally.
        // Do not use `ConformingProtocols` for the type here: that query is
        // intentionally transitive, so it would treat inherited parents as
        // already explicit and make this analyzer a no-op.
        for &proto_entity in &direct_conformances {
            let parent_protocols = collect_positive_conformances(cx, proto_entity);

            for parent_entity in parent_protocols {
                // Check if the struct/enum also explicitly conforms to this parent protocol.
                if explicit_conformances.contains(&parent_entity) {
                    continue;
                }

                // If the parent requirements themselves are still missing,
                // conformance-completeness will emit the more specific
                // "does not implement method/type" diagnostic. E421 is for the
                // case where the shape is otherwise present but the parent
                // protocol was not named explicitly.
                if !parent_requirements_satisfied(cx, cx.entity, parent_entity) {
                    continue;
                }

                let child_name = util::entity_name(cx.query, proto_entity);
                let parent_name = util::entity_name(cx.query, parent_entity);

                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message: format!(
                        "'{}' conforms to '{}' but not its parent protocol '{}'",
                        type_name, child_name, parent_name
                    ),
                    labels: vec![DiagLabel {
                        span: util::entity_span(cx.query, cx.entity),
                        message: format!("missing conformance to '{}'", parent_name),
                        is_primary: true,
                    }],
                    notes: vec![format!(
                        "'{}' inherits from '{}', so conforming types must also conform to it",
                        child_name, parent_name
                    )],
                });
            }
        }

        diags
    }
}
