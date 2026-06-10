//! # Visibility Consistency Analyzer
//!
//! Ensures that public APIs don't expose less-visible types:
//! - Public functions can't have private/internal/fileprivate parameter types
//! - Public functions can't have less-visible return types
//! - Public type aliases can't alias less-visible types
//! - Public fields can't have less-visible types
//!
//! Methods declared inside a public protocol are treated as implicitly public,
//! so the parameter/return checks apply to them as well.
//!
//! ## Diagnostics
//!
//! ### E430 -- `return_type_less_visible` (Error, Correctness)
//!
//! **Message:** "return type of '{name}' is less visible than the function"
//!
//! **Labels:**
//! - Primary: the function declaration
//!   - Span source: `util::entity_span` on the function entity
//!   - Message: "return type is less visible than function"
//!
//! **Notes:**
//! - "function is public but return type is {private|fileprivate|internal}"
//!
//! ### E431 -- `parameter_type_less_visible` (Error, Correctness)
//!
//! **Message:** "parameter type in '{name}' is less visible than the function"
//!
//! **Labels:**
//! - Primary: the function declaration
//!   - Span source: `util::entity_span` on the function entity
//!   - Message: "parameter type is less visible than function"
//!
//! **Notes:**
//! - "function is public but parameter type is {private|fileprivate|internal}"
//!
//! ### E432 -- `aliased_type_less_visible` (Error, Correctness)
//!
//! **Message:** "aliased type in '{name}' is less visible than the type alias"
//!
//! **Labels:**
//! - Primary: the type alias declaration
//!   - Span source: `util::entity_span` on the type alias entity
//!   - Message: "aliased type is less visible than alias"
//!
//! **Notes:**
//! - "type alias is public but aliased type is {private|fileprivate|internal}"
//!
//! ### E433 -- `field_type_less_visible` (Error, Correctness)
//!
//! **Message:** "field '{name}' has type less visible than the field"
//!
//! **Labels:**
//! - Primary: the field declaration
//!   - Span source: `util::entity_span` on the field entity
//!   - Message: "field type is less visible than field"
//!
//! **Notes:**
//! - "field is public but field type is {private|fileprivate|internal}"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast::AstType;
use kestrel_ast_builder::{Callable, NodeKind, TypeAnnotation, Vis};
use kestrel_name_res::{ResolveTypePath, TypeResolution};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E430",
        name: "return_type_less_visible",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E431",
        name: "parameter_type_less_visible",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E432",
        name: "aliased_type_less_visible",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E433",
        name: "field_type_less_visible",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct VisibilityAnalyzer;

impl Describe for VisibilityAnalyzer {
    fn id(&self) -> &'static str {
        "visibility"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for VisibilityAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[
            NodeKind::Function,
            NodeKind::Initializer,
            NodeKind::TypeAlias,
            NodeKind::Field,
        ]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        if !is_effectively_public(cx) {
            return vec![];
        }
        // Computed properties have a Callable component; their "type" is the
        // getter/setter signature, which is checked elsewhere.
        if cx.kind == NodeKind::Field && cx.query.has::<Callable>(cx.entity) {
            return vec![];
        }

        match cx.kind {
            NodeKind::Function | NodeKind::Initializer => check_callable(cx),
            NodeKind::TypeAlias => check_type_alias(cx),
            NodeKind::Field => check_field(cx),
            _ => vec![],
        }
    }
}

// ===== Per-kind checks =====

fn check_callable(cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
    let mut diags = Vec::new();
    let name = util::entity_name(cx.query, cx.entity);
    let span = util::entity_span(cx.query, cx.entity);

    if let Some(callable) = cx.query.get::<Callable>(cx.entity) {
        for param in &callable.params {
            let Some(ty) = &param.ty else { continue };
            if let Some(hit) = first_less_visible(cx, ty) {
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[1].id,
                    severity: DESCRIPTORS[1].default_severity,
                    message: format!(
                        "parameter type in '{}' is less visible than the function",
                        name
                    ),
                    labels: vec![DiagLabel {
                        span: span.clone(),
                        message: "parameter type is less visible than function".into(),
                        is_primary: true,
                    }],
                    notes: vec![format!(
                        "function is public but parameter type is {}",
                        vis_label(&hit)
                    )],
                });
                break; // one parameter diagnostic per function is enough
            }
        }
    }

    if let Some(TypeAnnotation(ret_ty)) = cx.query.get::<TypeAnnotation>(cx.entity)
        && let Some(hit) = first_less_visible(cx, ret_ty)
    {
        diags.push(AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: format!(
                "return type of '{}' is less visible than the function",
                name
            ),
            labels: vec![DiagLabel {
                span,
                message: "return type is less visible than function".into(),
                is_primary: true,
            }],
            notes: vec![format!(
                "function is public but return type is {}",
                vis_label(&hit)
            )],
        });
    }

    diags
}

fn check_type_alias(cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
    let Some(TypeAnnotation(ty)) = cx.query.get::<TypeAnnotation>(cx.entity) else {
        return vec![];
    };
    let Some(hit) = first_less_visible(cx, ty) else {
        return vec![];
    };
    let name = util::entity_name(cx.query, cx.entity);
    let span = util::entity_span(cx.query, cx.entity);
    vec![AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[2].id,
        severity: DESCRIPTORS[2].default_severity,
        message: format!(
            "aliased type in '{}' is less visible than the type alias",
            name
        ),
        labels: vec![DiagLabel {
            span,
            message: "aliased type is less visible than alias".into(),
            is_primary: true,
        }],
        notes: vec![format!(
            "type alias is public but aliased type is {}",
            vis_label(&hit)
        )],
    }]
}

fn check_field(cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
    let Some(TypeAnnotation(ty)) = cx.query.get::<TypeAnnotation>(cx.entity) else {
        return vec![];
    };
    let Some(hit) = first_less_visible(cx, ty) else {
        return vec![];
    };
    let name = util::entity_name(cx.query, cx.entity);
    let span = util::entity_span(cx.query, cx.entity);
    vec![AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[3].id,
        severity: DESCRIPTORS[3].default_severity,
        message: format!("field '{}' has type less visible than the field", name),
        labels: vec![DiagLabel {
            span,
            message: "field type is less visible than field".into(),
            is_primary: true,
        }],
        notes: vec![format!(
            "field is public but field type is {}",
            vis_label(&hit)
        )],
    }]
}

// ===== Helpers =====

/// True when this declaration sits on the package's public surface — either
/// itself marked `public`, or a method/init inside a public protocol (which
/// is implicitly public).
fn is_effectively_public(cx: &DeclContext<'_>) -> bool {
    if matches!(cx.query.get::<Vis>(cx.entity), Some(Vis::Public)) {
        return true;
    }
    if !matches!(cx.kind, NodeKind::Function | NodeKind::Initializer) {
        return false;
    }
    let Some(parent) = cx.query.parent_of(cx.entity) else {
        return false;
    };
    matches!(cx.query.get::<NodeKind>(parent), Some(NodeKind::Protocol))
        && matches!(cx.query.get::<Vis>(parent), Some(Vis::Public))
}

/// Walk an `AstType` and return the visibility of the first less-than-public
/// type entity referenced. Recurses into generic args, tuple elements,
/// function param/return types, optional/array/dictionary inner types, and
/// throws-result ok/err.
fn first_less_visible(cx: &DeclContext<'_>, ty: &AstType) -> Option<Vis> {
    match ty {
        AstType::Named { segments, .. } => {
            // Resolve the path against the declaration's enclosing scope. We
            // intentionally ignore "is this visible from here?" — we're not
            // checking access; we're checking whether the *referenced* entity
            // is less-than-public.
            let context = cx.query.parent_of(cx.entity).unwrap_or(cx.root);
            let segs: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
            if let TypeResolution::Found(e) = cx.query.query(ResolveTypePath {
                segments: segs,
                context,
                root: cx.root,
            }) && let Some(v) = cx.query.get::<Vis>(e)
                && !matches!(*v, Vis::Public)
            {
                return Some(v.clone());
            }
            for seg in segments {
                for arg in &seg.type_args {
                    if let Some(hit) = first_less_visible(cx, arg) {
                        return Some(hit);
                    }
                }
            }
            None
        },
        AstType::Tuple(elems, _) => elems.iter().find_map(|e| first_less_visible(cx, e)),
        AstType::Function {
            params,
            return_type,
            ..
        } => params
            .iter()
            .find_map(|p| first_less_visible(cx, p))
            .or_else(|| first_less_visible(cx, return_type)),
        AstType::Array(inner, _) | AstType::Optional(inner, _) => first_less_visible(cx, inner),
        AstType::Dictionary(k, v, _) => {
            first_less_visible(cx, k).or_else(|| first_less_visible(cx, v))
        },
        AstType::Result { ok, err, .. } => {
            first_less_visible(cx, ok).or_else(|| first_less_visible(cx, err))
        },
        AstType::Some { bounds, .. } => bounds.iter().find_map(|b| first_less_visible(cx, b)),
        AstType::Ref { inner, .. } => first_less_visible(cx, inner),
        AstType::Unit(_) | AstType::Never(_) | AstType::Inferred(_) => None,
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
