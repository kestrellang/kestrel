//! # Type Alias Cycle Analyzer
//!
//! Detects circular type alias dependencies. A cycle occurs when type aliases
//! reference each other, directly or transitively, creating an infinite
//! expansion:
//!
//! ```text
//! type A = B      // direct: A -> B -> A
//! type B = A
//!
//! type X = (Y, Z) // transitive: X -> Y -> X through a tuple
//! type Y = X
//! ```
//!
//! The cycle walk follows type alias references through tuples and function
//! types, but stops at struct/enum/protocol/type-parameter references and
//! heap-indirected containers (Array, Optional, Dictionary, Pointer). Those
//! introduce enough nominal distance that an alias inside them is not a
//! cyclic expansion of the outer alias.
//!
//! ## Diagnostics
//!
//! ### E447 -- `circular_type_alias` (Error, Correctness)
//!
//! **Message:** "circular type alias: '{A}' -> ... -> '{A}'"
//!
//! **Labels:**
//! - Primary: the origin type alias (the DFS root where the cycle was found)
//!   - Span source: `util::entity_span` on the type alias entity
//!   - Message: "cycle begins here"
//!
//! **Notes:** "type aliases cannot reference themselves, directly or indirectly"
//!
//! ### E448 -- `type_alias_contains_infer` (Warning, Correctness)
//!
//! _Reserved_ — not currently emitted by this analyzer.

use std::collections::HashSet;

use crate::compilation::cycle_util::{Cycle, CycleDetector};
use crate::context::CompilationContext;
use crate::diagnostic::*;
use crate::traits::{CompilationCheck, Describe};
use crate::util;
use kestrel_ast::AstType;
use kestrel_ast_builder::{NodeKind, TypeAnnotation};
use kestrel_hecs::Entity;
use kestrel_name_res::{ResolveTypePath, TypeResolution};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E447",
        name: "circular_type_alias",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E448",
        name: "type_alias_contains_infer",
        default_severity: Severity::Warning,
        category: Category::Correctness,
    },
];

pub struct TypeAliasCycleAnalyzer;

impl Describe for TypeAliasCycleAnalyzer {
    fn id(&self) -> &'static str {
        "type_alias_cycles"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl CompilationCheck for TypeAliasCycleAnalyzer {
    fn check(&self, cx: &CompilationContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();
        let mut checked: HashSet<Entity> = HashSet::new();

        let mut aliases = Vec::new();
        collect_aliases(cx, cx.root, &mut aliases);

        for origin in aliases {
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

/// Collect every type-alias entity with a concrete right-hand side
/// (protocol associated types without a `TypeAnnotation` have no RHS and
/// cannot be cyclic).
fn collect_aliases(cx: &CompilationContext<'_>, entity: Entity, out: &mut Vec<Entity>) {
    if cx.query.get::<NodeKind>(entity) == Some(&NodeKind::TypeAlias)
        && cx.query.get::<TypeAnnotation>(entity).is_some()
    {
        out.push(entity);
    }
    for &child in cx.query.children_of(entity) {
        collect_aliases(cx, child, out);
    }
}

fn detect_cycle_from(cx: &CompilationContext<'_>, origin: Entity) -> Option<Cycle> {
    let mut detector = CycleDetector::new();
    enter_alias(cx, origin, &mut detector).err()
}

/// Push `alias` onto the DFS stack and walk its RHS type looking for a
/// back-edge. Returns `Err(Cycle)` on detection; propagates up without
/// popping so the participant list captured by the detector is preserved.
fn enter_alias(
    cx: &CompilationContext<'_>,
    alias: Entity,
    detector: &mut CycleDetector,
) -> Result<(), Cycle> {
    detector.enter(alias)?;
    if let Some(ann) = cx.query.get::<TypeAnnotation>(alias) {
        check_type(cx, &ann.0, alias, detector)?;
    }
    detector.exit(alias);
    Ok(())
}

/// Recursively walk a type, chasing through tuples/function signatures into
/// any type-alias references we encounter.
fn check_type(
    cx: &CompilationContext<'_>,
    ty: &AstType,
    context: Entity,
    detector: &mut CycleDetector,
) -> Result<(), Cycle> {
    match ty {
        AstType::Named { segments, .. } => {
            let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
            let result = cx.query.query(ResolveTypePath {
                segments: seg_names,
                context,
                root: cx.root,
            });
            if let TypeResolution::Found(entity) = result
                && cx.query.get::<NodeKind>(entity) == Some(&NodeKind::TypeAlias) {
                    return enter_alias(cx, entity, detector);
                }
            Ok(())
        },
        AstType::Tuple(elements, _) => {
            for elem in elements {
                check_type(cx, elem, context, detector)?;
            }
            Ok(())
        },
        AstType::Function {
            params,
            return_type,
            ..
        } => {
            for p in params {
                check_type(cx, p, context, detector)?;
            }
            check_type(cx, return_type, context, detector)
        },
        // Array/Optional/Dictionary/Result/Pointer/Unit/Never/Inferred:
        // nominal indirection — an alias nested inside one of these does
        // not produce a self-expanding alias.
        _ => Ok(()),
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
        descriptor_id: "E447",
        severity: Severity::Error,
        message: format!("circular type alias: {}", cycle_str),
        labels: vec![DiagLabel {
            span: origin_span,
            message: "cycle begins here".into(),
            is_primary: true,
        }],
        notes: vec!["type aliases cannot reference themselves, directly or indirectly".into()],
    });
}
