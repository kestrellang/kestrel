//! # Observable Drop Analyzer
//!
//! Warns when a `Cloneable` type looks like its drop has an observable effect
//! but is not marked `@observabledrop`. The copy lifetime passes
//! (`kestrel-mir` `passes/copy_lifetime/`) drop implicit copies right after
//! their last read; that is invisible for memory-only drops, but for a shared
//! handle whose last drop closes a socket it would close it early.
//!
//! "Observable" is approximated by the call graph: the type's `deinit`
//! reaches — directly or through Kestrel functions — an `@extern` function
//! other than the C allocator and `mem*` functions (`MEMORY_EXTERNS`). Calls the walk cannot resolve
//! statically (protocol calls, closures) are ignored: this is a lint, not a
//! safety gate, and false alarms would teach people to ignore it.
//! `not Copyable` types are never implicitly copied and are skipped.
//!
//! ## Diagnostics
//!
//! ### E507 -- `observable_drop_unmarked` (Warning, Correctness)
//!
//! **Message:** "`Cloneable` type `{type}` has a `deinit` that calls `{callee}`; mark it `@observabledrop` so its copies are not dropped early"
//!
//! **Labels:**
//! - Primary: the type declaration
//!   - Span source: `util::entity_span` on the type entity
//!   - Message: "copies of this type may be dropped before the end of their scope"
//!
//! **Notes:**
//! - "`{callee}` reaches the external function `{symbol}`" (only when the
//!   callee is a wrapper rather than the extern itself)

use std::collections::HashSet;

use kestrel_debug::ktrace;

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Attributes, NodeKind};
use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::body::HirExpr;
use kestrel_hir_lower::LowerBody;
use kestrel_semantics::{CopySemantics, NominalCopySemantics};
use kestrel_type_infer::InferBody;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E507",
    name: "observable_drop_unmarked",
    default_severity: Severity::Warning,
    category: Category::Correctness,
}];

/// Externs that only allocate, release or move memory (the C allocator and
/// `mem*` functions); reaching them is not observable.
const MEMORY_EXTERNS: &[&str] = &[
    "malloc", "calloc", "realloc", "free", "memcpy", "memmove", "memset", "memcmp",
];

/// Bound on how deep the call walk follows Kestrel functions.
const MAX_DEPTH: usize = 16;

pub struct ObservableDropAnalyzer;

impl Describe for ObservableDropAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::ObservableDrop
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for ObservableDropAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Struct, NodeKind::Enum]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        if has_attribute(cx.query, cx.entity, "observabledrop") {
            return vec![];
        }
        let semantics = cx.query.query(NominalCopySemantics {
            entity: cx.entity,
            root: cx.root,
        });
        if semantics.semantics != CopySemantics::Cloneable {
            return vec![];
        }
        let Some(&deinit) = util::children_of_kind(cx.query, cx.entity, NodeKind::Deinit).first()
        else {
            return vec![];
        };
        let Some(hit) = first_observable_call(cx, deinit) else {
            return vec![];
        };

        let type_name = util::entity_name(cx.query, cx.entity);
        let callee = util::entity_name(cx.query, hit.callee);
        ktrace!(
            "observable-drop",
            "E507 on `{type_name}`: deinit calls `{callee}` -> extern `{}`",
            hit.symbol
        );
        let notes = if hit.callee == hit.extern_fn {
            vec![]
        } else {
            vec![format!(
                "`{callee}` reaches the external function `{}`",
                hit.symbol
            )]
        };
        vec![AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: format!(
                "`Cloneable` type `{type_name}` has a `deinit` that calls `{callee}`; \
                 mark it `@observabledrop` so its copies are not dropped early"
            ),
            labels: vec![DiagLabel {
                span: util::entity_span(cx.query, cx.entity),
                message: "copies of this type may be dropped before the end of their scope".into(),
                is_primary: true,
            }],
            notes,
        }]
    }
}

/// A call in the deinit that leads to a non-memory extern.
struct Hit {
    /// Function the deinit calls directly.
    callee: Entity,
    /// The extern it eventually reaches (may equal `callee`).
    extern_fn: Entity,
    symbol: String,
}

/// First call made directly by `deinit` whose call graph reaches an extern
/// other than a memory release.
fn first_observable_call(cx: &DeclContext<'_>, deinit: Entity) -> Option<Hit> {
    let mut visited = HashSet::from([deinit]);
    direct_callees(cx, deinit).into_iter().find_map(|callee| {
        let (extern_fn, symbol) = reaches_observable_extern(cx, callee, &mut visited, 0)?;
        Some(Hit {
            callee,
            extern_fn,
            symbol,
        })
    })
}

/// Does calling `func` reach an observable extern? Returns it and its symbol.
fn reaches_observable_extern(
    cx: &DeclContext<'_>,
    func: Entity,
    visited: &mut HashSet<Entity>,
    depth: usize,
) -> Option<(Entity, String)> {
    if let Some(attrs) = cx.query.get::<Attributes>(func)
        && attrs.0.iter().any(|a| a.name == "extern")
    {
        let symbol = attrs
            .extern_mangle_name()
            .unwrap_or_else(|| util::entity_name(cx.query, func));
        return (!MEMORY_EXTERNS.contains(&symbol.as_str())).then_some((func, symbol));
    }
    if depth >= MAX_DEPTH || !visited.insert(func) {
        return None;
    }
    direct_callees(cx, func)
        .into_iter()
        .find_map(|callee| reaches_observable_extern(cx, callee, visited, depth + 1))
}

/// Functions `func`'s body calls that resolve statically: plain calls and
/// method calls. Protocol calls and calls through values are skipped.
fn direct_callees(cx: &DeclContext<'_>, func: Entity) -> Vec<Entity> {
    let Some(hir) = cx.query.query(LowerBody {
        entity: func,
        root: cx.root,
    }) else {
        return vec![];
    };
    let Some(typed) = cx.query.query(InferBody {
        entity: func,
        root: cx.root,
    }) else {
        return vec![];
    };
    hir.exprs
        .iter()
        .filter_map(|(expr_id, expr)| match expr {
            HirExpr::Call { callee, .. } => match &hir.exprs[*callee] {
                HirExpr::Def(entity, _, _) => Some(*entity),
                _ => typed.resolutions.get(callee).copied(),
            },
            HirExpr::MethodCall { .. } => typed.resolutions.get(&expr_id).copied(),
            _ => None,
        })
        .collect()
}

fn has_attribute(query: &QueryContext<'_>, entity: Entity, name: &str) -> bool {
    query
        .get::<Attributes>(entity)
        .is_some_and(|attrs| attrs.0.iter().any(|a| a.name == name))
}
