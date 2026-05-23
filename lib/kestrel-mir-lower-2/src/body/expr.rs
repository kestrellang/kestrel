//! Expression lowering — HirExpr dispatch.

use kestrel_ast_builder::{Callable, NodeKind};
use kestrel_hir::body::{HirBlock, HirExpr, HirExprId};
use kestrel_mir_2::{
    FieldIdx, Immediate, MirTy, Operand, Place, Rvalue, TyId, UseMode,
};

use super::{BodyCtx, expr_span};
use crate::ty::{lower_resolved_ty, lower_type};

impl BodyCtx<'_, '_> {
    /// Lower an HIR expression to a mode-free Operand.
    pub fn lower_expr(&mut self, expr_id: HirExprId) -> Operand {
        let operand = self.lower_expr_no_promote(expr_id);
        self.apply_promotion(expr_id, operand)
    }

    /// Apply a recorded `FromValue.from(value)` promotion if type-infer
    /// stored one for this expression.
    fn apply_promotion(&mut self, expr_id: HirExprId, operand: Operand) -> Operand {
        let Some(typed) = self.typed.as_ref() else {
            return operand;
        };
        let Some(promotion) = typed.promotions.get(&expr_id) else {
            return operand;
        };
        let method = promotion.method;
        let target_ty = lower_resolved_ty(self.ctx, &promotion.target);
        self.ctx.register_name(method);
        let type_args = self.prepend_receiver_type_args(target_ty, vec![]);
        let callee = kestrel_mir_2::Callee::direct_with_args(method, type_args, None);
        self.emit_call_returning(callee, vec![(operand, kestrel_mir_2::ArgMode::Copy)], target_ty)
    }

    fn lower_expr_no_promote(&mut self, expr_id: HirExprId) -> Operand {
        let expr = self.hir.exprs[expr_id].clone();
        let span = expr_span(&self.hir, expr_id);
        let prev_span = self.current_span.replace(span);
        let result = self.lower_expr_inner(expr_id, &expr);
        self.current_span = prev_span;
        result
    }

    fn lower_expr_inner(&mut self, expr_id: HirExprId, expr: &HirExpr) -> Operand {
        match expr {
            HirExpr::Literal { value, .. } => self.lower_literal(expr_id, value),

            HirExpr::Local(hir_local, _) => {
                Operand::Place(Place::local(self.map_local(*hir_local)))
            }

            HirExpr::Tuple { elements, .. } => {
                let elems: Vec<(Operand, UseMode)> = elements
                    .iter()
                    .map(|&e| {
                        let op = self.lower_expr(e);
                        let ty = self.resolve_expr_type(e);
                        (op, self.use_mode_for(ty))
                    })
                    .collect();
                let ty = self.resolve_expr_type(expr_id);
                let dest = self.fresh_temp(ty);
                self.emit_tuple(Place::local(dest), elems);
                Operand::Place(Place::local(dest))
            }

            HirExpr::Field { base, name, .. } => {
                self.lower_field_access(expr_id, *base, name.as_str_or_empty())
            }

            HirExpr::TupleIndex { base, index, .. } => {
                let base_op = self.lower_expr(*base);
                let base_ty = self.resolve_expr_type(*base);
                let place = self.operand_to_place(base_op, base_ty);
                Operand::Place(place.tuple_index(*index))
            }

            HirExpr::Def(entity, _type_args, _) => self.lower_def(expr_id, *entity),

            HirExpr::OverloadSet { candidates, .. } => {
                if let Some(&resolved) = self.typed.as_ref().and_then(|t| t.resolutions.get(&expr_id)) {
                    self.ctx.register_name(resolved);
                    let type_args = self.resolve_type_args(expr_id);
                    Operand::Const(Immediate::function_ref(resolved, type_args, None))
                } else if let Some(&first) = candidates.first() {
                    self.ctx.register_name(first);
                    Operand::Const(Immediate::function_ref(first, vec![], None))
                } else {
                    Operand::Const(Immediate::error())
                }
            }

            HirExpr::ImplicitMember { name, args, .. } => {
                self.lower_implicit_member(expr_id, name.as_str_or_empty(), args.as_deref())
            }

            HirExpr::Call { callee, args, .. } => {
                self.lower_call_expr(expr_id, *callee, args)
            }
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
                args,
                ..
            } => self.lower_protocol_call_expr(
                expr_id,
                *receiver,
                *protocol,
                method.as_str_or_empty(),
                args,
            ),

            HirExpr::If {
                condition,
                then_body,
                else_body,
                ..
            } => self.lower_if(expr_id, *condition, then_body, else_body.as_ref()),

            HirExpr::Loop { body, label, .. } => {
                self.lower_loop(body, label.as_deref())
            }
            HirExpr::Break { label, .. } => self.lower_break(label.as_deref()),
            HirExpr::Continue { label, .. } => self.lower_continue(label.as_deref()),

            HirExpr::Return { value, .. } => {
                let ret_val = value
                    .map(|v| self.lower_expr(v))
                    .unwrap_or(Operand::Const(Immediate::unit()));
                self.emit_ret(ret_val);
                Operand::Const(Immediate::unit())
            }

            HirExpr::Assign { target, value, .. } => {
                self.lower_assign(expr_id, *target, *value)
            }

            HirExpr::Match {
                scrutinee, arms, ..
            } => self.lower_match(expr_id, *scrutinee, arms),

            HirExpr::Array { elements, .. } => {
                self.lower_array_literal(expr_id, elements)
            }
            HirExpr::Dict { entries, .. } => {
                self.lower_dict_literal(expr_id, entries)
            }

            HirExpr::Closure { params, body, .. } => {
                self.lower_closure_expr(expr_id, params, body)
            }

            HirExpr::Block { body, .. } => self.lower_hir_block(body),

            HirExpr::Sugar { inner, .. } => self.lower_expr(*inner),

            HirExpr::Error { .. } => Operand::Const(Immediate::error()),
        }
    }

    // === Field access ===

    fn lower_field_access(
        &mut self,
        expr_id: HirExprId,
        base: HirExprId,
        field_name: &str,
    ) -> Operand {
        let resolved = self
            .typed
            .as_ref()
            .and_then(|t| t.resolutions.get(&expr_id))
            .copied();

        let is_callable = resolved.is_some_and(|e| {
            self.ctx.world.get::<Callable>(e).is_some()
        });
        let is_static = resolved.is_some_and(|e| {
            self.ctx.world.get::<kestrel_ast_builder::Static>(e).is_some()
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
                let callee = kestrel_mir_2::Callee::Witness {
                    protocol,
                    method: kestrel_mir_2::WitnessMethodKey::simple(field_name),
                    self_type,
                    method_type_args,
                };
                return self.emit_call_returning(callee, vec![], result_ty);
            } else {
                let receiver_ty = self.resolve_expr_type(base);
                let base_val = self.lower_expr(base);
                let callee = kestrel_mir_2::Callee::Witness {
                    protocol,
                    method: kestrel_mir_2::WitnessMethodKey::simple(field_name),
                    self_type: receiver_ty,
                    method_type_args,
                };
                return self.emit_call_returning(callee, vec![(base_val, kestrel_mir_2::ArgMode::Ref)], result_ty);
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
                let callee = kestrel_mir_2::Callee::direct_with_args(getter_entity, type_args, None);
                return self.emit_call_returning(callee, vec![], result_ty);
            }

            let receiver_ty = self.resolve_expr_type(base);
            let base_val = self.lower_expr(base);
            let method_type_args = self.resolve_type_args(expr_id);

            if let Some(protocol) = self.ctx.is_protocol_method(getter_entity) {
                self.ctx.register_name(protocol);
                let key = self.ctx.witness_method_key(getter_entity);
                let callee = kestrel_mir_2::Callee::Witness {
                    protocol,
                    method: key,
                    self_type: receiver_ty,
                    method_type_args,
                };
                return self.emit_call_returning(callee, vec![(base_val, kestrel_mir_2::ArgMode::Ref)], result_ty);
            }

            let type_args = self.prepend_receiver_type_args(receiver_ty, method_type_args);
            let callee = kestrel_mir_2::Callee::direct_with_args(getter_entity, type_args, None);
            return self.emit_call_returning(callee, vec![(base_val, kestrel_mir_2::ArgMode::Ref)], result_ty);
        }

        // Static stored field
        if is_static {
            let static_entity = resolved.unwrap();
            self.ctx.register_name(static_entity);
            return Operand::Place(Place::global(static_entity));
        }

        // Stored field → place projection with FieldIdx
        let base_op = self.lower_expr(base);
        let base_ty = self.resolve_expr_type(base);
        let mut place = self.operand_to_place(base_op, base_ty);
        // Auto-deref: if the local is Pointer[T] (ref-capture) but the
        // expression type is T, insert a deref so field access goes through
        // the pointer.
        if let Some(local) = place.root_local() {
            let local_ty = self.body.local(local).ty;
            if let MirTy::Pointer(inner) = self.ctx.module.ty_arena.get(local_ty) {
                if *inner == base_ty {
                    place = place.deref();
                }
            }
        }

        let struct_entity = match self.ctx.module.ty_arena.get(base_ty) {
            MirTy::Named { entity, .. } => Some(*entity),
            _ => None,
        };
        if let Some(se) = struct_entity {
            if let Some(idx) = self.ctx.resolve_field_idx(se, field_name) {
                return Operand::Place(place.field(idx));
            }
        }
        // Fallback: field not found in lowered structs (generic type, etc.)
        // Use index 0 as placeholder — the type may be resolved during monomorphization
        Operand::Place(place.field(FieldIdx::new(0)))
    }

    // === Def (entity reference) ===

    fn lower_def(&mut self, expr_id: HirExprId, entity: kestrel_hecs::Entity) -> Operand {
        self.ctx.register_name(entity);
        let kind = self.ctx.world.get::<NodeKind>(entity).cloned();
        match kind {
            Some(NodeKind::Function | NodeKind::Initializer) => {
                let inferred_ty = self.resolve_expr_type(expr_id);
                if matches!(self.ctx.module.ty_arena.get(inferred_ty), MirTy::FuncThick { .. }) {
                    let dest = self.fresh_temp(inferred_ty);
                    self.emit_assign(
                        Place::local(dest),
                        Rvalue::ApplyPartial {
                            func: entity,
                            captures: vec![],
                        },
                    );
                    return Operand::Place(Place::local(dest));
                }
                let type_args = self.resolve_type_args(expr_id);
                Operand::Const(Immediate::function_ref(entity, type_args, None))
            }
            Some(NodeKind::EnumCase) => {
                let ty = self.resolve_expr_type(expr_id);
                let case_name = self
                    .ctx
                    .world
                    .get::<kestrel_ast_builder::Name>(entity)
                    .map(|n| n.0.clone())
                    .unwrap_or_else(|| panic!("ICE: enum case {:?} has no Name", entity));
                let enum_entity = self.ctx.world.parent_of(entity)
                    .unwrap_or_else(|| panic!("ICE: enum case {:?} has no parent", entity));
                let variant_idx = self.ctx.resolve_variant_idx(enum_entity, &case_name)
                    .unwrap_or_else(|| panic!(
                        "ICE: variant '{}' not found in enum {:?}", case_name, enum_entity
                    ));
                let dest = self.fresh_temp(ty);
                self.emit_enum_variant(Place::local(dest), ty, variant_idx, vec![]);
                Operand::Place(Place::local(dest))
            }
            Some(NodeKind::Field) => {
                if self.ctx.world.get::<Callable>(entity).is_some() {
                    let result_ty = self.resolve_expr_type(expr_id);
                    let callee = kestrel_mir_2::Callee::direct_with_args(entity, vec![], None);
                    self.emit_call_returning(callee, vec![], result_ty)
                } else {
                    Operand::Place(Place::global(entity))
                }
            }
            Some(NodeKind::TypeParameter | NodeKind::TypeAlias) => {
                Operand::Const(Immediate::unit())
            }
            _ => Operand::Const(Immediate::error()),
        }
    }

    // === Implicit member (.None, .Some(x)) ===

    fn lower_implicit_member(
        &mut self,
        expr_id: HirExprId,
        name: &str,
        args: Option<&[kestrel_hir::body::HirCallArg]>,
    ) -> Operand {
        let result_ty = self.resolve_expr_type(expr_id);

        let resolved = self
            .typed
            .as_ref()
            .and_then(|t| t.resolutions.get(&expr_id))
            .copied();
        let is_enum_case = resolved.is_none_or(|e| {
            self.ctx.world.get::<NodeKind>(e) == Some(&NodeKind::EnumCase)
        });

        if is_enum_case {
            let payload: Vec<(Operand, UseMode)> = args
                .map(|a| {
                    a.iter()
                        .map(|arg| {
                            let op = self.lower_expr(arg.value);
                            let ty = self.resolve_expr_type(arg.value);
                            (op, self.use_mode_for(ty))
                        })
                        .collect()
                })
                .unwrap_or_default();

            let enum_entity = match self.ctx.module.ty_arena.get(result_ty) {
                MirTy::Named { entity, .. } => *entity,
                other => panic!(
                    "ICE: enum case '{}' result type is not Named: {:?}", name, other
                ),
            };
            let variant_idx = self.ctx.resolve_variant_idx(enum_entity, name)
                .unwrap_or_else(|| panic!(
                    "ICE: variant '{}' not found in enum {:?}", name, enum_entity
                ));

            let dest = self.fresh_temp(result_ty);
            self.emit_enum_variant(Place::local(dest), result_ty, variant_idx, payload);
            Operand::Place(Place::local(dest))
        } else {
            // Static method call (e.g., .fromResidual)
            let resolved_entity = resolved.unwrap();
            self.ctx.register_name(resolved_entity);
            let call_args: Vec<(Operand, kestrel_mir_2::ArgMode)> = args
                .map(|a| {
                    a.iter()
                        .map(|arg| (self.lower_expr(arg.value), kestrel_mir_2::ArgMode::Copy))
                        .collect()
                })
                .unwrap_or_default();

            if let Some(protocol) = self.ctx.is_protocol_method(resolved_entity) {
                self.ctx.register_name(protocol);
                let key = self.ctx.witness_method_key(resolved_entity);
                let type_args = self.resolve_type_args(expr_id);
                let callee = kestrel_mir_2::Callee::Witness {
                    protocol,
                    method: key,
                    self_type: result_ty,
                    method_type_args: type_args,
                };
                self.emit_call_returning(callee, call_args, result_ty)
            } else {
                let mut type_args = self.resolve_type_args(expr_id);
                // Static methods on generic types need the parent's type args.
                // For .fromResidual on Result[T, E], prepend [T, E] from result_ty.
                type_args = self.prepend_receiver_type_args(result_ty, type_args);
                let self_type = if !type_args.is_empty() { Some(result_ty) } else { None };
                let callee = kestrel_mir_2::Callee::direct_with_args(resolved_entity, type_args, self_type);
                self.emit_call_returning(callee, call_args, result_ty)
            }
        }
    }

    // === Assign ===

    fn lower_assign(
        &mut self,
        _expr_id: HirExprId,
        target: HirExprId,
        value: HirExprId,
    ) -> Operand {
        // TODO: setter dispatch (Phase 4 stmt.rs)
        let rhs_ty = self.resolve_expr_type(value);
        let rhs = self.lower_expr(value);
        let lhs = self.lower_expr(target);
        if let Operand::Place(dest) = lhs {
            self.emit_value_transfer(dest, rhs, rhs_ty);
        }
        Operand::Const(Immediate::unit())
    }

    // === Block ===

    pub fn lower_hir_block(&mut self, block: &HirBlock) -> Operand {
        for &stmt_id in &block.stmts {
            self.lower_stmt(stmt_id);
            if self.is_terminated() {
                return Operand::Const(Immediate::unit());
            }
        }
        if let Some(tail) = block.tail_expr {
            self.lower_expr(tail)
        } else {
            Operand::Const(Immediate::unit())
        }
    }

    // === Helpers ===

    /// Resolve type args for an expression from inference results.
    pub fn resolve_type_args(&mut self, expr_id: HirExprId) -> Vec<TyId> {
        if let Some(typed) = self.typed.as_ref()
            && let Some(resolved_args) = typed.type_args.get(&expr_id)
        {
            return resolved_args
                .iter()
                .map(|ty| lower_resolved_ty(self.ctx, ty))
                .collect();
        }
        Vec::new()
    }

    /// Prepend the receiver type's args to method-level type args.
    /// Skips prepending if method_args already starts with the parent's args
    /// (the inference engine sometimes includes inherited params).
    pub fn prepend_receiver_type_args(
        &self,
        receiver_ty: TyId,
        method_args: Vec<TyId>,
    ) -> Vec<TyId> {
        let parent_args = match self.ctx.module.ty_arena.get(receiver_ty) {
            MirTy::Named { type_args, .. } => type_args.clone(),
            _ => Vec::new(),
        };
        if parent_args.is_empty() {
            return method_args;
        }
        // Always prepend parent type args. The caller truncates to the
        // function's type_params count to handle cases where inference
        // already included inherited params.
        let mut result = parent_args;
        result.extend(method_args);
        result
    }

    /// Resolve a type-ref expression (Def of a type entity) to its TyId.
    pub fn type_from_type_ref(&mut self, expr_id: HirExprId) -> TyId {
        let expr = &self.hir.exprs[expr_id];
        if let HirExpr::Def(entity, hir_args, _) = expr {
            let args: Vec<TyId> = hir_args.iter().map(|a| lower_type(self.ctx, a)).collect();
            self.ctx.register_name(*entity);
            crate::ty::lower_named_type(self.ctx, *entity, args)
        } else {
            self.resolve_expr_type(expr_id)
        }
    }

    /// Emit a call and return the result as an operand.
    pub fn emit_call_returning(
        &mut self,
        callee: kestrel_mir_2::Callee,
        args: Vec<(Operand, kestrel_mir_2::ArgMode)>,
        result_ty: TyId,
    ) -> Operand {
        let dest = self.fresh_temp(result_ty);
        self.emit_call(Some(Place::local(dest)), callee, args);
        Operand::Place(Place::local(dest))
    }
}
