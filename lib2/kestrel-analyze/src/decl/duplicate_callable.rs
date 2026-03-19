//! # Duplicate Callable Analyzer
//!
//! Detects duplicate function, initializer, and subscript signatures within a
//! type or extension. In Kestrel, overloading is label-based: two callables with
//! the same name and same parameter labels are duplicates regardless of parameter
//! or return types.
//!
//! The "duplicate key" is `(name, ordered_labels)`. For functions the name comes
//! from the Name component; for initializers it is `"init"`; for subscripts it is
//! `"(subscript)"`.
//!
//! ## Diagnostics
//!
//! ### KS426 -- `duplicate_callable` (Error, Correctness)
//!
//! **Message:** "duplicate {kind} signature: {signature}"
//!
//! **Labels:**
//! - Primary: the duplicate callable declaration
//!   - Span source: `util::entity_span` on the second callable entity
//!   - Message: "duplicate definition"
//! - Secondary: the first callable with the same signature
//!   - Span source: `util::entity_span` on the first callable entity
//!   - Message: "first defined here"
//!
//! **Notes:** (none)

use std::collections::HashMap;

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Callable, NodeKind};
use kestrel_span2::Span;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "KS426",
    name: "duplicate_callable",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct DuplicateCallableAnalyzer;

impl Describe for DuplicateCallableAnalyzer {
    fn id(&self) -> &'static str {
        "duplicate_callable"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

/// A key that uniquely identifies a callable signature for duplicate detection.
/// Two callables are duplicates iff they share the same (name, labels) pair.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct DuplicateKey {
    name: String,
    labels: Vec<Option<String>>,
}

impl DuplicateKey {
    /// Human-readable display like "foo(x:, y:)" or "init(_:, from:)"
    fn display(&self) -> String {
        if self.labels.is_empty() {
            return self.name.clone();
        }
        let label_strs: Vec<String> = self
            .labels
            .iter()
            .map(|l| match l {
                Some(s) => format!("{}:", s),
                None => "_:".into(),
            })
            .collect();
        format!("{}({})", self.name, label_strs.join(", "))
    }
}

/// Build the duplicate key for a callable child entity.
fn duplicate_key_for(
    child: kestrel_hecs::Entity,
    child_kind: &NodeKind,
    cx: &DeclContext<'_>,
) -> Option<(DuplicateKey, &'static str)> {
    let callable = cx.query.get::<Callable>(child)?;
    let labels: Vec<Option<String>> = callable.params.iter().map(|p| p.label.clone()).collect();

    match child_kind {
        NodeKind::Function => {
            let name = util::entity_name(cx.query, child);
            Some((DuplicateKey { name, labels }, "function"))
        }
        NodeKind::Initializer => Some((
            DuplicateKey {
                name: "init".into(),
                labels,
            },
            "initializer",
        )),
        NodeKind::Subscript => Some((
            DuplicateKey {
                name: "(subscript)".into(),
                labels,
            },
            "subscript",
        )),
        _ => None,
    }
}

impl DeclCheck for DuplicateCallableAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[
            NodeKind::Struct,
            NodeKind::Enum,
            NodeKind::Protocol,
            NodeKind::Extension,
        ]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Map from duplicate key to list of (span, kind_name)
        let mut seen: HashMap<DuplicateKey, Vec<(Span, &'static str)>> = HashMap::new();

        for &child in cx.query.children_of(cx.entity) {
            let Some(child_kind) = cx.query.get::<NodeKind>(child) else {
                continue;
            };

            let Some((key, kind_name)) = duplicate_key_for(child, child_kind, cx) else {
                continue;
            };

            let span = util::entity_span(cx.query, child);
            seen.entry(key).or_default().push((span, kind_name));
        }

        let mut diags = Vec::new();

        for (key, callables) in &seen {
            if callables.len() < 2 {
                continue;
            }

            // Report duplicate for each pair beyond the first
            let (first_span, _) = &callables[0];
            for (dup_span, kind_name) in &callables[1..] {
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message: format!("duplicate {} signature: {}", kind_name, key.display()),
                    labels: vec![
                        DiagLabel {
                            span: dup_span.clone(),
                            message: "duplicate definition".into(),
                            is_primary: true,
                        },
                        DiagLabel {
                            span: first_span.clone(),
                            message: "first defined here".into(),
                            is_primary: false,
                        },
                    ],
                    notes: vec![],
                });
            }
        }

        diags
    }
}
