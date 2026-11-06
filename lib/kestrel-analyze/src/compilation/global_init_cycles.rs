//! # Global Initializer Cycle Analyzer
//!
//! A global's initializer runs once, before `main`, after the initializers of
//! the globals it reads. An initializer that reads its own global, directly
//! or through other globals, has no valid order: it used to read the global's
//! zeroed storage (`let g: Int64 = g` printed 0).
//!
//! Dependencies come from `GlobalInitDeps`, the same query MIR uses to order
//! the initializers. Reads through function calls are not followed.
//!
//! ## Diagnostics
//!
//! ### E633 -- `circular_global_initializer` (Error, Correctness)
//!
//! **Message:** "global initializer depends on itself: '{a}' -> ... -> '{a}'"
//!
//! **Labels:**
//! - Primary: the first global of the cycle
//!   - Span source: `util::entity_span` on the global
//!   - Message: "cycle begins here"

use std::collections::HashSet;

use crate::compilation::cycle_util::{Cycle, CycleDetector};
use crate::context::CompilationContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, CompilationCheck, Describe};
use crate::util;
use kestrel_hecs::Entity;
use kestrel_hir_lower::{GlobalInitDeps, is_initialized_global};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E633",
    name: "circular_global_initializer",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct GlobalInitCycleAnalyzer;

impl Describe for GlobalInitCycleAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::GlobalInitCycles
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl CompilationCheck for GlobalInitCycleAnalyzer {
    fn check(&self, cx: &CompilationContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut globals = Vec::new();
        collect_globals(cx, cx.root, &mut globals);
        let mut diags = Vec::new();
        let mut checked: HashSet<Entity> = HashSet::new();
        for origin in globals {
            if !checked.insert(origin) {
                continue;
            }
            let mut detector = CycleDetector::new();
            if let Err(cycle) = enter(cx, origin, &mut detector) {
                checked.extend(cycle.participants.iter().copied());
                diags.push(cycle_diagnostic(cx, &cycle));
            }
        }
        diags
    }
}

fn collect_globals(cx: &CompilationContext<'_>, entity: Entity, out: &mut Vec<Entity>) {
    if is_initialized_global(cx.query, entity) {
        out.push(entity);
    }
    for &child in cx.query.children_of(entity) {
        collect_globals(cx, child, out);
    }
}

fn enter(
    cx: &CompilationContext<'_>,
    global: Entity,
    detector: &mut CycleDetector,
) -> Result<(), Cycle> {
    detector.enter(global)?;
    for dep in cx.query.query(GlobalInitDeps {
        entity: global,
        root: cx.root,
    }) {
        enter(cx, dep, detector)?;
    }
    detector.exit(global);
    Ok(())
}

fn cycle_diagnostic(cx: &CompilationContext<'_>, cycle: &Cycle) -> AnalyzeDiagnostic {
    let mut names: Vec<String> = cycle
        .participants
        .iter()
        .map(|&e| util::entity_name(cx.query, e))
        .collect();
    names.push(names[0].clone());
    AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[0].id,
        severity: DESCRIPTORS[0].default_severity,
        message: format!(
            "global initializer depends on itself: {}",
            names.join(" -> ")
        ),
        labels: vec![DiagLabel {
            span: util::entity_span(cx.query, cycle.participants[0]),
            message: "cycle begins here".into(),
            is_primary: true,
        }],
        notes: vec!["a global cannot read itself while it is being initialized".into()],
    }
}
