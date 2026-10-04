//! # Duplicate Builtin Analyzer
//!
//! A lang item is identified by its `@builtin(.X)` annotation alone
//! (`ResolveBuiltin` has no name-based lookup), so two declarations claiming
//! the same builtin would make the language's meaning depend on declaration
//! order. `BuiltinIndex` keeps the first annotation in declaration order;
//! every later one is reported here.
//!
//! ## Diagnostics
//!
//! ### E400 -- `duplicate_builtin` (Error, Correctness)
//!
//! **Message:** "duplicate @builtin(.{feature}): already declared by '{name}'"
//!
//! **Labels:**
//! - Primary: the later declaration
//!   - Span source: `util::entity_span` (declaration span)
//!   - Message: "second declaration of this builtin"
//! - Secondary: the declaration the index keeps
//!   - Message: "first declared here"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::NodeKind;
use kestrel_name_res::{BuiltinIndex, EntityBuiltin};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E400",
    name: "duplicate_builtin",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

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
        let index = cx.query.query(BuiltinIndex { root: cx.root });
        let Some(first) = index.get(&builtin).filter(|&e| e != cx.entity) else {
            return vec![];
        };
        vec![AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: format!(
                "duplicate @builtin(.{}): already declared by '{}'",
                builtin.name(),
                util::entity_name(cx.query, first)
            ),
            labels: vec![
                DiagLabel {
                    span: util::entity_span(cx.query, cx.entity),
                    message: "second declaration of this builtin".into(),
                    is_primary: true,
                },
                DiagLabel {
                    span: util::entity_span(cx.query, first),
                    message: "first declared here".into(),
                    is_primary: false,
                },
            ],
            notes: vec![],
        }]
    }
}
