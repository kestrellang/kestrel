//! Closure capture analysis — place-based (disjoint) capture plans.
//!
//! Single source of truth for "what does each closure capture". Runs
//! *after* type inference (it needs field resolutions + types), so it can
//! capture the *place* `self.cap` (a stored `Int64` field) by value instead
//! of capturing the whole `self`. That is what lets a closure over a
//! non-Copyable receiver compile: the receiver is never duplicated.
//!
//! Consumed by:
//! - MIR lowering (`kestrel-mir-lower`): env-struct fields, parent-side
//!   projection, and body rewriting (read the env value for `self.cap`).
//! - `kestrel-analyze` `ClosureAnalyzer` (E603/E605): the captured-root set.
//!
//! The analysis follows RFC 2229 ("disjoint closure captures"): it records the
//! *maximal* place each access touches, then reduces per root local to the
//! minimal set of disjoint prefixes. A whole-local use of a root collapses all
//! of that root to a single whole-local capture (the safety fallback, which
//! preserves the historical behavior for non-place receivers).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use kestrel_ast_builder::{Callable, NodeKind, Static};
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir::body::{
    HirBlock, HirClosureParam, HirExpr, HirExprId, HirPat, HirPatId, HirStmt, HirStmtId,
};
use kestrel_hir::res::LocalId;
use kestrel_hir_lower::LowerBody;

use crate::InferBody;
use crate::result::TypedBody;

// ===== Capture-plan data model =====

/// One projection step along an access path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProjElem {
    /// A stored struct field, identified by its resolved field entity.
    Field(Entity),
    /// A tuple element by index.
    TupleIndex(u32),
}

/// A captured place: a root local plus an ordered projection path.
/// An empty `path` means the whole local is captured.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlaceKey {
    pub root: LocalId,
    pub path: Vec<ProjElem>,
}

impl PlaceKey {
    fn whole(root: LocalId) -> Self {
        PlaceKey {
            root,
            path: Vec::new(),
        }
    }

    pub fn is_whole(&self) -> bool {
        self.path.is_empty()
    }

    /// True if `self.path` is a (non-strict) prefix of `other.path` and they
    /// share the same root.
    fn is_prefix_of(&self, other: &PlaceKey) -> bool {
        self.root == other.root
            && self.path.len() <= other.path.len()
            && self.path.iter().zip(&other.path).all(|(a, b)| a == b)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CaptureKind {
    /// Read-only access — by-copy when the projected type is Copyable.
    Read,
    /// The place (or a sub-place) is written through the closure — must be
    /// captured by reference so the write is observable.
    Write,
}

/// A single resolved capture in a closure's plan.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CapturedPlace {
    pub key: PlaceKey,
    pub kind: CaptureKind,
    /// A representative access expression for this place — the `self.cap`
    /// `HirExpr::Field` (or the root `Local` for whole-local captures). MIR
    /// lowering walks its HIR chain to resolve field indices and the
    /// projected type. Not part of place identity.
    pub repr: HirExprId,
}

/// Per-body capture plan: each closure `HirExprId` → its captured places,
/// in deterministic order.
#[derive(Clone, Debug, Default)]
pub struct ClosureCaptureMap {
    map: HashMap<HirExprId, Vec<CapturedPlace>>,
}

impl ClosureCaptureMap {
    /// The captures for a given closure expression (empty if none / unknown).
    pub fn get(&self, closure: HirExprId) -> &[CapturedPlace] {
        self.map.get(&closure).map_or(&[], |v| v.as_slice())
    }
}

/// Deterministic hash: sort closures by raw id, then hash each place vector
/// (vectors are already in deterministic order). Mirrors `TypedBody`'s manual
/// `Hash` so memoization is stable across runs.
impl std::hash::Hash for ClosureCaptureMap {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut pairs: Vec<_> = self.map.iter().collect();
        pairs.sort_by_key(|(k, _)| k.raw());
        for (k, places) in pairs {
            k.hash(state);
            places.hash(state);
        }
    }
}

// ===== ClosureCaptures query =====

/// Query: compute the place-based capture plan for every closure in a body.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ClosureCaptures {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for ClosureCaptures {
    // Arc-wrapped: memo cache hits clone the Output, so share one allocation
    // instead of deep-copying the per-body capture map.
    type Output = Arc<ClosureCaptureMap>;

    fn describe(&self) -> String {
        format!("ClosureCaptures(entity={:?})", self.entity)
    }

    fn execute(&self, query_ctx: &QueryContext<'_>) -> Arc<ClosureCaptureMap> {
        let Some(hir) = query_ctx.query(LowerBody {
            entity: self.entity,
            root: self.root,
        }) else {
            return Arc::new(ClosureCaptureMap::default());
        };
        let Some(typed) = query_ctx.query(InferBody {
            entity: self.entity,
            root: self.root,
        }) else {
            return Arc::new(ClosureCaptureMap::default());
        };

        let mut map = HashMap::new();
        for (expr_id, expr) in hir.exprs.iter() {
            if let HirExpr::Closure { params, body, .. } = expr {
                let places = analyze_closure(query_ctx, &typed, &hir, params, body);
                map.insert(expr_id, places);
            }
        }
        Arc::new(ClosureCaptureMap { map })
    }
}

// ===== Shared place resolution (also used by MIR lowering) =====

/// True if `entity` is a *stored instance field* (not a getter, computed
/// property, static, or protocol property). Mirrors the discrimination in
/// `kestrel-mir-lower` `lower_field_access`.
pub fn resolved_is_stored_field(world: &QueryContext<'_>, entity: Entity) -> bool {
    if world.get::<Callable>(entity).is_some() {
        return false; // getter / computed property
    }
    if world.get::<Static>(entity).is_some() {
        return false; // static stored field — not an instance place
    }
    if world.get::<NodeKind>(entity) != Some(&NodeKind::Field) {
        return false;
    }
    // A `NodeKind::Field` whose parent is a protocol is a protocol property
    // (witness-dispatched), not a stored field.
    if let Some(parent) = world.parent_of(entity)
        && world.get::<NodeKind>(parent) == Some(&NodeKind::Protocol)
    {
        return false;
    }
    true
}

/// Resolve an access expression to its `PlaceKey`, or `None` if it is not a
/// place chain (a call, literal, getter access, …). Shared with MIR lowering
/// so both layers agree on place identity.
pub fn place_key_of(
    world: &QueryContext<'_>,
    typed: &TypedBody,
    hir: &kestrel_hir::body::HirBody,
    expr_id: HirExprId,
) -> Option<PlaceKey> {
    match &hir.exprs[expr_id] {
        HirExpr::Local(local, _) => Some(PlaceKey::whole(*local)),
        HirExpr::Field { base, .. } => {
            let field_entity = typed.resolutions.get(&expr_id).copied()?;
            if !resolved_is_stored_field(world, field_entity) {
                return None;
            }
            let mut key = place_key_of(world, typed, hir, *base)?;
            key.path.push(ProjElem::Field(field_entity));
            Some(key)
        },
        HirExpr::TupleIndex { base, index, .. } => {
            let mut key = place_key_of(world, typed, hir, *base)?;
            key.path.push(ProjElem::TupleIndex(*index));
            Some(key)
        },
        HirExpr::Sugar { inner, .. } => place_key_of(world, typed, hir, *inner),
        _ => None,
    }
}

// ===== Per-closure analysis =====

/// A raw recorded place use, before disjoint-prefix reduction.
struct RawUse {
    key: PlaceKey,
    repr: HirExprId,
    write: bool,
}

fn analyze_closure(
    world: &QueryContext<'_>,
    typed: &TypedBody,
    hir: &kestrel_hir::body::HirBody,
    params: &[HirClosureParam],
    body: &HirBlock,
) -> Vec<CapturedPlace> {
    // Locals bound *inside* the closure are never captures of it.
    let mut internal = HashSet::new();
    for p in params {
        internal.insert(p.local);
        if let Some(pat) = p.pattern {
            collect_pat_bindings(hir, pat, &mut internal);
        }
    }
    collect_bound_locals_block(hir, body, &mut internal);

    let mut rec = Recorder {
        world,
        typed,
        hir,
        internal: &internal,
        uses: Vec::new(),
    };
    rec.walk_block(body, false);
    reduce(rec.uses)
}

/// Reduce raw place uses to a minimal disjoint capture plan, per root local.
fn reduce(uses: Vec<RawUse>) -> Vec<CapturedPlace> {
    // Group by root.
    let mut by_root: HashMap<LocalId, Vec<RawUse>> = HashMap::new();
    for u in uses {
        by_root.entry(u.key.root).or_default().push(u);
    }

    let mut out = Vec::new();
    for (root, group) in by_root {
        // Whole-local use collapses everything for this root.
        if let Some(whole) = group.iter().find(|u| u.key.is_whole()) {
            let write = group.iter().any(|u| u.write);
            out.push(CapturedPlace {
                key: PlaceKey::whole(root),
                kind: if write {
                    CaptureKind::Write
                } else {
                    CaptureKind::Read
                },
                repr: whole.repr,
            });
            continue;
        }

        // Minimal cover: keep a path iff no *other* present path is a strict
        // prefix of it. Every kept path corresponds to an actual access, so a
        // representative expr always exists.
        let mut kept: HashMap<Vec<ProjElem>, CapturedPlace> = HashMap::new();
        for u in &group {
            let subsumed = group.iter().any(|v| {
                v.key.path.len() < u.key.path.len() && prefix_of(&v.key.path, &u.key.path)
            });
            if subsumed {
                continue;
            }
            // Write-ness propagates from any use at or below this kept prefix.
            let write = group.iter().any(|v| u.key.is_prefix_of(&v.key) && v.write);
            kept.entry(u.key.path.clone()).or_insert(CapturedPlace {
                key: u.key.clone(),
                kind: if write {
                    CaptureKind::Write
                } else {
                    CaptureKind::Read
                },
                repr: u.repr,
            });
        }
        out.extend(kept.into_values());
    }

    // Deterministic order: by representative expr id.
    out.sort_by_key(|p| p.repr.raw());
    out
}

fn prefix_of(a: &[ProjElem], b: &[ProjElem]) -> bool {
    a.len() <= b.len() && a.iter().zip(b).all(|(x, y)| x == y)
}

// ===== Recording walk =====

struct Recorder<'a> {
    world: &'a QueryContext<'a>,
    typed: &'a TypedBody,
    hir: &'a kestrel_hir::body::HirBody,
    internal: &'a HashSet<LocalId>,
    uses: Vec<RawUse>,
}

impl Recorder<'_> {
    /// Record an external place. When `force_whole` (inside a nested closure),
    /// collapse to a whole-local capture — nested closures re-project on their
    /// own, and we do not (yet) thread sub-places through their env structs.
    fn record(&mut self, key: PlaceKey, repr: HirExprId, write: bool, force_whole: bool) {
        if self.internal.contains(&key.root) {
            return; // bound inside the closure — not a capture
        }
        let key = if force_whole {
            PlaceKey::whole(key.root)
        } else {
            key
        };
        self.uses.push(RawUse { key, repr, write });
    }

    fn walk_block(&mut self, block: &HirBlock, force_whole: bool) {
        for &stmt_id in &block.stmts {
            self.walk_stmt(stmt_id, force_whole);
        }
        if let Some(tail) = block.tail_expr {
            self.walk_expr(tail, force_whole);
        }
    }

    fn walk_stmt(&mut self, stmt_id: HirStmtId, force_whole: bool) {
        match &self.hir.stmts[stmt_id] {
            HirStmt::Let { value: Some(v), .. } => self.walk_expr(*v, force_whole),
            HirStmt::Expr { expr, .. } => self.walk_expr(*expr, force_whole),
            HirStmt::Let { value: None, .. } | HirStmt::Deinit { .. } => {},
        }
    }

    fn walk_expr(&mut self, expr_id: HirExprId, force_whole: bool) {
        // A place chain is captured as a unit — record it and stop (its only
        // child is the projection base, which is part of the place).
        if let Some(key) = place_key_of(self.world, self.typed, self.hir, expr_id) {
            self.record(key, expr_id, false, force_whole);
            return;
        }

        match &self.hir.exprs[expr_id] {
            HirExpr::Assign { target, value, .. } => {
                // A write to a place rooted at a capture needs by-ref capture.
                if let Some(key) = place_key_of(self.world, self.typed, self.hir, *target) {
                    self.record(key, *target, true, force_whole);
                } else {
                    self.walk_expr(*target, force_whole);
                }
                self.walk_expr(*value, force_whole);
            },
            HirExpr::Closure { body, .. } => {
                // Nested closure: its uses of our roots force whole-local.
                self.walk_block(body, true);
            },
            HirExpr::Call { callee, args, .. } => {
                self.walk_expr(*callee, force_whole);
                for arg in args {
                    self.walk_expr(arg.value, force_whole);
                }
            },
            HirExpr::MethodCall { receiver, args, .. }
            | HirExpr::ProtocolCall { receiver, args, .. } => {
                self.walk_expr(*receiver, force_whole);
                for arg in args {
                    self.walk_expr(arg.value, force_whole);
                }
            },
            HirExpr::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                self.walk_expr(*condition, force_whole);
                self.walk_block(then_body, force_whole);
                if let Some(eb) = else_body {
                    self.walk_block(eb, force_whole);
                }
            },
            HirExpr::Loop { body, .. } | HirExpr::Block { body, .. } => {
                self.walk_block(body, force_whole);
            },
            HirExpr::Match {
                scrutinee, arms, ..
            } => {
                self.walk_expr(*scrutinee, force_whole);
                for arm in arms {
                    if let Some(guard) = arm.guard {
                        self.walk_expr(guard, force_whole);
                    }
                    self.walk_expr(arm.body, force_whole);
                }
            },
            HirExpr::Tuple { elements, .. } | HirExpr::Array { elements, .. } => {
                for &e in elements {
                    self.walk_expr(e, force_whole);
                }
            },
            HirExpr::Dict { entries, .. } => {
                for entry in entries {
                    self.walk_expr(entry.key, force_whole);
                    self.walk_expr(entry.value, force_whole);
                }
            },
            HirExpr::Return { value: Some(v), .. } => self.walk_expr(*v, force_whole),
            HirExpr::ImplicitMember {
                args: Some(args), ..
            } => {
                for arg in args {
                    self.walk_expr(arg.value, force_whole);
                }
            },
            HirExpr::Sugar { inner, .. } => self.walk_expr(*inner, force_whole),
            // A field/tuple access that is *not* a capturable stored-field
            // place (a getter, computed property, …) still uses its base — so
            // the base must be captured (e.g. `self.count` getter → capture
            // whole `self`).
            HirExpr::Field { base, .. } | HirExpr::TupleIndex { base, .. } => {
                self.walk_expr(*base, force_whole);
            },
            // Leaves and place-chain heads already handled above.
            _ => {},
        }
    }
}

// ===== Bound-local collection =====

fn collect_bound_locals_block(
    hir: &kestrel_hir::body::HirBody,
    block: &HirBlock,
    out: &mut HashSet<LocalId>,
) {
    for &stmt_id in &block.stmts {
        match &hir.stmts[stmt_id] {
            HirStmt::Let { local, value, .. } => {
                out.insert(*local);
                if let Some(v) = value {
                    collect_bound_locals_expr(hir, *v, out);
                }
            },
            HirStmt::Expr { expr, .. } => collect_bound_locals_expr(hir, *expr, out),
            HirStmt::Deinit { .. } => {},
        }
    }
    if let Some(tail) = block.tail_expr {
        collect_bound_locals_expr(hir, tail, out);
    }
}

fn collect_bound_locals_expr(
    hir: &kestrel_hir::body::HirBody,
    expr_id: HirExprId,
    out: &mut HashSet<LocalId>,
) {
    match &hir.exprs[expr_id] {
        HirExpr::Closure { params, body, .. } => {
            for p in params {
                out.insert(p.local);
                if let Some(pat) = p.pattern {
                    collect_pat_bindings(hir, pat, out);
                }
            }
            collect_bound_locals_block(hir, body, out);
        },
        HirExpr::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            collect_bound_locals_expr(hir, *condition, out);
            collect_bound_locals_block(hir, then_body, out);
            if let Some(eb) = else_body {
                collect_bound_locals_block(hir, eb, out);
            }
        },
        HirExpr::Loop { body, .. } | HirExpr::Block { body, .. } => {
            collect_bound_locals_block(hir, body, out);
        },
        HirExpr::Match {
            scrutinee, arms, ..
        } => {
            collect_bound_locals_expr(hir, *scrutinee, out);
            for arm in arms {
                collect_pat_bindings(hir, arm.pattern, out);
                if let Some(guard) = arm.guard {
                    collect_bound_locals_expr(hir, guard, out);
                }
                collect_bound_locals_expr(hir, arm.body, out);
            }
        },
        HirExpr::Call { callee, args, .. } => {
            collect_bound_locals_expr(hir, *callee, out);
            for arg in args {
                collect_bound_locals_expr(hir, arg.value, out);
            }
        },
        HirExpr::MethodCall { receiver, args, .. }
        | HirExpr::ProtocolCall { receiver, args, .. } => {
            collect_bound_locals_expr(hir, *receiver, out);
            for arg in args {
                collect_bound_locals_expr(hir, arg.value, out);
            }
        },
        HirExpr::Field { base, .. } | HirExpr::TupleIndex { base, .. } => {
            collect_bound_locals_expr(hir, *base, out);
        },
        HirExpr::Assign { target, value, .. } => {
            collect_bound_locals_expr(hir, *target, out);
            collect_bound_locals_expr(hir, *value, out);
        },
        HirExpr::Tuple { elements, .. } | HirExpr::Array { elements, .. } => {
            for &e in elements {
                collect_bound_locals_expr(hir, e, out);
            }
        },
        HirExpr::Dict { entries, .. } => {
            for entry in entries {
                collect_bound_locals_expr(hir, entry.key, out);
                collect_bound_locals_expr(hir, entry.value, out);
            }
        },
        HirExpr::Return { value: Some(v), .. } => collect_bound_locals_expr(hir, *v, out),
        HirExpr::ImplicitMember {
            args: Some(args), ..
        } => {
            for arg in args {
                collect_bound_locals_expr(hir, arg.value, out);
            }
        },
        HirExpr::Sugar { inner, .. } => collect_bound_locals_expr(hir, *inner, out),
        _ => {},
    }
}

fn collect_pat_bindings(
    hir: &kestrel_hir::body::HirBody,
    pat_id: HirPatId,
    out: &mut HashSet<LocalId>,
) {
    match &hir.pats[pat_id] {
        HirPat::Binding { local, .. } => {
            out.insert(*local);
        },
        HirPat::At {
            binding,
            subpattern,
            ..
        } => {
            out.insert(*binding);
            collect_pat_bindings(hir, *subpattern, out);
        },
        HirPat::Tuple { prefix, suffix, .. } => {
            for &p in prefix.iter().chain(suffix) {
                collect_pat_bindings(hir, p, out);
            }
        },
        HirPat::Array {
            prefix,
            rest,
            suffix,
            ..
        } => {
            for &p in prefix.iter().chain(suffix) {
                collect_pat_bindings(hir, p, out);
            }
            if let Some(Some(local)) = rest {
                out.insert(*local);
            }
        },
        HirPat::Variant { args, .. } | HirPat::ImplicitVariant { args, .. } => {
            for arg in args {
                collect_pat_bindings(hir, arg.pattern, out);
            }
        },
        HirPat::Struct { fields, .. } => {
            for field in fields {
                if let Some(p) = field.pattern {
                    collect_pat_bindings(hir, p, out);
                }
            }
        },
        HirPat::Or { alternatives, .. } => {
            for &p in alternatives {
                collect_pat_bindings(hir, p, out);
            }
        },
        HirPat::Wildcard { .. }
        | HirPat::Literal { .. }
        | HirPat::Range { .. }
        | HirPat::Error { .. } => {},
    }
}
