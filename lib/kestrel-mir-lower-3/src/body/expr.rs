//! Expression lowering — HirExpr dispatch → ValueId.
//!
//! Every arm returns a `ValueId`. The key OSSA differences from MIR-2:
//! - No `Operand` / `Place` / `Rvalue` — everything is a `ValueId`.
//! - `emit_literal` instead of `lower_literal` (returns `ValueId` directly).
//! - `emit_struct_extract` / `emit_tuple_extract` instead of place projections.
//! - Calls use `Callee` + `Vec<CallArg>` instead of `Vec<(Operand, ArgMode)>`.
//!
//! Sub-modules handle the heavy lifting:
//! - `call` — call/method/protocol dispatch
//! - `control` — if/loop/break/continue/match
//! - `literal` — array/dict/string literal lowering
//! - `closure` — closure capture and thunk generation

use kestrel_ast_builder::{Callable, NodeKind, Settable};
use kestrel_hir::body::{HirCallArg, HirExpr, HirExprId};
use kestrel_mir_3::callee::Callee;
use kestrel_mir_3::inst::CallArg;
use kestrel_mir_3::item::witness::WitnessMethodKey;
use kestrel_mir_3::{FieldIdx, Immediate, MirTy, ParamConvention, ValueId};

use super::{OssaBodyCtx, expr_span};
use crate::ty::lower_resolved_ty;

impl OssaBodyCtx<'_, '_> {
    /// Lower an HIR expression to a ValueId, applying promotion if needed.
    pub fn lower_expr(&mut self, expr_id: HirExprId) -> ValueId {
        let value = self.lower_expr_no_promote(expr_id);
        self.apply_promotion(expr_id, value)
    }

    /// Apply a recorded `FromValue.from(value)` promotion if type-infer
    /// stored one for this expression.
    fn apply_promotion(&mut self, expr_id: HirExprId, value: ValueId) -> ValueId {
        let Some(typed) = self.typed.as_ref() else {
            return value;
        };
        let Some(promotion) = typed.promotions.get(&expr_id).cloned() else {
            return value;
        };
        let method = promotion.method;
        let target_ty = lower_resolved_ty(self.ctx, &promotion.target);
        self.ctx.register_name(method);
        let type_args = self.prepend_receiver_type_args(target_ty, vec![]);
        let callee = Callee::direct_with_args(method, type_args, None);
        let arg = self.prepare_call_arg(value, ParamConvention::Borrow);
        self.emit_call_returning(callee, vec![arg], target_ty)
    }

    fn lower_expr_no_promote(&mut self, expr_id: HirExprId) -> ValueId {
        let expr = self.hir.exprs[expr_id].clone();
        let span = expr_span(&self.hir, expr_id);
        let prev_span = self.current_span.replace(span);
        let result = self.lower_expr_inner(expr_id, &expr);
        self.current_span = prev_span;
        result
    }

    fn lower_expr_inner(&mut self, expr_id: HirExprId, expr: &HirExpr) -> ValueId {
        match expr {
            HirExpr::Literal { value, .. } => self.lower_literal(expr_id, value),

            HirExpr::Local(hir_local, _) => {
                if self.is_var_local(hir_local) {
                    let addr = self.map_local(*hir_local);
                    let ty = self.resolve_local_type(*hir_local);
                    let ownership = self.ownership_for(ty);
                    if ownership != kestrel_mir_3::value::Ownership::Owned {
                        return self.emit_load(addr, ty);
                    }
                    // A non-Copyable var read that reaches this value-producing
                    // (consuming) path can't be copied — move it out (Swift
                    // `load [take]`) and mark the slot uninitialized. Borrowing
                    // uses never reach here (they route through
                    // lower_expr_for_borrow / prepare_call_arg_for_expr). Copyable
                    // vars still snapshot via copy_addr.
                    if self.is_non_copyable(ty) {
                        debug_assert!(
                            self.var_init(*hir_local) != Some(super::VarInit::DefUninit),
                            "consuming read of an already-moved var — frontend should reject use-after-move"
                        );
                        let v = self.emit_take(addr, ty);
                        self.set_var_init(*hir_local, super::VarInit::DefUninit);
                        // Record the move in the in-memory drop flag (if any) so a
                        // later merge/scope-exit can reconcile divergent paths.
                        if let Some(flag) = self.var_flag(*hir_local) {
                            self.store_drop_flag(flag, false);
                        }
                        v
                    } else {
                        self.emit_copy_addr(addr, ty)
                    }
                } else {
                    let val = self.map_local(*hir_local);
                    self.emit_value_use(val)
                }
            },

            HirExpr::Tuple { elements, .. } => {
                let elems: Vec<ValueId> = elements.iter().map(|&e| self.lower_expr(e)).collect();
                let ty = self.resolve_expr_type(expr_id);
                self.emit_tuple(ty, elems)
            },

            HirExpr::Field { base, name, .. } => {
                // A captured projected place (e.g. `self.cap`) reads the env
                // value instead of projecting from a non-captured receiver.
                if let Some(v) = self.captured_place_value(expr_id) {
                    return self.emit_value_use(v);
                }
                self.lower_field_access(expr_id, *base, name.as_str_or_empty())
            },

            HirExpr::TupleIndex { base, index, .. } => {
                if let Some(v) = self.captured_place_value(expr_id) {
                    return self.emit_value_use(v);
                }
                let base_val = self.lower_expr_for_borrow(*base);
                let result_ty = self.resolve_expr_type(expr_id);
                self.emit_tuple_extract(base_val, *index, result_ty)
            },

            HirExpr::Def(entity, _type_args, _) => self.lower_def(expr_id, *entity),

            HirExpr::OverloadSet { candidates, .. } => {
                if let Some(&resolved) = self
                    .typed
                    .as_ref()
                    .and_then(|t| t.resolutions.get(&expr_id))
                {
                    self.ctx.register_name(resolved);
                    let type_args = self.resolve_type_args(expr_id);
                    self.emit_literal(Immediate::function_ref(resolved, type_args, None))
                } else if let Some(&first) = candidates.first() {
                    self.ctx.register_name(first);
                    self.emit_literal(Immediate::function_ref(first, vec![], None))
                } else {
                    self.emit_literal(Immediate::error())
                }
            },

            HirExpr::ImplicitMember { name, args, .. } => {
                self.lower_implicit_member(expr_id, name.as_str_or_empty(), args.as_deref())
            },

            // === Calls — delegate to call module (stubbed until Phase D) ===
            HirExpr::Call { callee, args, .. } => self.lower_call_expr(expr_id, *callee, args),
            HirExpr::MethodCall {
                receiver,
                method,
                type_args: hir_type_args,
                args,
                ..
            } => self.lower_method_call_expr(
                expr_id,
                *receiver,
                method.as_str_or_empty(),
                hir_type_args.as_deref(),
                args,
            ),
            HirExpr::ProtocolCall {
                receiver,
                protocol,
                method,
                type_args: hir_type_args,
                args,
                ..
            } => self.lower_protocol_call_expr(
                expr_id,
                *receiver,
                *protocol,
                method.as_str_or_empty(),
                hir_type_args.as_deref(),
                args,
            ),

            // === Control flow — delegate to control module (stubbed until Phase C) ===
            HirExpr::If {
                condition,
                then_body,
                else_body,
                ..
            } => self.lower_if(expr_id, *condition, then_body, else_body.as_ref()),

            HirExpr::Loop { body, label, .. } => self.lower_loop(body, label.as_deref()),
            HirExpr::Break { label, .. } => self.lower_break(label.as_deref()),
            HirExpr::Continue { label, .. } => self.lower_continue(label.as_deref()),

            HirExpr::Return { value, .. } => {
                let ret_val = if let Some(v) = value {
                    self.lower_expr(*v)
                } else {
                    self.emit_literal(Immediate::unit())
                };
                // @guaranteed values can't be returned — copy to @owned first
                let ret_val = if self.body.value(ret_val).ownership
                    == kestrel_mir_3::value::Ownership::Guaranteed
                {
                    let owned = self.emit_copy_value(ret_val);
                    self.emit_end_borrow(ret_val);
                    owned
                } else {
                    ret_val
                };
                self.drain_deferred_borrows();
                self.destroy_scopes_to_depth(0, &[ret_val]);
                self.emit_ret(ret_val);
                self.emit_literal(Immediate::unit())
            },

            HirExpr::Assign { target, value, .. } => self.lower_assign(expr_id, *target, *value),

            HirExpr::Match {
                scrutinee,
                arms,
                source,
                ..
            } => self.lower_match(expr_id, *scrutinee, arms, *source),

            // === Literals — delegate to literal module (stubbed until Phase E) ===
            HirExpr::Array { elements, .. } => self.lower_array_literal(expr_id, elements),
            HirExpr::Dict { entries, .. } => self.lower_dict_literal(expr_id, entries),

            // === Closure — delegate to closure module (stubbed until Phase E) ===
            HirExpr::Closure { params, body, .. } => self.lower_closure_expr(expr_id, params, body),

            HirExpr::Block { body, .. } => self.lower_hir_block(body),

            HirExpr::Sugar { inner, .. } => self.lower_expr(*inner),

            HirExpr::Error { .. } => self.emit_literal(Immediate::error()),
        }
    }

    // ================================================================
    // Field access
    // ================================================================

    pub(crate) fn lower_field_access(
        &mut self,
        expr_id: HirExprId,
        base: HirExprId,
        field_name: &str,
    ) -> ValueId {
        let resolved = self
            .typed
            .as_ref()
            .and_then(|t| t.resolutions.get(&expr_id))
            .copied();

        let is_callable = resolved.is_some_and(|e| self.ctx.world.get::<Callable>(e).is_some());
        let is_static = resolved.is_some_and(|e| {
            self.ctx
                .world
                .get::<kestrel_ast_builder::Static>(e)
                .is_some()
        });
        let is_protocol_property = !is_callable
            && resolved.is_some_and(|e| {
                matches!(self.ctx.world.get::<NodeKind>(e), Some(NodeKind::Field))
                    && self.ctx.world.parent_of(e).is_some_and(|p| {
                        matches!(self.ctx.world.get::<NodeKind>(p), Some(NodeKind::Protocol))
                    })
            });

        // Protocol property → witness dispatch
        if is_protocol_property {
            let property_entity = resolved.unwrap();
            let protocol = self.ctx.world.parent_of(property_entity).unwrap();
            self.ctx.register_name(protocol);
            let result_ty = self.resolve_expr_type(expr_id);
            let method_type_args = self.resolve_type_args(expr_id);
            if is_static {
                let self_type = self.type_from_type_ref(base);
                let callee = Callee::Witness {
                    protocol,
                    method: WitnessMethodKey::simple(field_name),
                    self_type,
                    method_type_args,
                };
                return self.emit_call_returning(callee, vec![], result_ty);
            } else {
                let receiver_ty = self.resolve_expr_type(base);
                let callee = Callee::Witness {
                    protocol,
                    method: WitnessMethodKey::simple(field_name),
                    self_type: receiver_ty,
                    method_type_args,
                };
                // Borrow the receiver place — a var-local base must not be copied.
                let arg = self.prepare_call_arg_for_expr(base, ParamConvention::Borrow);
                return self.emit_call_returning(callee, vec![arg], result_ty);
            }
        }

        // Computed property → getter call
        if is_callable {
            let getter_entity = resolved.unwrap();
            self.ctx.register_name(getter_entity);
            let result_ty = self.resolve_expr_type(expr_id);

            if is_static {
                let self_type = self.type_from_type_ref(base);
                let type_args = self.prepend_receiver_type_args(self_type, vec![]);
                let callee = Callee::direct_with_args(getter_entity, type_args, None);
                return self.emit_call_returning(callee, vec![], result_ty);
            }

            let receiver_ty = self.resolve_expr_type(base);
            let method_type_args = self.resolve_type_args(expr_id);
            // Borrow the receiver place — a var-local base must not be copied
            // (e.g. `buf.capacity` on a non-Copyable `var buf`).
            let arg = self.prepare_call_arg_for_expr(base, ParamConvention::Borrow);

            if let Some(protocol) = self.ctx.is_protocol_method(getter_entity) {
                self.ctx.register_name(protocol);
                let key = self.ctx.witness_method_key(getter_entity);
                let callee = Callee::Witness {
                    protocol,
                    method: key,
                    self_type: receiver_ty,
                    method_type_args,
                };
                return self.emit_call_returning(callee, vec![arg], result_ty);
            }

            let type_args = self.prepend_receiver_type_args(receiver_ty, method_type_args);
            let callee = Callee::direct_with_args(getter_entity, type_args, None);
            return self.emit_call_returning(callee, vec![arg], result_ty);
        }

        // Static stored field → global ref + load
        if is_static {
            let static_entity = resolved.unwrap();
            self.ctx.register_name(static_entity);
            let addr = self.emit_global_ref(static_entity);
            let ty = self.resolve_expr_type(expr_id);
            return self.emit_load(addr, ty);
        }

        // Stored field → StructExtract (borrow-based when base is @owned)
        let base_ty = self.resolve_expr_type(base);
        let result_ty = self.resolve_expr_type(expr_id);

        let struct_entity = match self.ctx.module.ty_arena.get(base_ty) {
            MirTy::Named { entity, .. } => Some(*entity),
            _ => None,
        };

        let field_idx = struct_entity
            .and_then(|se| self.ctx.resolve_field_idx(se, field_name))
            .unwrap_or_else(|| {
                debug_assert!(
                    false,
                    "ICE: stored field '{}' not found on struct {:?}",
                    field_name, struct_entity
                );
                FieldIdx::new(0)
            });

        // If the base roots at a var-local (a `mutating`/MutBorrow receiver or
        // a mutable local, bound to a stack address), project the field's
        // ADDRESS and read just the field. The fallback path below loads the
        // whole struct for a var-local base — an illegal copy when the struct
        // is non-Copyable, even if the field itself is Copyable (e.g.
        // `IntersperseIterator.next` reading `self.separator`). This is the
        // address-analog of `emit_struct_extract`: a @guaranteed place for a
        // non-Copyable field, and a clone (snapshot at read time) for a
        // Copyable one — the snapshot matters for read-then-mutate sequences
        // like `let v = self.x; self.x = self.x + 1` where `v` must be the old
        // value, not an alias of the now-written field.
        if let Some(base_addr) = self.try_field_addr_chain(base) {
            let field_addr = self.emit_field_addr(base_addr, base_ty, field_idx);
            if self.is_non_copyable(result_ty) {
                return self.emit_begin_borrow_addr(field_addr, result_ty);
            }
            return self.emit_copy_addr(field_addr, result_ty);
        }

        let base_val = self.lower_expr_for_borrow(base);
        self.emit_struct_extract(base_val, field_idx, result_ty)
    }

    // ================================================================
    // Def (entity reference)
    // ================================================================

    fn lower_def(&mut self, expr_id: HirExprId, entity: kestrel_hecs::Entity) -> ValueId {
        self.ctx.register_name(entity);
        let kind = self.ctx.world.get::<NodeKind>(entity).cloned();
        match kind {
            Some(NodeKind::Function | NodeKind::Initializer) => {
                let inferred_ty = self.resolve_expr_type(expr_id);
                // If the expression is used as a thick function value (closure),
                // wrap it in ApplyPartial with no captures.
                if matches!(
                    self.ctx.module.ty_arena.get(inferred_ty),
                    MirTy::FuncThick { .. }
                ) {
                    // Carry the entity's type args so monomorphization can resolve
                    // the partial application to the correct instance (same as the
                    // `function_ref` path below).
                    let type_args = self.resolve_type_args(expr_id);
                    let callee = Callee::direct_with_args(entity, type_args, None);
                    return self.emit_apply_partial(callee, vec![], inferred_ty);
                }
                let type_args = self.resolve_type_args(expr_id);
                self.emit_literal(Immediate::function_ref(entity, type_args, None))
            },
            Some(NodeKind::EnumCase) => {
                let ty = self.resolve_expr_type(expr_id);
                let case_name = self
                    .ctx
                    .world
                    .get::<kestrel_ast_builder::Name>(entity)
                    .map(|n| n.0.clone())
                    .unwrap_or_else(|| panic!("ICE: enum case {:?} has no Name", entity));
                let enum_entity = self
                    .ctx
                    .world
                    .parent_of(entity)
                    .unwrap_or_else(|| panic!("ICE: enum case {:?} has no parent", entity));
                let variant_idx = self
                    .ctx
                    .resolve_variant_idx(enum_entity, &case_name)
                    .unwrap_or_else(|| {
                        panic!(
                            "ICE: variant '{}' not found in enum {:?}",
                            case_name, enum_entity
                        )
                    });
                self.emit_enum_variant(ty, variant_idx, vec![])
            },
            Some(NodeKind::Field) => {
                if self.ctx.world.get::<Callable>(entity).is_some() {
                    // Computed property getter call (no receiver)
                    let result_ty = self.resolve_expr_type(expr_id);
                    let callee = Callee::direct_with_args(entity, vec![], None);
                    self.emit_call_returning(callee, vec![], result_ty)
                } else {
                    // Static stored field → load through global ref
                    let addr = self.emit_global_ref(entity);
                    let ty = self.resolve_expr_type(expr_id);
                    self.emit_load(addr, ty)
                }
            },
            Some(NodeKind::TypeParameter | NodeKind::TypeAlias) => {
                self.emit_literal(Immediate::unit())
            },
            _ => self.emit_literal(Immediate::error()),
        }
    }

    // ================================================================
    // Implicit member (.None, .Some(x), .fromResidual(x))
    // ================================================================

    fn lower_implicit_member(
        &mut self,
        expr_id: HirExprId,
        name: &str,
        args: Option<&[HirCallArg]>,
    ) -> ValueId {
        let result_ty = self.resolve_expr_type(expr_id);

        let resolved = self
            .typed
            .as_ref()
            .and_then(|t| t.resolutions.get(&expr_id))
            .copied();
        let is_enum_case =
            resolved.is_none_or(|e| self.ctx.world.get::<NodeKind>(e) == Some(&NodeKind::EnumCase));

        if is_enum_case {
            // Enum case with optional payload
            let payload: Vec<ValueId> = args
                .map(|a| a.iter().map(|arg| self.lower_expr(arg.value)).collect())
                .unwrap_or_default();

            let enum_entity = match self.ctx.module.ty_arena.get(result_ty) {
                MirTy::Named { entity, .. } => *entity,
                other => panic!(
                    "ICE: enum case '{}' result type is not Named: {:?}",
                    name, other
                ),
            };
            let variant_idx = self
                .ctx
                .resolve_variant_idx(enum_entity, name)
                .unwrap_or_else(|| {
                    panic!(
                        "ICE: variant '{}' not found in enum {:?}",
                        name, enum_entity
                    )
                });

            self.emit_enum_variant(result_ty, variant_idx, payload)
        } else {
            // Static method call (e.g., .fromResidual)
            let resolved_entity = resolved.unwrap();
            self.ctx.register_name(resolved_entity);
            let call_args: Vec<CallArg> = args
                .map(|a| {
                    a.iter()
                        .map(|arg| {
                            let val = self.lower_expr(arg.value);
                            self.prepare_call_arg(val, ParamConvention::Borrow)
                        })
                        .collect()
                })
                .unwrap_or_default();

            if let Some(protocol) = self.ctx.is_protocol_method(resolved_entity) {
                self.ctx.register_name(protocol);
                let key = self.ctx.witness_method_key(resolved_entity);
                let type_args = self.resolve_type_args(expr_id);
                let callee = Callee::Witness {
                    protocol,
                    method: key,
                    self_type: result_ty,
                    method_type_args: type_args,
                };
                self.emit_call_returning(callee, call_args, result_ty)
            } else {
                let mut type_args = self.resolve_type_args(expr_id);
                // Static methods on generic types need the parent's type args
                type_args = self.prepend_receiver_type_args(result_ty, type_args);
                let self_type = if !type_args.is_empty() {
                    Some(result_ty)
                } else {
                    None
                };
                let callee = Callee::direct_with_args(resolved_entity, type_args, self_type);
                self.emit_call_returning(callee, call_args, result_ty)
            }
        }
    }

    // ================================================================
    // Assign
    // ================================================================

    fn lower_assign(
        &mut self,
        _expr_id: HirExprId,
        target: HirExprId,
        value: HirExprId,
    ) -> ValueId {
        // Setter dispatch: computed properties, subscripts, field-subscripts
        if let Some(result) = self.try_lower_setter_assign(target, value) {
            return result;
        }

        // Simple stored assignment
        let rhs = self.lower_expr(value);
        let target_expr = self.hir.exprs[target].clone();
        match target_expr {
            HirExpr::Local(hir_local, _) => {
                if self.is_var_local(&hir_local) {
                    // `rhs` is lowered above (rhs-first) so `x = f(x)` is correct.
                    match self.var_init(hir_local) {
                        // Moved-out on all paths: slot is empty, just StoreInit.
                        Some(super::VarInit::DefUninit) => {
                            let addr = self.local_map[&hir_local].value();
                            self.emit_store_init(addr, rhs);
                            self.set_var_init(hir_local, super::VarInit::DefInit);
                        },
                        // Moved on some paths only: flag-guarded drop of the old
                        // value, then StoreInit. `emit_store_assign` would
                        // unconditionally drop the (possibly moved-out) slot.
                        Some(super::VarInit::MaybeUninit) => {
                            let ty = self.resolve_local_type(hir_local);
                            let flag = self
                                .var_flag(hir_local)
                                .expect("MaybeUninit var must have a drop flag");
                            let addr = self.local_map[&hir_local].value();
                            let remapped = self.emit_guarded_destroy(flag, addr, ty, &[rhs]);
                            let rhs = remapped[0];
                            let addr = self.local_map[&hir_local].value();
                            self.emit_store_init(addr, rhs);
                            self.store_drop_flag(flag, true);
                            self.set_var_init(hir_local, super::VarInit::DefInit);
                        },
                        // Definitely initialized: StoreAssign drops the old value.
                        _ => {
                            let addr = self.local_map[&hir_local].value();
                            self.emit_store_assign(addr, rhs);
                        },
                    }
                } else {
                    let old_val = self.local_map.get(&hir_local).map(|b| b.value());
                    if let Some(old) = old_val {
                        let ownership = self.body.value(old).ownership;
                        if ownership == kestrel_mir_3::value::Ownership::Owned {
                            self.emit_destroy_value(old);
                        }
                        self.tracker.rebind(&[old], &[rhs]);
                    }
                    self.local_map
                        .insert(hir_local, super::LocalBinding::Ssa(rhs));
                }
            },
            HirExpr::Field {
                ref base, ref name, ..
            } => {
                let base = *base;
                let field_name = name.as_str_or_empty().to_string();
                let base_ty = self.resolve_expr_type(base);

                let struct_entity = match self.ctx.module.ty_arena.get(base_ty) {
                    kestrel_mir_3::MirTy::Named { entity, .. } => Some(*entity),
                    _ => None,
                };
                let field_idx = struct_entity
                    .and_then(|e| self.ctx.resolve_field_idx(e, &field_name))
                    .unwrap_or_else(|| {
                        debug_assert!(
                            false,
                            "ICE: stored field '{}' not found on struct {:?}",
                            field_name, struct_entity
                        );
                        kestrel_mir_3::FieldIdx::new(0)
                    });

                if let Some(base_addr) = self.try_field_addr_chain(base) {
                    let field_addr = self.emit_field_addr(base_addr, base_ty, field_idx);
                    // In init bodies, self fields are uninitialized — use store_init.
                    let is_init_self = self.body_context.init_self_addr() == Some(base_addr);
                    if is_init_self {
                        self.emit_store_init(field_addr, rhs);
                    } else {
                        self.emit_store_assign(field_addr, rhs);
                    }
                } else {
                    let base_val = self.lower_expr(base);
                    let base_addr = self.emit_begin_mut_borrow(base_val);
                    let addr = self.emit_field_addr(base_addr, base_ty, field_idx);
                    self.emit_store_init(addr, rhs);
                    self.emit_end_mut_borrow(base_addr);
                }
            },
            HirExpr::Def(entity, _, _) => {
                // Static/global stored field: covers both `static var` members
                // and module-level globals (which lack the Static component).
                let is_global = self
                    .ctx
                    .world
                    .get::<kestrel_ast_builder::Static>(entity)
                    .is_some()
                    || self.ctx.module.statics.contains_key(&entity);
                if is_global {
                    self.ctx.register_name(entity);
                    let addr = self.emit_global_ref(entity);
                    self.emit_store_assign(addr, rhs);
                }
            },
            _ => {},
        }
        self.emit_literal(Immediate::unit())
    }

    // ----------------------------------------------------------------
    // Setter dispatch
    // ----------------------------------------------------------------

    /// If the assignment target is a computed property or subscript with a
    /// setter, emit a setter call and return unit. Otherwise return None
    /// so the caller falls through to stored assignment.
    fn try_lower_setter_assign(
        &mut self,
        target_id: HirExprId,
        value_id: HirExprId,
    ) -> Option<ValueId> {
        let target = self.hir.exprs[target_id].clone();
        match target {
            HirExpr::Field { base, name, .. } => {
                self.try_lower_field_setter(target_id, value_id, base, name.as_str_or_empty())
            },
            HirExpr::Def(entity, _, _) => self.try_lower_def_setter(value_id, entity),
            HirExpr::Call { callee, args, .. } => {
                self.try_lower_call_setter(target_id, value_id, callee, &args)
            },
            HirExpr::MethodCall {
                receiver,
                method,
                args,
                ..
            } => self.try_lower_method_call_setter(
                target_id,
                value_id,
                receiver,
                method.as_str_or_empty(),
                &args,
            ),
            _ => None,
        }
    }

    /// Arm 1: `obj.computedProp = v` / `Type.staticProp = v` / protocol property setter
    fn try_lower_field_setter(
        &mut self,
        target_id: HirExprId,
        value_id: HirExprId,
        base: HirExprId,
        field_name: &str,
    ) -> Option<ValueId> {
        let resolved = self
            .typed
            .as_ref()
            .and_then(|t| t.resolutions.get(&target_id))
            .copied()?;

        let is_field = self.ctx.world.get::<NodeKind>(resolved) == Some(&NodeKind::Field);
        let parent_is_protocol =
            is_field
                && self.ctx.world.parent_of(resolved).is_some_and(|p| {
                    self.ctx.world.get::<NodeKind>(p) == Some(&NodeKind::Protocol)
                });
        let has_callable = self.ctx.world.get::<Callable>(resolved).is_some();
        let has_settable = self.ctx.world.get::<Settable>(resolved).is_some();

        // Protocol property setter → witness dispatch with "{name}.set"
        if parent_is_protocol && !has_callable && has_settable {
            let protocol = self.ctx.world.parent_of(resolved).unwrap();
            self.ctx.register_name(protocol);
            let is_static = self
                .ctx
                .world
                .get::<kestrel_ast_builder::Static>(resolved)
                .is_some();
            let rhs = self.lower_expr(value_id);
            let method_type_args = self.resolve_type_args(target_id);
            let method = WitnessMethodKey::simple(format!("{field_name}.set"));

            if is_static {
                let self_type = self.type_from_type_ref(base);
                let rhs_arg = self.prepare_call_arg(rhs, ParamConvention::Borrow);
                let callee = Callee::Witness {
                    protocol,
                    method,
                    self_type,
                    method_type_args,
                };
                self.emit_call_void(callee, vec![rhs_arg]);
            } else {
                let receiver_ty = self.resolve_expr_type(base);
                let receiver_arg = self.prepare_call_arg_for_expr(base, ParamConvention::MutBorrow);
                let rhs_arg = self.prepare_call_arg(rhs, ParamConvention::Borrow);
                let callee = Callee::Witness {
                    protocol,
                    method,
                    self_type: receiver_ty,
                    method_type_args,
                };
                self.emit_call_void(callee, vec![receiver_arg, rhs_arg]);
            }
            return Some(self.emit_literal(Immediate::unit()));
        }

        // Concrete computed property setter
        let setter = self.ctx.find_setter_child(resolved)?;
        self.ctx.register_name(setter);
        let is_static = self
            .ctx
            .world
            .get::<kestrel_ast_builder::Static>(resolved)
            .is_some();
        let rhs = self.lower_expr(value_id);

        if is_static {
            let self_type = self.type_from_type_ref(base);
            let type_args = self.prepend_receiver_type_args(self_type, vec![]);
            let rhs_arg = self.prepare_call_arg(rhs, ParamConvention::Borrow);
            let callee = Callee::direct_with_args(setter, type_args, None);
            self.emit_call_void(callee, vec![rhs_arg]);
        } else {
            let receiver_ty = self.resolve_expr_type(base);
            let type_args = self.resolve_type_args(target_id);
            let type_args = self.prepend_receiver_type_args(receiver_ty, type_args);

            if let Some(protocol) = self.ctx.is_protocol_method(setter) {
                self.ctx.register_name(protocol);
                let key = self.ctx.witness_setter_key(setter);
                let receiver_arg = self.prepare_call_arg_for_expr(base, ParamConvention::MutBorrow);
                let rhs_arg = self.prepare_call_arg(rhs, ParamConvention::Borrow);
                let callee = Callee::Witness {
                    protocol,
                    method: key,
                    self_type: receiver_ty,
                    method_type_args: type_args,
                };
                self.emit_call_void(callee, vec![receiver_arg, rhs_arg]);
            } else {
                let receiver_arg = self.prepare_call_arg_for_expr(base, ParamConvention::MutBorrow);
                let rhs_arg = self.prepare_call_arg(rhs, ParamConvention::Borrow);
                let callee = Callee::direct_with_args(setter, type_args, None);
                self.emit_call_void(callee, vec![receiver_arg, rhs_arg]);
            }
        }
        Some(self.emit_literal(Immediate::unit()))
    }

    /// Arm 2: `globalComputedProp = v`
    fn try_lower_def_setter(
        &mut self,
        value_id: HirExprId,
        entity: kestrel_hecs::Entity,
    ) -> Option<ValueId> {
        let setter = self.ctx.find_setter_child(entity)?;
        self.ctx.register_name(setter);
        let rhs = self.lower_expr(value_id);
        let rhs_arg = self.prepare_call_arg(rhs, ParamConvention::Borrow);
        let callee = Callee::direct_with_args(setter, vec![], None);
        self.emit_call_void(callee, vec![rhs_arg]);
        Some(self.emit_literal(Immediate::unit()))
    }

    /// Arm 3: `h(i) = v` — subscript setter
    fn try_lower_call_setter(
        &mut self,
        target_id: HirExprId,
        value_id: HirExprId,
        callee_expr: HirExprId,
        args: &[HirCallArg],
    ) -> Option<ValueId> {
        let resolved = self
            .typed
            .as_ref()
            .and_then(|t| t.resolutions.get(&target_id))
            .copied()?;
        if self.ctx.world.get::<NodeKind>(resolved) != Some(&NodeKind::Subscript) {
            return None;
        }

        let setter = self.ctx.find_setter_child(resolved)?;
        self.ctx.register_name(setter);
        let is_static = self
            .ctx
            .world
            .get::<kestrel_ast_builder::Static>(resolved)
            .is_some();
        let subscript_args: Vec<ValueId> = args.iter().map(|a| self.lower_expr(a.value)).collect();
        let rhs = self.lower_expr(value_id);

        if is_static {
            let self_type = self.type_from_type_ref(callee_expr);
            let type_args = self.prepend_receiver_type_args(self_type, vec![]);
            let mut call_args: Vec<CallArg> = subscript_args
                .into_iter()
                .map(|v| self.prepare_call_arg(v, ParamConvention::Borrow))
                .collect();
            call_args.push(self.prepare_call_arg(rhs, ParamConvention::Borrow));
            let callee = Callee::direct_with_args(setter, type_args, None);
            self.emit_call_void(callee, call_args);
        } else {
            let receiver_ty = self.resolve_expr_type(callee_expr);
            let type_args = self.resolve_type_args(target_id);

            // Use prepare_call_arg_for_expr to get the var address directly
            // (via try_var_addr) instead of lower_expr which emits CopyAddr —
            // a shallow copy that breaks COW refcounting.
            let mut call_args =
                vec![self.prepare_call_arg_for_expr(callee_expr, ParamConvention::MutBorrow)];
            for v in subscript_args {
                call_args.push(self.prepare_call_arg(v, ParamConvention::Borrow));
            }
            call_args.push(self.prepare_call_arg(rhs, ParamConvention::Borrow));

            if let Some(protocol) = self.ctx.is_protocol_method(setter) {
                self.ctx.register_name(protocol);
                let key = self.ctx.witness_setter_key(setter);
                let callee = Callee::Witness {
                    protocol,
                    method: key,
                    self_type: receiver_ty,
                    method_type_args: type_args,
                };
                self.emit_call_void(callee, call_args);
            } else {
                let type_args = self.prepend_receiver_type_args(receiver_ty, type_args);
                let callee = Callee::direct_with_args(setter, type_args, None);
                self.emit_call_void(callee, call_args);
            }
        }
        Some(self.emit_literal(Immediate::unit()))
    }

    /// Arm 4: `obj.field(i) = v` — subscript setter through a struct field
    fn try_lower_method_call_setter(
        &mut self,
        target_id: HirExprId,
        value_id: HirExprId,
        receiver: HirExprId,
        method_name: &str,
        args: &[HirCallArg],
    ) -> Option<ValueId> {
        let resolved = self
            .typed
            .as_ref()
            .and_then(|t| t.resolutions.get(&target_id))
            .copied()?;
        if self.ctx.world.get::<NodeKind>(resolved) != Some(&NodeKind::Subscript) {
            return None;
        }

        let receiver_ty = self.resolve_expr_type(receiver);
        let receiver_entity = match self.ctx.module.ty_arena.get(receiver_ty) {
            MirTy::Named { entity, .. } => Some(*entity),
            _ => None,
        };
        let subscript_parent = self.ctx.world.parent_of(resolved);

        // Only handle subscript-on-field — if the subscript belongs directly
        // to the receiver type, the Call arm handles it.
        if subscript_parent.is_none() || subscript_parent == receiver_entity {
            return None;
        }

        // Skip computed-property subscript-set (stored fields only)
        let prefix_entity = receiver_entity.and_then(|recv| {
            self.ctx.world.children_of(recv).iter().copied().find(|&c| {
                self.ctx.world.get::<NodeKind>(c) == Some(&NodeKind::Field)
                    && self
                        .ctx
                        .world
                        .get::<kestrel_ast_builder::Name>(c)
                        .is_some_and(|n| n.0 == method_name)
            })
        });
        let is_computed_property =
            prefix_entity.is_some_and(|e| self.ctx.world.get::<Callable>(e).is_some());
        if is_computed_property {
            return None;
        }

        let setter = self.ctx.find_setter_child(resolved)?;
        self.ctx.register_name(setter);

        // Resolve field type and extract through it
        let recv_entity = receiver_entity?;
        let field_idx = self.ctx.resolve_field_idx(recv_entity, method_name)?;
        let mut field_ty = self
            .ctx
            .module
            .structs
            .get(&recv_entity)
            .and_then(|s| s.fields.get(field_idx.index()))
            .map(|f| f.ty)?;

        // Substitute struct type params → receiver's concrete type args
        if let MirTy::Named { type_args, .. } = self.ctx.module.ty_arena.get(receiver_ty) {
            let type_args = type_args.clone();
            if let Some(sdef) = self.ctx.module.structs.get(&recv_entity) {
                let mut subst = kestrel_mir_3::SubstMap::new();
                for (tp, &arg) in sdef.type_params.iter().zip(type_args.iter()) {
                    subst.type_params.insert(tp.entity, arg);
                }
                field_ty =
                    kestrel_mir_3::substitute(&mut self.ctx.module.ty_arena, field_ty, &subst);
            }
        }

        // Get the receiver's field address directly via try_var_addr + field_addr,
        // then MutBorrow the field. Avoids shallow CopyAddr that breaks COW refcounting.
        let subscript_args: Vec<ValueId> = args.iter().map(|a| self.lower_expr(a.value)).collect();
        let rhs = self.lower_expr(value_id);
        let type_args = self.resolve_type_args(target_id);

        let field_arg = if let Some(recv_addr) = self.try_var_addr(receiver) {
            let field_addr = self.emit_field_addr(recv_addr, receiver_ty, field_idx);
            let borrow = self.emit_begin_mut_borrow_addr(field_addr, field_ty);
            CallArg {
                value: borrow,
                convention: ParamConvention::MutBorrow,
            }
        } else {
            let receiver_val = self.lower_expr(receiver);
            let field_val = self.emit_struct_extract(receiver_val, field_idx, field_ty);
            self.prepare_call_arg(field_val, ParamConvention::MutBorrow)
        };

        let mut call_args = vec![field_arg];
        for v in subscript_args {
            call_args.push(self.prepare_call_arg(v, ParamConvention::Borrow));
        }
        call_args.push(self.prepare_call_arg(rhs, ParamConvention::Borrow));

        if let Some(protocol) = self.ctx.is_protocol_method(setter) {
            self.ctx.register_name(protocol);
            let key = self.ctx.witness_setter_key(setter);
            let callee = Callee::Witness {
                protocol,
                method: key,
                self_type: field_ty,
                method_type_args: type_args,
            };
            self.emit_call_void(callee, call_args);
        } else {
            let type_args = self.prepend_receiver_type_args(field_ty, type_args);
            let callee = Callee::direct_with_args(setter, type_args, None);
            self.emit_call_void(callee, call_args);
        }
        Some(self.emit_literal(Immediate::unit()))
    }
}
