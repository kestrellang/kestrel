//! Partial moves (docs/design/partial-moves.md): moving a non-Copyable field
//! out of an owned local while its other fields stay in place.
//!
//! Lowering needs to know, BEFORE a local is bound, which of its field paths
//! may be moved: each such path gets a presence drop flag allocated where the
//! local is declared, so the flag dominates every scope exit. `plan` finds
//! them with one pass over the body — the same consuming positions the move
//! checker uses (`let`/assign/return/tail values, consuming call operands,
//! tuple/array/enum-payload elements, if/match/block result tails).
//!
//! The plan is also the single decision point for WHICH projection
//! expressions move: `lower_expr` consults `exprs` and takes the field out of
//! the slot instead of borrowing it.

use std::collections::{HashMap, HashSet};

use kestrel_ast_builder::{Callable, ReceiverKind};
use kestrel_hir::body::{HirBlock, HirCallArg, HirExpr, HirExprId, HirStmt};
use kestrel_hir::res::LocalId as HirLocalId;
use kestrel_mir::item::witness::WitnessMethodKey;
use kestrel_mir::{FieldIdx, MirTy, TyId};

use super::{OssaBodyCtx, PathState, ScopeEntry, VarInit};
use kestrel_mir::ValueId;

/// Which projections of which locals this body may move out.
#[derive(Default, Clone)]
pub(crate) struct PartialMovePlan {
    /// Projection expressions lowered as a move (`Take` of the field).
    pub exprs: HashSet<HirExprId>,
    /// Per local: every field path that may be moved, in discovery order.
    pub paths: HashMap<HirLocalId, Vec<Vec<FieldIdx>>>,
}

impl PartialMovePlan {
    pub fn paths_of(&self, local: HirLocalId) -> &[Vec<FieldIdx>] {
        self.paths.get(&local).map_or(&[], |v| v.as_slice())
    }
}

impl OssaBodyCtx<'_, '_> {
    /// Scan the body for consuming uses of non-Copyable projections rooted
    /// at a local.
    pub(crate) fn plan_partial_moves(&mut self) -> PartialMovePlan {
        let operands = self.consuming_operands();
        let mut plan = PartialMovePlan::default();
        for op in operands {
            let Some((local, path)) = self.place_path(op) else {
                continue;
            };
            if path.is_empty() || !self.projection_moves(op) {
                continue;
            }
            plan.exprs.insert(op);
            let paths = plan.paths.entry(local).or_default();
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
        plan
    }

    /// A projection of this type moves rather than copies: not Copyable, or
    /// a copy behavior only known after mono (same rule as whole-var reads).
    fn projection_moves(&mut self, expr: HirExprId) -> bool {
        let ty = self.resolve_expr_type(expr);
        self.is_non_copyable(ty) || self.copy_behavior_is_mono_dependent(ty)
    }

    /// `(root local, field path)` of a stored-field / tuple-element chain.
    /// `None` for anything else (calls, accessors, ref-typed slots).
    pub(crate) fn place_path(&mut self, expr: HirExprId) -> Option<(HirLocalId, Vec<FieldIdx>)> {
        match self.hir.exprs[expr].clone() {
            HirExpr::Local(local, _) => Some((local, Vec::new())),
            HirExpr::Field { base, name, .. } => {
                let (local, mut path) = self.place_path(base)?;
                let base_ty = self.resolve_expr_type(base);
                let MirTy::Named { entity, .. } = self.ctx.module.ty_arena.get(base_ty) else {
                    return None;
                };
                let idx = self
                    .ctx
                    .resolve_field_idx(*entity, name.as_str_or_empty())?;
                if self.field_is_ref(base_ty, idx) {
                    return None;
                }
                path.push(idx);
                Some((local, path))
            },
            HirExpr::TupleIndex { base, index, .. } => {
                let (local, mut path) = self.place_path(base)?;
                let base_ty = self.resolve_expr_type(base);
                let idx = FieldIdx::new(index as usize);
                if self.field_is_ref(base_ty, idx) {
                    return None;
                }
                path.push(idx);
                Some((local, path))
            },
            _ => None,
        }
    }

    /// A ref-typed slot's storage is its pointee, not the container.
    fn field_is_ref(&self, container: TyId, idx: FieldIdx) -> bool {
        let arena = &self.ctx.module.ty_arena;
        let elem = match arena.get(container) {
            MirTy::Tuple(elems) => elems.get(idx.index()).copied(),
            MirTy::Named { entity, .. } => self.ctx.resolve_field_ty(*entity, idx),
            _ => None,
        };
        elem.is_some_and(|t| matches!(arena.get(t), MirTy::Ref { .. }))
    }

    /// Every expression in a position that takes ownership of its value.
    fn consuming_operands(&self) -> Vec<HirExprId> {
        let mut out = Vec::new();
        let tail = |b: &HirBlock, out: &mut Vec<HirExprId>| out.extend(b.tail_expr);
        out.extend(self.hir.tail_expr);
        for (_, stmt) in self.hir.stmts.iter() {
            if let HirStmt::Let { value: Some(v), .. } = stmt {
                out.push(*v);
            }
        }
        for (id, expr) in self.hir.exprs.iter() {
            match expr {
                HirExpr::Assign { value, .. } => out.push(*value),
                HirExpr::Return { value: Some(v), .. } => out.push(*v),
                HirExpr::Tuple { elements, .. } | HirExpr::Array { elements, .. } => {
                    out.extend(elements.iter().copied());
                },
                HirExpr::If {
                    then_body,
                    else_body,
                    ..
                } => {
                    tail(then_body, &mut out);
                    if let Some(e) = else_body {
                        tail(e, &mut out);
                    }
                },
                HirExpr::Match { arms, .. } => out.extend(arms.iter().map(|a| a.body)),
                HirExpr::Block { body, .. } => tail(body, &mut out),
                HirExpr::ImplicitMember { args: Some(a), .. } => {
                    out.extend(a.iter().map(|a| a.value));
                },
                HirExpr::Call { callee, args, .. } => {
                    let entity = match &self.hir.exprs[*callee] {
                        HirExpr::Def(e, _, _) => Some(*e),
                        _ => self.typed_resolution(*callee),
                    };
                    self.consuming_args(entity, None, args, &mut out);
                },
                HirExpr::MethodCall { receiver, args, .. } => {
                    self.consuming_args(self.typed_resolution(id), Some(*receiver), args, &mut out);
                },
                HirExpr::ProtocolCall {
                    receiver,
                    protocol,
                    method,
                    args,
                    ..
                } => {
                    let labels = args.iter().map(|a| a.label.clone()).collect();
                    let key = WitnessMethodKey::new(method.as_str_or_empty(), labels);
                    let entity = self.find_protocol_method_entity(*protocol, &key);
                    self.consuming_args(entity, Some(*receiver), args, &mut out);
                },
                _ => {},
            }
        }
        out
    }

    fn typed_resolution(&self, expr: HirExprId) -> Option<kestrel_hecs::Entity> {
        self.typed.as_ref()?.resolutions.get(&expr).copied()
    }

    /// Arguments (and receiver) bound to `consuming` parameters of `callee`.
    fn consuming_args(
        &self,
        callee: Option<kestrel_hecs::Entity>,
        receiver: Option<HirExprId>,
        args: &[HirCallArg],
        out: &mut Vec<HirExprId>,
    ) {
        let Some(callable) = callee.and_then(|c| self.ctx.world.get::<Callable>(c)) else {
            return;
        };
        if let (Some(r), Some(ReceiverKind::Consuming)) = (receiver, &callable.receiver) {
            out.push(r);
        }
        for (arg, param) in args.iter().zip(&callable.params) {
            if param.is_consuming {
                out.push(arg.value);
            }
        }
    }
}

impl OssaBodyCtx<'_, '_> {
    /// Take a planned projection (`p.a`, `t.0`, `p.a.b`) out of its owned var
    /// slot and record the path as moved. `None` when the root is not a
    /// tracked owned var with this path planned (a borrowed or captured
    /// root): the caller lowers it as an ordinary read.
    pub(crate) fn lower_partial_move(&mut self, expr: HirExprId) -> Option<ValueId> {
        let (local, path) = self.place_path(expr)?;
        match self.var_entry(local) {
            Some(ScopeEntry::Var {
                borrowed: false,
                paths,
                ..
            }) if paths.iter().any(|p| p.path == path) => {},
            _ => return None,
        }
        let addr = self.try_field_addr_chain(expr)?;
        let ty = self.resolve_expr_type(expr);
        let value = self.emit_take(addr, ty);
        self.record_var_path(local, &path, VarInit::DefUninit);
        Some(value)
    }

    /// `p = v` / `p.a = v` over a place with a moved-out part: drop what is
    /// still there (per field), then `StoreInit` — a `StoreAssign` would drop
    /// the moved field again. Returns `false` (nothing emitted) when the
    /// target is not partially moved.
    pub(crate) fn store_into_partially_moved(&mut self, target: HirExprId, rhs: ValueId) -> bool {
        let Some((local, path)) = self.place_path(target) else {
            return false;
        };
        if !self.var_partially_moved(local, &path) {
            return false;
        }
        let Some(ScopeEntry::Var {
            addr: slot,
            ty: slot_ty,
            init,
            flag,
            paths,
            ..
        }) = self.var_entry(local).cloned()
        else {
            return false;
        };
        let addr = match self.try_field_addr_chain(target) {
            Some(a) if !path.is_empty() => a,
            _ => self.whole_slot_addr(slot),
        };
        let ty = self.resolve_expr_type(target);
        // The state of the target place itself: its own tracked path, else
        // the nearest tracked ancestor, else the whole slot.
        let rest = paths
            .iter()
            .filter(|p| path.starts_with(&p.path))
            .max_by_key(|p| p.path.len())
            .map_or((init, flag), |p| (p.init, Some(p.flag)));
        let _ = slot_ty;
        self.drop_var_parts(addr, ty, &path, rest, &paths);
        self.emit_store_init(addr, rhs);
        if path.is_empty() {
            self.record_var_whole(local, VarInit::DefInit);
        } else {
            self.record_var_path(local, &path, VarInit::DefInit);
        }
        true
    }

    /// Drop every still-present part of the place at `prefix` (whose address
    /// is `base` and type `base_ty`), field by field in reverse declaration
    /// order — the drop shim's order. `rest` is the state (and presence flag)
    /// of parts no tracked path names. A fully present field is one
    /// `DestroyAddr`; a maybe-present one is flag-guarded; a moved one is
    /// skipped; a field with moved parts below it recurses.
    pub(crate) fn drop_var_parts(
        &mut self,
        base: ValueId,
        base_ty: TyId,
        prefix: &[FieldIdx],
        rest: (VarInit, Option<ValueId>),
        paths: &[PathState],
    ) {
        let field_tys = self.container_field_tys(base_ty);
        for (i, &fty) in field_tys.iter().enumerate().rev() {
            let mut child = prefix.to_vec();
            child.push(FieldIdx::new(i));
            let own = paths
                .iter()
                .find(|p| p.path == child)
                .map_or(rest, |p| (p.init, Some(p.flag)));
            if own.0 == VarInit::DefUninit {
                continue;
            }
            let field_addr = self.emit_field_addr(base, base_ty, FieldIdx::new(i));
            let split_below = paths.iter().any(|p| {
                p.path.len() > child.len() && p.path.starts_with(&child) && p.init != own.0
            });
            if split_below {
                self.drop_var_parts(field_addr, fty, &child, own, paths);
                continue;
            }
            match own {
                (VarInit::DefInit, _) => self.emit_destroy_addr(field_addr, fty),
                (VarInit::MaybeUninit, Some(flag)) => {
                    self.emit_guarded_destroy(flag, field_addr, fty, &[]);
                },
                // A maybe-moved part without a flag cannot be decided:
                // leak rather than risk a double drop (as whole vars do).
                _ => {},
            }
        }
    }

    /// Give each owned SSA param with planned partial moves a var slot.
    pub(crate) fn spill_partially_moved_params(&mut self, params_len: usize) {
        let params: Vec<HirLocalId> = self.hir.params.iter().take(params_len).copied().collect();
        for local in params {
            if self.partial.paths_of(local).is_empty() {
                continue;
            }
            let Some(super::LocalBinding::Ssa(value)) = self.local_map.get(&local).copied() else {
                continue;
            };
            if self.body.value(value).ownership != kestrel_mir::value::Ownership::Owned {
                continue;
            }
            let ty = self.body.value(value).ty;
            let addr = self.emit_uninit(ty);
            self.emit_store_init(addr, value);
            let name = self.hir.locals[local].name.clone();
            self.body.value_names.insert(addr, name);
            self.local_map.insert(local, super::LocalBinding::Var(addr));
            let flag = self.maybe_alloc_var_flag(ty);
            self.track_var(addr, ty, Some(local), flag);
        }
    }

    /// Field types of a struct or tuple, in declaration order.
    fn container_field_tys(&mut self, ty: TyId) -> Vec<TyId> {
        if let MirTy::Tuple(elems) = self.ctx.module.ty_arena.get(ty) {
            return elems.clone();
        }
        self.struct_field_tys(ty)
    }

    /// A var whose tracked field paths no longer all agree with the whole
    /// slot: scope exit must drop it field by field.
    pub(crate) fn var_entry_is_split(entry: &ScopeEntry) -> bool {
        match entry {
            ScopeEntry::Var {
                init,
                paths,
                borrowed: false,
                ..
            } => paths.iter().any(|p| p.init != *init),
            _ => false,
        }
    }
}
