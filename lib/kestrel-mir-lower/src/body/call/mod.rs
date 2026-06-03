pub mod args;
pub mod intrinsic;

use kestrel_ast_builder::{Callable, Gettable, InitEffect, NodeKind, Static};
use kestrel_hecs::Entity;
use kestrel_hir::body::{HirCallArg, HirExpr, HirExprId};
use kestrel_hir::ty::HirTy;
use kestrel_mir::callee::Callee;
use kestrel_mir::inst::CallArg;
use kestrel_mir::item::witness::WitnessMethodKey;
use kestrel_mir::terminator::{SwitchArm, SwitchCase};
use kestrel_mir::{
    FieldIdx, Immediate, MirTy, Ownership, ParamConvention, TyId, ValueId, VariantIdx,
};

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

        if self.body_context.is_protocol_extension() {
            if let MirTy::Named { entity, type_args } =
                self.ctx.module.ty_arena.get(receiver_ty).clone()
            {
                if type_args.is_empty()
                    && self.ctx.world.get::<NodeKind>(entity) == Some(&NodeKind::Protocol)
                {
                    receiver_ty = crate::ty::build_self_type(self.ctx, entity);
                }
            }
        }

        let resolved_entity = self
            .typed
            .as_ref()
            .and_then(|t| t.resolutions.get(&expr_id))
            .copied();

        let is_static = resolved_entity.is_some_and(|e| self.ctx.world.get::<Static>(e).is_some());

        let Some(resolved) = resolved_entity else {
            return self.emit_literal(Immediate::error());
        };

        // Field-stored thick/thin function: lower as field access + indirect call.
        // `self.predicate(arg)` where `predicate` is a struct field of function type.
        let entity_kind = self.ctx.world.get::<NodeKind>(resolved).cloned();
        if matches!(entity_kind, Some(NodeKind::Field)) {
            let base_ty = self.resolve_expr_type(receiver_expr);
            let struct_entity = match self.ctx.module.ty_arena.get(base_ty) {
                MirTy::Named { entity, .. } => Some(*entity),
                _ => None,
            };
            let field_info = struct_entity.and_then(|se| {
                self.ctx
                    .resolve_field_idx(se, method_name)
                    .map(|idx| (se, idx))
            });
            if let Some((se, field_idx)) = field_info {
                if let Some(field_ty) = self.ctx.resolve_field_ty(se, field_idx) {
                    if matches!(
                        self.ctx.module.ty_arena.get(field_ty),
                        MirTy::FuncThick { .. } | MirTy::FuncThin { .. }
                    ) {
                        let base_val = self.lower_expr_for_borrow(receiver_expr);
                        let field_val = self.emit_struct_extract(base_val, field_idx, field_ty);
                        let call_args = self.lower_call_args_default(args);
                        let callee = match self.ctx.module.ty_arena.get(field_ty) {
                            MirTy::FuncThin { .. } => Callee::Thin(field_val),
                            _ => Callee::Thick(field_val),
                        };
                        return self.emit_call_returning(callee, call_args, result_ty);
                    }
                }
            }
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

        // Function-typed receiver: indirect call (all Borrow by ABI)
        if matches!(
            self.ctx.module.ty_arena.get(receiver_ty),
            MirTy::FuncThick { .. } | MirTy::FuncThin { .. }
        ) {
            let receiver_val = self.lower_expr(receiver_expr);
            let call_args = self.lower_call_args_default(args);
            let callee = match self.ctx.module.ty_arena.get(receiver_ty) {
                MirTy::FuncThin { .. } => Callee::Thin(receiver_val),
                _ => Callee::Thick(receiver_val),
            };
            return self.emit_call_returning(callee, call_args, result_ty);
        }

        // Collect conventions early (needed for arg lowering), but defer
        // callee construction until after the field-subscript rewrite which
        // may update receiver_ty.
        let protocol_method = self.ctx.is_protocol_method(resolved);
        let (conventions, witness_key) = if let Some(protocol) = protocol_method {
            self.ctx.register_name(protocol);
            let key = self.ctx.witness_method_key(resolved);
            let convs = self.collect_witness_conventions(protocol, &key);
            (convs, Some(key))
        } else {
            self.ctx.register_name(resolved);
            let convs = self.collect_conventions(resolved);
            (convs, None)
        };

        let mut call_args = if is_static {
            self.lower_call_args(args, &conventions, 0)
        } else {
            let recv_conv = conventions
                .first()
                .copied()
                .unwrap_or(ParamConvention::Borrow);
            let receiver_arg = self.prepare_call_arg_for_expr(receiver_expr, recv_conv);
            let mut a = vec![receiver_arg];
            a.extend(self.lower_call_args(args, &conventions, 1));
            a
        };

        // Field-subscript rewrite — may update receiver_ty
        if let Some(&field_entity) = self
            .typed
            .as_ref()
            .and_then(|t| t.field_subscripts.get(&expr_id))
        {
            let recv_conv = conventions
                .first()
                .copied()
                .unwrap_or(ParamConvention::Borrow);
            let (new_receiver_ty, new_args) = self.rewrite_field_subscript(
                receiver_expr,
                receiver_ty,
                call_args,
                field_entity,
                method_name,
                recv_conv,
            );
            receiver_ty = new_receiver_ty;
            call_args = new_args;
        }

        // Build callee with the (possibly updated) receiver_ty
        let callee = if let Some(protocol) = protocol_method {
            Callee::Witness {
                protocol,
                method: witness_key.unwrap(),
                self_type: receiver_ty,
                method_type_args,
            }
        } else {
            let mut type_args = self.prepend_receiver_type_args(receiver_ty, method_type_args);
            if let Some(mir_func) = self.ctx.module.functions.get(&resolved) {
                type_args.truncate(mir_func.type_params.len());
            }
            Callee::direct_with_args(resolved, type_args, None)
        };

        let conv_offset = if is_static { 0 } else { 1 };
        self.expand_default_args(
            &mut call_args,
            resolved,
            args.len(),
            &conventions,
            conv_offset,
        );

        self.emit_call_returning(callee, call_args, result_ty)
    }

    fn rewrite_field_subscript(
        &mut self,
        receiver_expr: HirExprId,
        receiver_ty: TyId,
        mut call_args: Vec<CallArg>,
        field_entity: Entity,
        field_name: &str,
        receiver_convention: ParamConvention,
    ) -> (TyId, Vec<CallArg>) {
        let recv_entity = match self.ctx.module.ty_arena.get(receiver_ty) {
            MirTy::Named { entity, .. } => *entity,
            _ => return (receiver_ty, call_args),
        };

        // Stored field: extract the field value and use it as the receiver
        if let Some(field_idx) = self.ctx.resolve_field_idx(recv_entity, field_name) {
            let field_ty = self
                .ctx
                .module
                .structs
                .get(&recv_entity)
                .and_then(|s| s.fields.get(field_idx.index()))
                .map(|f| f.ty);

            let Some(mut field_ty) = field_ty else {
                return (receiver_ty, call_args);
            };

            if let MirTy::Named { type_args, .. } = self.ctx.module.ty_arena.get(receiver_ty) {
                let type_args = type_args.clone();
                if let Some(sdef) = self.ctx.module.structs.get(&recv_entity) {
                    let mut subst = kestrel_mir::substitute::SubstMap::new();
                    for (tp, &arg) in sdef.type_params.iter().zip(type_args.iter()) {
                        subst.type_params.insert(tp.entity, arg);
                    }
                    field_ty = kestrel_mir::substitute::substitute(
                        &mut self.ctx.module.ty_arena,
                        field_ty,
                        &subst,
                    );
                }
            }

            if !call_args.is_empty() {
                // Borrow-like receiver (subscript get/set): borrow the field
                // IN PLACE via its address when the base is addressable
                // (a var / field chain). `emit_struct_extract` would yield a
                // @guaranteed *value* that the callee — which takes self by
                // address — must spill to memory, and for a non-trivial element
                // type (e.g. `Array[String]`) that spill is a clone that
                // bitwise-aliases heap elements and then over-releases them
                // (corrupting `bag.items(i)` on a struct field). The address
                // path mirrors how `bag.field.prop` (e.g. `.count`) is lowered.
                if matches!(
                    receiver_convention,
                    ParamConvention::Borrow | ParamConvention::MutBorrow
                ) {
                    if let Some(base_addr) = self.try_field_addr_chain(receiver_expr) {
                        let old_receiver = call_args[0].value;
                        if self.body.value(old_receiver).borrow_source.is_some() {
                            self.emit_end_borrow(old_receiver);
                        }
                        let faddr = self.emit_field_addr(base_addr, receiver_ty, field_idx);
                        let borrow = if receiver_convention == ParamConvention::MutBorrow {
                            self.emit_begin_mut_borrow_addr(faddr, field_ty)
                        } else {
                            self.emit_begin_borrow_addr(faddr, field_ty)
                        };
                        call_args[0] = CallArg {
                            value: borrow,
                            convention: receiver_convention,
                        };
                        return (field_ty, call_args);
                    }
                }
                // Fallback (non-addressable base, e.g. `makeBag().items(i)`):
                // extract the field value and use it as the receiver.
                let old_receiver = call_args[0].value;
                // If the old receiver is a borrow, end it, get the source, extract, re-borrow
                if let Some(source) = self.body.value(old_receiver).borrow_source {
                    self.emit_end_borrow(old_receiver);
                    let field_val = self.emit_struct_extract(source, field_idx, field_ty);
                    let field_arg = self.prepare_call_arg(field_val, receiver_convention);
                    call_args[0] = field_arg;
                } else {
                    let field_val = self.emit_struct_extract(old_receiver, field_idx, field_ty);
                    let field_arg = self.prepare_call_arg(field_val, receiver_convention);
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
            let field_arg = self.prepare_call_arg(getter_result, receiver_convention);
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

        self.ctx.register_name(protocol);
        let method_type_args = self.resolve_type_args(expr_id);
        let labels: Vec<Option<String>> = args.iter().map(|a| a.label.clone()).collect();
        let method_key = WitnessMethodKey::new(method_name, labels);

        let conventions = self.collect_witness_conventions(protocol, &method_key);

        let recv_conv = conventions
            .first()
            .copied()
            .unwrap_or(ParamConvention::Borrow);
        let receiver_arg = self.prepare_call_arg_for_expr(receiver_expr, recv_conv);
        let mut call_args = vec![receiver_arg];
        call_args.extend(self.lower_call_args(args, &conventions, 1));

        if let Some(method_entity) = self.find_protocol_method_entity(protocol, &method_key) {
            self.expand_default_args(&mut call_args, method_entity, args.len(), &conventions, 1);
        }

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

        let entity = if let Some(&resolved) = self
            .typed
            .as_ref()
            .and_then(|t| t.resolutions.get(&expr_id))
        {
            resolved
        } else if let Some(&resolved) = self
            .typed
            .as_ref()
            .and_then(|t| t.resolutions.get(&callee_expr))
        {
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
            && !matches!(
                entity_kind,
                Some(
                    NodeKind::Struct
                        | NodeKind::Enum
                        | NodeKind::Protocol
                        | NodeKind::Initializer
                        | NodeKind::Function
                )
            )
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

        let has_receiver = self
            .ctx
            .world
            .get::<Callable>(entity)
            .is_some_and(|c| c.receiver.is_some());

        // Resolve conventions and build callee before lowering args
        let (conventions, callee) = if let Some(protocol) = self.ctx.is_protocol_method(entity) {
            self.ctx.register_name(protocol);
            let key = self.ctx.witness_method_key(entity);
            let convs = self.collect_witness_conventions(protocol, &key);
            let self_type = if key.name == "init" {
                result_ty
            } else {
                self.resolve_expr_type(callee_expr)
            };
            let callee = Callee::Witness {
                protocol,
                method: key,
                self_type,
                method_type_args: type_args,
            };
            (convs, callee)
        } else {
            let convs = self.collect_conventions(entity);
            let callee = if has_receiver {
                let receiver_ty = self.resolve_expr_type(callee_expr);
                let mut ta = self.prepend_receiver_type_args(receiver_ty, type_args);
                if let Some(mir_func) = self.ctx.module.functions.get(&entity) {
                    ta.truncate(mir_func.type_params.len());
                }
                Callee::direct_with_args(entity, ta, None)
            } else {
                Callee::direct_with_args(entity, type_args, None)
            };
            (convs, callee)
        };

        let conv_offset = if has_receiver { 1 } else { 0 };
        let mut call_args = self.lower_call_args(args, &conventions, conv_offset);
        if has_receiver {
            let recv_conv = conventions
                .first()
                .copied()
                .unwrap_or(ParamConvention::Borrow);
            let receiver_arg = self.prepare_call_arg_for_expr(callee_expr, recv_conv);
            call_args.insert(0, receiver_arg);
        }

        self.expand_default_args(
            &mut call_args,
            entity,
            args.len(),
            &conventions,
            conv_offset,
        );

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

        let payload: Vec<ValueId> = args.iter().map(|arg| self.lower_expr(arg.value)).collect();

        Some(self.emit_enum_variant(result_ty, variant_idx, payload))
    }

    fn emit_struct_construct(
        &mut self,
        _struct_entity: Entity,
        args: &[HirCallArg],
        result_ty: TyId,
    ) -> ValueId {
        let fields: Vec<(FieldIdx, ValueId)> = args
            .iter()
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
        let init_effect = self.ctx.world.get::<InitEffect>(entity).cloned();
        if init_effect.is_some() {
            return self.emit_failable_init_call(
                entity,
                type_args,
                args,
                result_ty,
                init_effect.unwrap(),
            );
        }

        let ptr_ty = self.ctx.module.ty_arena.pointer(result_ty);
        let one = self.emit_literal(Immediate::i64(1));
        let self_addr = self.emit_op1(kestrel_mir::Op::StackAlloc(result_ty), one, ptr_ty);

        let (conventions, callee) = if let Some(protocol) = self.ctx.is_protocol_method(entity) {
            self.ctx.register_name(protocol);
            let key = self.ctx.witness_method_key(entity);
            let convs = self.collect_witness_conventions(protocol, &key);
            let callee = Callee::Witness {
                protocol,
                method: key,
                self_type: result_ty,
                method_type_args: type_args,
            };
            (convs, callee)
        } else {
            let convs = self.collect_conventions(entity);
            let callee = Callee::direct_with_args(entity, type_args, None);
            (convs, callee)
        };

        // Self arg is the stack-allocated output pointer (always MutBorrow).
        let mut call_args = vec![CallArg {
            value: self_addr,
            convention: ParamConvention::MutBorrow,
        }];
        call_args.extend(self.lower_call_args(args, &conventions, 1));
        self.expand_default_args(&mut call_args, entity, args.len(), &conventions, 1);

        self.emit_call_void(callee, call_args);
        let ownership = self.ownership_for(result_ty);
        if ownership == Ownership::Owned {
            self.emit_take(self_addr, result_ty)
        } else {
            self.emit_load(self_addr, result_ty)
        }
    }

    /// Failable/throwing init: the init body writes fields into `self` and
    /// returns `Optional[()]` or `Result[(), E]`. The call site unwraps
    /// `Optional[T]` → allocates `Pointer[T]`, calls the init, then branches
    /// on the return discriminant to wrap the result.
    fn emit_failable_init_call(
        &mut self,
        entity: Entity,
        type_args: Vec<TyId>,
        args: &[HirCallArg],
        result_ty: TyId,
        effect: InitEffect,
    ) -> ValueId {
        // result_ty is Optional[T] or Result[T, E]. Extract inner struct type.
        let (inner_ty, enum_entity, success_name, failure_name) = match &effect {
            InitEffect::Failable => {
                let (entity, inner) = match self.ctx.module.ty_arena.get(result_ty) {
                    MirTy::Named { entity, type_args } if !type_args.is_empty() => {
                        (*entity, type_args[0])
                    },
                    _ => panic!("failable init result_ty must be Optional[T]"),
                };
                (inner, entity, "Some", "None")
            },
            InitEffect::Throwing => {
                let (entity, inner) = match self.ctx.module.ty_arena.get(result_ty) {
                    MirTy::Named { entity, type_args } if !type_args.is_empty() => {
                        (*entity, type_args[0])
                    },
                    _ => panic!("throwing init result_ty must be Result[T, E]"),
                };
                (inner, entity, "Ok", "Err")
            },
        };

        // Allocate self as Pointer[T], not Pointer[Optional[T]]
        let ptr_ty = self.ctx.module.ty_arena.pointer(inner_ty);
        let one = self.emit_literal(Immediate::i64(1));
        let self_addr = self.emit_op1(kestrel_mir::Op::StackAlloc(inner_ty), one, ptr_ty);

        // Build the init return type: Optional[()] or Result[(), E]
        let unit_ty = self.ctx.module.ty_arena.unit();
        let err_ty = match &effect {
            InitEffect::Throwing => match self.ctx.module.ty_arena.get(result_ty) {
                MirTy::Named { type_args, .. } if type_args.len() >= 2 => type_args[1],
                _ => unit_ty,
            },
            _ => unit_ty,
        };
        let init_ret_ty = match &effect {
            InitEffect::Failable => self.ctx.module.ty_arena.named(enum_entity, vec![unit_ty]),
            InitEffect::Throwing => self
                .ctx
                .module
                .ty_arena
                .named(enum_entity, vec![unit_ty, err_ty]),
        };

        let (conventions, callee) = if let Some(protocol) = self.ctx.is_protocol_method(entity) {
            self.ctx.register_name(protocol);
            let key = self.ctx.witness_method_key(entity);
            let convs = self.collect_witness_conventions(protocol, &key);
            let callee = Callee::Witness {
                protocol,
                method: key,
                self_type: inner_ty,
                method_type_args: type_args,
            };
            (convs, callee)
        } else {
            let convs = self.collect_conventions(entity);
            let callee = Callee::direct_with_args(entity, type_args, None);
            (convs, callee)
        };

        let mut call_args = vec![CallArg {
            value: self_addr,
            convention: ParamConvention::MutBorrow,
        }];
        call_args.extend(self.lower_call_args(args, &conventions, 1));
        self.expand_default_args(&mut call_args, entity, args.len(), &conventions, 1);

        // Call returns Optional[()] or Result[(), E]
        let init_ret = self.emit_call_returning(callee, call_args, init_ret_ty);

        // Resolve variant indices
        let success_idx = self
            .ctx
            .resolve_variant_idx(enum_entity, success_name)
            .unwrap_or(VariantIdx::new(0));
        let failure_idx = self
            .ctx
            .resolve_variant_idx(enum_entity, failure_name)
            .unwrap_or(VariantIdx::new(1));

        // Extract discriminant as I32 for switching (same pattern as match lowering)
        let disc = self.emit_discriminant(init_ret);

        // Set up branching region — thread live values through both arms
        let saved_tracker = self.tracker.clone();
        self.tracker = super::LiveTracker::from_live(&self.all_live_tracked());
        let live_vals = self.tracker.values();
        let descs = self.tracker.descs();

        let result_ownership = self.ownership_for(result_ty);
        let mut merge_descs: Vec<(TyId, Ownership)> = vec![(result_ty, result_ownership)];
        merge_descs.extend(&descs);
        let (success_block, success_params) = self.new_block_with_params(&descs);
        let (failure_block, failure_params) = self.new_block_with_params(&descs);
        let (merge_block, merge_param_vals) = self.new_block_with_params(&merge_descs);

        self.emit_switch(
            disc,
            vec![
                SwitchArm {
                    pattern: SwitchCase::Variant(success_idx),
                    target: success_block,
                    args: live_vals.clone(),
                },
                SwitchArm {
                    pattern: SwitchCase::Variant(failure_idx),
                    target: failure_block,
                    args: live_vals.clone(),
                },
            ],
        );

        let snapshot = self.snapshot_scope();

        // Find positions in live_vals so we can use rebound versions after branch
        let self_addr_pos = live_vals.iter().position(|&v| v == self_addr);
        let init_ret_pos = live_vals.iter().position(|&v| v == init_ret);

        // -- Success: take self from the pointer, wrap in Some/Ok --
        self.switch_to(success_block);
        self.rebind_scope_values(&live_vals, &success_params);
        let rebound_self_addr = self_addr_pos
            .map(|pos| success_params[pos])
            .unwrap_or(self_addr);
        let self_val = self.emit_take(rebound_self_addr, inner_ty);
        let wrapped = self.emit_enum_variant(result_ty, success_idx, vec![self_val]);
        let tracker_vals = self.tracker.values();
        let mut args = vec![wrapped];
        args.extend(&tracker_vals);
        self.emit_jump(merge_block, args);

        // -- Failure: emit None/Err, destroy the uninitialized self allocation --
        self.restore_scope(&snapshot);
        self.switch_to(failure_block);
        self.rebind_scope_values(&live_vals, &failure_params);
        let none_val = if failure_name == "None" {
            self.emit_enum_variant(result_ty, failure_idx, vec![])
        } else {
            // Throwing: extract error payload from init_ret and re-wrap as Result[T, E]
            let rebound_init_ret = init_ret_pos
                .map(|pos| failure_params[pos])
                .unwrap_or(init_ret);
            let error_payload =
                self.emit_enum_payload(rebound_init_ret, failure_idx, FieldIdx::new(0), err_ty);
            self.emit_enum_variant(result_ty, failure_idx, vec![error_payload])
        };
        let tracker_vals = self.tracker.values();
        let mut args = vec![none_val];
        args.extend(&tracker_vals);
        self.emit_jump(merge_block, args);

        // -- Merge --
        self.restore_scope(&snapshot);
        self.switch_to(merge_block);
        let merge_live = &merge_param_vals[1..];
        self.rebind_scope_values(&live_vals, merge_live);
        let result_param = merge_param_vals[0];
        if result_ownership == Ownership::Owned {
            self.track_owned(result_param);
        }

        self.tracker = saved_tracker;
        self.tracker.rebind(&live_vals, merge_live);
        result_param
    }

    fn lower_indirect_call(
        &mut self,
        expr_id: HirExprId,
        callee_expr: HirExprId,
        args: &[HirCallArg],
    ) -> ValueId {
        let callee_ty = self.resolve_expr_type(callee_expr);
        let callee_val = self.lower_callee_value(callee_expr);
        let result_ty = self.resolve_expr_type(expr_id);
        let call_args = self.lower_call_args_default(args);

        let callee = match self.ctx.module.ty_arena.get(callee_ty) {
            MirTy::FuncThin { .. } => Callee::Thin(callee_val),
            _ => Callee::Thick(callee_val),
        };

        self.emit_call_returning(callee, call_args, result_ty)
    }

    /// Lower a call's callee to its value. A call reads (does not consume) its
    /// callee, so a non-var local — e.g. a captured closure called more than
    /// once — is used directly rather than moved/copied. The callee value is
    /// not an OSSA operand of `Call`, so leaving it @owned and live is correct:
    /// it is dropped once at scope exit.
    fn lower_callee_value(&mut self, callee_expr: HirExprId) -> ValueId {
        if let HirExpr::Local(hir_local, _) = &self.hir.exprs[callee_expr] {
            if !self.is_var_local(hir_local) {
                return self.map_local(*hir_local);
            }
        }
        self.lower_expr(callee_expr)
    }

    fn resolve_callee_entity_from_expr(&self, callee_expr: HirExprId) -> Option<Entity> {
        if let Some(&resolved) = self
            .typed
            .as_ref()
            .and_then(|t| t.resolutions.get(&callee_expr))
        {
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
                let parent =
                    self.ctx.world.parent_of(case_entity).unwrap_or_else(|| {
                        panic!("ICE: enum case {:?} has no parent", case_entity)
                    });
                let is_generic = self
                    .ctx
                    .world
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
            },
            MirTy::Error => {
                if let Some(parent) = self.ctx.world.parent_of(case_entity) {
                    self.ctx.register_name(parent);
                    let type_args = self.resolve_type_args(callee_expr);
                    self.ctx.module.ty_arena.named(parent, type_args)
                } else {
                    inferred
                }
            },
            _ => inferred,
        }
    }
}
