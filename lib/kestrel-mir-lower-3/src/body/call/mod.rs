pub mod args;
pub mod intrinsic;

use kestrel_ast_builder::{Callable, Gettable, NodeKind, Static};
use kestrel_hecs::Entity;
use kestrel_hir::body::{HirCallArg, HirExpr, HirExprId};
use kestrel_hir::ty::HirTy;
use kestrel_mir_3::callee::Callee;
use kestrel_mir_3::inst::CallArg;
use kestrel_mir_3::item::witness::WitnessMethodKey;
use kestrel_mir_3::{FieldIdx, Immediate, MirTy, Op, Ownership, ParamConvention, TyId, ValueId};

use super::OssaBodyCtx;
use crate::ty::lower_type;

impl OssaBodyCtx<'_, '_> {
    pub fn lower_call_expr(
        &mut self,
        expr_id: HirExprId,
        callee_expr: HirExprId,
        args: &[HirCallArg],
    ) -> ValueId {
        if let Some(entity) = self.resolve_callee_entity_from_expr(callee_expr) {
            if let Some(val) = intrinsic::try_intrinsic(self, expr_id, callee_expr, entity, args) {
                return val;
            }
        }

        if let Some(val) = self.try_enum_construct(expr_id, callee_expr, args) {
            return val;
        }

        self.emit_resolved_call(expr_id, callee_expr, args)
    }

    pub fn lower_method_call_expr(
        &mut self,
        expr_id: HirExprId,
        receiver_expr: HirExprId,
        method_name: &str,
        hir_type_args: Option<&[HirTy]>,
        args: &[HirCallArg],
    ) -> ValueId {
        let mut receiver_ty = self.resolve_expr_type(receiver_expr);
        let result_ty = self.resolve_expr_type(expr_id);

        if self.in_protocol_extension {
            if let MirTy::Named { entity, type_args } = self.ctx.module.ty_arena.get(receiver_ty).clone() {
                if type_args.is_empty()
                    && self.ctx.world.get::<NodeKind>(entity) == Some(&NodeKind::Protocol)
                {
                    receiver_ty = crate::ty::build_self_type(self.ctx, entity);
                }
            }
        }

        let resolved_entity = self.typed.as_ref()
            .and_then(|t| t.resolutions.get(&expr_id))
            .copied();

        let is_static = resolved_entity
            .is_some_and(|e| self.ctx.world.get::<Static>(e).is_some());

        let mut call_args = if is_static {
            self.lower_call_args_default(args)
        } else {
            let receiver_val = self.lower_expr(receiver_expr);
            let receiver_arg = self.prepare_call_arg(receiver_val, ParamConvention::Borrow);
            let mut a = vec![receiver_arg];
            a.extend(self.lower_call_args_default(args));
            a
        };

        let Some(resolved) = resolved_entity else {
            return self.emit_literal(Immediate::error());
        };

        // Field-subscript rewrite
        if let Some(&field_entity) = self.typed.as_ref().and_then(|t| t.field_subscripts.get(&expr_id)) {
            let (new_receiver_ty, new_args) =
                self.rewrite_field_subscript(receiver_ty, call_args, field_entity, method_name);
            receiver_ty = new_receiver_ty;
            call_args = new_args;
        }

        let method_type_args = if let Some(hir_args) = hir_type_args {
            let inferred = self.resolve_type_args(expr_id);
            if inferred.is_empty() {
                hir_args.iter().map(|ty| lower_type(self.ctx, ty)).collect()
            } else {
                inferred
            }
        } else {
            self.resolve_type_args(expr_id)
        };

        // Function-typed receiver: indirect call
        if matches!(self.ctx.module.ty_arena.get(receiver_ty), MirTy::FuncThick { .. } | MirTy::FuncThin { .. }) {
            let receiver_arg = call_args.remove(0);
            let callee = match self.ctx.module.ty_arena.get(receiver_ty) {
                MirTy::FuncThin { .. } => Callee::Thin(receiver_arg.value),
                _ => Callee::Thick(receiver_arg.value),
            };
            return self.emit_call_returning(callee, call_args, result_ty);
        }

        self.expand_default_args(&mut call_args, resolved, args.len());

        let callee = if let Some(protocol) = self.ctx.is_protocol_method(resolved) {
            self.ctx.register_name(protocol);
            let key = self.ctx.witness_method_key(resolved);
            self.apply_witness_conventions(&mut call_args, protocol, &key);
            Callee::Witness {
                protocol,
                method: key,
                self_type: receiver_ty,
                method_type_args,
            }
        } else {
            self.ctx.register_name(resolved);
            self.apply_conventions(&mut call_args, resolved);
            let mut type_args = self.prepend_receiver_type_args(receiver_ty, method_type_args);
            if let Some(mir_func) = self.ctx.module.functions.iter().find(|f| f.entity == resolved) {
                type_args.truncate(mir_func.type_params.len());
            }
            Callee::direct_with_args(resolved, type_args, None)
        };

        self.emit_call_returning(callee, call_args, result_ty)
    }

    fn rewrite_field_subscript(
        &mut self,
        receiver_ty: TyId,
        mut call_args: Vec<CallArg>,
        field_entity: Entity,
        field_name: &str,
    ) -> (TyId, Vec<CallArg>) {
        let recv_entity = match self.ctx.module.ty_arena.get(receiver_ty) {
            MirTy::Named { entity, .. } => *entity,
            _ => return (receiver_ty, call_args),
        };

        // Stored field: extract the field value and use it as the receiver
        if let Some(field_idx) = self.ctx.resolve_field_idx(recv_entity, field_name) {
            let field_ty = self.ctx.module.structs.iter()
                .find(|s| s.entity == recv_entity)
                .and_then(|s| s.fields.get(field_idx.index()))
                .map(|f| f.ty);

            let Some(mut field_ty) = field_ty else {
                return (receiver_ty, call_args);
            };

            if let MirTy::Named { type_args, .. } = self.ctx.module.ty_arena.get(receiver_ty) {
                let type_args = type_args.clone();
                if let Some(sdef) = self.ctx.module.structs.iter().find(|s| s.entity == recv_entity) {
                    let mut subst = kestrel_mir_3::substitute::SubstMap::new();
                    for (tp, &arg) in sdef.type_params.iter().zip(type_args.iter()) {
                        subst.type_params.insert(tp.entity, arg);
                    }
                    field_ty = kestrel_mir_3::substitute::substitute(
                        &mut self.ctx.module.ty_arena, field_ty, &subst,
                    );
                }
            }

            if !call_args.is_empty() {
                // Extract the field from the receiver value
                let old_receiver = call_args[0].value;
                // If the old receiver is a borrow, end it, get the source, extract, re-borrow
                if let Some(source) = self.body.value(old_receiver).borrow_source {
                    self.emit_end_borrow(old_receiver);
                    let field_val = self.emit_struct_extract(source, field_idx, field_ty);
                    let field_arg = self.prepare_call_arg(field_val, ParamConvention::Borrow);
                    call_args[0] = field_arg;
                } else {
                    let field_val = self.emit_struct_extract(old_receiver, field_idx, field_ty);
                    let field_arg = self.prepare_call_arg(field_val, ParamConvention::Borrow);
                    call_args[0] = field_arg;
                }
                return (field_ty, call_args);
            }
            return (receiver_ty, call_args);
        }

        // Computed property: call the getter
        if self.ctx.world.get::<Gettable>(field_entity).is_some() {
            let field_ty = crate::ty::resolve_type_annotation(self.ctx, field_entity);
            self.ctx.register_name(field_entity);
            let type_args = self.prepend_receiver_type_args(receiver_ty, vec![]);
            let callee = Callee::direct_with_args(field_entity, type_args, None);
            let old_receiver = call_args.remove(0);
            let getter_result = self.emit_call_returning(callee, vec![old_receiver], field_ty);
            let field_arg = self.prepare_call_arg(getter_result, ParamConvention::Borrow);
            call_args.insert(0, field_arg);
            return (field_ty, call_args);
        }

        (receiver_ty, call_args)
    }

    pub fn lower_protocol_call_expr(
        &mut self,
        expr_id: HirExprId,
        receiver_expr: HirExprId,
        protocol: Entity,
        method_name: &str,
        _hir_type_args: Option<&[HirTy]>,
        args: &[HirCallArg],
    ) -> ValueId {
        let receiver_ty = self.resolve_expr_type(receiver_expr);
        let result_ty = self.resolve_expr_type(expr_id);
        let receiver_val = self.lower_expr(receiver_expr);

        let receiver_arg = self.prepare_call_arg(receiver_val, ParamConvention::Borrow);
        let mut call_args = vec![receiver_arg];
        call_args.extend(self.lower_call_args_default(args));

        self.ctx.register_name(protocol);
        let method_type_args = self.resolve_type_args(expr_id);
        let labels: Vec<Option<String>> = args.iter().map(|a| a.label.clone()).collect();
        let method_key = WitnessMethodKey::new(method_name, labels);

        if let Some(method_entity) = self.find_protocol_method_entity(protocol, &method_key) {
            self.expand_default_args(&mut call_args, method_entity, args.len());
        }

        self.apply_witness_conventions(&mut call_args, protocol, &method_key);

        let callee = Callee::Witness {
            protocol,
            method: method_key,
            self_type: receiver_ty,
            method_type_args,
        };

        self.emit_call_returning(callee, call_args, result_ty)
    }

    fn emit_resolved_call(
        &mut self,
        expr_id: HirExprId,
        callee_expr: HirExprId,
        args: &[HirCallArg],
    ) -> ValueId {
        let result_ty = self.resolve_expr_type(expr_id);

        let entity = if let Some(&resolved) = self.typed.as_ref().and_then(|t| t.resolutions.get(&expr_id)) {
            resolved
        } else if let Some(&resolved) = self.typed.as_ref().and_then(|t| t.resolutions.get(&callee_expr)) {
            resolved
        } else if let HirExpr::Def(e, _, _) = &self.hir.exprs[callee_expr] {
            *e
        } else if let HirExpr::OverloadSet { candidates, .. } = &self.hir.exprs[callee_expr] {
            match candidates.first() {
                Some(&e) => e,
                None => return self.emit_literal(Immediate::error()),
            }
        } else {
            return self.lower_indirect_call(expr_id, callee_expr, args);
        };

        self.ctx.register_name(entity);

        let entity_kind = self.ctx.world.get::<NodeKind>(entity).cloned();
        let has_callable = self.ctx.world.get::<Callable>(entity).is_some();
        if matches!(entity_kind, Some(NodeKind::Field)) && !has_callable {
            return self.lower_indirect_call(expr_id, callee_expr, args);
        }
        if !has_callable
            && !matches!(entity_kind, Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol | NodeKind::Initializer | NodeKind::Function))
        {
            return self.lower_indirect_call(expr_id, callee_expr, args);
        }

        let is_init = self.is_init_function(entity).is_some();
        let type_args = self.resolve_call_type_args(expr_id, callee_expr, entity, is_init);

        if is_init {
            return self.emit_init_call(entity, type_args, args, result_ty);
        }

        if self.is_struct_entity(entity) {
            return self.emit_struct_construct(entity, args, result_ty);
        }

        let has_receiver = self.ctx.world.get::<Callable>(entity)
            .is_some_and(|c| c.receiver.is_some());

        let mut call_args = self.lower_call_args_default(args);
        if has_receiver {
            let receiver_ty = self.resolve_expr_type(callee_expr);
            let receiver_val = self.lower_expr(callee_expr);
            let receiver_arg = self.prepare_call_arg(receiver_val, ParamConvention::Borrow);
            call_args.insert(0, receiver_arg);
        }

        self.expand_default_args(&mut call_args, entity, args.len());

        let callee = if let Some(protocol) = self.ctx.is_protocol_method(entity) {
            self.ctx.register_name(protocol);
            let key = self.ctx.witness_method_key(entity);
            let self_type = if key.name == "init" {
                result_ty
            } else {
                self.resolve_expr_type(callee_expr)
            };
            self.apply_witness_conventions(&mut call_args, protocol, &key);
            Callee::Witness {
                protocol,
                method: key,
                self_type,
                method_type_args: type_args,
            }
        } else {
            self.apply_conventions(&mut call_args, entity);
            if has_receiver {
                let receiver_ty = self.resolve_expr_type(callee_expr);
                let mut ta = self.prepend_receiver_type_args(receiver_ty, type_args);
                if let Some(mir_func) = self.ctx.module.functions.iter().find(|f| f.entity == entity) {
                    ta.truncate(mir_func.type_params.len());
                }
                Callee::direct_with_args(entity, ta, None)
            } else {
                Callee::direct_with_args(entity, type_args, None)
            }
        };

        self.emit_call_returning(callee, call_args, result_ty)
    }

    fn try_enum_construct(
        &mut self,
        expr_id: HirExprId,
        callee_expr: HirExprId,
        args: &[HirCallArg],
    ) -> Option<ValueId> {
        let entity = match &self.hir.exprs[callee_expr] {
            HirExpr::Def(e, _, _) => *e,
            _ => return None,
        };
        if self.ctx.world.get::<NodeKind>(entity) != Some(&NodeKind::EnumCase) {
            return None;
        }

        let result_ty = self.resolve_enum_case_type(expr_id, callee_expr, entity);
        let case_name = self.ctx.world.get::<kestrel_ast_builder::Name>(entity)
            .map(|n| n.0.clone())
            .unwrap_or_else(|| panic!("ICE: enum case {:?} has no Name", entity));
        let enum_entity = self.ctx.world.parent_of(entity)
            .unwrap_or_else(|| panic!("ICE: enum case {:?} has no parent", entity));
        let variant_idx = self.ctx.resolve_variant_idx(enum_entity, &case_name)
            .unwrap_or_else(|| panic!("ICE: variant '{}' not found in enum {:?}", case_name, enum_entity));

        let payload: Vec<ValueId> = args.iter()
            .map(|arg| self.lower_expr(arg.value))
            .collect();

        Some(self.emit_enum_variant(result_ty, variant_idx, payload))
    }

    fn emit_struct_construct(
        &mut self,
        struct_entity: Entity,
        args: &[HirCallArg],
        result_ty: TyId,
    ) -> ValueId {
        let fields: Vec<(FieldIdx, ValueId)> = args.iter()
            .enumerate()
            .map(|(i, arg)| {
                let val = self.lower_expr(arg.value);
                (FieldIdx::new(i), val)
            })
            .collect();

        self.emit_struct(result_ty, fields)
    }

    fn emit_init_call(
        &mut self,
        entity: Entity,
        type_args: Vec<TyId>,
        args: &[HirCallArg],
        result_ty: TyId,
    ) -> ValueId {
        // Allocate stack space for self (StackAlloc, not Uninit — opaque init
        // calls can't satisfy Uninit's sub-field tracking requirements).
        let ptr_ty = self.ctx.module.ty_arena.pointer(result_ty);
        let one = self.emit_literal(Immediate::i64(1));
        let self_addr = self.emit_op1(kestrel_mir_3::Op::StackAlloc(result_ty), one, ptr_ty);

        let mut call_args = vec![CallArg {
            value: self_addr,
            convention: ParamConvention::MutBorrow,
        }];
        call_args.extend(self.lower_call_args_default(args));
        self.expand_default_args(&mut call_args, entity, args.len());

        let callee = if let Some(protocol) = self.ctx.is_protocol_method(entity) {
            self.ctx.register_name(protocol);
            let key = self.ctx.witness_method_key(entity);
            self.apply_witness_conventions(&mut call_args, protocol, &key);
            Callee::Witness {
                protocol,
                method: key,
                self_type: result_ty,
                method_type_args: type_args,
            }
        } else {
            self.apply_conventions(&mut call_args, entity);
            Callee::direct_with_args(entity, type_args, None)
        };

        self.emit_call_void(callee, call_args);
        // Take the initialized value from the stack slot.
        // Use Take (not Load) so non-trivial types get @owned ownership.
        let ownership = self.ownership_for(result_ty);
        if ownership == Ownership::Owned {
            self.emit_take(self_addr, result_ty)
        } else {
            self.emit_load(self_addr, result_ty)
        }
    }

    fn lower_indirect_call(
        &mut self,
        expr_id: HirExprId,
        callee_expr: HirExprId,
        args: &[HirCallArg],
    ) -> ValueId {
        let callee_ty = self.resolve_expr_type(callee_expr);
        let callee_val = self.lower_expr(callee_expr);
        let result_ty = self.resolve_expr_type(expr_id);
        let call_args = self.lower_call_args_default(args);

        let callee = match self.ctx.module.ty_arena.get(callee_ty) {
            MirTy::FuncThin { .. } => Callee::Thin(callee_val),
            _ => Callee::Thick(callee_val),
        };

        self.emit_call_returning(callee, call_args, result_ty)
    }

    fn resolve_callee_entity_from_expr(&self, callee_expr: HirExprId) -> Option<Entity> {
        if let Some(&resolved) = self.typed.as_ref().and_then(|t| t.resolutions.get(&callee_expr)) {
            return Some(resolved);
        }
        match &self.hir.exprs[callee_expr] {
            HirExpr::Def(e, _, _) => Some(*e),
            _ => None,
        }
    }

    fn is_struct_entity(&self, entity: Entity) -> bool {
        self.ctx.world.get::<NodeKind>(entity) == Some(&NodeKind::Struct)
    }

    fn resolve_enum_case_type(
        &mut self,
        expr_id: HirExprId,
        callee_expr: HirExprId,
        case_entity: Entity,
    ) -> TyId {
        let inferred = self.resolve_expr_type(expr_id);
        match self.ctx.module.ty_arena.get(inferred).clone() {
            MirTy::Named { type_args, .. } if type_args.is_empty() => {
                let parent = self.ctx.world.parent_of(case_entity)
                    .unwrap_or_else(|| panic!("ICE: enum case {:?} has no parent", case_entity));
                let is_generic = self.ctx.world
                    .get::<kestrel_ast_builder::TypeParams>(parent)
                    .is_some_and(|tp| !tp.0.is_empty());
                if is_generic {
                    let type_args = self.resolve_type_args(callee_expr);
                    if !type_args.is_empty() {
                        self.ctx.register_name(parent);
                        return self.ctx.module.ty_arena.named(parent, type_args);
                    }
                }
                inferred
            }
            MirTy::Error => {
                if let Some(parent) = self.ctx.world.parent_of(case_entity) {
                    self.ctx.register_name(parent);
                    let type_args = self.resolve_type_args(callee_expr);
                    self.ctx.module.ty_arena.named(parent, type_args)
                } else {
                    inferred
                }
            }
            _ => inferred,
        }
    }
}
