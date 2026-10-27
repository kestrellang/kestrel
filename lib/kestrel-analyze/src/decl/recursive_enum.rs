//! # Recursive Enum Analyzer
//!
//! Checks enums whose cases reach the enum itself by value, using the shared
//! `kestrel_semantics::RecursiveCases` walk (the same answer MIR lowering
//! uses to decide which cases of an `indirect enum` are boxed).
//!
//! A plain enum has value semantics and a fixed size, so it cannot contain
//! itself: that is E429. An `indirect enum` boxes exactly its recursive cases
//! (`kestrel_semantics::IndirectCases`); the remaining diagnostics are about
//! when that boxing is pointless or impossible.
//!
//! ## Diagnostics
//!
//! ### E429 -- `recursive_enum` (Error, Correctness)
//! **Message:** "recursive enum requires `indirect`"
//!
//! **Labels:**
//! - Primary: the enum declaration when a case names the enum directly,
//!   otherwise the first recursive case
//!   - Span source: `util::entity_span` on the enum or `EnumCase` entity
//!   - Message: "recursive enum requires `indirect`"
//!
//! **Notes:**
//! - "add 'indirect' before the enum declaration to allow recursive cases"
//!
//! ### E405 -- `indirect_without_recursion` (Warning, Usage)
//! **Message:** "`indirect` has no effect: enum '{name}' has no recursive case"
//!
//! **Labels:**
//! - Primary: the enum declaration
//!   - Span source: `util::entity_span` on the enum entity
//!   - Message: "no case of this enum contains it"
//!
//! **Notes:**
//! - "only cases that contain the enum itself are stored behind a box"
//!
//! ### E406 -- `indirect_enum_requires_stdlib` (Error, Correctness)
//! **Message:** "indirect enums require the standard library"
//!
//! **Labels:**
//! - Primary: the enum declaration
//!   - Span source: `util::entity_span` on the enum entity
//!   - Message: "boxing a recursive case needs `std.memory.OwnedBox`"
//!
//! **Notes:** (none)
//!
//! ### E407 -- `indirect_polymorphic_recursion` (Error, Correctness)
//! **Message:** "indirect enum '{name}' contains itself with different type arguments"
//!
//! **Labels:**
//! - Primary: the offending case
//!   - Span source: `util::entity_span` on the `EnumCase` entity
//!   - Message: "this case changes the enum's type arguments"
//!
//! **Notes:**
//! - "a recursive case must use the enum's own type parameters unchanged; otherwise every level needs a new instantiation"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{IsIndirect, NodeKind};
use kestrel_hir::Builtin;
use kestrel_name_res::ResolveBuiltin;
use kestrel_semantics::{RecursiveCase, RecursiveCases};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E429",
        name: "recursive_enum",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E405",
        name: "indirect_without_recursion",
        default_severity: Severity::Warning,
        category: Category::Usage,
    },
    DiagnosticDescriptor {
        id: "E406",
        name: "indirect_enum_requires_stdlib",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E407",
        name: "indirect_polymorphic_recursion",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct RecursiveEnumAnalyzer;

impl Describe for RecursiveEnumAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::RecursiveEnum
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for RecursiveEnumAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Enum]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let indirect = cx.query.get::<IsIndirect>(cx.entity).is_some();
        let cases = cx.query.query(RecursiveCases {
            entity: cx.entity,
            root: cx.root,
            through_indirect: indirect,
        });
        if indirect {
            check_indirect(cx, &cases)
        } else {
            check_plain(cx, &cases)
        }
    }
}

/// E429: a plain enum that contains itself.
fn check_plain(cx: &DeclContext<'_>, cases: &[RecursiveCase]) -> Vec<AnalyzeDiagnostic> {
    let Some(rec) = cases.first() else {
        return vec![];
    };
    // Direct recursion: primary on the enum decl. Otherwise on the case.
    let primary = if rec.direct { cx.entity } else { rec.case };
    vec![diagnostic(
        &DESCRIPTORS[0],
        util::entity_span(cx.query, primary),
        "recursive enum requires `indirect`".into(),
        "recursive enum requires `indirect`",
        vec!["add 'indirect' before the enum declaration to allow recursive cases".into()],
    )]
}

/// E405–E407 for an `indirect enum`.
fn check_indirect(cx: &DeclContext<'_>, cases: &[RecursiveCase]) -> Vec<AnalyzeDiagnostic> {
    let name = util::entity_name(cx.query, cx.entity);
    let enum_span = util::entity_span(cx.query, cx.entity);
    if cases.is_empty() {
        return vec![diagnostic(
            &DESCRIPTORS[1],
            enum_span,
            format!("`indirect` has no effect: enum '{name}' has no recursive case"),
            "no case of this enum contains it",
            vec!["only cases that contain the enum itself are stored behind a box".into()],
        )];
    }
    let owned_box = cx.query.query(ResolveBuiltin {
        builtin: Builtin::OwnedBox,
        root: cx.root,
    });
    if owned_box.is_none() {
        return vec![diagnostic(
            &DESCRIPTORS[2],
            enum_span,
            "indirect enums require the standard library".into(),
            "boxing a recursive case needs `std.memory.OwnedBox`",
            vec![],
        )];
    }
    cases
        .iter()
        .filter(|c| !c.regular)
        .map(|c| {
            diagnostic(
                &DESCRIPTORS[3],
                util::entity_span(cx.query, c.case),
                format!("indirect enum '{name}' contains itself with different type arguments"),
                "this case changes the enum's type arguments",
                vec![
                    "a recursive case must use the enum's own type parameters unchanged; \
                     otherwise every level needs a new instantiation"
                        .into(),
                ],
            )
        })
        .collect()
}

fn diagnostic(
    descriptor: &DiagnosticDescriptor,
    span: kestrel_span::Span,
    message: String,
    label: &str,
    notes: Vec<String>,
) -> AnalyzeDiagnostic {
    AnalyzeDiagnostic {
        descriptor_id: descriptor.id,
        severity: descriptor.default_severity,
        message,
        labels: vec![DiagLabel {
            span,
            message: label.into(),
            is_primary: true,
        }],
        notes,
    }
}
