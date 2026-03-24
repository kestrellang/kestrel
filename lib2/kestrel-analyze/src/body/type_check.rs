//! # Type Check Analyzer
//!
//! Surfaces type inference errors as analyzer diagnostics. In lib2, the
//! constraint solver catches most type errors (mismatches, missing members,
//! non-conformance). This analyzer converts those into structured diagnostics.
//!
//! ## Diagnostics
//!
//! ### E100 — `type_mismatch` (Error, Correctness)
//!
//! **Message:** "type mismatch: {detail}"
//!
//! **Labels:**
//! - Primary: the expression that produced the error
//!   - Span source: `InferError.span()` (from the solver)
//!   - Message: (varies by error kind)
//!
//! **Notes:** (none)

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};
use kestrel_type_infer::error::InferError;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E100",
    name: "type_mismatch",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct TypeCheckAnalyzer;

impl Describe for TypeCheckAnalyzer {
    fn id(&self) -> &'static str {
        "type_check"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for TypeCheckAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Convert each inference error into an analyzer diagnostic.
        // The error_details vector has human-readable descriptions at
        // matching indices.
        cx.typed
            .errors
            .iter()
            .zip(cx.typed.error_details.iter())
            .filter_map(|(err, detail)| {
                // Skip HIR-propagated errors — those originate from earlier phases
                // (parse errors, name resolution errors) and would be duplicates.
                if matches!(err, InferError::FromHir { .. }) {
                    return None;
                }

                let (message, label_msg) = format_error(err, detail);

                Some(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message,
                    labels: vec![DiagLabel {
                        span: err.span().clone(),
                        message: label_msg,
                        is_primary: true,
                    }],
                    notes: vec![],
                })
            })
            .collect()
    }
}

/// Build user-facing message and label text from an inference error.
fn format_error(err: &InferError, detail: &str) -> (String, String) {
    match err {
        InferError::TypeMismatch { .. } => (
            format!("type mismatch: {}", detail),
            "incompatible types".into(),
        ),
        InferError::DoesNotConform { .. } => (
            format!("protocol conformance error: {}", detail),
            "type does not conform".into(),
        ),
        InferError::NoMember { name, .. } => (
            format!("no member '{}' found: {}", name, detail),
            format!("'{}' not found", name),
        ),
        InferError::AmbiguousMember { name, .. } => (
            format!("ambiguous member '{}': {}", name, detail),
            "multiple candidates".into(),
        ),
        InferError::MemberNotVisible { name, .. } => (
            format!("member '{}' is not accessible: {}", name, detail),
            "not visible".into(),
        ),
        InferError::NoAssociatedType { name, .. } => (
            format!("no associated type '{}': {}", name, detail),
            format!("'{}' not found", name),
        ),
        InferError::InfiniteType { .. } => (
            "infinite type detected".into(),
            "recursive type".into(),
        ),
        InferError::ImplicitMemberNotFound { name, .. } => (
            format!("implicit member '.{}' not found: {}", name, detail),
            format!("'.{}' not found", name),
        ),
        InferError::ArgCountMismatch { expected, got, .. } => (
            format!("wrong number of arguments: expected {}, got {}", expected, got),
            format!("expected {} argument(s)", expected),
        ),
        InferError::LabelMismatch { expected, got, .. } => {
            let exp = expected.as_deref().unwrap_or("_");
            let g = got.as_deref().unwrap_or("_");
            (
                format!("wrong argument label: expected '{}', got '{}'", exp, g),
                format!("expected '{}'", exp),
            )
        }
        InferError::InstanceMethodAsStatic { name, .. } => (
            format!("instance method '{}' cannot be called on a type", name),
            "not a static method".into(),
        ),
        InferError::TypeParamAsValue { .. } => (
            "type parameter cannot be used as a value".into(),
            "not a value".into(),
        ),
        InferError::FromHir { .. } => unreachable!("filtered above"),
    }
}
