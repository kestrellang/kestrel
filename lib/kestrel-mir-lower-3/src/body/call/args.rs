use kestrel_ast_builder::{Attributes, Callable, NodeKind};
use kestrel_hecs::Entity;
use kestrel_hir::body::{HirCallArg, HirExpr, HirExprId};
use kestrel_hir_lower::LowerBody;
use kestrel_mir_3::inst::CallArg;
use kestrel_mir_3::item::witness::WitnessMethodKey;
use kestrel_mir_3::{Immediate, MirTy, ParamConvention, TyId, ValueId};
use kestrel_type_infer::InferBody;

use crate::body::{HirRef, OssaBodyCtx, TypedRef};
use crate::ty::{lower_resolved_ty, lower_type, resolve_type_annotation};

impl OssaBodyCtx<'_, '_> {
    /// Lower call args with the callee's resolved conventions.
    /// `offset` skips receiver slots in `conventions` (e.g. 1 if
    /// conventions[0] is the receiver, and `args` are the non-receiver params).
    pub(crate) fn lower_call_args(
        &mut self,
        args: &[HirCallArg],
        conventions: &[ParamConvention],
        offset: usize,
    ) -> Vec<CallArg> {
        args.iter()
            .enumerate()
            .map(|(i, arg)| {
                let conv = conventions.get(offset + i).copied()
                    .unwrap_or(ParamConvention::Borrow);
                self.prepare_call_arg_for_expr(arg.value, conv)
            })
            .collect()
    }

    /// Lower call args with Borrow convention for all params.
    /// Used for indirect/closure calls where the callee's conventions
    /// aren't known at the call site.
    pub(crate) fn lower_call_args_default(&mut self, args: &[HirCallArg]) -> Vec<CallArg> {
        args.iter()
            .map(|arg| {
                let val = self.lower_expr(arg.value);
                self.prepare_call_arg(val, ParamConvention::Borrow)
            })
            .collect()
    }

    /// Fill in missing default arguments by inline-lowering each default
    /// expression into the current function body.
    /// `conventions` and `conv_offset` index into the callee's param
    /// conventions so defaults get the right Borrow/Consuming treatment.
    pub(crate) fn expand_default_args(
        &mut self,
        call_args: &mut Vec<CallArg>,
        callee_entity: Entity,
        explicit_count: usize,
        conventions: &[ParamConvention],
        conv_offset: usize,
    ) {
        let Some(callable) = self.ctx.world.get::<Callable>(callee_entity) else {
            return;
        };
        if explicit_count >= callable.params.len() {
            return;
        }

        let defaults: Vec<Entity> = callable.params[explicit_count..]
            .iter()
            .filter_map(|p| p.default_entity)
            .collect();

        for (di, default_entity) in defaults.into_iter().enumerate() {
            let param_ty = resolve_type_annotation(self.ctx, default_entity);
            let default_val = self.lower_default_arg_inline(default_entity, param_ty);
            let conv = conventions.get(conv_offset + explicit_count + di).copied()
                .unwrap_or(ParamConvention::Borrow);
            let arg = self.prepare_call_arg(default_val, conv);
            call_args.push(arg);
        }
    }

    fn lower_default_arg_inline(
        &mut self,
        default_entity: Entity,
        _param_ty: TyId,
    ) -> ValueId {
        let Some(default_hir) = self.ctx.query.query(LowerBody {
            entity: default_entity,
            root: self.ctx.root,
        }) else {
            return self.emit_literal(Immediate::error());
        };
        let default_typed = self.ctx.query.query(InferBody {
            entity: default_entity,
            root: self.ctx.root,
        });

        let tail = default_hir.tail_expr;

        let saved_hir = std::mem::replace(&mut self.hir, HirRef::Owned(default_hir));
        let saved_typed = std::mem::replace(&mut self.typed, default_typed.map(TypedRef::Owned));
        let saved_local_map = std::mem::take(&mut self.local_map);

        // Create values for the default body's HIR locals
        let default_locals: Vec<_> = self.hir.locals.iter().map(|(id, l)| (id, l.clone())).collect();
        for (hir_id, _local) in &default_locals {
            let ty = self.resolve_local_type(*hir_id);
            let val = self.alloc_value_auto(ty);
            self.local_map.insert(*hir_id, val);
        }

        let result = if let Some(tail_id) = tail {
            self.lower_expr(tail_id)
        } else {
            self.emit_literal(Immediate::unit())
        };

        self.hir = saved_hir;
        self.typed = saved_typed;
        self.local_map = saved_local_map;

        result
    }

    /// Look up param conventions for a callee from its FunctionDef or ECS Callable.
    pub(crate) fn collect_conventions(&self, callee_entity: Entity) -> Vec<ParamConvention> {
        // Try MIR FunctionDef first
        if let Some(callee) = self.ctx.module.functions.iter().find(|f| f.entity == callee_entity) {
            if callee.extern_info.is_some() {
                // Extern: all Consuming
                return callee.params.iter().map(|_| ParamConvention::Consuming).collect();
            }
            return callee.params.iter().map(|p| p.convention).collect();
        }
        // ECS fallback
        let Some(callable) = self.ctx.world.get::<Callable>(callee_entity) else {
            return Vec::new();
        };
        let is_extern = self.ctx.world.get::<Attributes>(callee_entity)
            .is_some_and(|attrs| attrs.0.iter().any(|a| a.name == "extern"));
        if is_extern {
            return callable.params.iter().map(|_| ParamConvention::Consuming).collect();
        }

        let mut convs = Vec::new();
        if callable.receiver.is_some() {
            let conv = match callable.receiver.as_ref().unwrap() {
                kestrel_ast_builder::ReceiverKind::Borrowing => ParamConvention::Borrow,
                kestrel_ast_builder::ReceiverKind::Mutating => ParamConvention::MutBorrow,
                kestrel_ast_builder::ReceiverKind::Consuming => ParamConvention::Consuming,
            };
            convs.push(conv);
        }
        for param in &callable.params {
            let conv = if param.is_consuming {
                ParamConvention::Consuming
            } else if param.is_mut {
                ParamConvention::MutBorrow
            } else {
                ParamConvention::Borrow
            };
            convs.push(conv);
        }
        convs
    }

    /// Look up param conventions for a witness (protocol method) call.
    pub(crate) fn collect_witness_conventions(
        &self,
        protocol: Entity,
        method: &WitnessMethodKey,
    ) -> Vec<ParamConvention> {
        let Some(method_entity) = self.find_protocol_method_entity(protocol, method) else {
            return Vec::new();
        };
        let Some(callable) = self.ctx.world.get::<Callable>(method_entity) else {
            return Vec::new();
        };

        let mut conventions = Vec::new();
        if let Some(ref receiver) = callable.receiver {
            let conv = match receiver {
                kestrel_ast_builder::ReceiverKind::Borrowing => ParamConvention::Borrow,
                kestrel_ast_builder::ReceiverKind::Mutating => ParamConvention::MutBorrow,
                kestrel_ast_builder::ReceiverKind::Consuming => ParamConvention::Consuming,
            };
            conventions.push(conv);
        }
        for param in &callable.params {
            let conv = if param.is_consuming {
                ParamConvention::Consuming
            } else if param.is_mut {
                ParamConvention::MutBorrow
            } else {
                ParamConvention::Borrow
            };
            conventions.push(conv);
        }
        conventions
    }

    pub(crate) fn find_protocol_method_entity(
        &self,
        protocol: Entity,
        method: &WitnessMethodKey,
    ) -> Option<Entity> {
        let members = self.ctx.query.query(kestrel_name_res::ProtocolMembersByName {
            protocol,
            name: method.name.clone(),
            context: self.ctx.root,
            root: self.ctx.root,
        });
        for member in &members {
            if let Some(callable) = self.ctx.world.get::<Callable>(member.entity) {
                let member_labels: Vec<Option<&str>> =
                    callable.params.iter().map(|p| p.label.as_deref()).collect();
                let key_labels: Vec<Option<&str>> =
                    method.labels.iter().map(|l| l.as_deref()).collect();
                if member_labels == key_labels {
                    return Some(member.entity);
                }
            } else if method.labels.is_empty() {
                return Some(member.entity);
            }
        }
        None
    }

    /// Unified type-arg resolution for calls.
    pub(crate) fn resolve_call_type_args(
        &mut self,
        expr_id: HirExprId,
        callee_expr: HirExprId,
        callee_entity: Entity,
        is_init: bool,
    ) -> Vec<TyId> {
        let mut type_args = self.resolve_type_args(expr_id);

        let callee_is_call = matches!(self.hir.exprs[callee_expr], HirExpr::Call { .. });
        if type_args.is_empty() && !is_init && !callee_is_call {
            type_args = self.resolve_type_args(callee_expr);
        }

        let has_error = type_args.iter().any(|&a| {
            matches!(self.ctx.module.ty_arena.get(a), MirTy::Error)
        });
        if has_error || (type_args.is_empty() && !is_init) {
            if let Some(fallback) = self.extract_explicit_type_args(callee_expr) {
                type_args = fallback;
            } else if has_error {
                type_args.retain(|&a| !matches!(self.ctx.module.ty_arena.get(a), MirTy::Error));
            }
        }

        if type_args.is_empty() {
            type_args = self.infer_parent_type_args(callee_entity, expr_id, callee_expr);
        }

        type_args
    }

    fn extract_explicit_type_args(&mut self, expr_id: HirExprId) -> Option<Vec<TyId>> {
        let expr = &self.hir.exprs[expr_id];
        match expr {
            HirExpr::Def(_, args, _) if !args.is_empty() => {
                Some(args.iter().map(|ty| lower_type(self.ctx, ty)).collect())
            }
            HirExpr::OverloadSet { type_args, .. } if !type_args.is_empty() => {
                Some(type_args.iter().map(|ty| lower_type(self.ctx, ty)).collect())
            }
            HirExpr::MethodCall { type_args: Some(args), .. } if !args.is_empty() => {
                Some(args.iter().map(|ty| lower_type(self.ctx, ty)).collect())
            }
            _ => None,
        }
    }

    fn infer_parent_type_args(
        &mut self,
        func_entity: Entity,
        expr_id: HirExprId,
        callee_expr: HirExprId,
    ) -> Vec<TyId> {
        let parent = if let Some(func_def) = self.ctx.module.functions.iter().find(|f| f.entity == func_entity) {
            if func_def.type_params.is_empty() {
                return Vec::new();
            }
            match &func_def.kind {
                kestrel_mir_3::item::function::FunctionKind::StaticMethod { parent }
                | kestrel_mir_3::item::function::FunctionKind::Method { parent, .. }
                | kestrel_mir_3::item::function::FunctionKind::Initializer { parent } => *parent,
                _ => return Vec::new(),
            }
        } else {
            let Some(parent) = self.ctx.world.parent_of(func_entity) else {
                return Vec::new();
            };
            match self.ctx.world.get::<NodeKind>(parent) {
                Some(NodeKind::Struct | NodeKind::Enum) => parent,
                Some(NodeKind::Extension) => {
                    match self.ctx.query.query(kestrel_name_res::ExtensionTargetEntity {
                        extension: parent,
                        root: self.ctx.root,
                    }) {
                        Some(target) => target,
                        None => return Vec::new(),
                    }
                }
                _ => return Vec::new(),
            }
        };

        let result_ty = self.resolve_expr_type(expr_id);
        match self.ctx.module.ty_arena.get(result_ty) {
            MirTy::Named { type_args, .. } if !type_args.is_empty() => type_args.clone(),
            _ => {
                let callee_ty = self.resolve_expr_type(callee_expr);
                match self.ctx.module.ty_arena.get(callee_ty) {
                    MirTy::Named { type_args, .. } if !type_args.is_empty() => type_args.clone(),
                    _ => Vec::new(),
                }
            }
        }
    }

    pub(crate) fn is_init_function(&self, entity: Entity) -> Option<Entity> {
        if let Some(f) = self.ctx.module.functions.iter().find(|f| f.entity == entity) {
            match f.kind {
                kestrel_mir_3::item::function::FunctionKind::Initializer { parent } => Some(parent),
                _ => None,
            }
        } else if self.ctx.world.get::<NodeKind>(entity) == Some(&NodeKind::Initializer) {
            self.ctx.world.parent_of(entity)
        } else {
            None
        }
    }
}
