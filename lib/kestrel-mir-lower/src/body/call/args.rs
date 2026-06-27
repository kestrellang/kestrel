use kestrel_ast_builder::arg_binding::{BindParam, Binding, bind_arguments};
use kestrel_ast_builder::{Attributes, Callable, DeclSpan, NodeKind};
use kestrel_hecs::Entity;
use kestrel_hir::body::{HirCallArg, HirExpr, HirExprId};
use kestrel_hir_lower::LowerBody;
use kestrel_mir::inst::CallArg;
use kestrel_mir::item::witness::WitnessMethodKey;
use kestrel_mir::{Immediate, MirTy, ParamConvention, TyId, ValueId};
use kestrel_reporting::{Diagnostic, Label};
use kestrel_span::Span;
use kestrel_type_infer::InferBody;

use crate::body::{HirRef, OssaBodyCtx, TypedRef};
use crate::items::function_sig::receiver_convention;
use crate::ty::{lower_type, resolve_type_annotation};

fn conventions_from_callable(callable: &Callable, is_extern: bool) -> Vec<ParamConvention> {
    if is_extern {
        let count = callable.receiver.as_ref().map_or(0, |_| 1) + callable.params.len();
        return vec![ParamConvention::Consuming; count];
    }
    let mut convs = Vec::new();
    if let Some(recv) = &callable.receiver {
        convs.push(receiver_convention(recv));
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
                let conv = conventions
                    .get(offset + i)
                    .copied()
                    .unwrap_or(ParamConvention::Borrow);
                self.prepare_call_arg_for_expr(arg.value, conv)
            })
            .collect()
    }

    /// Lower a call's explicit arguments AND fill in defaults, producing one
    /// `CallArg` per (non-receiver) parameter in declaration order.
    ///
    /// Unlike `lower_call_args` + `expand_default_args`, this uses
    /// `arg_binding::bind_arguments`, so defaulted parameters may be skipped
    /// anywhere (leading/middle/trailing), not just at the trailing end. Each
    /// explicit argument is lowered with the convention of the parameter it
    /// actually binds to. Explicit arguments are evaluated in source order
    /// (left-to-right); defaults are lowered afterward, in their slots.
    ///
    /// `conventions`/`conv_offset` index the callee's full convention list,
    /// `conv_offset` skipping any receiver slot.
    pub(crate) fn lower_call_args_bound(
        &mut self,
        args: &[HirCallArg],
        callee_entity: Entity,
        conventions: &[ParamConvention],
        conv_offset: usize,
        callee_type_args: &[TyId],
    ) -> Vec<CallArg> {
        // Snapshot the parameter shape (owned) so the world borrow is released
        // before we lower any argument expressions.
        let shape: Option<Vec<(Option<String>, Option<Entity>)>> =
            self.ctx.world.get::<Callable>(callee_entity).map(|c| {
                c.params
                    .iter()
                    .map(|p| (p.label.clone(), p.default_entity))
                    .collect()
            });
        let Some(shape) = shape else {
            // No parameter metadata: lower in source order (no defaults to fill).
            return self.lower_call_args(args, conventions, conv_offset);
        };

        let bind_params: Vec<BindParam> = shape
            .iter()
            .map(|(label, default)| BindParam::new(label.as_deref(), default.is_some()))
            .collect();
        let arg_labels: Vec<Option<&str>> = args.iter().map(|a| a.label.as_deref()).collect();
        let plan = match bind_arguments(&bind_params, &arg_labels) {
            Ok(plan) => plan,
            // Binding was already validated during inference; if it somehow fails
            // here, fall back to the legacy positional path rather than panic.
            Err(_) => {
                let mut ca = self.lower_call_args(args, conventions, conv_offset);
                self.expand_default_args(
                    &mut ca,
                    callee_entity,
                    args.len(),
                    conventions,
                    conv_offset,
                    callee_type_args,
                );
                return ca;
            },
        };

        // For each explicit (source) arg, the parameter slot it binds to — used
        // to pick its convention. `bind_arguments` forbids reordering, so source
        // order maps to slots monotonically.
        let mut arg_slot = vec![0usize; args.len()];
        for (pi, binding) in plan.iter().enumerate() {
            if let Binding::Arg(ai) = binding {
                arg_slot[*ai] = pi;
            }
        }

        // Lower explicit args in source order, each with its bound param's convention.
        let mut prepared = Vec::with_capacity(args.len());
        for (ai, arg) in args.iter().enumerate() {
            let conv = conventions
                .get(conv_offset + arg_slot[ai])
                .copied()
                .unwrap_or(ParamConvention::Borrow);
            prepared.push(self.prepare_call_arg_for_expr(arg.value, conv));
        }

        // Assemble in parameter order: explicit args via a source-order cursor,
        // defaults lowered into their slots.
        let mut prepared = prepared.into_iter();
        let mut result = Vec::with_capacity(plan.len());
        for (pi, binding) in plan.iter().enumerate() {
            match binding {
                Binding::Arg(_) => {
                    if let Some(arg) = prepared.next() {
                        result.push(arg);
                    }
                },
                Binding::Default => {
                    let Some(default_entity) = shape[pi].1 else {
                        continue; // required-but-missing: impossible post-typecheck
                    };
                    let param_ty = resolve_type_annotation(self.ctx, default_entity);
                    let default_val =
                        self.lower_default_arg_inline(default_entity, param_ty, callee_type_args);
                    let conv = conventions
                        .get(conv_offset + pi)
                        .copied()
                        .unwrap_or(ParamConvention::Borrow);
                    result.push(self.prepare_call_arg(default_val, conv));
                },
            }
        }
        result
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
        callee_type_args: &[TyId],
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
            let default_val =
                self.lower_default_arg_inline(default_entity, param_ty, callee_type_args);
            let conv = conventions
                .get(conv_offset + explicit_count + di)
                .copied()
                .unwrap_or(ParamConvention::Borrow);
            let arg = self.prepare_call_arg(default_val, conv);
            call_args.push(arg);
        }
    }

    /// Build the type substitution (#148) mapping the callee's type params to a
    /// call site's concrete type args, for inline-lowering a default-argument
    /// expression. `default_entity` is the defaulted PARAM; its parent is the
    /// callee, whose `TypeParams` are zipped positionally with `callee_type_args`.
    /// Returns `None` (no substitution) when the callee is non-generic or no
    /// args were supplied.
    fn default_arg_subst_map(
        &self,
        default_entity: Entity,
        callee_type_args: &[TyId],
    ) -> Option<kestrel_mir::SubstMap> {
        if callee_type_args.is_empty() {
            return None;
        }
        let callee = self.ctx.world.parent_of(default_entity)?;
        let tps = self.ctx.world.get::<kestrel_ast_builder::TypeParams>(callee)?;
        let mut subst = kestrel_mir::SubstMap::new();
        for (&tp, &arg) in tps.0.iter().zip(callee_type_args.iter()) {
            subst.type_params.insert(tp, arg);
        }
        (!subst.type_params.is_empty()).then_some(subst)
    }

    fn lower_default_arg_inline(
        &mut self,
        default_entity: Entity,
        _param_ty: TyId,
        callee_type_args: &[TyId],
    ) -> ValueId {
        let Some(default_hir) = self.ctx.query.query(LowerBody {
            entity: default_entity,
            root: self.ctx.root,
        }) else {
            // The parameter has a default value but no lowerable body — a
            // structural inconsistency (a type error would still yield a body
            // with error nodes), so surface it instead of silently substituting
            // a garbage value.
            let span = self
                .ctx
                .world
                .get::<DeclSpan>(default_entity)
                .map(|s| s.0.clone())
                .unwrap_or_else(|| Span::synthetic(0));
            self.ctx.query.accumulate(
                Diagnostic::error()
                    .with_message("could not lower the default argument value")
                    .with_labels(vec![
                        Label::primary(span.file_id, span.range())
                            .with_message("default value could not be lowered to MIR"),
                    ]),
            );
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
        // The default body was inferred against the callee's generic type params;
        // substitute them to this call's concrete args so the callee's TypeParam
        // doesn't leak into the caller's body (#148). Nested default lowering is
        // not possible (a default expr can't itself call with defaults mid-lower
        // in a way that re-enters here before restore), so a plain save/restore
        // of the scalar field is sufficient.
        let saved_subst = self
            .default_arg_subst
            .replace(self.default_arg_subst_map(default_entity, callee_type_args).unwrap_or_default());

        // Create values for the default body's HIR locals
        let default_locals: Vec<_> = self
            .hir
            .locals
            .iter()
            .map(|(id, l)| (id, l.clone()))
            .collect();
        for (hir_id, _local) in &default_locals {
            let ty = self.resolve_local_type(*hir_id);
            let val = self.alloc_value_auto(ty);
            self.local_map
                .insert(*hir_id, crate::body::LocalBinding::Ssa(val));
        }

        let result = if let Some(tail_id) = tail {
            self.lower_expr(tail_id)
        } else {
            self.emit_literal(Immediate::unit())
        };

        self.hir = saved_hir;
        self.typed = saved_typed;
        self.local_map = saved_local_map;
        self.default_arg_subst = saved_subst;

        result
    }

    /// Look up param conventions for a callee from its FunctionDef or ECS Callable.
    pub(crate) fn collect_conventions(&self, callee_entity: Entity) -> Vec<ParamConvention> {
        // Try MIR FunctionDef first
        if let Some(callee) = self.ctx.module.functions.get(&callee_entity) {
            if callee.extern_info.is_some() {
                return callee
                    .params
                    .iter()
                    .map(|_| ParamConvention::Consuming)
                    .collect();
            }
            return callee.params.iter().map(|p| p.convention).collect();
        }
        // ECS fallback
        let Some(callable) = self.ctx.world.get::<Callable>(callee_entity) else {
            return Vec::new();
        };
        let is_extern = self
            .ctx
            .world
            .get::<Attributes>(callee_entity)
            .is_some_and(|attrs| attrs.0.iter().any(|a| a.name == "extern"));
        conventions_from_callable(callable, is_extern)
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
        conventions_from_callable(callable, false)
    }

    pub(crate) fn find_protocol_method_entity(
        &self,
        protocol: Entity,
        method: &WitnessMethodKey,
    ) -> Option<Entity> {
        let members = self
            .ctx
            .query
            .query(kestrel_name_res::ProtocolMembersByName {
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

        let has_error = type_args
            .iter()
            .any(|&a| matches!(self.ctx.module.ty_arena.get(a), MirTy::Error));
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
            },
            HirExpr::OverloadSet { type_args, .. } if !type_args.is_empty() => Some(
                type_args
                    .iter()
                    .map(|ty| lower_type(self.ctx, ty))
                    .collect(),
            ),
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
        let _parent = if let Some(func_def) = self.ctx.module.functions.get(&func_entity) {
            if func_def.type_params.is_empty() {
                return Vec::new();
            }
            match &func_def.kind {
                kestrel_mir::item::function::FunctionKind::StaticMethod { parent }
                | kestrel_mir::item::function::FunctionKind::Method { parent, .. }
                | kestrel_mir::item::function::FunctionKind::Initializer { parent } => *parent,
                _ => return Vec::new(),
            }
        } else {
            let Some(parent) = self.ctx.world.parent_of(func_entity) else {
                return Vec::new();
            };
            match self.ctx.world.get::<NodeKind>(parent) {
                Some(NodeKind::Struct | NodeKind::Enum) => parent,
                Some(NodeKind::Extension) => {
                    match self
                        .ctx
                        .query
                        .query(kestrel_name_res::ExtensionTargetEntity {
                            extension: parent,
                            root: self.ctx.root,
                        }) {
                        Some(target) => target,
                        None => return Vec::new(),
                    }
                },
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
            },
        }
    }

    pub(crate) fn is_init_function(&self, entity: Entity) -> Option<Entity> {
        if let Some(f) = self.ctx.module.functions.get(&entity) {
            match f.kind {
                kestrel_mir::item::function::FunctionKind::Initializer { parent } => Some(parent),
                _ => None,
            }
        } else if self.ctx.world.get::<NodeKind>(entity) == Some(&NodeKind::Initializer) {
            self.ctx.world.parent_of(entity)
        } else {
            None
        }
    }
}
