//! G25 commit 1 — detection-only probe on `solve_associated`'s R4 name
//! fallback ([`InferCtx::assoc_sub_by_name`]).
//!
//! R4 answers `X.Item` with a memo entry filed under a *different* entity
//! that is merely spelled `Item`. The claim under test is that every such hit
//! is standing in for a declared equality clause (`where TargetIterator.Item =
//! Item`). For each hit this asks the question directly: does any equality
//! clause in scope, as `WhereClausesOf` resolves it, relate the asked entity
//! to the matched one?
//!
//! - `JUSTIFIED` — a clause names both entities, one per side.
//! - `UNJUSTIFIED` — every equality clause in scope resolved, none relates them.
//! - `UNKNOWN` — no clause relates them, but some clause that might (its
//!   left side is spelled with the shared name, or its right side is one of
//!   the pair) has a left side that resolves to no entity, so it cannot be
//!   ruled out.
//!
//! A clause's left side `X.Item` is resolved the way production resolves it
//! (`find_assoc_type_in_bounds`, which asks off `Param(X)`) and, failing that,
//! off `TypeAlias(X)` — the receiver kind an associated-type `X` actually has.
//! Each clause logs `!prod` when only the second succeeded: production's own
//! memo registration for that clause is then dead.
//!
//! Inert unless `KESTREL_DEBUG=audit-assoc-eq`: the caller gates the call, and
//! nothing here writes to the context. Delete with the fallback (G25 commit 4).

use kestrel_ast_builder::{Name, NodeKind};
use kestrel_hecs::Entity;
use kestrel_hir::ty::HirTy;

use crate::ctx::InferCtx;
use crate::resolve::WhereClause;
use crate::ty::{TyKind, TySlot, TyVar};
use crate::where_clauses::WhereClausesOf;

/// One equality clause as the probe sees it: both sides reduced to the
/// associated-type / param entity they name, or `None` if they name none.
struct EqClause {
    holder: Entity,
    variant: &'static str,
    lhs: Option<Entity>,
    /// Whether `find_assoc_type_in_bounds` — the production call — resolved
    /// `lhs`. Always true for `DirectEquality`, whose lhs is the param itself.
    prod_lhs: bool,
    rhs: Option<Entity>,
    /// The assoc name spelled on the left (`Item` in `X.Item`), if any.
    lhs_name: Option<String>,
    /// Source-ish spelling for the log, e.g. `Iterable.TargetIterator.Item`.
    lhs_text: String,
}

/// Log one R4 hit: `asked` (queried off `base`) was answered by the entry
/// filed as `matched` (off `matched_base`).
pub(crate) fn report(
    ctx: &InferCtx<'_>,
    base: Option<TyVar>,
    asked: Entity,
    matched_base: Option<TyVar>,
    matched: Entity,
) {
    let clauses = equality_clauses_in_scope(ctx, asked, matched);
    let shared_name = ctx.query_ctx.get::<Name>(asked).map(|n| n.0.as_str());
    // A clause whose left side could not be resolved, and which might be about
    // this pair — the one thing that stops a hit being called UNJUSTIFIED.
    let undecidable = |c: &EqClause| {
        c.lhs.is_none()
            && (c.lhs_name.as_deref() == shared_name
                || c.rhs.is_some_and(|r| r == asked || r == matched))
    };
    let relates = |c: &EqClause| {
        matches!((c.lhs, c.rhs), (Some(l), Some(r))
            if (l == asked && r == matched) || (l == matched && r == asked))
    };
    let verdict = if clauses.iter().any(relates) {
        "JUSTIFIED"
    } else if clauses.iter().any(undecidable) {
        "UNKNOWN"
    } else {
        "UNJUSTIFIED"
    };
    let eqs = clauses
        .iter()
        .map(|c| {
            let ent = |e: Option<Entity>| e.map_or_else(|| "?".to_string(), |e| path(ctx, e));
            format!(
                "{}@{}:{}<{}{}>={}{}",
                c.variant,
                path(ctx, c.holder),
                c.lhs_text,
                ent(c.lhs),
                if c.prod_lhs { "" } else { "!prod" },
                ent(c.rhs),
                if relates(c) { "*" } else { "" },
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    kestrel_debug::ktrace!(
        "audit-assoc-eq",
        "{verdict} asked={} asked_base={} matched={} matched_base={} base_rel={} owner={} eqs=[{eqs}]",
        path(ctx, asked),
        kind_text(ctx, base, 2),
        path(ctx, matched),
        kind_text(ctx, matched_base, 2),
        base_relation(ctx, base, matched_base),
        path(ctx, ctx.owner),
    );
}

/// Every equality clause that could speak to this pair, from `WhereClausesOf`:
/// the body owner and its ancestors (function / extension / type clauses such
/// as `where B.Item = A.Item`), plus each protocol that declares one of the two
/// entities, its own clauses, and those on its associated types (the stdlib
/// bridge lives on `Iterable.TargetIterator`).
fn equality_clauses_in_scope(ctx: &InferCtx<'_>, asked: Entity, matched: Entity) -> Vec<EqClause> {
    let q = ctx.query_ctx;
    let mut holders: Vec<Entity> =
        std::iter::successors(Some(ctx.owner), |&e| q.parent_of(e)).collect();
    for proto in [asked, matched].into_iter().filter_map(|e| q.parent_of(e)) {
        holders.push(proto);
        holders.extend(
            q.children_of(proto)
                .iter()
                .copied()
                .filter(|&c| q.get::<NodeKind>(c) == Some(&NodeKind::TypeAlias)),
        );
    }
    let mut seen = std::collections::HashSet::new();
    holders.retain(|h| seen.insert(*h));

    let mut out = Vec::new();
    for holder in holders {
        let clauses = q.query(WhereClausesOf {
            entity: holder,
            root: ctx.root,
        });
        for clause in clauses {
            out.push(match clause {
                WhereClause::Bound { .. } => continue,
                WhereClause::TypeEquality {
                    param,
                    assoc_name,
                    rhs,
                } => {
                    let prod = crate::find_assoc_type_in_bounds(ctx, param, &assoc_name);
                    let as_alias = || {
                        let alias = TyKind::TypeAlias {
                            entity: param,
                            args: Vec::new(),
                        };
                        crate::assoc_entity_on(ctx, &alias, &assoc_name)
                    };
                    EqClause {
                        holder,
                        variant: "TypeEq",
                        lhs: prod.or_else(as_alias),
                        prod_lhs: prod.is_some(),
                        rhs: hir_entity(&rhs),
                        lhs_text: format!("{}.{assoc_name}", path(ctx, param)),
                        lhs_name: Some(assoc_name),
                    }
                },
                WhereClause::DirectEquality { param, rhs } => EqClause {
                    holder,
                    variant: "DirectEq",
                    lhs: Some(param),
                    prod_lhs: true,
                    rhs: hir_entity(&rhs),
                    lhs_text: path(ctx, param),
                    lhs_name: None,
                },
            });
        }
    }
    out
}

/// The associated-type or param entity a clause side names. Anything else
/// (a concrete nominal, a tuple, `Self`) names no entity the fallback could
/// have confused, so it is `None`.
fn hir_entity(ty: &HirTy) -> Option<Entity> {
    match ty {
        HirTy::AliasUse { entity, args, .. } if args.is_empty() => Some(*entity),
        HirTy::Param(entity, _) => Some(*entity),
        HirTy::AssocProjection { assoc, .. } => Some(*assoc),
        _ => None,
    }
}

/// How the query's receiver relates to the matched entry's. `proj-of-entry`
/// is the bridge's expected shape: asked off `S.TargetIterator`, filed off `S`.
fn base_relation(ctx: &InferCtx<'_>, asked: Option<TyVar>, entry: Option<TyVar>) -> &'static str {
    let (Some(a), Some(e)) = (asked, entry) else {
        return if entry.is_none() {
            "entry-baseless"
        } else {
            "query-baseless"
        };
    };
    let (a, e) = (ctx.resolve(a), ctx.resolve(e));
    if a == e {
        return "same";
    }
    match ctx.slot(a) {
        TySlot::Resolved(TyKind::AssocProjection { base, .. }) if ctx.resolve(*base) == e => {
            "proj-of-entry"
        },
        _ => "other",
    }
}

/// Short rendering of a resolved TyVar, `depth` levels into projections.
fn kind_text(ctx: &InferCtx<'_>, tv: Option<TyVar>, depth: u32) -> String {
    let Some(tv) = tv else {
        return "-".to_string();
    };
    let tv = ctx.resolve(tv);
    let kind = match ctx.slot(tv) {
        TySlot::Resolved(k) => k,
        _ => return format!("?{}", tv.0),
    };
    match kind {
        TyKind::Param { entity } => format!("Param({})", path(ctx, *entity)),
        TyKind::AssocProjection { base, assoc } if depth > 0 => format!(
            "Proj({}.{})",
            kind_text(ctx, Some(*base), depth - 1),
            path(ctx, *assoc)
        ),
        TyKind::Struct { entity, .. }
        | TyKind::Enum { entity, .. }
        | TyKind::Protocol { entity, .. }
        | TyKind::SelfType { entity }
        | TyKind::TypeAlias { entity, .. } => {
            format!("{}({})", kind_tag(kind), path(ctx, *entity))
        },
        other => kind_tag(other).to_string(),
    }
}

fn kind_tag(kind: &TyKind) -> &'static str {
    match kind {
        TyKind::Struct { .. } => "Struct",
        TyKind::Enum { .. } => "Enum",
        TyKind::Protocol { .. } => "Protocol",
        TyKind::SelfType { .. } => "SelfType",
        TyKind::TypeAlias { .. } => "TypeAlias",
        TyKind::Tuple(_) => "Tuple",
        TyKind::Function { .. } => "Function",
        TyKind::Param { .. } => "Param",
        TyKind::AssocProjection { .. } => "Proj",
        TyKind::Never => "Never",
        TyKind::Opaque { .. } => "Opaque",
        TyKind::Ref { .. } => "Ref",
        TyKind::Error => "Error",
    }
}

/// `Parent.Name` for audit output.
fn path(ctx: &InferCtx<'_>, e: Entity) -> String {
    let q = ctx.query_ctx;
    let name = |e: Entity| {
        q.get::<Name>(e)
            .map_or_else(|| "?".to_string(), |n| n.0.clone())
    };
    match q.parent_of(e) {
        Some(p) => format!("{}.{}", name(p), name(e)),
        None => name(e),
    }
}
