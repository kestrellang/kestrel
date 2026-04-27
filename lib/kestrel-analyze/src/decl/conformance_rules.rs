//! # Conformance Rules Analyzer
//!
//! Structural rules about which conformances are legal on a single
//! declaration. Runs against the resolved conformance facts from
//! `kestrel-semantics` so positive/negative protocol refinement is
//! handled uniformly regardless of whether the relevant protocol is
//! builtin, user-defined, or transitively inherited.
//!
//! ## Diagnostics
//!
//! ### E422 -- `disallowed_enum_conformance` (Error, Correctness)
//!
//! **Message:** "enum '{enum_name}' cannot conform to protocol '{protocol_name}'"
//!
//! **Labels:**
//! - Primary: the enum declaration
//!   - Span source: `util::entity_span` on the enum entity
//!   - Message: "enums cannot conform to this protocol"
//!
//! **Notes:**
//! - "'{protocol_name}' only allows struct conformance"
//!
//! ### E423 -- `conflicting_copyable_opt_out` (Error, Correctness)
//!
//! **Message:** "cannot conform to `{protocol_name}` and opt out of `Copyable`"
//!
//! **Labels:**
//! - Primary: the positive conformance entry
//!   - Span source: the AST span of the positive conformance type
//!   - Message: "this conformance requires `Copyable`"
//! - Secondary: the negative conformance entry
//!   - Span source: the AST span of the `not Copyable` type
//!   - Message: "this opts out of `Copyable`"
//!
//! **Notes:**
//! - "`{protocol_name}` refines `Copyable`; a type cannot do both"
//!
//! ### E424 -- `negative_conformance_requires_language_feature` (Error, Correctness)
//!
//! **Message:** "'{name}' is not a language feature protocol"
//!
//! **Labels:**
//! - Primary: the `not P` entry
//!   - Span source: the AST span of the negative conformance type
//!   - Message: "negative conformance is not allowed here"
//!
//! **Notes:**
//! - "`not` is only legal on builtin protocols with implicit conformance (e.g. `Copyable`)"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::NodeKind;
use kestrel_hir::Builtin;
use kestrel_hir::builtin::BuiltinKind;
use kestrel_name_res::{EntityBuiltin, ResolveBuiltin};
use kestrel_semantics::{ProtocolAllowsNegativeConformance, ProtocolRefines, ResolvedConformances};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E422",
        name: "disallowed_enum_conformance",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E423",
        name: "conflicting_copyable_opt_out",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E424",
        name: "negative_conformance_requires_language_feature",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct ConformanceRulesAnalyzer;

impl Describe for ConformanceRulesAnalyzer {
    fn id(&self) -> &'static str {
        "conformance_rules"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for ConformanceRulesAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Struct, NodeKind::Enum]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let set = cx.query.query(ResolvedConformances {
            entity: cx.entity,
            root: cx.root,
        });
        if set.items.is_empty() {
            return vec![];
        }

        let mut diags = Vec::new();
        check_disallowed_enum(cx, &set, &mut diags);
        check_copyable_conflict(cx, &set, &mut diags);
        check_negative_requires_builtin(cx, &set, &mut diags);
        diags
    }
}

fn check_disallowed_enum(
    cx: &DeclContext<'_>,
    set: &kestrel_semantics::ResolvedConformanceSet,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    if cx.kind != NodeKind::Enum {
        return;
    }
    let enum_name = util::entity_name(cx.query, cx.entity);
    let span = util::entity_span(cx.query, cx.entity);

    for item in set.positives() {
        let Some(proto_entity) = item.protocol() else {
            continue;
        };
        let Some(builtin) = cx.query.query(EntityBuiltin {
            entity: proto_entity,
        }) else {
            continue;
        };
        let BuiltinKind::Protocol {
            disallow_enum_conformance: true,
            ..
        } = builtin.kind()
        else {
            continue;
        };

        let proto_name = util::entity_name(cx.query, proto_entity);
        diags.push(AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: format!(
                "enum '{}' cannot conform to protocol '{}'",
                enum_name, proto_name
            ),
            labels: vec![DiagLabel {
                span: span.clone(),
                message: "enums cannot conform to this protocol".into(),
                is_primary: true,
            }],
            notes: vec![format!("'{}' only allows struct conformance", proto_name)],
        });
    }
}

fn check_copyable_conflict(
    cx: &DeclContext<'_>,
    set: &kestrel_semantics::ResolvedConformanceSet,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let Some(copyable) = cx.query.query(ResolveBuiltin {
        builtin: Builtin::Copyable,
        root: cx.root,
    }) else {
        return;
    };

    let negated_copyable = set
        .negatives()
        .find(|item| item.protocol() == Some(copyable));
    let Some(neg_item) = negated_copyable else {
        return;
    };

    for pos_item in set.positives() {
        let Some(pos_proto) = pos_item.protocol() else {
            continue;
        };
        let refines = cx.query.query(ProtocolRefines {
            protocol: pos_proto,
            base: copyable,
            root: cx.root,
        });
        if !refines {
            continue;
        }

        let proto_name = util::entity_name(cx.query, pos_proto);
        diags.push(AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[1].id,
            severity: DESCRIPTORS[1].default_severity,
            message: format!(
                "cannot conform to `{}` and opt out of `Copyable`",
                proto_name
            ),
            labels: vec![
                DiagLabel {
                    span: pos_item.span.clone(),
                    message: "this conformance requires `Copyable`".to_string(),
                    is_primary: true,
                },
                DiagLabel {
                    span: neg_item.span.clone(),
                    message: "this opts out of `Copyable`".into(),
                    is_primary: false,
                },
            ],
            notes: vec![format!(
                "`{}` refines `Copyable`; a type cannot do both",
                proto_name
            )],
        });
    }
}

fn check_negative_requires_builtin(
    cx: &DeclContext<'_>,
    set: &kestrel_semantics::ResolvedConformanceSet,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    for item in set.negatives() {
        let Some(protocol) = item.protocol() else {
            continue;
        };
        let allows = cx
            .query
            .query(ProtocolAllowsNegativeConformance { protocol });
        if allows {
            continue;
        }

        let name = util::entity_name(cx.query, protocol);
        diags.push(AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[2].id,
            severity: DESCRIPTORS[2].default_severity,
            message: format!("'{}' is not a language feature protocol", name),
            labels: vec![DiagLabel {
                span: item.span.clone(),
                message: "negative conformance is not allowed here".into(),
                is_primary: true,
            }],
            notes: vec![
                "`not` is only legal on builtin protocols with implicit conformance (e.g. `Copyable`)"
                    .into(),
            ],
        });
    }
}
