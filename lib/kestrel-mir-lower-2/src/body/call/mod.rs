//! Call dispatch — try_* chain + resolved-entity dispatch.

pub mod args;
pub mod intrinsic;

use kestrel_ast_builder::{Callable, Gettable, NodeKind, Static};
use kestrel_hecs::Entity;
use kestrel_hir::body::{HirCallArg, HirExpr, HirExprId};
use kestrel_hir::ty::HirTy;
use kestrel_mir_2::item::function::FunctionKind;
use kestrel_mir_2::{
    ArgMode, Callee, FieldIdx, Immediate, MirTy, Operand, Place, Rvalue, TyId, UseMode,
    WitnessMethodKey,
};

use super::BodyCtx;
use crate::ty::lower_type;

impl BodyCtx<'_, '_> {
    // === Call expression entry point ===

    pub fn lower_call_expr(
        &mut self,
        expr_id: HirExprId,
        callee_expr: HirExprId,
        args: &[HirCallArg],
    ) -> Operand {
        // Priority 1: intrinsic → Op (includes panic)
        if let Some(entity) = self.resolve_callee_entity_from_expr(callee_expr) {
            if let Some(op) = intrinsic::try_intrinsic(self, expr_id, callee_expr, entity, args) {
                return op;
            }
        }

        // Priority 2: enum case construction → EnumVariant
        if let Some(op) = self.try_enum_construct(expr_id, callee_expr, args) {
            return op;
        }

        // Everything else: resolve entity, type args, build callee, emit
        self.emit_resolved_call(expr_id, callee_expr, args)
    }

    pub fn lower_method_call_expr(
        &mut self,
        expr_id: HirExprId,
        receiver_expr: HirExprId,
        method_name: &str,
        hir_type_args: Option<&[HirTy]>,
        args: &[HirCallArg],
    ) -> Operand {
        let mut receiver_ty = self.resolve_expr_type(receiver_expr);
        let result_ty = self.resolve_expr_type(expr_id);

        // In protocol extensions, a bare `Named(protocol, [])` receiver from
        // inference needs its type params filled in — `build_self_type` produces
        // `Named(protocol, [TypeParam...])` which the monomorphizer substitutes.
        if self.in_protocol_extension {
            if let MirTy::Named { entity, type_args } = self.ctx.module.ty_arena.get(receiver_ty).clone() {
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

        // Static methods: no receiver in args
        let is_static = resolved_entity
            .is_some_and(|e| self.ctx.world.get::<Static>(e).is_some());

        let mut call_args = if is_static {
            self.lower_call_args_default(args)
        } else {
            let receiver_val = self.lower_expr(receiver_expr);
            let receiver_mode = if self.is_copy_or_clone_type(receiver_ty) {
                ArgMode::Copy
            } else {
                ArgMode::Ref
            };
            let mut a = vec![(receiver_val, receiver_mode)];
            a.extend(self.lower_call_args_default(args));
            a
        };

        let Some(resolved) = resolved_entity else {
            return Operand::Const(Immediate::error());
        };

        // Field-subscript: type inference flagged `self.field(args)` as a call
        // through a field. Interpose a field projection so the receiver is
        // the field value, not `self`.
        let (receiver_ty, mut call_args) =
            if let Some(&field_entity) = self.typed.as_ref().and_then(|t| t.field_subscripts.get(&expr_id))
            {
                self.rewrite_field_subscript(receiver_ty, call_args, field_entity, method_name)
            } else {
                (receiver_ty, call_args)
            };

        // Resolve type args
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

        // Function-typed receiver: the "method call" is actually calling a
        // function value (e.g., `self.predicate(char)` where predicate is a
        // stored closure field). Redirect to indirect call.
        if matches!(self.ctx.module.ty_arena.get(receiver_ty), MirTy::FuncThick { .. } | MirTy::FuncThin { .. }) {
            let receiver_val = call_args.remove(0).0;
            let func_place = self.operand_to_place(receiver_val, receiver_ty);
            let callee = match self.ctx.module.ty_arena.get(receiver_ty) {
                MirTy::FuncThin { .. } => Callee::Thin(func_place),
                _ => Callee::Thick(func_place),
            };
            let dest = self.fresh_temp(result_ty);
            self.emit_call(Some(Place::local(dest)), callee, call_args);
            return Operand::Place(Place::local(dest));
        }

        self.expand_default_args(&mut call_args, resolved, args.len());

        // Build callee — one protocol-vs-direct branch
        let callee = if let Some(protocol) = self.ctx.is_protocol_method(resolved) {
            self.ctx.register_name(protocol);
            let key = self.ctx.witness_method_key(resolved);
            self.apply_witness_param_modes(&mut call_args, protocol, &key);
            Callee::Witness {
                protocol,
                method: key,
                self_type: receiver_ty,
                method_type_args,
            }
        } else {
            self.ctx.register_name(resolved);
            self.apply_param_modes(&mut call_args, resolved);
            let mut type_args = self.prepend_receiver_type_args(receiver_ty, method_type_args);
            if let Some(mir_func) = self.ctx.module.functions.iter().find(|f| f.entity == resolved) {
                type_args.truncate(mir_func.type_params.len());
            }
            Callee::direct_with_args(resolved, type_args, None)
        };

        let dest = self.fresh_temp(result_ty);
        self.emit_call(Some(Place::local(dest)), callee, call_args);
        Operand::Place(Place::local(dest))
    }

    /// Rewrite a field-subscript call: replace the receiver with the field value.
    /// For stored fields, projects into the struct; for computed properties, calls the getter.
    fn rewrite_field_subscript(
        &mut self,
        receiver_ty: TyId,
        mut call_args: Vec<(Operand, ArgMode)>,
        field_entity: Entity,
        field_name: &str,
    ) -> (TyId, Vec<(Operand, ArgMode)>) {
        let recv_entity = match self.ctx.module.ty_arena.get(receiver_ty) {
            MirTy::Named { entity, .. } => *entity,
            _ => return (receiver_ty, call_args),
        };

        // Stored field: project into the struct, substituting the struct's
        // type params with the receiver's concrete type args
        if let Some(field_idx) = self.ctx.resolve_field_idx(recv_entity, field_name) {
            let field_ty = self
                .ctx
                .module
                .structs
                .iter()
                .find(|s| s.entity == recv_entity)
                .and_then(|s| s.fields.get(field_idx.index()))
                .map(|f| f.ty);

            let Some(mut field_ty) = field_ty else {
                return (receiver_ty, call_args);
            };

            // Substitute struct type params → receiver type args so nested
            // generics (e.g. Route[T].middleware where T is Router's param)
            // use the caller's type param entities, not the field struct's.
            if let MirTy::Named { type_args, .. } = self.ctx.module.ty_arena.get(receiver_ty) {
                let type_args = type_args.clone();
                if let Some(sdef) = self.ctx.module.structs.iter().find(|s| s.entity == recv_entity) {
                    let mut subst = kestrel_mir_2::substitute::SubstMap::new();
                    for (tp, &arg) in sdef.type_params.iter().zip(type_args.iter()) {
                        subst.type_params.insert(tp.entity, arg);
                    }
                    field_ty = kestrel_mir_2::substitute::substitute(&mut self.ctx.module.ty_arena, field_ty, &subst);
                }
            }

            if let Some((old_receiver, _)) = call_args.first() {
                if let Some(place) = old_receiver.as_place() {
                    let field_place = place.clone().field(field_idx);
                    let field_mode = if self.is_copy_or_clone_type(field_ty) {
                        ArgMode::Copy
                    } else {
                        ArgMode::Ref
                    };
                    call_args[0] = (Operand::Place(field_place), field_mode);
                    return (field_ty, call_args);
                }
            }
            return (receiver_ty, call_args);
        }

        // Computed property: call the getter, use result as subscript receiver
        if self.ctx.world.get::<Gettable>(field_entity).is_some() {
            let field_ty = crate::ty::resolve_type_annotation(self.ctx, field_entity);

            self.ctx.register_name(field_entity);
            let type_args = self.prepend_receiver_type_args(receiver_ty, vec![]);
            let callee = Callee::direct_with_args(field_entity, type_args, None);
            let (old_receiver, _) = call_args.remove(0);
            let getter_dest = self.fresh_temp(field_ty);
            self.emit_call(
                Some(Place::local(getter_dest)),
                callee,
                vec![(old_receiver, ArgMode::Ref)],
            );
            let field_mode = if self.is_copy_or_clone_type(field_ty) {
                ArgMode::Copy
            } else {
                ArgMode::Ref
            };
            call_args.insert(0, (Operand::Place(Place::local(getter_dest)), field_mode));
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
        args: &[HirCallArg],
    ) -> Operand {
        let receiver_ty = self.resolve_expr_type(receiver_expr);
        let result_ty = self.resolve_expr_type(expr_id);
        let receiver_val = self.lower_expr(receiver_expr);

        let receiver_mode = if self.is_copy_or_clone_type(receiver_ty) {
            ArgMode::Copy
        } else {
            ArgMode::Ref
        };
        let mut call_args = vec![(receiver_val, receiver_mode)];
        call_args.extend(self.lower_call_args_default(args));

        self.ctx.register_name(protocol);
        let method_type_args = self.resolve_type_args(expr_id);
        let labels: Vec<Option<String>> = args.iter().map(|a| a.label.clone()).collect();
        let method_key = WitnessMethodKey::new(method_name, labels);

        if let Some(method_entity) = self.find_protocol_method_entity(protocol, &method_key) {
            self.expand_default_args(&mut call_args, method_entity, args.len());
        }

        self.apply_witness_param_modes(&mut call_args, protocol, &method_key);

        let callee = Callee::Witness {
            protocol,
            method: method_key,
            self_type: receiver_ty,
            method_type_args,
        };

        let dest = self.fresh_temp(result_ty);
        self.emit_call(Some(Place::local(dest)), callee, call_args);
        Operand::Place(Place::local(dest))
    }

    // === Resolved-entity dispatch (the common path) ===

    fn emit_resolved_call(
        &mut self,
        expr_id: HirExprId,
        callee_expr: HirExprId,
        args: &[HirCallArg],
    ) -> Operand {
        let result_ty = self.resolve_expr_type(expr_id);

        // Find the resolved entity
        let entity = if let Some(&resolved) = self.typed.as_ref().and_then(|t| t.resolutions.get(&expr_id))
        {
            resolved
        } else if let Some(&resolved) =
            self.typed.as_ref().and_then(|t| t.resolutions.get(&callee_expr))
        {
            resolved
        } else if let HirExpr::Def(e, _, _) = &self.hir.exprs[callee_expr] {
            *e
        } else if let HirExpr::OverloadSet { candidates, .. } = &self.hir.exprs[callee_expr] {
            match candidates.first() {
                Some(&e) => e,
                None => return Operand::Const(Immediate::error()),
            }
        } else {
            // Indirect call (function pointer)
            return self.lower_indirect_call(expr_id, callee_expr, args);
        };

        self.ctx.register_name(entity);

        // Stored function field (e.g., `self.predicate(arg)`) — read the field,
        // call through the function value. Must check before is_init/Callable tests.
        let entity_kind = self.ctx.world.get::<NodeKind>(entity).cloned();
        let has_callable = self.ctx.world.get::<Callable>(entity).is_some();
        if matches!(entity_kind, Some(NodeKind::Field)) && !has_callable {
            return self.lower_indirect_call(expr_id, callee_expr, args);
        }
        // Broader check: if the entity has no Callable and isn't a struct/enum/protocol,
        // it's probably a field or variable with a function type — use indirect call.
        if !has_callable
            && !matches!(entity_kind, Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol | NodeKind::Initializer | NodeKind::Function))
        {
            return self.lower_indirect_call(expr_id, callee_expr, args);
        }

        let is_init = self.is_init_function(entity).is_some();
        let type_args = self.resolve_call_type_args(expr_id, callee_expr, entity, is_init);

        // Init call: allocate self, prepend, special handling
        if is_init {
            return self.emit_init_call(entity, type_args, args, result_ty);
        }

        // Struct memberwise construction (no explicit init)
        if self.is_struct_entity(entity) {
            return self.emit_struct_construct(entity, args, result_ty);
        }

        // Receiver handling for resolved subscripts/computed properties
        let has_receiver = self
            .ctx
            .world
            .get::<Callable>(entity)
            .is_some_and(|c| c.receiver.is_some());

        let mut call_args = self.lower_call_args_default(args);
        if has_receiver {
            let receiver_ty = self.resolve_expr_type(callee_expr);
            let receiver_val = self.lower_expr(callee_expr);
            let receiver_mode = if self.is_copy_or_clone_type(receiver_ty) {
                ArgMode::Copy
            } else {
                ArgMode::Ref
            };
            call_args.insert(0, (receiver_val, receiver_mode));
        }

        self.expand_default_args(&mut call_args, entity, args.len());

        // Protocol vs direct — one branch
        let callee = if let Some(protocol) = self.ctx.is_protocol_method(entity) {
            self.ctx.register_name(protocol);
            let key = self.ctx.witness_method_key(entity);
            let self_type = if key.name == "init" {
                result_ty
            } else {
                self.resolve_expr_type(callee_expr)
            };
            self.apply_witness_param_modes(&mut call_args, protocol, &key);
            Callee::Witness {
                protocol,
                method: key,
                self_type,
                method_type_args: type_args,
            }
        } else {
            self.apply_param_modes(&mut call_args, entity);
            if has_receiver {
                let receiver_ty = self.resolve_expr_type(callee_expr);
                let mut ta = self.prepend_receiver_type_args(receiver_ty, type_args);
                if let Some(mir_func) = self.ctx.module.functions.iter().find(|f| f.entity == entity) {
                    ta.truncate(mir_func.type_params.len());
                }
                // Direct methods don't need self_type — receiver type args are
                // already prepended into type_args. Only protocol default methods
                // need self_type (they use Witness callee, not this branch).
                Callee::direct_with_args(entity, ta, None)
            } else {
                Callee::direct_with_args(entity, type_args, None)
            }
        };

        let dest = self.fresh_temp(result_ty);
        self.emit_call(Some(Place::local(dest)), callee, call_args);
        Operand::Place(Place::local(dest))
    }

    // === Enum case construction ===

    fn try_enum_construct(
        &mut self,
        expr_id: HirExprId,
        callee_expr: HirExprId,
        args: &[HirCallArg],
    ) -> Option<Operand> {
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
        let enum_entity = self.ctx.world.parent_of(entity)
            .unwrap_or_else(|| panic!("ICE: enum case {:?} has no parent", entity));
        let variant_idx = self.ctx.resolve_variant_idx(enum_entity, &case_name)
            .unwrap_or_else(|| panic!(
                "ICE: variant '{}' not found in enum {:?}", case_name, enum_entity
            ));

        let payload: Vec<(Operand, UseMode)> = args
            .iter()
            .map(|arg| {
                let op = self.lower_expr(arg.value);
                let ty = self.resolve_expr_type(arg.value);
                (op, self.use_mode_for(ty))
            })
            .collect();

        let dest = self.fresh_temp(result_ty);
        self.emit_enum_variant(Place::local(dest), result_ty, variant_idx, payload);
        Some(Operand::Place(Place::local(dest)))
    }

    // === Struct memberwise construction ===

    fn emit_struct_construct(
        &mut self,
        struct_entity: Entity,
        args: &[HirCallArg],
        result_ty: TyId,
    ) -> Operand {
        let field_names: Vec<String> = self
            .ctx
            .module
            .structs
            .iter()
            .find(|s| s.entity == struct_entity)
            .map(|s| s.fields.iter().map(|f| f.name.clone()).collect())
            .unwrap_or_default();

        let fields: Vec<(kestrel_mir_2::FieldIdx, Operand, UseMode)> = args
            .iter()
            .enumerate()
            .map(|(i, arg)| {
                let op = self.lower_expr(arg.value);
                let ty = self.resolve_expr_type(arg.value);
                let mode = self.use_mode_for(ty);
                (kestrel_mir_2::FieldIdx::new(i), op, mode)
            })
            .collect();

        let dest = self.fresh_temp(result_ty);
        self.emit_construct(Place::local(dest), result_ty, fields);
        Operand::Place(Place::local(dest))
    }

    // === Init call ===

    fn emit_init_call(
        &mut self,
        entity: Entity,
        type_args: Vec<TyId>,
        args: &[HirCallArg],
        result_ty: TyId,
    ) -> Operand {
        let self_local = self.fresh_temp(result_ty);
        // Mark the local as initialized before the call — the init function
        // writes through the RefMut reference, but the verifier only tracks
        // dest and Move args.  Uninit tells init-state this storage is live.
        self.push_stmt(kestrel_mir_2::statement::StatementKind::Uninit {
            dest: Place::local(self_local),
        });
        let self_ref = Operand::Place(Place::local(self_local));

        let mut call_args = vec![(self_ref, ArgMode::RefMut)];
        call_args.extend(self.lower_call_args_default(args));
        self.expand_default_args(&mut call_args, entity, args.len());

        let callee = if let Some(protocol) = self.ctx.is_protocol_method(entity) {
            self.ctx.register_name(protocol);
            let key = self.ctx.witness_method_key(entity);
            self.apply_witness_param_modes(&mut call_args, protocol, &key);
            Callee::Witness {
                protocol,
                method: key,
                self_type: result_ty,
                method_type_args: type_args,
            }
        } else {
            self.apply_param_modes(&mut call_args, entity);
            // Init functions are direct implementations — self_type is only
            // needed for protocol default methods (where the body has an
            // implicit Self TypeParam). Omitting it avoids duplicate
            // instantiations that differ only by self_type.
            Callee::direct_with_args(entity, type_args, None)
        };

        self.emit_call(None, callee, call_args);
        Operand::Place(Place::local(self_local))
    }

    // === Indirect call ===

    fn lower_indirect_call(
        &mut self,
        expr_id: HirExprId,
        callee_expr: HirExprId,
        args: &[HirCallArg],
    ) -> Operand {
        let callee_ty = self.resolve_expr_type(callee_expr);
        let callee_val = self.lower_expr(callee_expr);
        let result_ty = self.resolve_expr_type(expr_id);
        let call_args = self.lower_call_args_default(args);

        let place = self.operand_to_place(callee_val, callee_ty);
        let callee = match self.ctx.module.ty_arena.get(callee_ty) {
            MirTy::FuncThin { .. } => Callee::Thin(place),
            _ => Callee::Thick(place),
        };

        let dest = self.fresh_temp(result_ty);
        self.emit_call(Some(Place::local(dest)), callee, call_args);
        Operand::Place(Place::local(dest))
    }

    // === Helpers ===

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

    /// Resolve the enum type for an enum case constructor call. Inference
    /// sometimes doesn't record a type for the outer `Call` expression, so
    /// fall back to constructing `Named { entity: parent_enum, type_args }`
    /// from the case entity's parent and any available type arguments.
    fn resolve_enum_case_type(
        &mut self,
        expr_id: HirExprId,
        callee_expr: HirExprId,
        case_entity: Entity,
    ) -> TyId {
        let inferred = self.resolve_expr_type(expr_id);
        match self.ctx.module.ty_arena.get(inferred).clone() {
            MirTy::Named { type_args, .. } if type_args.is_empty() => {
                // Check if the enum is generic — if so, supplement type args
                let parent = self.ctx.world.parent_of(case_entity)
                    .unwrap_or_else(|| panic!("ICE: enum case {:?} has no parent", case_entity));
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
            }
            MirTy::Error => {
                // Inference didn't record a type — construct from parent enum entity
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
