//! # Struct Cycle Analyzer
//!
//! Detects circular struct containment that would produce infinite-size types.
//! A struct that (transitively) contains itself by value cannot be represented
//! in memory because its layout would be infinitely recursive.
//!
//! The walk follows struct field types through other structs and tuples, but
//! stops at heap-indirected types (Array, Optional, pointers) and function
//! types. Computed properties are skipped since they don't store values.
//!
//! ## Diagnostics
//!
//! ### E449 -- `self_containing_struct` (Error, Correctness)
//!
//! **Message:** "struct '{name}' cannot contain itself"
//!
//! **Labels:**
//! - Primary: the field on the origin struct whose type re-enters the origin
//!   - Span source: `util::entity_span` on the self-referencing field entity
//!   - Message: "self-referencing field"
//!
//! **Notes:** "use an Array or Optional to break the cycle with heap indirection"
//!
//! ### E450 -- `circular_struct_containment` (Error, Correctness)
//!
//! **Message:** "circular struct containment: '{A}' -> ... -> '{A}'"
//!
//! **Labels:**
//! - Primary: the first *direct* `AstType::Named` opening in the DFS stack
//!   (a field whose top-level type is a bare Named reference to a cycle
//!   participant). If every struct in the cycle was entered via indirection
//!   (e.g. tuple-wrapped), falls back to the first tuple-entry field.
//!   - Span source: `util::entity_span` on the chosen field entity
//!   - Message: "cycle begins here"
//!
//! **Notes:** "use an Array or Optional to break the cycle with heap indirection"

use std::collections::HashSet;

use crate::compilation::cycle_util::{Cycle, CycleDetector};
use crate::context::CompilationContext;
use crate::diagnostic::*;
use crate::traits::{CompilationCheck, Describe};
use crate::util;
use kestrel_ast::AstType;
use kestrel_ast_builder::{Callable, NodeKind, TypeAnnotation};
use kestrel_hecs::Entity;
use kestrel_name_res::{ResolveTypePath, TypeResolution};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E449",
        name: "self_containing_struct",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E450",
        name: "circular_struct_containment",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct StructCycleAnalyzer;

impl Describe for StructCycleAnalyzer {
    fn id(&self) -> &'static str {
        "struct_cycles"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl CompilationCheck for StructCycleAnalyzer {
    fn check(&self, cx: &CompilationContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();
        let mut checked: HashSet<Entity> = HashSet::new();

        let mut structs = Vec::new();
        collect_structs(cx, cx.root, &mut structs);

        for origin in structs {
            if checked.contains(&origin) {
                continue;
            }
            if let Some(report) = detect_cycle_from(cx, origin) {
                emit_cycle_diagnostic(cx, &report.cycle, report.label_field, &mut diags);
                for &e in &report.cycle.participants {
                    checked.insert(e);
                }
            }
            checked.insert(origin);
        }

        diags
    }
}

/// Recursively collect all Struct entities from the module tree.
fn collect_structs(cx: &CompilationContext<'_>, entity: Entity, out: &mut Vec<Entity>) {
    if cx.query.get::<NodeKind>(entity) == Some(&NodeKind::Struct) {
        out.push(entity);
    }
    for &child in cx.query.children_of(entity) {
        collect_structs(cx, child, out);
    }
}

/// A cycle together with the field chosen to carry the primary diagnostic label.
struct CycleReport {
    cycle: Cycle,
    label_field: Entity,
}

/// DFS state that parallels the `CycleDetector`'s stack. For each struct
/// currently on the DFS path it tracks two related facts about the *edge*
/// from the parent struct that led here:
///
/// - `direct`: `Some(field)` iff that field's top-level `TypeAnnotation` was
///   a bare `AstType::Named` referencing the struct. Tuple/array/optional
///   wrappers are "indirect" (None).
/// - `entry`: `Some(field)` for every non-origin entry — the parent-struct
///   field that eventually led here (via Named or through a tuple).
///
/// We always push before `CycleDetector::enter`, so on a back-edge the
/// closing attempt's edge stays on these stacks for [`pick_label_field`].
#[derive(Default)]
struct OpeningStack {
    direct: Vec<Option<Entity>>,
    entry: Vec<Option<Entity>>,
}

impl OpeningStack {
    fn push(&mut self, direct: Option<Entity>, entry: Option<Entity>) {
        self.direct.push(direct);
        self.entry.push(entry);
    }
    fn pop(&mut self) {
        self.direct.pop();
        self.entry.pop();
    }
}

fn detect_cycle_from(cx: &CompilationContext<'_>, origin: Entity) -> Option<CycleReport> {
    let mut detector = CycleDetector::new();
    let mut stack = OpeningStack::default();
    match enter_struct(cx, origin, &mut detector, &mut stack, None, None) {
        Err(cycle) => {
            let label_field = pick_label_field(&stack, origin);
            Some(CycleReport { cycle, label_field })
        },
        Ok(()) => None,
    }
}

/// Pick the primary-label field. Prefers the first `direct` opening on the
/// stack; otherwise, the first `entry` opening (fallback for all-indirect
/// cycles); otherwise, the origin itself (shouldn't happen in practice).
fn pick_label_field(stack: &OpeningStack, origin: Entity) -> Entity {
    stack
        .direct
        .iter()
        .copied()
        .flatten()
        .next()
        .or_else(|| stack.entry.iter().copied().flatten().next())
        .unwrap_or(origin)
}

/// True for stored-property fields (non-computed). Skips computed properties,
/// which are represented as Fields carrying a Callable getter.
fn is_stored_field(cx: &CompilationContext<'_>, e: Entity) -> bool {
    cx.query.get::<NodeKind>(e) == Some(&NodeKind::Field) && cx.query.get::<Callable>(e).is_none()
}

/// Walk a field's type, following struct references transitively through
/// tuples. Stops at heap-indirected types (Array, Optional, Dictionary,
/// Pointer) and function types.
///
/// `direct`/`entry` describe the edge that led to this recursion: `direct`
/// carries the field only while we're at the top level of a bare Named field,
/// and gets stripped to `None` when descending into a tuple. `entry` persists
/// through tuple indirection so a cycle closed entirely through tuples can
/// still be labelled on the parent's field.
fn check_type(
    cx: &CompilationContext<'_>,
    ty: &AstType,
    context: Entity,
    detector: &mut CycleDetector,
    stack: &mut OpeningStack,
    direct: Option<Entity>,
    entry: Option<Entity>,
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
                && cx.query.get::<NodeKind>(entity) == Some(&NodeKind::Struct)
            {
                return enter_struct(cx, entity, detector, stack, direct, entry);
            }
            Ok(())
        },
        AstType::Tuple(elements, _) => {
            // Tuple is indirection: struct entries inside aren't direct, but
            // `entry` stays pointing at the enclosing field so the fallback
            // label picker can find a meaningful span.
            for elem in elements {
                check_type(cx, elem, context, detector, stack, None, entry)?;
            }
            Ok(())
        },
        // Array, Optional, Dictionary, Result, Function, Pointer — indirection,
        // stop here.
        _ => Ok(()),
    }
}

/// Push the incoming edge onto the opening stack, then try to enter `entity`
/// on the cycle detector. Push happens *before* enter so the closing attempt's
/// edge remains on the stack when a back-edge returns `Err`.
fn enter_struct(
    cx: &CompilationContext<'_>,
    entity: Entity,
    detector: &mut CycleDetector,
    stack: &mut OpeningStack,
    direct: Option<Entity>,
    entry: Option<Entity>,
) -> Result<(), Cycle> {
    stack.push(direct, entry);
    detector.enter(entity)?;
    for &child in cx.query.children_of(entity) {
        if !is_stored_field(cx, child) {
            continue;
        }
        let Some(ann) = cx.query.get::<TypeAnnotation>(child) else {
            continue;
        };
        let child_direct = match &ann.0 {
            AstType::Named { .. } => Some(child),
            _ => None,
        };
        check_type(
            cx,
            &ann.0,
            entity,
            detector,
            stack,
            child_direct,
            Some(child),
        )?
    }
    stack.pop();
    detector.exit(entity);
    Ok(())
}

fn emit_cycle_diagnostic(
    cx: &CompilationContext<'_>,
    cycle: &Cycle,
    starting_field: Entity,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let participants = &cycle.participants;
    if participants.is_empty() {
        return;
    }
    let origin = participants[0];
    let origin_name = util::entity_name(cx.query, origin);
    let field_span = util::entity_span(cx.query, starting_field);

    if participants.len() == 1 {
        // Self-cycle: origin contains itself directly.
        diags.push(AnalyzeDiagnostic {
            descriptor_id: "E449",
            severity: Severity::Error,
            message: format!("struct '{}' cannot contain itself", origin_name),
            labels: vec![DiagLabel {
                span: field_span,
                message: "self-referencing field".into(),
                is_primary: true,
            }],
            notes: vec!["use an Array or Optional to break the cycle with heap indirection".into()],
        });
    } else {
        // Multi-struct cycle. Close the loop by appending the origin name
        // so the display reads "A -> B -> C -> A".
        let mut names: Vec<String> = participants
            .iter()
            .map(|&e| util::entity_name(cx.query, e))
            .collect();
        names.push(origin_name);
        let cycle_str = names.join(" -> ");

        diags.push(AnalyzeDiagnostic {
            descriptor_id: "E450",
            severity: Severity::Error,
            message: format!("circular struct containment: {}", cycle_str),
            labels: vec![DiagLabel {
                span: field_span,
                message: "cycle begins here".into(),
                is_primary: true,
            }],
            notes: vec!["use an Array or Optional to break the cycle with heap indirection".into()],
        });
    }
}
