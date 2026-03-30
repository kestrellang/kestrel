//! # Duplicate Symbol Analyzer
//!
//! Checks that no two type declarations within a scope share the same name.
//! Scopes that can contain types: Module, Struct, Enum, Protocol, Extension.
//! Type-like children checked: Struct, Enum, Protocol, TypeAlias.
//! Also checks member-level duplicates: Field vs Field, Field vs Function.
//! Function-to-Function duplicates with the same name are skipped (handled
//! by DuplicateCallableAnalyzer which considers full signatures).
//!
//! ## Diagnostics
//!
//! ### E424 -- `duplicate_symbol_same_kind` (Error, Correctness)
//!
//! **Message:** "duplicate definition of {kind} '{name}'"
//!
//! **Labels:**
//! - Primary: the duplicate declaration
//!   - Span source: `util::entity_span` on the second child entity
//!   - Message: "{kind} defined here"
//! - Secondary: the first declaration with the same name
//!   - Span source: `util::entity_span` on the first child entity
//!   - Message: "first defined as {kind} here"
//!
//! **Notes:** (none)
//!
//! ### E425 -- `duplicate_symbol_different_kind` (Error, Correctness)
//!
//! **Message:** "'{name}' is already defined as a {original_kind}"
//!
//! **Labels:**
//! - Primary: the duplicate declaration
//!   - Span source: `util::entity_span` on the second child entity
//!   - Message: "{new_kind} defined here"
//! - Secondary: the first declaration with the same name
//!   - Span source: `util::entity_span` on the first child entity
//!   - Message: "first defined as {original_kind} here"
//!
//! **Notes:** (none)

use std::collections::HashMap;

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{CstNode, Name, NodeKind};
use kestrel_span2::Span;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E424",
        name: "duplicate_symbol_same_kind",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E425",
        name: "duplicate_symbol_different_kind",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct DuplicateSymbolAnalyzer;

impl Describe for DuplicateSymbolAnalyzer {
    fn id(&self) -> &'static str {
        "duplicate_symbol"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

/// Classify a child NodeKind as a type-like or member-like symbol, returning
/// a human-readable kind string. Returns None for kinds we don't duplicate-check.
fn kind_description(kind: &NodeKind) -> Option<&'static str> {
    match kind {
        NodeKind::Struct => Some("struct"),
        NodeKind::Enum => Some("enum"),
        NodeKind::Protocol => Some("protocol"),
        NodeKind::TypeAlias => Some("type alias"),
        NodeKind::Field => Some("field"),
        NodeKind::Function => Some("function"),
        _ => None,
    }
}

/// Returns true if this kind is a type-like declaration (checked for
/// duplicate type names within namespace scopes).
fn is_type_kind(kind: &NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol | NodeKind::TypeAlias
    )
}

/// Returns true if this kind is a member-like declaration (checked for
/// duplicate members within types that have members).
fn is_member_kind(kind: &NodeKind) -> bool {
    matches!(kind, NodeKind::Field | NodeKind::Function)
}

impl DeclCheck for DuplicateSymbolAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[
            NodeKind::Module,
            NodeKind::Struct,
            NodeKind::Enum,
            NodeKind::Protocol,
            NodeKind::Extension,
        ]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        // Check duplicate type names among type-like children
        check_duplicates(cx, &mut diags, is_type_kind);

        // For types that have members, also check member-level duplicates.
        // Function-to-function with the same name is allowed (overloading by labels).
        if matches!(
            cx.kind,
            NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol | NodeKind::Extension
        ) {
            check_member_duplicates(cx, &mut diags);
        }

        diags
    }
}

/// Check for duplicate named children of the given category (type-like or member-like).
fn check_duplicates(
    cx: &DeclContext<'_>,
    diags: &mut Vec<AnalyzeDiagnostic>,
    filter: fn(&NodeKind) -> bool,
) {
    // name -> (first span, kind description)
    let mut seen: HashMap<String, (Span, &'static str)> = HashMap::new();

    for &child in cx.query.children_of(cx.entity) {
        let Some(child_kind) = cx.query.get::<NodeKind>(child) else {
            continue;
        };
        if !filter(child_kind) {
            continue;
        }

        let Some(desc) = kind_description(child_kind) else {
            continue;
        };

        // Skip anonymous entities and qualified type alias bindings
        // (e.g., `type Iterator.Item = T` and `type Container.Item = T`
        // are for different protocols — not duplicates)
        let Some(name_comp) = cx.query.get::<Name>(child) else {
            continue;
        };
        if *child_kind == NodeKind::TypeAlias {
            if let Some(cst) = cx.query.get::<CstNode>(child) {
                use kestrel_syntax_tree2::SyntaxKind;
                if cst.0.children().any(|c| c.kind() == SyntaxKind::AssociatedTypeTarget) {
                    continue;
                }
            }
        }
        let name = name_comp.0.clone();
        let span = util::entity_span(cx.query, child);

        if let Some((first_span, first_desc)) = seen.get(&name) {
            if desc == *first_desc {
                // Same kind duplicate (E424)
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message: format!("duplicate definition of {} '{}'", desc, name),
                    labels: vec![
                        DiagLabel {
                            span,
                            message: format!("{} defined here", desc),
                            is_primary: true,
                        },
                        DiagLabel {
                            span: first_span.clone(),
                            message: format!("first defined as {} here", first_desc),
                            is_primary: false,
                        },
                    ],
                    notes: vec![],
                });
            } else {
                // Different kind duplicate (E425)
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[1].id,
                    severity: DESCRIPTORS[1].default_severity,
                    message: format!("'{}' is already defined as a {}", name, first_desc),
                    labels: vec![
                        DiagLabel {
                            span,
                            message: format!("{} defined here", desc),
                            is_primary: true,
                        },
                        DiagLabel {
                            span: first_span.clone(),
                            message: format!("first defined as {} here", first_desc),
                            is_primary: false,
                        },
                    ],
                    notes: vec![],
                });
            }
        } else {
            seen.insert(name, (span, desc));
        }
    }
}

/// Check for duplicate members within a type. Fields and functions are checked,
/// but function-to-function duplicates are skipped (handled by DuplicateCallableAnalyzer).
fn check_member_duplicates(cx: &DeclContext<'_>, diags: &mut Vec<AnalyzeDiagnostic>) {
    // name -> (first kind, first span, kind description)
    let mut seen: HashMap<String, (NodeKind, Span, &'static str)> = HashMap::new();

    for &child in cx.query.children_of(cx.entity) {
        let Some(child_kind) = cx.query.get::<NodeKind>(child) else {
            continue;
        };
        if !is_member_kind(child_kind) {
            continue;
        }

        let Some(desc) = kind_description(child_kind) else {
            continue;
        };

        // Skip anonymous entities (e.g. associated type bindings like `type Iterator.Item = T`)
        let Some(name_comp) = cx.query.get::<Name>(child) else {
            continue;
        };
        let name = name_comp.0.clone();
        let span = util::entity_span(cx.query, child);

        if let Some((first_kind, first_span, first_desc)) = seen.get(&name) {
            // Skip function-to-function: overloading is allowed and handled
            // by the DuplicateCallableAnalyzer (which checks full signatures).
            if child_kind == &NodeKind::Function && first_kind == &NodeKind::Function {
                continue;
            }

            if desc == *first_desc {
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message: format!("duplicate definition of {} '{}'", desc, name),
                    labels: vec![
                        DiagLabel {
                            span,
                            message: format!("{} defined here", desc),
                            is_primary: true,
                        },
                        DiagLabel {
                            span: first_span.clone(),
                            message: format!("first defined as {} here", first_desc),
                            is_primary: false,
                        },
                    ],
                    notes: vec![],
                });
            } else {
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[1].id,
                    severity: DESCRIPTORS[1].default_severity,
                    message: format!("'{}' is already defined as a {}", name, first_desc),
                    labels: vec![
                        DiagLabel {
                            span,
                            message: format!("{} defined here", desc),
                            is_primary: true,
                        },
                        DiagLabel {
                            span: first_span.clone(),
                            message: format!("first defined as {} here", first_desc),
                            is_primary: false,
                        },
                    ],
                    notes: vec![],
                });
            }
        } else {
            seen.insert(name, (child_kind.clone(), span, desc));
        }
    }
}
