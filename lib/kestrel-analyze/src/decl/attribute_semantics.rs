//! # Attribute Semantics Analyzer
//!
//! The attribute checks that need more than the attribute's own text and the
//! kind of declaration it is on (those are `AttributeValidation`'s): whether
//! the attribute fits *this* type.
//!
//! - `@nonExhaustive` on an enum that no other module can see.
//! - `@layout(.Packed)` on an enum, or on a struct with an unsized tail.
//! - `@layout(.C)` on an enum with a payload.
//! - A field of a packed struct that is not bitwise `Copyable`: a read copies
//!   the field out of its possibly misaligned slot, so it must be a plain copy.
//!
//! ## Diagnostics
//!
//! ### E910 — `attribute_invalid_for_type` (Error, Correctness)
//!
//! **Messages:**
//! - "'@nonExhaustive' has no effect: '{name}' is not visible outside its module"
//! - "'@layout(.Packed)' cannot be applied to an enum"
//! - "'@layout(.Packed)' cannot be applied to a type with an unsized tail"
//! - "'@layout(.C)' on an enum needs every case to be payload-less; '{Enum.Case}' has a payload"
//!
//! **Labels:**
//! - Primary: the attribute (span from `AttrsOf`)
//! - Secondary (payload case only): the case, "has a payload"
//!
//! ### E911 — `packed_field_not_copyable` (Error, Correctness)
//!
//! **Message:** "field '{field}' of packed struct '{name}' is not Copyable; packed fields are copied on every read"
//!
//! **Labels:**
//! - Primary: the field declaration, "not Copyable"
//! - Secondary: the `@layout(.Packed)` attribute, "packed here"

use kestrel_ast_builder::attrs::{LayoutOption, ParsedAttr, ParsedAttrs, attrs_of};
use kestrel_ast_builder::{Callable, FieldClass, NodeKind, Vis};
use kestrel_hecs::Entity;
use kestrel_hir_lower::LowerTypeAnnotation;
use kestrel_semantics::sizedness::{hir_type_is_sized, last_stored_field_type};
use kestrel_semantics::{CopySemantics, hir_type_copy_semantics};
use kestrel_span::Span;

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E910",
        name: "attribute_invalid_for_type",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E911",
        name: "packed_field_not_copyable",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct AttributeSemanticsAnalyzer;

impl Describe for AttributeSemanticsAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::AttributeSemantics
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for AttributeSemanticsAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Struct, NodeKind::Enum]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let attrs = attrs_of(cx.query, cx.entity);
        let mut diags = Vec::new();
        diags.extend(non_exhaustive_without_effect(cx, &attrs));
        match (cx.query.get::<NodeKind>(cx.entity), attrs.layout()) {
            (Some(NodeKind::Enum), Some(LayoutOption::Packed)) => {
                diags.push(invalid_for_type(
                    layout_span(&attrs),
                    "'@layout(.Packed)' cannot be applied to an enum".into(),
                    "only a struct can be packed",
                ));
            },
            (Some(NodeKind::Enum), Some(LayoutOption::C)) => {
                diags.extend(c_enum_with_payload(cx, &attrs));
            },
            (Some(NodeKind::Struct), Some(LayoutOption::Packed)) => {
                diags.extend(packed_struct(cx, &attrs));
            },
            _ => {},
        }
        diags
    }
}

fn invalid_for_type(span: Span, message: String, label: &str) -> AnalyzeDiagnostic {
    AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[0].id,
        severity: DESCRIPTORS[0].default_severity,
        message,
        labels: vec![DiagLabel {
            span,
            message: label.into(),
            is_primary: true,
        }],
        notes: vec![],
    }
}

/// The span of the `@layout` attribute. Only called once `layout()` is Some.
fn layout_span(attrs: &ParsedAttrs) -> Span {
    attrs
        .span_of(|a| matches!(a, ParsedAttr::Layout(_)))
        .expect("layout() returned Some")
}

fn non_exhaustive_without_effect(
    cx: &DeclContext<'_>,
    attrs: &ParsedAttrs,
) -> Option<AnalyzeDiagnostic> {
    let span = attrs.span_of(|a| matches!(a, ParsedAttr::NonExhaustive))?;
    if cx.query.get::<Vis>(cx.entity) == Some(&Vis::Public) {
        return None;
    }
    let mut diag = invalid_for_type(
        span,
        format!(
            "'@nonExhaustive' has no effect: '{}' is not visible outside its module",
            util::entity_name(cx.query, cx.entity)
        ),
        "has no effect",
    );
    diag.notes
        .push("only a 'public' enum can gain cases that other modules must handle".into());
    Some(diag)
}

/// `@layout(.C)` makes an enum a C `int`, which has no room for a payload.
fn c_enum_with_payload(cx: &DeclContext<'_>, attrs: &ParsedAttrs) -> Option<AnalyzeDiagnostic> {
    let case = util::children_of_kind(cx.query, cx.entity, NodeKind::EnumCase)
        .into_iter()
        .find(|&c| {
            cx.query
                .get::<Callable>(c)
                .is_some_and(|callable| !callable.params.is_empty())
        })?;
    let mut diag = invalid_for_type(
        layout_span(attrs),
        format!(
            "'@layout(.C)' on an enum needs every case to be payload-less; '{}.{}' has a payload",
            util::entity_name(cx.query, cx.entity),
            util::entity_name(cx.query, case)
        ),
        "C enums are plain integers",
    );
    diag.labels.push(DiagLabel {
        span: util::entity_span(cx.query, case),
        message: "has a payload".into(),
        is_primary: false,
    });
    Some(diag)
}

/// A packed struct has a fixed size and copies every field on read: no
/// unsized tail (E910), and only bitwise-Copyable fields (E911).
fn packed_struct(cx: &DeclContext<'_>, attrs: &ParsedAttrs) -> Vec<AnalyzeDiagnostic> {
    let packed_span = layout_span(attrs);
    let mut diags = Vec::new();
    if let Some(tail) = last_stored_field_type(cx.query, cx.entity, cx.root)
        && !hir_type_is_sized(cx.query, &tail, cx.entity, cx.root)
    {
        diags.push(invalid_for_type(
            packed_span.clone(),
            "'@layout(.Packed)' cannot be applied to a type with an unsized tail".into(),
            "packed types have a fixed size",
        ));
    }
    for field in stored_fields(cx) {
        let Some(ty) = cx.query.query(LowerTypeAnnotation {
            entity: field,
            root: cx.root,
        }) else {
            continue;
        };
        if hir_type_copy_semantics(cx.query, &ty, cx.entity, cx.root) == CopySemantics::Copyable {
            continue;
        }
        diags.push(AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[1].id,
            severity: DESCRIPTORS[1].default_severity,
            message: format!(
                "field '{}' of packed struct '{}' is not Copyable; packed fields are copied on every read",
                util::entity_name(cx.query, field),
                util::entity_name(cx.query, cx.entity)
            ),
            labels: vec![
                DiagLabel {
                    span: util::entity_span(cx.query, field),
                    message: "not Copyable".into(),
                    is_primary: true,
                },
                DiagLabel {
                    span: packed_span.clone(),
                    message: "packed here".into(),
                    is_primary: false,
                },
            ],
            notes: vec![],
        });
    }
    diags
}

fn stored_fields(cx: &DeclContext<'_>) -> Vec<Entity> {
    cx.query
        .children_of(cx.entity)
        .iter()
        .copied()
        .filter(|&c| {
            cx.query
                .get::<FieldClass>(c)
                .is_some_and(FieldClass::is_stored_instance)
        })
        .collect()
}
