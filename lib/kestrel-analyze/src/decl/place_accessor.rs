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
//!
//! ### E636 -- `modifying_accessor_without_receiver` (Error, Correctness)
//! `modifying get` / `modifying ref` on a static member or a global: there
//! is no `self` for the accessor to modify.
//!
//! ### E637 -- `invalid_failable_member` (Error, Correctness)
//! A failable member (`var first?: T`, `subscript(...)? -> T`) that is
//! stored (no read accessor) or declares `set`: a place that may not exist
//! can't be assigned. It may declare a `mutating ref` (`Optional[&mutating
//! T]`), written through with a `&mutating` payload binder.
//!
//! Observers (E638) and `lazy` (E639) are their own analyzers
//! (`observer.rs`, `lazy.rs`); this one skips members they own.

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{
    Computed, DeclaredAccessors, Failable, Gettable, Lazy, NodeKind, Static, Valued,
};
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
    DiagnosticDescriptor {
        id: "E636",
        name: "modifying_accessor_without_receiver",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E637",
        name: "invalid_failable_member",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

/// Look a descriptor up by its E-code.
fn descriptor(id: &str) -> &'static DiagnosticDescriptor {
    DESCRIPTORS
        .iter()
        .find(|d| d.id == id)
        .expect("descriptor id must be declared in this analyzer's DESCRIPTORS")
}

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
        let declared = cx
            .query
            .get::<DeclaredAccessors>(cx.entity)
            .copied()
            .unwrap_or_default();
        // A malformed `lazy` (a plain stored field) and observers (a
        // stored property's synthesized setter) are checked by their own
        // analyzers; the provider checks below don't apply to them.
        let malformed_lazy = cx
            .query
            .get::<Lazy>(cx.entity)
            .is_some_and(|l| l.cache.is_none());
        if malformed_lazy || declared.any_observer() {
            return Vec::new();
        }
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
        let has_getter = cx.query.get::<Valued>(cx.entity).is_some();
        let span = util::entity_span(cx.query, cx.entity);

        // E621: ref accessors are rejected in protocols and protocol
        // extensions (the stage-1.5 scope restriction: concrete inherent
        // declarations only).
        if (has_ref || has_mutating_ref || declared.any_modifying()) && in_protocol_scope(cx) {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: descriptor("E621").id,
                severity: descriptor("E621").default_severity,
                message: "`ref` and `modifying` accessors are not allowed in protocols or \
                          protocol extensions"
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

        // E619: at most one READ provider (`get`, `modifying get`, `ref`,
        // `modifying ref`). Counted from the clauses as written: `get` and
        // `modifying get` share the member's body slot, so the built
        // entities alone can't show the duplicate.
        if declared.read_providers() > 1 || (has_getter && has_ref) {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: descriptor("E619").id,
                severity: descriptor("E619").default_severity,
                message: "duplicate read provider: this member declares more than one of `get`, \
                          `modifying get`, `ref` and `modifying ref`"
                    .into(),
                labels: vec![DiagLabel {
                    span: span.clone(),
                    message: "reads need exactly one provider".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
        }

        // E637: a failable member is a computed place with no `set`. Writes
        // go through a `mutating ref` (`Optional[&mutating T]`), reached by
        // a `&mutating` payload binder.
        if cx.query.has::<Failable>(cx.entity) {
            let problem = if declared.read_providers() == 0 && !has_getter && !has_ref {
                Some((
                    "a failable member needs a read accessor",
                    "declare `get` (returning `T?`) or `ref` (lending `Optional[&T]`)",
                ))
            } else if declared.set || has_setter_child {
                Some((
                    "a failable place cannot have a `set` accessor",
                    "assignment to a place that may not exist is rejected (E214)",
                ))
            } else {
                None
            };
            if let Some((message, note)) = problem {
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: descriptor("E637").id,
                    severity: descriptor("E637").default_severity,
                    message: message.into(),
                    labels: vec![DiagLabel {
                        span: span.clone(),
                        message: "declared failable with `?`".into(),
                        is_primary: true,
                    }],
                    notes: vec![note.into()],
                });
            }
        }

        // E636: `modifying` means the accessor changes `self`, so it needs a
        // receiver — static members and module-level globals have none.
        if declared.any_modifying() && !has_instance_receiver(cx) {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: descriptor("E636").id,
                severity: descriptor("E636").default_severity,
                message: "`modifying` accessors need an instance receiver".into(),
                labels: vec![DiagLabel {
                    span: span.clone(),
                    message: "this member is static or global, so there is no `self` to modify"
                        .into(),
                    is_primary: true,
                }],
                notes: vec!["use a plain `get` / `ref`".into()],
            });
        }

        // E620: at most one WRITE provider.
        if has_setter_child && has_mutating_ref {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: descriptor("E620").id,
                severity: descriptor("E620").default_severity,
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
        let has_read_provider = has_getter || has_ref || declared.read_providers() > 0;
        let field_set_only = matches!(cx.query.get::<NodeKind>(cx.entity), Some(NodeKind::Field))
            && cx.query.get::<Computed>(cx.entity).is_some()
            && cx.query.get::<Gettable>(cx.entity).is_none();
        if !has_read_provider && (has_setter_child || has_mutating_ref || field_set_only) {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: descriptor("E622").id,
                severity: descriptor("E622").default_severity,
                message: "this member has a write provider but no read provider".into(),
                labels: vec![DiagLabel {
                    span,
                    message: "add a `get` or `ref` accessor".into(),
                    is_primary: true,
                }],
                notes: vec![
                    "reads always go through `get` or `ref`; a `mutating ref` or `set` \
                             alone leaves reads unprovided"
                        .into(),
                ],
            });
        }

        diags
    }
}

/// Does this member have a `self`: an instance member of a type scope.
pub(crate) fn has_instance_receiver(cx: &DeclContext<'_>) -> bool {
    !cx.query.has::<Static>(cx.entity)
        && cx
            .query
            .parent_of(cx.entity)
            .and_then(|p| cx.query.get::<NodeKind>(p))
            .is_some_and(NodeKind::is_type_scope)
}

/// Is this member declared in a protocol body or a protocol extension?
pub(crate) fn in_protocol_scope(cx: &DeclContext<'_>) -> bool {
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
            .is_some_and(|target| cx.query.get::<NodeKind>(target) == Some(&NodeKind::Protocol)),
        _ => false,
    }
}
