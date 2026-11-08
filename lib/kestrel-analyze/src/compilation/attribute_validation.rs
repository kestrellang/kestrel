//! # Attribute Validation Analyzer
//!
//! Reports every attribute shape error: unknown attributes, wrong targets,
//! repeats, conflicts, and argument errors. The checks themselves live next to
//! the attribute registry (`kestrel_ast_builder::attrs`), which records them
//! as `AttrError`s on the `AttrsOf` query; this analyzer only forwards them, so
//! there is one place that decides what is malformed and one that reports it.
//!
//! Runs as a `CompilationCheck` so the filter is the presence of attributes,
//! not a `NodeKind` enumeration. Each entity's errors come from its cached
//! `AttrsOf`, so the walk itself is cheap.
//!
//! Checks about meaning rather than shape stay with their own analyzers and
//! codes: E400/E401 (`@builtin`), E605/E609–E611 (`@extern` FFI rules),
//! E615–E618 (`@main`).
//!
//! ## Diagnostics
//!
//! All labels: Primary span source is the `AstAttribute` name span, or the
//! `AstAttributeArg` span for argument errors (E903, E904, E907). Secondary
//! labels, where present, point at the other attribute involved.
//!
//! ### E461 — `unknown_attribute` (Error, Correctness)
//! **Message:** "unknown attribute '{name}'" (+ "; did you mean '@{x}'?")
//!
//! ### E612 — `extern_requires_calling_convention` (Error, Correctness)
//! **Message:** "'@extern' needs a calling convention: '@extern(.C)'".
//! Primary label on the declaration (its pre-registry location), secondary on
//! the attribute.
//!
//! ### E900 — `attribute_wrong_target` (Error, Correctness)
//! **Message:** "'@{name}' cannot be applied to {target}; it applies to {targets}"
//!
//! ### E901 — `duplicate_attribute` (Error, Correctness)
//! **Message:** "duplicate attribute '@{name}'"
//!
//! ### E902 — `attribute_missing_argument` (Error, Correctness)
//! **Message:** "'@{name}' needs {what}"
//!
//! ### E903 — `attribute_bad_value` (Error, Correctness)
//! **Message:** "'.{case}' is not a {noun}; expected {cases}" / "expected {kind}, found '{text}'" / "unknown {noun} '.{case}'"
//!
//! ### E904 — `attribute_bad_label` (Error, Correctness)
//! **Message:** "'@{name}' has no argument '{label}'" / "argument '{label}' given twice" / "positional argument after a labeled one"
//!
//! ### E905 — `attribute_needs_body` (Error, Correctness)
//! **Message:** "'@{name}' needs a body; {target} has none"
//!
//! ### E906 — `conflicting_attributes` (Error, Correctness)
//! **Message:** "'@{a}' cannot be combined with '@{b}': {reason}"
//!
//! ### E907 — `attribute_too_many_arguments` (Error, Correctness)
//! **Message:** "'@{name}' takes at most one argument"
//!
//! ### E909 — `attribute_bad_alignment` (Error, Correctness)
//! **Message:** "'@align({n})' must be a power of two between 1 and 4096"
//!
//! ### E914 — `deprecated_attribute_spelling` (Warning, Style)
//! **Message:** "'.{old}' is the old spelling; write '.{New}'" (`@platform(.linux)`)
//!
//! **Notes:** (none)

use crate::context::CompilationContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, CompilationCheck, Describe};
use kestrel_ast_builder::Attributes;
use kestrel_ast_builder::attrs::{AttrError, attrs_of};
use kestrel_hecs::Entity;

const fn error(id: &'static str, name: &'static str) -> DiagnosticDescriptor {
    DiagnosticDescriptor {
        id,
        name,
        default_severity: Severity::Error,
        category: Category::Correctness,
    }
}

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    error("E461", "unknown_attribute"),
    error("E612", "extern_requires_calling_convention"),
    error("E900", "attribute_wrong_target"),
    error("E901", "duplicate_attribute"),
    error("E902", "attribute_missing_argument"),
    error("E903", "attribute_bad_value"),
    error("E904", "attribute_bad_label"),
    error("E905", "attribute_needs_body"),
    error("E906", "conflicting_attributes"),
    error("E907", "attribute_too_many_arguments"),
    error("E909", "attribute_bad_alignment"),
    DiagnosticDescriptor {
        id: "E914",
        name: "deprecated_attribute_spelling",
        default_severity: Severity::Warning,
        category: Category::Style,
    },
];

pub struct AttributeValidationAnalyzer;

impl Describe for AttributeValidationAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::AttributeValidation
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl CompilationCheck for AttributeValidationAnalyzer {
    fn check(&self, cx: &CompilationContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();
        walk(cx, cx.root, &mut diags);
        diags
    }
}

fn walk(cx: &CompilationContext<'_>, entity: Entity, diags: &mut Vec<AnalyzeDiagnostic>) {
    if cx.query.has::<Attributes>(entity) {
        diags.extend(attrs_of(cx.query, entity).errors.iter().map(to_diagnostic));
    }
    for &child in cx.query.children_of(entity) {
        walk(cx, child, diags);
    }
}

fn to_diagnostic(err: &AttrError) -> AnalyzeDiagnostic {
    let descriptor = DESCRIPTORS
        .iter()
        .find(|d| d.id == err.code)
        .unwrap_or_else(|| panic!("attribute error code {} has no descriptor", err.code));
    let mut labels = vec![DiagLabel {
        span: err.span.clone(),
        message: err.label.clone(),
        is_primary: true,
    }];
    if let Some((span, message)) = &err.secondary {
        labels.push(DiagLabel {
            span: span.clone(),
            message: message.clone(),
            is_primary: false,
        });
    }
    AnalyzeDiagnostic {
        descriptor_id: descriptor.id,
        severity: if err.warning {
            Severity::Warning
        } else {
            descriptor.default_severity
        },
        message: err.message.clone(),
        labels,
        notes: err.notes.clone(),
    }
}
