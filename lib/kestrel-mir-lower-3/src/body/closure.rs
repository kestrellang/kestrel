//! Closure lowering — env struct, synthetic call function, ApplyPartial.
//!
//! Captures are decided by the post-inference `ClosureCaptures` query (see
//! kestrel-type-infer/src/captures.rs) — the single source of truth. This
//! module only *emits* that decision; it does not recompute captures.
//!
//! Each captured item is one of:
//! - a **whole local** (`Whole`) — captured by value if Copyable, else by
//!   reference (a `Pointer[T]` to a stack snapshot);
//! - a **projected place** (`Place`, e.g. `self.cap`) — always a Copyable,
//!   read-only field captured by value. Non-Copyable or written places fall
//!   back to whole-local capture (so behavior never regresses), because the
//!   by-reference projection of an arbitrary place isn't expressible yet.
//!
//! OSSA notes:
//! - Everything is ValueId, no Place/Operand/Rvalue.
//! - Whole-local captures bind into `local_map`; projected captures bind into
//!   `place_capture_map`, consulted when the body reads/borrows `self.cap`.
//! - Parent materializes a projected capture by walking the access chain from
//!   the real receiver (`StructExtract`), then copying the Copyable field —
//!   the receiver itself is never duplicated.

use std::collections::HashMap;
use std::mem;

use kestrel_hir::body::{HirBlock, HirClosureParam, HirExpr, HirExprId};
use kestrel_hir::res::LocalId as HirLocalId;
use kestrel_mir_3::body::OssaBody;
use kestrel_mir_3::item::function::{FunctionDef, FunctionKind, ParamDef};
use kestrel_mir_3::item::struct_def::{FieldDef, StructDef};
use kestrel_mir_3::value::{Ownership, ValueDef};
use kestrel_mir_3::callee::Callee;
use kestrel_mir_3::{FieldIdx, Immediate, MirTy, Op, ParamConvention, TyId, ValueId};
use kestrel_type_infer::captures::{CaptureKind, CapturedPlace};

use super::{LocalBinding, LoopInfo, OssaBodyCtx, ScopeFrame};

/// One resolved capture slot for a closure, after applying copyability to the
/// abstract `ClosureCaptures` plan.
enum CaptureSlot {
    /// Capture the whole local (by value if Copyable, else by reference).
    Whole(HirLocalId),
    /// Capture a Copyable, read-only projected place (e.g. `self.cap`) by value.
    Place(CapturedPlace),
}

/// Saved parent state during closure body lowering.
struct SavedState {
    body: OssaBody,
    current_block: Option<kestrel_mir_3::BlockId>,
    local_map: HashMap<HirLocalId, LocalBinding>,
    place_capture_map: HashMap<kestrel_type_infer::captures::PlaceKey, ValueId>,
    loop_stack: Vec<LoopInfo>,
    scope_stack: Vec<ScopeFrame>,
    tracker: super::LiveTracker,
    func_entity: kestrel_hecs::Entity,
    temp_counter: u32,
    body_context: super::BodyContext,
}

impl OssaBodyCtx<'_, '_> {
    pub fn lower_closure_expr(
        &mut self,
        expr_id: HirExprId,
        params: &[HirClosureParam],
        body: &HirBlock,
    ) -> ValueId {
        let closure_ty = self.resolve_expr_type(expr_id);

        // Captures decided post-inference; we only apply copyability here.
        let slots = self.plan_slots(expr_id);

        let closure_idx = self.ctx.closure_counter;
        self.ctx.closure_counter += 1;
        let parent_name = self
            .ctx
            .module
            .functions
            .get(&self.func_entity)
            .map(|f| f.name.clone())
            .unwrap_or_default();
        let closure_name = format!("{}.closure.{}", parent_name, closure_idx);

        // Determine param and return types from the closure's function type
        let (param_tys, ret_ty) = match self.ctx.module.ty_arena.get(closure_ty) {
            MirTy::FuncThick { params, ret } => {
                (params.iter().map(|(ty, _)| *ty).collect::<Vec<_>>(), *ret)
            },
            _ => {
                let p: Vec<TyId> = params
                    .iter()
                    .map(|p| self.resolve_local_type(p.local))
                    .collect();
                let unit = self.ctx.module.ty_arena.unit();
                (p, unit)
            },
        };

        // Per-slot capture type (whole-local type, or projected place type).
        let slot_tys: Vec<TyId> = slots.iter().map(|s| self.slot_ty(s)).collect();

        // Create env struct for captures
        let env_struct_entity = if !slots.is_empty() {
            let env_struct_name = format!("{}.env", closure_name);
            let env_entity = self.ctx.next_synthetic_entity();
            self.ctx.module.register_name(env_entity, &env_struct_name);

            let mut env_def = StructDef::new(env_entity, &env_struct_name);
            env_def.type_params = self
                .ctx
                .module
                .functions
                .get(&self.func_entity)
                .map(|f| f.type_params.clone())
                .unwrap_or_default();
            for (i, &cap_ty) in slot_tys.iter().enumerate() {
                // Non-copy (whole-local) captures are stored as pointers (by-ref).
                let field_ty = if self.is_copy_type(cap_ty) {
                    cap_ty
                } else {
                    self.ctx.module.ty_arena.pointer(cap_ty)
                };
                env_def.add_field(FieldDef::new(&format!("cap{i}"), field_ty));
            }
            let entity = env_def.entity;
            self.ctx.module.add_struct(env_def);
            Some(entity)
        } else {
            None
        };

        // Create closure function def
        let closure_entity = self.ctx.next_synthetic_entity();
        self.ctx.module.register_name(closure_entity, &closure_name);

        let mut func_def = FunctionDef::new(closure_entity, &closure_name, ret_ty);
        func_def.type_params = self
            .ctx
            .module
            .functions
            .get(&self.func_entity)
            .map(|f| f.type_params.clone())
            .unwrap_or_default();
        func_def.kind = if let Some(env_entity) = env_struct_entity {
            FunctionKind::ClosureCall {
                env_struct: env_entity,
            }
        } else {
            FunctionKind::Closure {
                parent_func: self.func_entity,
            }
        };

        // Build closure body
        let mut closure_body = OssaBody::new();

        // Env parameter — first value in the closure body.
        // Type is Pointer[EnvStruct] or Pointer[Unit] for no-capture closures.
        let env_ty = if let Some(env_entity) = env_struct_entity {
            let tp_entities: Vec<kestrel_hecs::Entity> = self
                .ctx
                .module
                .functions
                .get(&self.func_entity)
                .map(|f| f.type_params.iter().map(|tp| tp.entity).collect())
                .unwrap_or_default();
            let env_type_args: Vec<TyId> = tp_entities
                .iter()
                .map(|&e| self.ctx.intern(MirTy::TypeParam(e)))
                .collect();
            let named = self.ctx.module.ty_arena.named(env_entity, env_type_args);
            self.ctx.module.ty_arena.pointer(named)
        } else {
            let unit = self.ctx.module.ty_arena.unit();
            self.ctx.module.ty_arena.pointer(unit)
        };

        // Env param is the first ValueId (index 0) in the closure body
        let env_val = closure_body.alloc_value(ValueDef::owned(env_ty));
        func_def.params.push(ParamDef::new(
            "env",
            env_val,
            env_ty,
            ParamConvention::Consuming,
        ));
        closure_body.param_count += 1;

        // Closure params — sequential ValueIds after env
        let mut closure_local_map: HashMap<HirLocalId, LocalBinding> = HashMap::new();
        for (i, cp) in params.iter().enumerate() {
            let ty = param_tys
                .get(i)
                .copied()
                .unwrap_or_else(|| self.ctx.module.ty_arena.error());
            let val = closure_body.alloc_value(ValueDef::owned(ty));
            func_def.params.push(ParamDef::new(
                &self.hir.locals[cp.local].name,
                val,
                ty,
                ParamConvention::Consuming,
            ));
            closure_body.param_count += 1;
            closure_local_map.insert(cp.local, LocalBinding::Ssa(val));
        }

        // Entry block
        let entry_block = closure_body.alloc_block();
        closure_body.entry = entry_block;

        // Save parent state
        let saved = SavedState {
            body: mem::replace(&mut self.body, closure_body),
            current_block: self.current_block.take(),
            local_map: mem::replace(&mut self.local_map, closure_local_map),
            place_capture_map: mem::take(&mut self.place_capture_map),
            loop_stack: mem::take(&mut self.loop_stack),
            scope_stack: mem::take(&mut self.scope_stack),
            tracker: mem::replace(&mut self.tracker, super::LiveTracker::from_live(&[])),
            func_entity: self.func_entity,
            temp_counter: self.temp_counter,
            body_context: std::mem::replace(&mut self.body_context, super::BodyContext::Normal),
        };
        self.current_block = Some(entry_block);
        self.temp_counter = 0;
        // body_context already set to Normal by the mem::replace above
        self.push_scope();

        // Emit loads from env struct for captured slots.
        //
        // Copy-captures: env field is T — extract the value directly.
        // Ref-captures (non-Copyable whole locals): env field is Pointer[T] —
        // extract the pointer, then load through it to get the T value.
        if env_struct_entity.is_some() {
            // Load the env struct value from the env pointer
            let env_struct_ty = match self.ctx.module.ty_arena.get(env_ty) {
                MirTy::Pointer(inner) => *inner,
                _ => unreachable!("env_ty must be Pointer[EnvStruct]"),
            };
            let env_struct_val = self.emit_load(env_val, env_struct_ty);

            // Borrow the env struct for multi-field extraction.
            // Each extract from a borrow produces @guaranteed; we copy to get @owned.
            let env_borrow = self.emit_begin_borrow(env_struct_val);

            for (i, slot) in slots.iter().enumerate() {
                let cap_ty = slot_tys[i];
                let is_ref_capture = !self.is_copy_type(cap_ty);
                let field_ty = if is_ref_capture {
                    self.ctx.module.ty_arena.pointer(cap_ty)
                } else {
                    cap_ty
                };

                // Extract from borrow → @guaranteed, then copy → @owned
                let field_val = self.emit_struct_extract(env_borrow, FieldIdx::new(i), field_ty);
                let owned_field = self.emit_copy_value(field_val);

                let value = if is_ref_capture {
                    let loaded = self.emit_load(owned_field, cap_ty);
                    self.emit_destroy_value(owned_field);
                    loaded
                } else {
                    owned_field
                };

                match slot {
                    CaptureSlot::Whole(root) => {
                        self.local_map.insert(*root, LocalBinding::Ssa(value));
                    },
                    CaptureSlot::Place(cp) => {
                        self.place_capture_map.insert(cp.key.clone(), value);
                    },
                }
            }

            self.emit_end_borrow(env_borrow);
            self.emit_destroy_value(env_struct_val);
        }

        // Lower closure body
        let body_val = self.lower_hir_block(body);
        if !self.is_terminated() {
            self.destroy_scope_except(&[body_val]);
            self.emit_ret(body_val);
        }

        // Extract closure body and restore parent
        let completed_body = mem::replace(&mut self.body, saved.body);
        self.current_block = saved.current_block;
        self.local_map = saved.local_map;
        self.place_capture_map = saved.place_capture_map;
        self.loop_stack = saved.loop_stack;
        self.scope_stack = saved.scope_stack;
        self.tracker = saved.tracker;
        self.func_entity = saved.func_entity;
        self.temp_counter = saved.temp_counter;
        self.body_context = saved.body_context;

        // Attach body and register function
        func_def.body = Some(completed_body);
        self.ctx.module.add_function(func_def);

        // Emit ApplyPartial in parent scope — materialize each capture.
        let mut captures: Vec<ValueId> = Vec::new();
        for (i, slot) in slots.iter().enumerate() {
            let cap = self.materialize_capture(slot, slot_tys[i]);
            captures.push(cap);
        }

        // The closure inherits the parent's type params (same entities), so the
        // partial application binds them by identity. Carrying these type args
        // lets monomorphization resolve the closure/thunk to the correct
        // instance — without them every `read[T]` collapsed to the first thunk.
        let parent_tp_entities: Vec<kestrel_hecs::Entity> = self
            .ctx
            .module
            .functions
            .get(&self.func_entity)
            .map(|f| f.type_params.iter().map(|tp| tp.entity).collect())
            .unwrap_or_default();
        let type_args: Vec<TyId> = parent_tp_entities
            .iter()
            .map(|&e| self.ctx.intern(MirTy::TypeParam(e)))
            .collect();
        let callee = Callee::direct_with_args(closure_entity, type_args, None);
        self.emit_apply_partial(callee, captures, closure_ty)
    }

    // === Capture planning ===

    /// Translate the abstract capture plan into concrete slots, applying
    /// copyability (only the MIR layer knows it). A projected place is kept as
    /// a by-value place capture only when it is Copyable and read-only;
    /// otherwise the whole root is captured (preserving historical behavior).
    fn plan_slots(&mut self, closure_id: HirExprId) -> Vec<CaptureSlot> {
        let plan: Vec<CapturedPlace> = self.captures.get(closure_id).to_vec();

        // First pass: which roots must be captured whole.
        let mut whole_roots: std::collections::HashSet<HirLocalId> =
            std::collections::HashSet::new();
        for cp in &plan {
            if cp.key.is_whole() {
                whole_roots.insert(cp.key.root);
                continue;
            }
            let pty = self.resolve_expr_type(cp.repr);
            if !(self.is_copy_type(pty) && cp.kind == CaptureKind::Read) {
                whole_roots.insert(cp.key.root);
            }
        }

        // Second pass: build ordered, deduplicated slots (plan is already in a
        // deterministic order).
        let mut slots = Vec::new();
        let mut added_whole: std::collections::HashSet<HirLocalId> =
            std::collections::HashSet::new();
        for cp in plan {
            let root = cp.key.root;
            if !cp.key.is_whole() && !whole_roots.contains(&root) {
                slots.push(CaptureSlot::Place(cp));
            } else if added_whole.insert(root) {
                slots.push(CaptureSlot::Whole(root));
            }
        }
        slots
    }

    fn slot_ty(&mut self, slot: &CaptureSlot) -> TyId {
        match slot {
            CaptureSlot::Whole(root) => self.resolve_local_type(*root),
            CaptureSlot::Place(cp) => self.resolve_expr_type(cp.repr),
        }
    }

    /// Materialize a capture in the parent scope for packing into the env.
    fn materialize_capture(&mut self, slot: &CaptureSlot, cap_ty: TyId) -> ValueId {
        match slot {
            CaptureSlot::Whole(root) => self.materialize_whole(*root, cap_ty),
            CaptureSlot::Place(cp) => {
                // By-value projection of a Copyable field — copy the field,
                // never the receiver.
                let v = self.lower_place_value_parent(cp.repr);
                if self.body.value(v).ownership == kestrel_mir_3::value::Ownership::Guaranteed {
                    self.emit_copy_value(v)
                } else {
                    v
                }
            },
        }
    }

    fn materialize_whole(&mut self, root: HirLocalId, cap_ty: TyId) -> ValueId {
        let mir_val = self.map_local(root);
        if self.is_copy_type(cap_ty) {
            // Copy capture: snapshot the value. Var locals are address-based —
            // load the value from the address.
            if self.is_var_local(&root) {
                self.emit_copy_addr(mir_val, cap_ty)
            } else {
                self.emit_value_use(mir_val)
            }
        } else {
            // Ref capture: materialize the value into a stack slot and capture
            // the slot address (Pointer[cap_ty], matching the env field). How we
            // fill the slot depends on the captured value's ownership:
            //   - Var local   → load through its address.
            //   - @owned      → MOVE into the slot; the env now owns the value.
            //                   This is how an escaping closure keeps a
            //                   non-Copyable capture (e.g. a comparator) alive:
            //                   escaping closure params are `consuming`, so they
            //                   arrive @owned and are moved, never aliased.
            //   - @guaranteed → borrow-capture: store the borrowed bits directly
            //                   (no CopyValue — copying a non-Copyable @thick
            //                   value is illegal). The slot aliases the borrow's
            //                   storage. Sound only for a non-escaping closure
            //                   (e.g. the `and`/`or` short-circuit thunk that
            //                   captures a called-not-stored predicate); the
            //                   convention guarantees an escaping closure's
            //                   captures are @owned and take the move branch.
            let ptr_ty = self.ctx.module.ty_arena.pointer(cap_ty);
            let one = self.emit_literal(Immediate::i64(1));
            let addr = self.emit_op1(Op::StackAlloc(cap_ty), one, ptr_ty);
            if self.is_var_local(&root) {
                let value = self.emit_copy_addr(mir_val, cap_ty);
                self.emit_store_init(addr, value);
            } else if self.body.value(mir_val).ownership == Ownership::Owned {
                let value = self.emit_move_value(mir_val);
                self.emit_store_init(addr, value);
            } else {
                self.emit_store_init_borrowed(addr, mir_val);
            }
            addr
        }
    }

    /// Project a place value from the *real* receiver in the parent scope
    /// (walking `StructExtract`/`TupleExtract`), bypassing the body-side
    /// capture interception. Used only for by-value projected captures.
    fn lower_place_value_parent(&mut self, repr: HirExprId) -> ValueId {
        match self.hir.exprs[repr].clone() {
            HirExpr::Local(..) => self.lower_expr_for_borrow(repr),
            HirExpr::Field { base, name, .. } => {
                let base_val = self.lower_place_value_parent(base);
                let base_ty = self.resolve_expr_type(base);
                let result_ty = self.resolve_expr_type(repr);
                let struct_entity = match self.ctx.module.ty_arena.get(base_ty) {
                    MirTy::Named { entity, .. } => Some(*entity),
                    _ => None,
                };
                let field_idx = struct_entity
                    .and_then(|se| self.ctx.resolve_field_idx(se, name.as_str_or_empty()))
                    .unwrap_or_else(|| FieldIdx::new(0));
                self.emit_struct_extract(base_val, field_idx, result_ty)
            },
            HirExpr::TupleIndex { base, index, .. } => {
                let base_val = self.lower_place_value_parent(base);
                let result_ty = self.resolve_expr_type(repr);
                self.emit_tuple_extract(base_val, index, result_ty)
            },
            _ => self.lower_expr(repr),
        }
    }
}
