//! # Duplicate Builtin Analyzer
//!
//! A lang item is identified by its `@builtin(.X)` annotation alone
//! (`ResolveBuiltin` has no name-based lookup), so two declarations claiming
//! the same builtin would make the language's meaning depend on declaration
//! order. `BuiltinIndex` keeps the first annotation in declaration order;
//! every later one is reported here. While a stdlib is loaded, only `std`
//! may declare lang items (`kestrel_name_res::builtin_annotation_allowed`,
//! shared with the index so the two cannot disagree).
//!
//! ## Diagnostics
//!
//! ### E400 -- `duplicate_builtin` (Error, Correctness)
//!
//! **Message:** "duplicate @builtin(.{feature}): already declared by '{name}'" ({feature} as written)
//!
//! **Labels:**
//! - Primary: the later declaration's `@builtin` attribute
//!   - Span source: the `AstAttribute` span (declaration span if absent)
//!   - Message: "second declaration of this builtin"
//! - Secondary: the kept declaration's `@builtin` attribute
//!   - Message: "first declared here"
//!
//! ### E401 -- `builtin_outside_stdlib` (Error, Correctness)
//!
//! **Message:** "@builtin(.{feature}) is reserved for the standard library"
//!
//! Fires only when a top-level `std` module exists; `--no-std` programs and
//! test preludes declare their own lang items. Checked before E400 — a
//! rejected annotation never enters the index, so it is reported once.
//!
//! **Labels:**
//! - Primary: the `@builtin` attribute
//!   - Message: "lang items can only be declared in `std`"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::NodeKind;
use kestrel_name_res::{BuiltinIndex, EntityBuiltin, builtin_annotation_allowed};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E400",
        name: "duplicate_builtin",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E401",
        name: "builtin_outside_stdlib",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct DuplicateBuiltinAnalyzer;

impl Describe for DuplicateBuiltinAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::DuplicateBuiltin
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for DuplicateBuiltinAnalyzer {
    /// Every kind that can carry attributes.
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[
            NodeKind::Struct,
            NodeKind::Enum,
            NodeKind::EnumCase,
            NodeKind::Protocol,
            NodeKind::Function,
            NodeKind::Initializer,
            NodeKind::Field,
            NodeKind::Subscript,
            NodeKind::TypeAlias,
        ]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let Some(builtin) = cx.query.query(EntityBuiltin { entity: cx.entity }) else {
            return vec![];
        };
        let feature = written_feature(cx).unwrap_or_else(|| format!(".{}", builtin.name()));

        // User code cannot claim a lang item while a stdlib supplies them.
        // Such an annotation never enters the index, so it is not also a
        // duplicate: report E401 alone.
        if !builtin_annotation_allowed(cx.query, cx.root, cx.entity) {
            return vec![AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[1].id,
                severity: DESCRIPTORS[1].default_severity,
                message: format!("@builtin({feature}) is reserved for the standard library"),
                labels: vec![DiagLabel {
                    span: builtin_span(cx.query, cx.entity),
                    message: "lang items can only be declared in `std`".into(),
                    is_primary: true,
                }],
                notes: vec![],
            }];
        }

        let index = cx.query.query(BuiltinIndex { root: cx.root });
        let Some(first) = index.get(&builtin).filter(|&e| e != cx.entity) else {
            return vec![];
        };
        vec![AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: format!(
                "duplicate @builtin({feature}): already declared by '{}'",
                util::entity_name(cx.query, first)
            ),
            labels: vec![
                DiagLabel {
                    span: builtin_span(cx.query, cx.entity),
                    message: "second declaration of this builtin".into(),
                    is_primary: true,
                },
                DiagLabel {
                    span: builtin_span(cx.query, first),
                    message: "first declared here".into(),
                    is_primary: false,
                },
            ],
            notes: vec![],
        }]
    }
}

/// The entity's `@builtin` attribute.
fn builtin_attr<'a>(
    query: &'a kestrel_hecs::QueryContext<'_>,
    entity: kestrel_hecs::Entity,
) -> Option<&'a kestrel_ast_builder::AstAttribute> {
    let attrs = query.get::<kestrel_ast_builder::Attributes>(entity)?;
    attrs.0.iter().find(|a| a.name == "builtin")
}

/// Span of the `@builtin(...)` attribute, falling back to the declaration.
fn builtin_span(
    query: &kestrel_hecs::QueryContext<'_>,
    entity: kestrel_hecs::Entity,
) -> kestrel_span::Span {
    builtin_attr(query, entity)
        .map(|a| a.span.clone())
        .unwrap_or_else(|| util::entity_span(query, entity))
}

/// The feature as written in the annotation (`.AddOperatorProtocol`), which
/// is not always `Builtin::name()` (`Addable`).
fn written_feature(cx: &DeclContext<'_>) -> Option<String> {
    Some(
        builtin_attr(cx.query, cx.entity)?
            .args
            .first()?
            .value
            .clone(),
    )
}
