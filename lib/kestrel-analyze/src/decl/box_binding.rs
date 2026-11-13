//! # Box Binding Analyzer
//!
//! Each boxing feature reads one tagged alias in `std.memory.box_bindings`
//! (`@builtin(.ClosureBox) type ClosureBox[T] = Shared[T]`, …) and calls only
//! protocol requirements on the box it names. The protocols a feature needs
//! are [`Builtin::box_binding_bound`]; this analyzer checks them where the
//! alias is declared, so binding a box that cannot do the job fails in std
//! rather than as an ICE at a lowering site (docs/design/shared-box.md).
//!
//! ## Diagnostics
//!
//! ### E470 -- `invalid_box_binding` (Error, Correctness)
//!
//! **Message:** "@builtin(.{feature}) binds '{box}', which does not conform to '{protocol}'"
//! or "@builtin(.{feature}) must name a box type"
//!
//! **Labels:**
//! - Primary: the binding alias
//!   - Span source: `util::entity_span` on the type alias entity (declaration span)
//!   - Message: "box binding declared here"
//!
//! **Notes:**
//! - "'{feature}' needs {protocols} from its box" (the full bound)

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::NodeKind;
use kestrel_hir::Builtin;
use kestrel_name_res::{EntityBuiltin, ResolveBuiltin};
use kestrel_semantics::DeclaresConformanceTo;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E470",
    name: "invalid_box_binding",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct BoxBindingAnalyzer;

impl Describe for BoxBindingAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::BoxBinding
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for BoxBindingAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::TypeAlias]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let Some(feature) = cx.query.query(EntityBuiltin { entity: cx.entity }) else {
            return vec![];
        };
        let Some(bound) = feature.box_binding_bound() else {
            return vec![];
        };
        let resolve = |builtin| {
            cx.query.query(ResolveBuiltin {
                builtin,
                root: cx.root,
            })
        };
        // The binding denotes its alias target, followed through role aliases.
        let Some(target) = resolve(feature) else {
            return vec![report(
                cx,
                format!("@builtin(.{}) must name a box type", feature.name()),
                bound,
                feature,
            )];
        };
        // A missing protocol is a broken std too, but E141-style "is std
        // loaded" errors cover it; only judge protocols that exist.
        let missing = bound.iter().copied().find(|&protocol| {
            resolve(protocol).is_some_and(|protocol| {
                !cx.query.query(DeclaresConformanceTo {
                    entity: target,
                    protocol,
                    root: cx.root,
                })
            })
        });
        let Some(missing) = missing else {
            return vec![];
        };
        vec![report(
            cx,
            format!(
                "@builtin(.{}) binds '{}', which does not conform to '{}'",
                feature.name(),
                util::entity_name(cx.query, target),
                missing.name()
            ),
            bound,
            feature,
        )]
    }
}

/// One E470 on the alias, noting the feature's whole bound.
fn report(
    cx: &DeclContext<'_>,
    message: String,
    bound: &[Builtin],
    feature: Builtin,
) -> AnalyzeDiagnostic {
    let protocols: Vec<String> = bound.iter().map(|p| format!("'{}'", p.name())).collect();
    AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[0].id,
        severity: DESCRIPTORS[0].default_severity,
        message,
        labels: vec![DiagLabel {
            span: util::entity_span(cx.query, cx.entity),
            message: "box binding declared here".into(),
            is_primary: true,
        }],
        notes: vec![format!(
            "'{}' needs {} from its box",
            feature.name(),
            protocols.join(" and ")
        )],
    }
}
