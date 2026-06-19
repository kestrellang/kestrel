//! # Place-Accessor Analyzer (stage-1.5 references)
//!
//! Validates `ref { … }` / `mutating ref { … }` accessor blocks on
//! subscripts and computed properties: a member has at most ONE read
//! provider (`get` XOR `ref`) and ONE write provider (`set` XOR
//! `mutating ref`), must have a read provider, and ref accessors are
//! rejected in protocols and protocol extensions (witness ref-returns are
//! out of scope; the stage-1.5 restriction is concrete inherent decls).
//!
//! ## Diagnostics
//!
//! ### E619 -- `duplicate_read_provider` (Error, Correctness)
//! `get { }` and `ref { }` on one member.
//!
//! ### E620 -- `duplicate_write_provider` (Error, Correctness)
//! `set { }` and `mutating ref { }` on one member.
//!
//! ### E621 -- `ref_accessor_in_protocol` (Error, Correctness)
//! A ref accessor on a protocol member or a protocol-extension member.
//!
//! ### E622 -- `accessor_missing_read_provider` (Error, Correctness)
//! An accessor block with a write provider but no `get`/`ref` (set-only
//! or mutating-ref-only members; reads would have no provider).

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Body, Computed, Gettable, NodeKind};
use kestrel_hir_lower::PlaceAccessors;
use kestrel_name_res::ExtensionTargetEntity;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E619",
        name: "duplicate_read_provider",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E620",
        name: "duplicate_write_provider",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E621",
        name: "ref_accessor_in_protocol",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E622",
        name: "accessor_missing_read_provider",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct PlaceAccessorAnalyzer;

impl Describe for PlaceAccessorAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::PlaceAccessor
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for PlaceAccessorAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Subscript, NodeKind::Field]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();
        let accessors = cx.query.query(PlaceAccessors { entity: cx.entity });
        let has_setter_child = cx
            .query
            .children_of(cx.entity)
            .iter()
            .any(|&c| cx.query.get::<NodeKind>(c) == Some(&NodeKind::Setter));
        let has_ref = accessors.is_some_and(|a| a.ref_accessor.is_some());
        let has_mutating_ref = accessors.is_some_and(|a| a.mutating_ref_accessor.is_some());
        // The parent carries the getter BODY when a `get` clause (or the
        // shorthand form) is present; pure-ref members are bodyless.
        let has_getter = cx.query.get::<Body>(cx.entity).is_some();
        let span = util::entity_span(cx.query, cx.entity);

        // E621: ref accessors are rejected in protocols and protocol
        // extensions (the stage-1.5 scope restriction: concrete inherent
        // declarations only).
        if (has_ref || has_mutating_ref) && in_protocol_scope(cx) {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[2].id,
                severity: DESCRIPTORS[2].default_severity,
                message: "`ref` accessors are not allowed in protocols or protocol extensions"
                    .into(),
                labels: vec![DiagLabel {
                    span: span.clone(),
                    message: "declare the accessor on a concrete type".into(),
                    is_primary: true,
                }],
                notes: vec![
                    "witness-dispatched reference returns are outside the stage-1.5 reference \
                     rules; protocol members use `get`/`set`"
                        .into(),
                ],
            });
            // The provider checks below would only cascade.
            return diags;
        }

        // E619: at most one READ provider.
        if has_getter && has_ref {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[0].id,
                severity: DESCRIPTORS[0].default_severity,
                message: "duplicate read provider: this member declares both `get` and `ref`"
                    .into(),
                labels: vec![DiagLabel {
                    span: span.clone(),
                    message: "reads need exactly one provider — keep `get` OR `ref`".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
        }

        // E620: at most one WRITE provider.
        if has_setter_child && has_mutating_ref {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[1].id,
                severity: DESCRIPTORS[1].default_severity,
                message: "duplicate write provider: this member declares both `set` and \
                          `mutating ref`"
                    .into(),
                labels: vec![DiagLabel {
                    span: span.clone(),
                    message: "writes need exactly one provider — keep `set` OR `mutating ref`"
                        .into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
        }

        // E622: a write provider with NO read provider (set-only /
        // mutating-ref-only). Gated on the member actually having an
        // accessor block: a write-provider child (or a Computed field
        // marked Settable-without-Gettable). Protocol requirement forms
        // (`{ get set }`, bodyless, no children) never reach this — the
        // protocol scope returns above never fires for them either, but
        // they have no write-provider CHILD to begin with.
        let has_read_provider = has_getter || has_ref;
        let field_set_only = matches!(cx.query.get::<NodeKind>(cx.entity), Some(NodeKind::Field))
            && cx.query.get::<Computed>(cx.entity).is_some()
            && cx.query.get::<Gettable>(cx.entity).is_none();
        if !has_read_provider && (has_setter_child || has_mutating_ref || field_set_only) {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[3].id,
                severity: DESCRIPTORS[3].default_severity,
                message: "this member has a write provider but no read provider".into(),
                labels: vec![DiagLabel {
                    span,
                    message: "add a `get` or `ref` accessor".into(),
                    is_primary: true,
                }],
                notes: vec!["reads always go through `get` or `ref`; a `mutating ref` or `set` \
                             alone leaves reads unprovided"
                    .into()],
            });
        }

        diags
    }
}

/// Is this member declared in a protocol body or a protocol extension?
fn in_protocol_scope(cx: &DeclContext<'_>) -> bool {
    let Some(parent) = cx.query.parent_of(cx.entity) else {
        return false;
    };
    match cx.query.get::<NodeKind>(parent) {
        Some(NodeKind::Protocol) => true,
        Some(NodeKind::Extension) => cx
            .query
            .query(ExtensionTargetEntity {
                extension: parent,
                root: cx.root,
            })
            .is_some_and(|target| {
                cx.query.get::<NodeKind>(target) == Some(&NodeKind::Protocol)
            }),
        _ => false,
    }
}
