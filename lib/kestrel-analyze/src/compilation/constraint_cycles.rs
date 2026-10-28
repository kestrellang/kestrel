//! # Constraint Cycle Analyzer
//!
//! Detects circular generic-constraint dependencies. A cycle occurs when
//! where-clause bounds create a dependency loop between type parameters of
//! the same generic owner:
//!
//! ```text
//! func swap[T, U]() where T: Container[U], U: Container[T]
//! //                      ^-- T depends on U, U depends on T
//! ```
//!
//! The algorithm works per-owner (function/struct/protocol/extension/alias
//! that carries both `TypeParams` and a `WhereClause`):
//!
//! 1. Build the set of that owner's own type parameters (entity IDs).
//! 2. For each `WhereConstraint::Bound`, resolve the subject to a local
//!    type parameter (single-segment subjects only — `Self.Item`-style
//!    associated-type bounds don't produce param-to-param edges).
//! 3. Walk each bound type for references to other local type parameters
//!    and add `subject -> ref` edges to a small adjacency map.
//! 4. DFS over the adjacency map looking for back-edges.
//!
//! The dependency graph is always tiny (params of a single generic owner),
//! so allocation and complexity are negligible.
//!
//! ## Diagnostics
//!
//! ### E451 -- `circular_constraint` (Error, Correctness)
//!
//! **Message:** "circular generic constraint: '{A}' -> ... -> '{A}'"
//!
//! **Labels:**
//! - Primary: the origin type parameter
//!   - Span source: `util::entity_span` on the type parameter entity
//!   - Message: "cycle begins here"
//!
//! **Notes:** "type parameter constraints cannot reference each other cyclically"

use std::collections::{HashMap, HashSet};

use crate::compilation::cycle_util::{Cycle, CycleDetector};
use crate::context::CompilationContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, CompilationCheck, Describe};
use crate::util;
use kestrel_ast::AstType;
use kestrel_ast_builder::{NodeKind, TypeParams, WhereClause, WhereConstraint};
use kestrel_hecs::Entity;
use kestrel_name_res::{ResolveTypePath, TypeResolution};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E451",
    name: "circular_constraint",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ConstraintCycleAnalyzer;

impl Describe for ConstraintCycleAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::ConstraintCycles
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl CompilationCheck for ConstraintCycleAnalyzer {
    fn check(&self, cx: &CompilationContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();
        let mut owners = Vec::new();
        collect_generic_owners(cx, cx.root, &mut owners);

        for owner in owners {
            if let Some(cycle) = detect_cycle_for_owner(cx, owner) {
                emit_cycle_diagnostic(cx, &cycle, &mut diags);
            }
        }

        diags
    }
}

/// Every entity that carries both `TypeParams` and a `WhereClause` is a
/// candidate owner. Walking the tree from root catches functions, structs,
/// enums, protocols, type aliases and extensions uniformly.
fn collect_generic_owners(cx: &CompilationContext<'_>, entity: Entity, out: &mut Vec<Entity>) {
    let has_params = cx.query.get::<TypeParams>(entity).is_some();
    let has_where = cx.query.get::<WhereClause>(entity).is_some();
    if has_params && has_where && cx.query.get::<NodeKind>(entity).is_some() {
        out.push(entity);
    }
    for &child in cx.query.children_of(entity) {
        collect_generic_owners(cx, child, out);
    }
}

fn detect_cycle_for_owner(cx: &CompilationContext<'_>, owner: Entity) -> Option<Cycle> {
    let type_params = cx.query.get::<TypeParams>(owner)?;
    let params: Vec<Entity> = type_params.0.clone();
    if params.is_empty() {
        return None;
    }
    let param_set: HashSet<Entity> = params.iter().copied().collect();

    let where_clause = cx.query.get::<WhereClause>(owner)?;
    let mut adj: HashMap<Entity, Vec<Entity>> = HashMap::new();

    for constraint in &where_clause.0 {
        let WhereConstraint::Bound {
            subject, protocols, ..
        } = constraint
        else {
            continue;
        };
        let Some(subject_param) = resolve_local_param(cx, subject, owner, &param_set) else {
            continue;
        };
        let mut refs = Vec::new();
        for proto in protocols {
            collect_param_refs(cx, proto, owner, &param_set, &mut refs);
        }
        // `where T: Proto[T]` is a legitimate self-referential bound, not a
        // cycle — only edges between *distinct* params can form one.
        refs.retain(|&r| r != subject_param);
        if !refs.is_empty() {
            adj.entry(subject_param).or_default().extend(refs);
        }
    }

    if adj.is_empty() {
        return None;
    }

    // DFS from each param; first cycle wins (one diagnostic per owner).
    for &start in &params {
        let mut detector = CycleDetector::new();
        if let Err(cycle) = dfs(start, &adj, &mut detector) {
            return Some(cycle);
        }
    }
    None
}

/// Resolve an AstType to a local type-parameter entity, or None if it isn't
/// a bare single-segment reference to one of `owner`'s type params.
fn resolve_local_param(
    cx: &CompilationContext<'_>,
    ty: &AstType,
    context: Entity,
    param_set: &HashSet<Entity>,
) -> Option<Entity> {
    let AstType::Named { segments, .. } = ty else {
        return None;
    };
    if segments.len() != 1 {
        return None;
    }
    let TypeResolution::Found(entity) = cx.query.query(ResolveTypePath {
        segments: vec![segments[0].name.clone()],
        context,
        root: cx.root,
    }) else {
        return None;
    };
    if param_set.contains(&entity) {
        Some(entity)
    } else {
        None
    }
}

/// Append every local-type-parameter entity referenced anywhere inside
/// `ty` to `out`. Walks into type arguments, tuple elements, function
/// signatures, and container-type inners so `Array[T]`, `(T, U)`, and
/// `Container[T]` all contribute their parameter references.
fn collect_param_refs(
    cx: &CompilationContext<'_>,
    ty: &AstType,
    context: Entity,
    param_set: &HashSet<Entity>,
    out: &mut Vec<Entity>,
) {
    match ty {
        AstType::Named { segments, .. } => {
            if let Some(first) = segments.first()
                && let TypeResolution::Found(e) = cx.query.query(ResolveTypePath {
                    segments: vec![first.name.clone()],
                    context,
                    root: cx.root,
                })
                && param_set.contains(&e)
            {
                out.push(e);
            }
            for seg in segments {
                for t in &seg.type_args {
                    collect_param_refs(cx, t, context, param_set, out);
                }
            }
        },
        AstType::Tuple(elems, _) => {
            for elem in elems {
                collect_param_refs(cx, elem, context, param_set, out);
            }
        },
        AstType::Function {
            params,
            return_type,
            ..
        } => {
            for p in params {
                collect_param_refs(cx, p, context, param_set, out);
            }
            collect_param_refs(cx, return_type, context, param_set, out);
        },
        AstType::Array(inner, _) | AstType::Optional(inner, _) => {
            collect_param_refs(cx, inner, context, param_set, out);
        },
        AstType::Dictionary(k, v, _) => {
            collect_param_refs(cx, k, context, param_set, out);
            collect_param_refs(cx, v, context, param_set, out);
        },
        AstType::Result { ok, err, .. } => {
            collect_param_refs(cx, ok, context, param_set, out);
            collect_param_refs(cx, err, context, param_set, out);
        },
        AstType::Some { bounds, .. } => {
            for b in bounds {
                collect_param_refs(cx, b, context, param_set, out);
            }
        },
        AstType::Ref { inner, .. } => {
            collect_param_refs(cx, inner, context, param_set, out);
        },
        AstType::Unit(_) | AstType::Never(_) | AstType::Inferred(_) => {},
    }
}

fn dfs(
    node: Entity,
    adj: &HashMap<Entity, Vec<Entity>>,
    detector: &mut CycleDetector,
) -> Result<(), Cycle> {
    detector.enter(node)?;
    if let Some(neighbors) = adj.get(&node) {
        for &next in neighbors {
            dfs(next, adj, detector)?;
        }
    }
    detector.exit(node);
    Ok(())
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
        descriptor_id: "E451",
        severity: Severity::Error,
        message: format!("circular generic constraint: {}", cycle_str),
        labels: vec![DiagLabel {
            span: origin_span,
            message: "cycle begins here".into(),
            is_primary: true,
        }],
        notes: vec!["type parameter constraints cannot reference each other cyclically".into()],
    });
}
