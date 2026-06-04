//! # Type Alias Validation Analyzer
//!
//! Validates type alias declarations:
//!
//! 1. **Bounds only in protocols** — `type Item: Protocol` bounds are only
//!    valid inside protocol declarations. Struct/module-level type aliases
//!    with bounds are errors.
//! 2. **Requires `= Type`** — Non-protocol type aliases must have a definition
//!    (`type Foo = Int`). Abstract associated types without a definition are only
//!    allowed inside protocols.
//! 3. **Qualified binding validation** — `type Protocol.Item = Concrete` must
//!    reference a protocol the parent type conforms to, and that protocol must
//!    declare the associated type.
//! 4. **Unqualified binding ambiguity** — If multiple conformed protocols declare
//!    the same associated type name, the binding is ambiguous.
//! 5. **Constraint satisfaction** — The bound type must satisfy any protocol
//!    constraints on the associated type.
//!
//! Checks 1-5 are fully implemented. Check 6 (constraint satisfaction on
//! associated type bounds) is not yet implemented.
//!
//! ## Diagnostics
//!
//! ### E441 -- `associated_type_bounds_in_wrong_context` (Error, Correctness)
//!
//! **Message:** "type alias '{name}' cannot have bounds outside a protocol"
//!
//! **Labels:**
//! - Primary: the type alias declaration
//!   - Span source: `util::entity_span` on the type alias entity
//!   - Message: "bounds not allowed here"
//!
//! **Notes:**
//! - "associated type bounds (`type T: Protocol`) are only valid inside protocol declarations"
//!
//! ### E442 -- `type_alias_requires_type` (Error, Correctness)
//!
//! **Message:** "type alias '{name}' requires a type definition"
//!
//! **Labels:**
//! - Primary: the type alias declaration
//!   - Span source: `util::entity_span` on the type alias entity
//!   - Message: "add '= Type' to provide a definition"
//!
//! **Notes:** (none)
//!
//! ### E443 -- `qualified_binding_not_conforming` (Error, Correctness)
//!
//! **Message:** "'{type_name}' does not conform to '{protocol_name}'"
//!
//! **Labels:**
//! - Primary: the qualified binding
//!   - Span source: `util::entity_span` on the type alias entity
//!   - Message: "qualified binding references non-conformed protocol"
//!
//! **Notes:** (none)
//!
//! ### E444 -- `qualified_binding_wrong_protocol` (Error, Correctness)
//!
//! **Message:** "protocol '{protocol}' has no associated type '{type_name}'"
//!
//! **Labels:**
//! - Primary: the qualified binding
//!   - Span source: `util::entity_span` on the type alias entity
//!   - Message: "no such associated type in protocol"
//!
//! **Notes:** (none)
//!
//! ### E445 -- `ambiguous_associated_type` (Error, Correctness)
//!
//! **Message:** "associated type '{name}' is ambiguous between protocols: {list}"
//!
//! **Labels:**
//! - Primary: the unqualified binding
//!   - Span source: `util::entity_span` on the type alias entity
//!   - Message: "use a qualified binding to disambiguate"
//!
//! **Notes:** (none)
//!
//! ### E446 -- `associated_type_constraint_not_satisfied` (Error, Correctness)
//!
//! **Message:** "type '{bound_type}' does not satisfy constraint '{protocol}' on associated type '{name}'"
//!
//! **Labels:**
//! - Primary: the type alias binding
//!   - Span source: `util::entity_span` on the type alias entity
//!   - Message: "constraint not satisfied"
//!
//! **Notes:** (none)

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{
    ConformanceItem, Conformances, Name, NodeKind, QualifiedTarget, TypeAnnotation,
};
use kestrel_name_res::{ConformingProtocols, ExtensionsFor, ResolveTypePath, TypeResolution};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E441",
        name: "associated_type_bounds_in_wrong_context",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E442",
        name: "type_alias_requires_type",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E443",
        name: "qualified_binding_not_conforming",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E444",
        name: "qualified_binding_wrong_protocol",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E445",
        name: "ambiguous_associated_type",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E446",
        name: "associated_type_constraint_not_satisfied",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct TypeAliasValidationAnalyzer;

impl Describe for TypeAliasValidationAnalyzer {
    fn id(&self) -> &'static str {
        "type_alias_validation"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for TypeAliasValidationAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::TypeAlias]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        // Determine context: is this inside a protocol, concrete type, extension, or module?
        let parent_kind = cx
            .query
            .parent_of(cx.entity)
            .and_then(|p| cx.query.get::<NodeKind>(p).cloned());

        let is_protocol_context = matches!(parent_kind, Some(NodeKind::Protocol));

        // Check 1: Conformances on a type alias are bounds (type Item: Protocol).
        // Only valid inside protocol declarations.
        if !is_protocol_context
            && let Some(conformances) = cx.query.get::<Conformances>(cx.entity)
            && !conformances.0.is_empty()
        {
            let name = util::entity_name(cx.query, cx.entity);
            let span = util::entity_span(cx.query, cx.entity);
            diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[0].id,
                        severity: DESCRIPTORS[0].default_severity,
                        message: format!(
                            "type alias cannot have bounds outside a protocol: '{}'",
                            name
                        ),
                        labels: vec![DiagLabel {
                            span,
                            message: "bounds not allowed here".into(),
                            is_primary: true,
                        }],
                        notes: vec![
                            "associated type bounds (`type T: Protocol`) are only valid inside protocol declarations".into(),
                        ],
                    });
        }

        // Check 2: Non-protocol type aliases require `= Type` definition.
        // Inside protocols, abstract associated types (no definition) are allowed.
        if !is_protocol_context && cx.query.get::<TypeAnnotation>(cx.entity).is_none() {
            let name = util::entity_name(cx.query, cx.entity);
            let span = util::entity_span(cx.query, cx.entity);
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[1].id,
                severity: DESCRIPTORS[1].default_severity,
                message: format!("type alias requires a type definition: '{}'", name),
                labels: vec![DiagLabel {
                    span,
                    message: "add '= Type' to provide a definition".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
        }

        // Checks 3-5 apply to type aliases inside concrete types (struct/enum)
        // or extensions — not inside protocols or at module level.
        let parent = cx.query.parent_of(cx.entity);
        let parent_kind = parent.and_then(|p| cx.query.get::<NodeKind>(p).cloned());
        let is_type_context = matches!(
            parent_kind,
            Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Extension)
        );

        if is_type_context && !is_protocol_context {
            let alias_name = util::entity_name(cx.query, cx.entity);
            let span = util::entity_span(cx.query, cx.entity);
            let parent_entity = parent.unwrap();

            // Determine the conforming type entity — for extensions, get the target
            let conforming_type = if parent_kind == Some(NodeKind::Extension) {
                use kestrel_name_res::ExtensionTargetEntity;
                cx.query.query(ExtensionTargetEntity {
                    extension: parent_entity,
                    root: cx.root,
                })
            } else {
                Some(parent_entity)
            };

            if let Some(type_entity) = conforming_type {
                let is_qualified = is_qualified_binding(cx, cx.entity);

                if is_qualified {
                    // Check 3 & 4: Qualified binding validation
                    check_qualified_binding(
                        cx,
                        cx.entity,
                        type_entity,
                        &alias_name,
                        &span,
                        &mut diags,
                    );
                } else {
                    // Check 5: Unqualified binding is ambiguous when the type
                    // conforms to multiple protocols that each declare an
                    // associated type with the same name. Fires on struct/enum
                    // bodies and extensions — the user must disambiguate with
                    // a qualified binding (`type Proto.Item = ...`).
                    check_unqualified_ambiguity(
                        cx,
                        parent_entity,
                        type_entity,
                        &alias_name,
                        &span,
                        &mut diags,
                    );
                }
            }
        }

        diags
    }
}

/// Check if a type alias entity has a qualified binding (`type P.Assoc = ...`).
fn is_qualified_binding(cx: &DeclContext<'_>, entity: kestrel_hecs::Entity) -> bool {
    cx.query.get::<QualifiedTarget>(entity).is_some()
}

/// Extract the protocol path segments from a qualified binding's QualifiedTarget.
/// For `type Iterator.Item = Int`, returns vec!["Iterator"].
/// For `type Add[Int].Output = Int`, returns vec!["Add"].
fn extract_qualified_protocol_segments(
    cx: &DeclContext<'_>,
    entity: kestrel_hecs::Entity,
) -> Option<Vec<String>> {
    let target = cx.query.get::<QualifiedTarget>(entity)?;
    let kestrel_ast::AstType::Named { segments, .. } = &target.0 else {
        return None;
    };
    let names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    if names.is_empty() { None } else { Some(names) }
}

/// Check 3 & 4: Validate a qualified binding like `type Iterator.Item = Int`.
/// E443: parent type doesn't conform to the named protocol.
/// E444: protocol doesn't declare that associated type.
fn check_qualified_binding(
    cx: &DeclContext<'_>,
    alias_entity: kestrel_hecs::Entity,
    type_entity: kestrel_hecs::Entity,
    alias_name: &str,
    span: &kestrel_span::Span,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let Some(proto_segments) = extract_qualified_protocol_segments(cx, alias_entity) else {
        return;
    };
    let proto_name = proto_segments.join(".");

    // Resolve the protocol name to an entity
    let context = cx.query.parent_of(alias_entity).unwrap_or(cx.root);
    let resolution = cx.query.query(ResolveTypePath {
        segments: proto_segments,
        context,
        root: cx.root,
    });
    let TypeResolution::Found(protocol_entity) = resolution else {
        return;
    };
    if cx.query.get::<NodeKind>(protocol_entity) != Some(&NodeKind::Protocol) {
        return;
    }

    // Check 3 (E443): Does the parent type conform to this protocol?
    let conforming = cx.query.query(ConformingProtocols {
        entity: type_entity,
        root: cx.root,
    });
    if !conforming.contains(&protocol_entity) {
        let type_name = util::entity_name(cx.query, type_entity);
        diags.push(AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[2].id,
            severity: DESCRIPTORS[2].default_severity,
            message: format!("'{}' does not conform to '{}'", type_name, proto_name,),
            labels: vec![DiagLabel {
                span: span.clone(),
                message: "qualified binding references non-conformed protocol".into(),
                is_primary: true,
            }],
            notes: vec![],
        });
        return; // Don't check E444 if E443 fires
    }

    // Check 4 (E444): Does the protocol declare this associated type?
    let has_assoc_type = cx.query.children_of(protocol_entity).iter().any(|&child| {
        cx.query.get::<NodeKind>(child) == Some(&NodeKind::TypeAlias)
            && cx
                .query
                .get::<Name>(child)
                .is_some_and(|n| n.0 == alias_name)
    });
    if !has_assoc_type {
        diags.push(AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[3].id,
            severity: DESCRIPTORS[3].default_severity,
            message: format!(
                "protocol '{}' does not have associated type '{}'",
                proto_name, alias_name,
            ),
            labels: vec![DiagLabel {
                span: span.clone(),
                message: "no such associated type in protocol".into(),
                is_primary: true,
            }],
            notes: vec![],
        });
    }
}

/// Check 5 (E445): Unqualified binding is ambiguous if multiple conformed protocols
/// declare the same associated type name, unless the extra protocols are already
/// covered by qualified bindings in extensions.
///
/// `binding_parent` is the entity that directly owns the binding — the struct
/// / enum for `struct S: A, B { type X = ... }`, or the extension for
/// `extend S: A { type X = ... }`. Only that entity's declared conformances
/// (and their refinement chains) participate in ambiguity checking — we
/// deliberately don't cross into conformances added by *other* extensions,
/// which is why stdlib patterns like `struct Array[T]: _ExpressibleByArrayLiteral`
/// + `extend Array[T]: ArrayMatchable { type Element = T }` stay unambiguous.
fn check_unqualified_ambiguity(
    cx: &DeclContext<'_>,
    binding_parent: kestrel_hecs::Entity,
    type_entity: kestrel_hecs::Entity,
    alias_name: &str,
    span: &kestrel_span::Span,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let conforming = refinement_closure(cx, binding_parent);

    // Find all conformed protocols that declare an associated type with this name
    let mut matching_protocols: Vec<(kestrel_hecs::Entity, String)> = Vec::new();
    for &proto in &conforming {
        let has_assoc = cx.query.children_of(proto).iter().any(|&child| {
            cx.query.get::<NodeKind>(child) == Some(&NodeKind::TypeAlias)
                && cx
                    .query
                    .get::<Name>(child)
                    .is_some_and(|n| n.0 == alias_name)
        });
        if has_assoc {
            matching_protocols.push((proto, util::entity_name(cx.query, proto)));
        }
    }

    if matching_protocols.len() <= 1 {
        return;
    }

    // Collect protocols already covered by qualified bindings in extensions.
    // e.g., `extend Iterator: Iterable { type Iterable.Item = Self.Item }`
    // means Iterable.Item is already bound — no ambiguity for that protocol.
    let mut covered = protocols_covered_by_qualified_bindings(cx, type_entity, alias_name);

    // Also cover protocols declared directly on this extension's conformance list.
    // e.g., `extend Array[T]: ArrayMatchable { type Element = T }` — the
    // unqualified Element is clearly for ArrayMatchable since the extension
    // declares that conformance.
    if let Some(parent) = cx.query.parent_of(cx.entity)
        && cx.query.get::<NodeKind>(parent) == Some(&NodeKind::Extension)
    {
        covered.extend(protocols_declared_on_extension(cx, parent));
    }

    // Filter out covered protocols
    let uncovered: Vec<&str> = matching_protocols
        .iter()
        .filter(|(entity, _)| !covered.contains(entity))
        .map(|(_, name)| name.as_str())
        .collect();

    if uncovered.len() > 1 {
        let proto_list = uncovered.join(", ");
        diags.push(AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[4].id,
            severity: DESCRIPTORS[4].default_severity,
            message: format!(
                "ambiguous associated type '{}': declared in protocols {}",
                alias_name, proto_list,
            ),
            labels: vec![DiagLabel {
                span: span.clone(),
                message: "use a qualified binding to disambiguate".into(),
                is_primary: true,
            }],
            notes: vec![],
        });
    }
}

/// Transitively walk refinement conformances starting from `entity`'s direct
/// conformances. Crosses protocol-to-protocol refinement edges (e.g.
/// `protocol B: A`) but NOT extension-added conformances (e.g. `extend S: A`).
/// This is the scope the ambiguity check uses — see `check_unqualified_ambiguity`.
fn refinement_closure(
    cx: &DeclContext<'_>,
    entity: kestrel_hecs::Entity,
) -> Vec<kestrel_hecs::Entity> {
    let mut out: Vec<kestrel_hecs::Entity> = Vec::new();
    let mut seen: std::collections::HashSet<kestrel_hecs::Entity> =
        std::collections::HashSet::new();
    let mut stack: Vec<kestrel_hecs::Entity> = Vec::new();
    seed_refinement_closure(cx, entity, &mut stack, &mut seen);
    while let Some(proto) = stack.pop() {
        out.push(proto);
        seed_refinement_closure(cx, proto, &mut stack, &mut seen);
    }
    out
}

fn seed_refinement_closure(
    cx: &DeclContext<'_>,
    entity: kestrel_hecs::Entity,
    stack: &mut Vec<kestrel_hecs::Entity>,
    seen: &mut std::collections::HashSet<kestrel_hecs::Entity>,
) {
    let Some(conformances) = cx.query.get::<Conformances>(entity) else {
        return;
    };
    for item in &conformances.0 {
        let ConformanceItem::Positive(ast_ty, _) = item else {
            continue;
        };
        let kestrel_ast::AstType::Named { segments, .. } = ast_ty else {
            continue;
        };
        let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
        let result = cx.query.query(ResolveTypePath {
            segments: seg_names,
            context: entity,
            root: cx.root,
        });
        let TypeResolution::Found(resolved) = result else {
            continue;
        };
        if cx.query.get::<NodeKind>(resolved) != Some(&NodeKind::Protocol) {
            continue;
        }
        if seen.insert(resolved) {
            stack.push(resolved);
        }
    }
}

/// Find protocols whose associated type is already provided by a qualified
/// binding in an extension. For example, `extend Iterator: Iterable { type
/// Iterable.Item = Self.Item }` covers `Iterable` for `Item`.
fn protocols_covered_by_qualified_bindings(
    cx: &DeclContext<'_>,
    type_entity: kestrel_hecs::Entity,
    alias_name: &str,
) -> Vec<kestrel_hecs::Entity> {
    let mut covered = Vec::new();

    // Check extensions of the type entity AND extensions of protocols it conforms to.
    // The key case: `extend Iterator: Iterable { type Iterable.Item = ... }` targets
    // Iterator (a protocol), not the concrete type. So we need to check extensions of
    // all conformed protocols too.
    let mut targets_to_check = vec![type_entity];
    let conforming = cx.query.query(ConformingProtocols {
        entity: type_entity,
        root: cx.root,
    });
    targets_to_check.extend(conforming.iter());

    for &target in &targets_to_check {
        let extensions = cx.query.query(ExtensionsFor {
            target,
            root: cx.root,
        });

        for ext in &extensions {
            // Walk the extension's TypeAlias children
            for &child in cx.query.children_of(*ext) {
                if cx.query.get::<NodeKind>(child) != Some(&NodeKind::TypeAlias) {
                    continue;
                }
                // Must have the same name as the alias we're checking
                if cx
                    .query
                    .get::<Name>(child)
                    .is_none_or(|n| n.0 != alias_name)
                {
                    continue;
                }
                // Must be a qualified binding (has AssociatedTypeTarget in CST)
                if !is_qualified_binding(cx, child) {
                    continue;
                }
                // Resolve which protocol the qualified binding targets
                if let Some(proto_segments) = extract_qualified_protocol_segments(cx, child) {
                    let context = cx.query.parent_of(child).unwrap_or(cx.root);
                    let resolution = cx.query.query(ResolveTypePath {
                        segments: proto_segments,
                        context,
                        root: cx.root,
                    });
                    if let TypeResolution::Found(proto_entity) = resolution {
                        covered.push(proto_entity);
                    }
                }
            }
        }
    }

    covered
}

/// Return the protocol entities the given extension block declares in its
/// own conformance list. For `extend Array[T]: ArrayMatchable { ... }`,
/// returns `[ArrayMatchable_entity]`.
///
/// Intentionally local-only — this asks "what conformances does *this
/// extension block* add?", not "what protocols does the target type
/// transitively conform to." Callers wanting the transitive set use
/// `ConformingProtocols` instead.
fn protocols_declared_on_extension(
    cx: &DeclContext<'_>,
    extension: kestrel_hecs::Entity,
) -> Vec<kestrel_hecs::Entity> {
    let Some(conformances) = cx.query.get::<Conformances>(extension) else {
        return vec![];
    };
    let context = cx.query.parent_of(extension).unwrap_or(cx.root);
    conformances
        .0
        .iter()
        .filter_map(|item| {
            let kestrel_ast_builder::ConformanceItem::Positive(ast_ty, _) = item else {
                return None;
            };
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
        })
        .collect()
}
