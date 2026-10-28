//! # Protocol Inheritance Cycle Analyzer
//!
//! Detects circular protocol-inheritance chains. A protocol that (transitively)
//! inherits from itself has no well-defined method table:
//!
//! ```text
//! protocol Foo: Foo {}          // self-cycle
//!
//! protocol A: B {}              // two-way cycle
//! protocol B: A {}
//!
//! protocol A: B {}              // three-way cycle
//! protocol B: C {}
//! protocol C: A {}
//! ```
//!
//! The walk follows every positive conformance on each protocol. Negative
//! conformances (`T: not Protocol`) don't contribute to the inheritance
//! graph. We stop as soon as we re-enter a protocol already on the DFS
//! stack.
//!
//! ## Diagnostics
//!
//! ### E459 -- `circular_protocol_inheritance` (Error, Correctness)
//!
//! **Message:** "circular protocol inheritance: '{A}' -> ... -> '{A}'"
//!
//! **Labels:**
//! - Primary: the origin protocol (DFS root where the cycle was found)
//!   - Span source: `util::entity_span` on the protocol entity
//!   - Message: "cycle begins here"
//!
//! **Notes:** "protocols cannot inherit from themselves, directly or indirectly"

use std::collections::HashSet;

use crate::compilation::cycle_util::{Cycle, CycleDetector};
use crate::context::CompilationContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, CompilationCheck, Describe};
use crate::util;
use kestrel_ast::AstType;
use kestrel_ast_builder::{ConformanceItem, Conformances, NodeKind};
use kestrel_hecs::Entity;
use kestrel_name_res::{ResolveTypePath, TypeResolution};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E459",
    name: "circular_protocol_inheritance",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ProtocolCycleAnalyzer;

impl Describe for ProtocolCycleAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::ProtocolCycles
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl CompilationCheck for ProtocolCycleAnalyzer {
    fn check(&self, cx: &CompilationContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();
        let mut checked: HashSet<Entity> = HashSet::new();

        let mut protocols = Vec::new();
        collect_protocols(cx, cx.root, &mut protocols);

        for origin in protocols {
            if checked.contains(&origin) {
                continue;
            }
            if let Some(cycle) = detect_cycle_from(cx, origin) {
                emit_cycle_diagnostic(cx, &cycle, &mut diags);
                for &e in &cycle.participants {
                    checked.insert(e);
                }
            }
            checked.insert(origin);
        }

        diags
    }
}

fn collect_protocols(cx: &CompilationContext<'_>, entity: Entity, out: &mut Vec<Entity>) {
    if cx.query.get::<NodeKind>(entity) == Some(&NodeKind::Protocol) {
        out.push(entity);
    }
    for &child in cx.query.children_of(entity) {
        collect_protocols(cx, child, out);
    }
}

fn detect_cycle_from(cx: &CompilationContext<'_>, origin: Entity) -> Option<Cycle> {
    let mut detector = CycleDetector::new();
    enter_protocol(cx, origin, &mut detector).err()
}

/// Push `protocol` onto the DFS stack and recurse through each positive
/// conformance that resolves to another protocol.
fn enter_protocol(
    cx: &CompilationContext<'_>,
    protocol: Entity,
    detector: &mut CycleDetector,
) -> Result<(), Cycle> {
    detector.enter(protocol)?;
    if let Some(conformances) = cx.query.get::<Conformances>(protocol) {
        for item in &conformances.0 {
            let ConformanceItem::Positive(ast_type, _) = item else {
                continue;
            };
            if let Some(parent) = resolve_protocol(cx, ast_type, protocol) {
                enter_protocol(cx, parent, detector)?;
            }
        }
    }
    detector.exit(protocol);
    Ok(())
}

/// Resolve a conformance item's AstType to a protocol entity, or None if the
/// reference points to something that isn't a protocol (or fails to resolve).
fn resolve_protocol(
    cx: &CompilationContext<'_>,
    ast_type: &AstType,
    context: Entity,
) -> Option<Entity> {
    let AstType::Named { segments, .. } = ast_type else {
        return None;
    };
    let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    let TypeResolution::Found(entity) = cx.query.query(ResolveTypePath {
        segments: seg_names,
        context,
        root: cx.root,
    }) else {
        return None;
    };
    if cx.query.get::<NodeKind>(entity) == Some(&NodeKind::Protocol) {
        Some(entity)
    } else {
        None
    }
}

fn emit_cycle_diagnostic(
    cx: &CompilationContext<'_>,
    cycle: &Cycle,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let participants = &cycle.participants;
    if participants.is_empty() {
        return;
    }
    let origin = participants[0];
    let origin_name = util::entity_name(cx.query, origin);
    let origin_span = util::entity_span(cx.query, origin);

    let mut names: Vec<String> = participants
        .iter()
        .map(|&e| util::entity_name(cx.query, e))
        .collect();
    names.push(origin_name.clone());
    let cycle_str = names.join(" -> ");

    diags.push(AnalyzeDiagnostic {
        descriptor_id: "E459",
        severity: Severity::Error,
        message: format!("circular protocol inheritance: {}", cycle_str),
        labels: vec![DiagLabel {
            span: origin_span,
            message: "cycle begins here".into(),
            is_primary: true,
        }],
        notes: vec!["protocols cannot inherit from themselves, directly or indirectly".into()],
    });
}
