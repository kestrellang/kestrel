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
use kestrel_ast_builder::Vis;
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
            format!(
                "protocol conformance error: does not satisfy constraint: {}",
                detail
            ),
            "type does not conform".into(),
        ),
        InferError::NoMember { name, .. } => (
            // `detail` already comes formatted as "no method 'X' on type 'Y'"
            // (see kestrel_type_infer::result::describe_error), so we surface
            // it directly as the diagnostic message.
            detail.to_string(),
            format!("'{}' not found", name),
        ),
        InferError::AmbiguousMember { name, .. } => (
            format!("ambiguous member '{}': {}", name, detail),
            "multiple candidates".into(),
        ),
        InferError::MemberNotVisible {
            name, visibility, ..
        } => {
            let vis = vis_label(visibility);
            (
                format!(
                    "member '{}' is {} and not accessible from this scope",
                    name, vis
                ),
                format!("{} member", vis),
            )
        },
        InferError::NoAssociatedType { name, .. } => (
            format!("no associated type '{}': {}", name, detail),
            format!("'{}' not found", name),
        ),
        InferError::InfiniteType { .. } => {
            ("infinite type detected".into(), "recursive type".into())
        },
        InferError::ImplicitMemberNotFound { name, .. } => (
            format!("implicit member '.{}' not found: {}", name, detail),
            format!("'.{}' not found", name),
        ),
        InferError::ArgCountMismatch { expected, got, .. } => (
            format!(
                "wrong number of arguments: expected {}, got {}",
                expected, got
            ),
            format!("expected {} argument(s)", expected),
        ),
        InferError::LabelMismatch { expected, got, .. } => {
            let exp = expected.as_deref().unwrap_or("_");
            let g = got.as_deref().unwrap_or("_");
            (
                format!("wrong argument label: expected '{}', got '{}'", exp, g),
                format!("expected '{}'", exp),
            )
        },
        InferError::InstanceMethodAsStatic { name, .. } => (
            format!("instance method '{}' cannot be called on a type", name),
            "not a static method".into(),
        ),
        InferError::TypeParamAsValue { .. } => (
            "type parameter cannot be used as a value".into(),
            "not a value".into(),
        ),
        InferError::TypeArgCountMismatch { expected, got, .. } => {
            if *got < *expected {
                (
                    format!("too few type arguments: expected {expected}, got {got}"),
                    format!("expected {expected} type argument(s)"),
                )
            } else {
                (
                    format!("too many type arguments: expected {expected}, got {got}"),
                    format!("expected {expected} type argument(s)"),
                )
            }
        },
        InferError::NoMatchingOverload { name, .. } => (
            format!("no matching overload for '{name}'"),
            format!("no matching overload for '{name}'"),
        ),
        InferError::MemberwiseInitArity {
            struct_name,
            expected,
            got,
            ..
        } => (
            format!(
                "struct '{struct_name}' has {expected} field(s), but {got} argument(s) were provided"
            ),
            format!("expected {expected} argument(s)"),
        ),
        InferError::MemberwiseInitLabel {
            struct_name,
            expected,
            got,
            ..
        } => {
            let got_desc = got
                .as_deref()
                .map(|s| format!("'{}'", s))
                .unwrap_or_else(|| "unlabeled".into());
            (
                format!(
                    "argument for struct '{struct_name}' has {got_desc} label, but expected '{expected}'"
                ),
                format!("expected label '{expected}'"),
            )
        },
        InferError::ItWrongArity { expected, .. } => (
            format!("implicit 'it' parameter used in {expected}-parameter context"),
            "'it' requires exactly 1 parameter".into(),
        ),
        InferError::LiteralNotAccepted { .. } => (
            format!("does not conform to protocol: {}", detail),
            "type does not accept this literal".into(),
        ),
        InferError::UnresolvedTypeParam { .. } => {
            ("cannot infer type parameter".into(), detail.to_string())
        },
        InferError::CannotInferType { .. } => (
            "could not infer type".into(),
            "add a type annotation".into(),
        ),
        InferError::TupleIndexOnNonTuple { index, .. } => (
            format!("cannot index into non-tuple type: {}", detail),
            format!("'.{}' requires a tuple receiver", index),
        ),
        InferError::TupleIndexOutOfBounds { arity, index, .. } => (
            format!(
                "tuple index {index} out of bounds for {arity}-element tuple"
            ),
            format!("valid indices are 0..{}", arity.saturating_sub(1)),
        ),
        InferError::MemberAccessOnPrimitive { name, .. } => (
            format!("cannot access member on type: {}", detail),
            format!("'{}' not available", name),
        ),
        InferError::PrimitiveMethodNotCalled { method, .. } => (
            detail.to_string(),
            format!("add () to call '{}'", method),
        ),
        InferError::FromHir { .. } => unreachable!("filtered above"),
    }
}

fn vis_label(v: &Vis) -> &'static str {
    match v {
        Vis::Public => "public",
        Vis::Internal => "internal",
        Vis::Fileprivate => "fileprivate",
        Vis::Private => "private",
    }
}
