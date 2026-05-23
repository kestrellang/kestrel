//! Shared arg lowering, type-arg resolution, mode assignment, defaults.

use std::collections::HashMap;

use kestrel_ast_builder::{Attributes, Callable, NodeKind};
use kestrel_hecs::Entity;
use kestrel_hir::body::{HirCallArg, HirExpr, HirExprId};
use kestrel_hir::res::LocalId as HirLocalId;
use kestrel_hir_lower::LowerBody;
use kestrel_mir_2::body::LocalDef;
use kestrel_mir_2::{ArgMode, BlockId, Immediate, LocalId, MirTy, Operand, ParamConvention, TyId};
use kestrel_type_infer::InferBody;

use crate::body::{BodyCtx, HirRef, TypedRef};
use crate::ty::{lower_resolved_ty, lower_type, resolve_type_annotation};

impl BodyCtx<'_, '_> {
    /// Lower call args to (Operand, ArgMode) pairs with default mode (Copy for
    /// copyable, Move for non-copyable). Mode overrides happen in `apply_param_modes`.
    pub(crate) fn lower_call_args_default(
        &mut self,
        args: &[HirCallArg],
    ) -> Vec<(Operand, ArgMode)> {
        args.iter()
            .map(|arg| {
                let op = self.lower_expr(arg.value);
                let ty = self.resolve_expr_type(arg.value);
                let mode = if self.is_copy_type(ty) {
                    ArgMode::Copy
                } else {
                    ArgMode::Ref
                };
                (op, mode)
            })
            .collect()
    }


    /// Fill in missing default arguments for a call by inline-lowering
    /// each default expression into the current function body.
    ///
    /// `callee_entity` is the resolved function/method entity.
    /// `explicit_count` is the number of explicit HIR args (not counting receiver).
    pub(crate) fn expand_default_args(
        &mut self,
        call_args: &mut Vec<(Operand, ArgMode)>,
        callee_entity: Entity,
        explicit_count: usize,
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

        for default_entity in defaults {
            let param_ty = resolve_type_annotation(self.ctx, default_entity);
            let default_val = self.lower_default_arg_inline(default_entity, param_ty);
            let mode = if self.is_copy_type(param_ty) {
                ArgMode::Copy
            } else {
                ArgMode::Ref
            };
            call_args.push((default_val, mode));
        }
    }

    /// Lower a default parameter expression inline into the current
    /// function body. Temporarily swaps the active HirBody/TypedBody
    /// to the default entity's, lowers the tail expression, then restores.
    fn lower_default_arg_inline(
        &mut self,
        default_entity: Entity,
        param_ty: TyId,
    ) -> Operand {
        let Some(default_hir) = self.ctx.query.query(LowerBody {
            entity: default_entity,
            root: self.ctx.root,
        }) else {
            return Operand::Const(Immediate::error());
        };
        let default_typed = self.ctx.query.query(InferBody {
            entity: default_entity,
            root: self.ctx.root,
        });

        let tail = default_hir.tail_expr;

        // Save current state
        let saved_hir = std::mem::replace(&mut self.hir, HirRef::Owned(default_hir));
        let saved_typed = std::mem::replace(
            &mut self.typed,
            default_typed.map(TypedRef::Owned),
        );
        let saved_local_map = std::mem::take(&mut self.local_map);

        // Create locals for the default body's HIR locals
        let default_locals: Vec<_> = self.hir.locals.iter().map(|(id, l)| (id, l.name.clone())).collect();
        for (hir_id, name) in &default_locals {
            let ty = self.resolve_local_type(*hir_id);
            let mir_local = LocalDef::new(name, ty);
            let mir_id = self.body.add_local(mir_local);
            self.local_map.insert(*hir_id, mir_id);
        }

        // Lower the tail expression
        let result = if let Some(tail_id) = tail {
            self.lower_expr(tail_id)
        } else {
            Operand::Const(Immediate::unit())
        };

        // Restore
        self.hir = saved_hir;
        self.typed = saved_typed;
        self.local_map = saved_local_map;

        result
    }

    /// Apply param modes from the callee's declared conventions.
    pub(crate) fn apply_param_modes(
        &self,
        call_args: &mut [(Operand, ArgMode)],
        callee_entity: Entity,
    ) {
        // Try MIR FunctionDef first
        if let Some(callee) = self
            .ctx
            .module
            .functions
            .iter()
            .find(|f| f.entity == callee_entity)
        {
            if callee.extern_info.is_some() {
                return;
            }
            for (arg, param) in call_args.iter_mut().zip(callee.params.iter()) {
                arg.1 = match param.convention {
                    ParamConvention::Borrow => ArgMode::Ref,
                    ParamConvention::MutBorrow => ArgMode::RefMut,
                    ParamConvention::Consuming => ArgMode::Move,
                };
            }
            return;
        }
        // ECS fallback for forward references
        let Some(callable) = self.ctx.world.get::<Callable>(callee_entity) else {
            return;
        };
        let is_extern = self
            .ctx
            .world
            .get::<Attributes>(callee_entity)
            .is_some_and(|attrs| attrs.0.iter().any(|a| a.name == "extern"));
        if is_extern {
            return;
        }
        let skip = if callable.receiver.is_some() { 1 } else { 0 };
        for (arg, param) in call_args.iter_mut().skip(skip).zip(callable.params.iter()) {
            arg.1 = if param.is_consuming {
                ArgMode::Move
            } else if param.is_mut {
                ArgMode::RefMut
            } else {
                ArgMode::Ref
            };
        }
    }

    /// Apply param modes for witness calls from the protocol method's conventions.
    pub(crate) fn apply_witness_param_modes(
        &self,
        call_args: &mut [(Operand, ArgMode)],
        protocol: Entity,
        method: &kestrel_mir_2::WitnessMethodKey,
    ) {
        let Some(method_entity) = self.find_protocol_method_entity(protocol, method) else {
            return;
        };
        let Some(callable) = self.ctx.world.get::<Callable>(method_entity) else {
            return;
        };
        let skip = if let Some(ref receiver) = callable.receiver {
            if !call_args.is_empty() {
                call_args[0].1 = match receiver {
                    kestrel_ast_builder::ReceiverKind::Borrowing => ArgMode::Ref,
                    kestrel_ast_builder::ReceiverKind::Mutating => ArgMode::RefMut,
                    kestrel_ast_builder::ReceiverKind::Consuming => {
                        if matches!(call_args[0].1, ArgMode::Copy) {
                            ArgMode::Copy
                        } else {
                            ArgMode::Move
                        }
                    }
                };
            }
            1
        } else {
            0
        };
        for (arg, param) in call_args.iter_mut().skip(skip).zip(callable.params.iter()) {
            arg.1 = if param.is_consuming {
                if matches!(arg.1, ArgMode::Copy) {
                    ArgMode::Copy
                } else {
                    ArgMode::Move
                }
            } else if param.is_mut {
                ArgMode::RefMut
            } else {
                ArgMode::Ref
            };
        }
    }

    pub(crate) fn find_protocol_method_entity(
        &self,
        protocol: Entity,
        method: &kestrel_mir_2::WitnessMethodKey,
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

    /// Unified type-arg resolution for calls. Single cascade, called once.
    pub(crate) fn resolve_call_type_args(
        &mut self,
        expr_id: HirExprId,
        callee_expr: HirExprId,
        callee_entity: Entity,
        is_init: bool,
    ) -> Vec<TyId> {
        // 1. Inference on call expression
        let mut type_args = self.resolve_type_args(expr_id);

        // 2. Inference on callee expression (skip for inits and chained calls)
        let callee_is_call = matches!(
            self.hir.exprs[callee_expr],
            HirExpr::Call { .. }
        );
        if type_args.is_empty() && !is_init && !callee_is_call {
            type_args = self.resolve_type_args(callee_expr);
        }

        // 3. Explicit AST type args
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

        // 4. Parent struct type args (for static methods on generic types)
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
            HirExpr::MethodCall {
                type_args: Some(args),
                ..
            } if !args.is_empty() => Some(args.iter().map(|ty| lower_type(self.ctx, ty)).collect()),
            _ => None,
        }
    }

    fn infer_parent_type_args(
        &mut self,
        func_entity: Entity,
        expr_id: HirExprId,
        callee_expr: HirExprId,
    ) -> Vec<TyId> {
        // Find parent from FunctionDef or ECS
        let parent = if let Some(func_def) = self
            .ctx
            .module
            .functions
            .iter()
            .find(|f| f.entity == func_entity)
        {
            if func_def.type_params.is_empty() {
                return Vec::new();
            }
            match &func_def.kind {
                kestrel_mir_2::item::function::FunctionKind::StaticMethod { parent }
                | kestrel_mir_2::item::function::FunctionKind::Method { parent, .. }
                | kestrel_mir_2::item::function::FunctionKind::Initializer { parent } => *parent,
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

        // Try inference type args from the result type
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

    /// Check if an entity is an init function. Returns the parent struct entity.
    pub(crate) fn is_init_function(&self, entity: Entity) -> Option<Entity> {
        if let Some(f) = self.ctx.module.functions.iter().find(|f| f.entity == entity) {
            match f.kind {
                kestrel_mir_2::item::function::FunctionKind::Initializer { parent } => {
                    Some(parent)
                }
                _ => None,
            }
        } else {
            if self.ctx.world.get::<NodeKind>(entity) == Some(&NodeKind::Initializer) {
                self.ctx.world.parent_of(entity)
            } else {
                None
            }
        }
    }
}
