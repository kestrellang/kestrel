//! # Unknown Attribute Analyzer
//!
//! Walks every entity in the compilation and warns on any `@name` attribute
//! whose name is not in the compiler's known-attribute set. Runs as a
//! `CompilationCheck` (not a `DeclCheck`) so the filter is the presence of an
//! `Attributes` component, not a `NodeKind` enumeration — any entity that
//! carries attributes is covered uniformly.
//!
//! ## Diagnostics
//!
//! ### E461 — `unknown_attribute` (Warning, Correctness)
//!
//! **Message:** "unknown attribute '{name}'"
//!
//! **Labels:**
//! - Primary: the `@name(...)` attribute site
//!   - Span source: `AstAttribute::span` populated by the AST builder
//!   - Message: "unknown attribute"
//!
//! **Notes:** (none)

use crate::context::CompilationContext;
use crate::diagnostic::*;
use crate::traits::{CompilationCheck, Describe};
use kestrel_ast_builder::Attributes;
use kestrel_hecs::Entity;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E461",
    name: "unknown_attribute",
    default_severity: Severity::Warning,
    category: Category::Correctness,
}];

/// Attributes the compiler recognizes. Keep in sync with the consumers in
/// `kestrel-mir-lower` (`@extern`, `@fileconstant`), `kestrel-name-res`
/// (`@builtin`), `kestrel-analyze/decl/extern_ffi_safe.rs` (`@extern`), and
/// `kestrel-compiler-driver` / frontmatter (`@platform`). `@dummy` is a
/// test-only placeholder kept recognized so parser-level attribute tests
/// stay free of semantic noise.
const KNOWN_ATTRIBUTES: &[&str] = &["builtin", "dummy", "extern", "fileconstant", "platform"];

pub struct UnknownAttributeAnalyzer;

impl Describe for UnknownAttributeAnalyzer {
    fn id(&self) -> &'static str {
        "unknown_attribute"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl CompilationCheck for UnknownAttributeAnalyzer {
    fn check(&self, cx: &CompilationContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();
        walk(cx, cx.root, &mut diags);
        diags
    }
}

fn walk(cx: &CompilationContext<'_>, entity: Entity, diags: &mut Vec<AnalyzeDiagnostic>) {
    if let Some(attrs) = cx.query.get::<Attributes>(entity) {
        for attr in &attrs.0 {
            if !KNOWN_ATTRIBUTES.contains(&attr.name.as_str()) {
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message: format!("unknown attribute '{}'", attr.name),
                    labels: vec![DiagLabel {
                        span: attr.span.clone(),
                        message: "unknown attribute".into(),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
            }
        }
    }

    for &child in cx.query.children_of(entity) {
        walk(cx, child, diags);
    }
}
