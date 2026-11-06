//! # Lazy Property Analyzer
//!
//! Validates `lazy var x: T = expr`.
//!
//! ## Diagnostics
//!
//! ### E639 -- `invalid_lazy` (Error, Correctness)
//! `lazy` anywhere but an instance `var` of a concrete type with a type
//! annotation and an `= expr` initializer, and no accessors (the field then
//! builds as plain stored). Also a `lazy` property whose type isn't
//! Copyable: its synthesized getter returns a copy of the cached value.
//! The body check reports it on the getter, in place of the move checker's
//! generic use-after-move on the cache.

use crate::context::{BodyContext, DeclContext};
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, BodyCheck, DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{
    Computed, DeclaredAccessors, Failable, FieldClass, FieldMutability, Lazy, NodeKind,
    TypeAnnotation, Valued,
};
use kestrel_semantics::CopySemantics;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E639",
    name: "invalid_lazy",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

static BORROWED: &[&DiagnosticDescriptor] = &[&DESCRIPTORS[0]];

pub struct LazyAnalyzer;

impl Describe for LazyAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::Lazy
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for LazyAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Field]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // A malformed `lazy` builds as a plain stored field; say why.
        match cx.query.get::<Lazy>(cx.entity) {
            Some(lazy) if lazy.cache.is_none() => check_lazy(cx).into_iter().collect(),
            _ => Vec::new(),
        }
    }
}

/// Borrows E639 for a well-formed `lazy` whose type isn't Copyable, seen
/// from its getter body (where the value's type is resolved).
pub struct LazyBodyAnalyzer;

impl Describe for LazyBodyAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::LazyBody
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        &[]
    }
    fn borrowed_descriptors(&self) -> &'static [&'static DiagnosticDescriptor] {
        BORROWED
    }
}

impl BodyCheck for LazyBodyAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        if !is_non_copyable_lazy_getter(cx) {
            return Vec::new();
        }
        vec![AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: "a `lazy` property's type must be Copyable".into(),
            labels: vec![DiagLabel {
                span: util::entity_span(cx.query, cx.entity),
                message: "declared `lazy`".into(),
                is_primary: true,
            }],
            notes: vec![
                "each read returns a copy of the cached value; store an `Optional` and fill it \
                 in a `mutating` method instead"
                    .into(),
            ],
        }]
    }
}

/// Is this body the getter of a well-formed `lazy` property whose type
/// isn't Copyable? The move checker skips such a body: this analyzer
/// reports it instead.
pub(crate) fn is_non_copyable_lazy_getter(cx: &BodyContext<'_>) -> bool {
    let well_formed = cx
        .query
        .get::<Lazy>(cx.entity)
        .is_some_and(|l| l.cache.is_some());
    if !well_formed {
        return false;
    }
    let Some(ty) = cx.hir.tail_expr.and_then(|t| cx.typed.expr_types.get(&t)) else {
        return false;
    };
    crate::body::move_tracking::ty_copy_semantics(cx, ty) == CopySemantics::NotCopyable
}

/// E639: why a `lazy` field could not be built (its `Lazy` has no cache).
fn check_lazy(cx: &DeclContext<'_>) -> Option<AnalyzeDiagnostic> {
    let q = cx.query;
    let nominal = q.get::<FieldClass>(cx.entity).is_some_and(|c| {
        !c.is_static && matches!(c.owner, kestrel_ast_builder::FieldOwner::Nominal)
    });
    let (message, note) = if q.get::<FieldMutability>(cx.entity) == Some(&FieldMutability::Let) {
        (
            "a `lazy` property must be a `var`",
            "its cache is written on first read",
        )
    } else if !nominal {
        (
            "`lazy` is only supported on instance properties of a struct or enum",
            "static and global properties, protocol members and extensions can't be `lazy`",
        )
    } else if q.has::<DeclaredAccessors>(cx.entity) || q.has::<Computed>(cx.entity) {
        (
            "a `lazy` property can't have accessors",
            "`lazy` already supplies the property's get and set",
        )
    } else if q.has::<Failable>(cx.entity) {
        (
            "a `lazy` property can't be failable",
            "declare it `lazy var name: T? = expr` instead",
        )
    } else if !q.has::<TypeAnnotation>(cx.entity) {
        (
            "a `lazy` property needs a type annotation",
            "write `lazy var name: T = expr`",
        )
    } else if !q.has::<Valued>(cx.entity) {
        (
            "a `lazy` property needs an initializer",
            "the initializer runs on first read: `lazy var name: T = expr`",
        )
    } else {
        return None;
    };
    Some(AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[0].id,
        severity: DESCRIPTORS[0].default_severity,
        message: message.into(),
        labels: vec![DiagLabel {
            span: util::entity_span(q, cx.entity),
            message: "declared `lazy`".into(),
            is_primary: true,
        }],
        notes: vec![note.into()],
    })
}
