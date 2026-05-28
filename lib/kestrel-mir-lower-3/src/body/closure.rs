//! Closure lowering — env struct, synthetic call function, ApplyPartial.
//!
//! Uses save/restore via `SavedState` to swap OssaBodyCtx state for the
//! closure body. The closure body shares the parent's HirBody arena — we
//! can't create a fresh OssaBodyCtx without conflicting borrows.
//!
//! OSSA differences from MIR-2:
//! - Everything is ValueId, no Place/Operand/Rvalue.
//! - Env fields are read via `emit_struct_extract` (copy captures) or
//!   `emit_load` on a Pointer[T] extracted from the env (ref captures).
//! - Parent materializes ref captures via `emit_begin_borrow` → the
//!   resulting @guaranteed pointer is bitwise-copyable for ApplyPartial.
//! - `emit_apply_partial(entity, captures, ty) → ValueId`.

use std::collections::{HashMap, HashSet};
use std::mem;

use kestrel_hir::body::{
    HirBlock, HirClosureParam, HirExpr, HirExprId, HirStmt, HirStmtId,
};
use kestrel_hir::res::LocalId as HirLocalId;
use kestrel_mir_3::body::OssaBody;
use kestrel_mir_3::item::function::{FunctionDef, FunctionKind, ParamDef};
use kestrel_mir_3::item::struct_def::{FieldDef, StructDef};
use kestrel_mir_3::value::ValueDef;
use kestrel_mir_3::{
    FieldIdx, Immediate, MirTy, Op, ParamConvention, TyId, ValueId,
};

use super::{LocalBinding, LoopInfo, OssaBodyCtx, ScopeFrame};

/// Saved parent state during closure body lowering.
struct SavedState {
    body: OssaBody,
    current_block: Option<kestrel_mir_3::BlockId>,
    local_map: HashMap<HirLocalId, LocalBinding>,
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

        // Identify captured locals
        let closure_param_locals: HashSet<HirLocalId> =
            params.iter().map(|p| p.local).collect();
        let captured_locals = self.find_captures(body, &closure_param_locals);

        let closure_idx = self.ctx.closure_counter;
        self.ctx.closure_counter += 1;
        let parent_name = self.ctx.module.functions.get(&self.func_entity)
            .map(|f| f.name.clone())
            .unwrap_or_default();
        let closure_name = format!("{}.closure.{}", parent_name, closure_idx);

        // Determine param and return types from the closure's function type
        let (param_tys, ret_ty) = match self.ctx.module.ty_arena.get(closure_ty) {
            MirTy::FuncThick { params, ret } => (
                params.iter().map(|(ty, _)| *ty).collect::<Vec<_>>(),
                *ret,
            ),
            _ => {
                let p: Vec<TyId> = params
                    .iter()
                    .map(|p| self.resolve_local_type(p.local))
                    .collect();
                let unit = self.ctx.module.ty_arena.unit();
                (p, unit)
            }
        };

        // Create env struct for captures
        let env_struct_entity = if !captured_locals.is_empty() {
            let env_struct_name = format!("{}.env", closure_name);
            let env_entity = self.ctx.next_synthetic_entity();
            self.ctx.module.register_name(env_entity, &env_struct_name);

            let mut env_def = StructDef::new(env_entity, &env_struct_name);
            env_def.type_params = self.ctx.module.functions.get(&self.func_entity)
                .map(|f| f.type_params.clone())
                .unwrap_or_default();
            for &captured in &captured_locals {
                let cap_ty = self.resolve_local_type(captured);
                let cap_name = self.hir.locals[captured].name.clone();
                // Non-copy captures are stored as pointers (by-ref)
                let field_ty = if self.is_copy_type(cap_ty) {
                    cap_ty
                } else {
                    self.ctx.module.ty_arena.pointer(cap_ty)
                };
                env_def.add_field(FieldDef::new(&cap_name, field_ty));
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
        func_def.type_params = self.ctx.module.functions.get(&self.func_entity)
            .map(|f| f.type_params.clone())
            .unwrap_or_default();
        func_def.kind = if let Some(env_entity) = env_struct_entity {
            FunctionKind::ClosureCall { env_struct: env_entity }
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
            let tp_entities: Vec<kestrel_hecs::Entity> = self.ctx.module.functions
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
        func_def.params.push(ParamDef::new("env", env_val, env_ty, ParamConvention::Consuming));
        closure_body.param_count += 1;

        // Closure params — sequential ValueIds after env
        let mut closure_local_map: HashMap<HirLocalId, LocalBinding> = HashMap::new();
        for (i, cp) in params.iter().enumerate() {
            let ty = param_tys.get(i).copied().unwrap_or_else(|| self.ctx.module.ty_arena.error());
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

        // Allocate ValueIds for captured locals (will be loaded from env in entry)
        let mut capture_value_ids = Vec::new();
        for &captured in &captured_locals {
            let cap_ty = self.resolve_local_type(captured);
            let val = closure_body.alloc_value(ValueDef::owned(cap_ty));
            closure_local_map.insert(captured, LocalBinding::Ssa(val));
            capture_value_ids.push(val);
        }

        // Entry block
        let entry_block = closure_body.alloc_block();
        closure_body.entry = entry_block;

        // Save parent state
        let saved = SavedState {
            body: mem::replace(&mut self.body, closure_body),
            current_block: self.current_block.take(),
            local_map: mem::replace(&mut self.local_map, closure_local_map),
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

        // Emit loads from env struct for captured locals.
        //
        // Copy-captures: env field is T — extract the value directly via
        // StructExtract on the dereferenced env pointer.
        //
        // Ref-captures: env field is Pointer[T] — extract the pointer,
        // then load through it to get the T value.
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

            for (i, &captured) in captured_locals.iter().enumerate() {
                let capture_val = capture_value_ids[i];
                let cap_ty = self.body.value(capture_val).ty;
                let is_ref_capture = !self.is_copy_type(cap_ty);
                let field_ty = if is_ref_capture {
                    self.ctx.module.ty_arena.pointer(cap_ty)
                } else {
                    cap_ty
                };

                // Extract from borrow → @guaranteed, then copy → @owned
                let field_val = self.emit_struct_extract(env_borrow, FieldIdx::new(i), field_ty);
                let owned_field = self.emit_copy_value(field_val);

                if is_ref_capture {
                    let loaded = self.emit_load(owned_field, cap_ty);
                    self.local_map.insert(captured, LocalBinding::Ssa(loaded));
                    self.emit_destroy_value(owned_field);
                } else {
                    self.local_map.insert(captured, LocalBinding::Ssa(owned_field));
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
        self.loop_stack = saved.loop_stack;
        self.scope_stack = saved.scope_stack;
        self.tracker = saved.tracker;
        self.func_entity = saved.func_entity;
        self.temp_counter = saved.temp_counter;
        self.body_context = saved.body_context;

        // Attach body and register function
        func_def.body = Some(completed_body);
        self.ctx.module.add_function(func_def);

        // Emit ApplyPartial in parent scope.
        //
        // Copy captures pass the value directly (emit_value_use copies @owned).
        // Ref captures materialize a borrow — BeginBorrow produces a
        // @guaranteed pointer that is bitwise-trivial, suitable for packing
        // into the env struct.
        let mut captures: Vec<ValueId> = Vec::new();
        for &hir_local in &captured_locals {
            let mir_val = self.map_local(hir_local);
            let cap_ty = self.resolve_local_type(hir_local);
            if self.is_copy_type(cap_ty) {
                // Copy capture: snapshot the value.
                // Var locals are address-based — load the value from the address
                // so the closure captures the value, not the raw address pointer.
                if self.is_var_local(&hir_local) {
                    let loaded = self.emit_copy_addr(mir_val, cap_ty);
                    captures.push(loaded);
                } else {
                    let use_val = self.emit_value_use(mir_val);
                    captures.push(use_val);
                }
            } else {
                // Ref capture: copy value into a stack slot, capture the address.
                let ptr_ty = self.ctx.module.ty_arena.pointer(cap_ty);
                let one = self.emit_literal(Immediate::i64(1));
                let addr = self.emit_op1(Op::StackAlloc(cap_ty), one, ptr_ty);
                let copy = if self.is_var_local(&hir_local) {
                    self.emit_copy_addr(mir_val, cap_ty)
                } else {
                    self.emit_copy_value(mir_val)
                };
                self.emit_store_init(addr, copy);
                captures.push(addr);
            }
        }

        self.emit_apply_partial(closure_entity, captures, closure_ty)
    }

    // === Capture collection ===
    // These methods only read HIR data and the local_map — no MIR types.

    fn find_captures(
        &self,
        body: &HirBlock,
        closure_params: &HashSet<HirLocalId>,
    ) -> Vec<HirLocalId> {
        let mut captures = Vec::new();
        let mut seen = HashSet::new();
        self.collect_captures_block(body, closure_params, &mut captures, &mut seen);
        captures
    }

    fn collect_captures_block(
        &self,
        block: &HirBlock,
        closure_params: &HashSet<HirLocalId>,
        captures: &mut Vec<HirLocalId>,
        seen: &mut HashSet<HirLocalId>,
    ) {
        for &stmt_id in &block.stmts {
            self.collect_captures_stmt(stmt_id, closure_params, captures, seen);
        }
        if let Some(tail) = block.tail_expr {
            self.collect_captures_expr(tail, closure_params, captures, seen);
        }
    }

    fn collect_captures_stmt(
        &self,
        stmt_id: HirStmtId,
        closure_params: &HashSet<HirLocalId>,
        captures: &mut Vec<HirLocalId>,
        seen: &mut HashSet<HirLocalId>,
    ) {
        match &self.hir.stmts[stmt_id] {
            HirStmt::Let { value, .. } => {
                if let Some(expr) = value {
                    self.collect_captures_expr(*expr, closure_params, captures, seen);
                }
            }
            HirStmt::Expr { expr, .. } => {
                self.collect_captures_expr(*expr, closure_params, captures, seen);
            }
            HirStmt::Deinit { .. } => {}
        }
    }

    fn collect_captures_expr(
        &self,
        expr_id: HirExprId,
        closure_params: &HashSet<HirLocalId>,
        captures: &mut Vec<HirLocalId>,
        seen: &mut HashSet<HirLocalId>,
    ) {
        match &self.hir.exprs[expr_id] {
            HirExpr::Local(local_id, _) => {
                if !closure_params.contains(local_id)
                    && self.local_map.contains_key(local_id)
                    && seen.insert(*local_id)
                {
                    captures.push(*local_id);
                }
            }
            HirExpr::Call { callee, args, .. } => {
                self.collect_captures_expr(*callee, closure_params, captures, seen);
                for arg in args {
                    self.collect_captures_expr(arg.value, closure_params, captures, seen);
                }
            }
            HirExpr::MethodCall { receiver, args, .. }
            | HirExpr::ProtocolCall { receiver, args, .. } => {
                self.collect_captures_expr(*receiver, closure_params, captures, seen);
                for arg in args {
                    self.collect_captures_expr(arg.value, closure_params, captures, seen);
                }
            }
            HirExpr::Field { base, .. } | HirExpr::TupleIndex { base, .. } => {
                self.collect_captures_expr(*base, closure_params, captures, seen);
            }
            HirExpr::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                self.collect_captures_expr(*condition, closure_params, captures, seen);
                self.collect_captures_block(then_body, closure_params, captures, seen);
                if let Some(eb) = else_body {
                    self.collect_captures_block(eb, closure_params, captures, seen);
                }
            }
            HirExpr::Loop { body, .. } => {
                self.collect_captures_block(body, closure_params, captures, seen);
            }
            HirExpr::Match {
                scrutinee, arms, ..
            } => {
                self.collect_captures_expr(*scrutinee, closure_params, captures, seen);
                for arm in arms {
                    if let Some(guard) = arm.guard {
                        self.collect_captures_expr(guard, closure_params, captures, seen);
                    }
                    self.collect_captures_expr(arm.body, closure_params, captures, seen);
                }
            }
            HirExpr::Block { body, .. } => {
                self.collect_captures_block(body, closure_params, captures, seen);
            }
            HirExpr::Assign { target, value, .. } => {
                self.collect_captures_expr(*target, closure_params, captures, seen);
                self.collect_captures_expr(*value, closure_params, captures, seen);
            }
            HirExpr::Tuple { elements, .. } | HirExpr::Array { elements, .. } => {
                for &e in elements {
                    self.collect_captures_expr(e, closure_params, captures, seen);
                }
            }
            HirExpr::Dict { entries, .. } => {
                for entry in entries {
                    self.collect_captures_expr(entry.key, closure_params, captures, seen);
                    self.collect_captures_expr(entry.value, closure_params, captures, seen);
                }
            }
            HirExpr::Return { value, .. } => {
                if let Some(v) = value {
                    self.collect_captures_expr(*v, closure_params, captures, seen);
                }
            }
            HirExpr::ImplicitMember { args, .. } => {
                if let Some(call_args) = args {
                    for arg in call_args {
                        self.collect_captures_expr(arg.value, closure_params, captures, seen);
                    }
                }
            }
            HirExpr::Sugar { inner, .. } => {
                self.collect_captures_expr(*inner, closure_params, captures, seen);
            }
            HirExpr::Closure { body, .. } => {
                self.collect_captures_block(body, closure_params, captures, seen);
            }
            HirExpr::Literal { .. }
            | HirExpr::Def(..)
            | HirExpr::OverloadSet { .. }
            | HirExpr::Break { .. }
            | HirExpr::Continue { .. }
            | HirExpr::Error { .. } => {}
        }
    }
}
