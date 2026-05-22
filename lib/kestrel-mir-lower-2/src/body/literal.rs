//! Literal lowering — primitives, strings, arrays, dicts.

use kestrel_ast_builder::NodeKind;
use kestrel_hecs::Entity;
use kestrel_hir::body::{HirDictEntry, HirExprId, HirLiteral};
use kestrel_mir_2::{
    ArgMode, Callee, FieldIdx, Immediate, IntBits, MirTy, Op, Operand, ParamConvention,
    Place, Rvalue, Signedness, TyId, UseMode,
};

use super::BodyCtx;
use crate::ty::resolve_callable_types;

impl BodyCtx<'_, '_> {
    pub fn lower_literal(&mut self, expr_id: HirExprId, lit: &HirLiteral) -> Operand {
        let result_ty = self.resolve_expr_type(expr_id);

        // Named struct → wrap via literal protocol init
        if let MirTy::Named { entity, .. } = self.ctx.module.ty_arena.get(result_ty).clone() {
            match lit {
                HirLiteral::String { value: content, .. } => {
                    if let Some(init) = self.find_string_literal_init(entity) {
                        let ptr = Operand::Const(Immediate::string(content.clone()));
                        let len = Operand::Const(Immediate::i64(content.len() as i128));
                        self.ctx.register_name(init);
                        let callee = Callee::direct_with_args(init, vec![], Some(result_ty));
                        return self.emit_init_literal_call(callee, vec![
                            (ptr, ArgMode::Copy), (len, ArgMode::Copy),
                        ], result_ty);
                    }
                    return self.lower_literal_primitive(lit, result_ty);
                }
                HirLiteral::Null => {
                    if let Some(init) = self.find_null_literal_init(entity) {
                        self.ctx.register_name(init);
                        let type_args = self.prepend_receiver_type_args(result_ty, vec![]);
                        let callee = Callee::direct_with_args(init, type_args, Some(result_ty));
                        return self.emit_init_literal_call(callee, vec![], result_ty);
                    }
                    return self.lower_literal_primitive(lit, result_ty);
                }
                _ => {}
            }

            let (label, protocol) = match lit {
                HirLiteral::Bool(_) => ("boolLiteral", kestrel_hir::Builtin::ExpressibleByBoolLiteral),
                HirLiteral::Integer(_) => ("intLiteral", kestrel_hir::Builtin::ExpressibleByIntegerLiteral),
                HirLiteral::Float(_) => ("floatLiteral", kestrel_hir::Builtin::ExpressibleByFloatLiteral),
                HirLiteral::Char(_) => ("charLiteral", kestrel_hir::Builtin::ExpressibleByCharLiteral),
                _ => return self.lower_literal_primitive(lit, result_ty),
            };

            if let Some(init) = self.find_literal_init(entity, protocol, label) {
                let param_ty = self.resolve_init_param_type(init).unwrap_or(result_ty);
                let primitive = self.lower_literal_primitive(lit, param_ty);
                self.ctx.register_name(init);
                let callee = Callee::direct_with_args(init, vec![], Some(result_ty));
                return self.emit_init_literal_call(callee, vec![(primitive, ArgMode::Copy)], result_ty);
            }
        }

        self.lower_literal_primitive(lit, result_ty)
    }

    fn lower_literal_primitive(&self, lit: &HirLiteral, target_ty: TyId) -> Operand {
        match lit {
            HirLiteral::Integer(v) => {
                let imm = match self.ctx.module.ty_arena.get(target_ty) {
                    MirTy::I8 => Immediate::i8(*v as i128),
                    MirTy::I16 => Immediate::i16(*v as i128),
                    MirTy::I32 => Immediate::i32(*v as i128),
                    _ => Immediate::i64(*v as i128),
                };
                Operand::Const(imm)
            }
            HirLiteral::Float(v) => {
                let imm = match self.ctx.module.ty_arena.get(target_ty) {
                    MirTy::F32 => Immediate::f32(*v),
                    _ => Immediate::f64(*v),
                };
                Operand::Const(imm)
            }
            HirLiteral::Bool(v) => Operand::Const(Immediate::bool(*v)),
            HirLiteral::String { value, .. } => Operand::Const(Immediate::string(value.clone())),
            HirLiteral::Char(c) => Operand::Const(Immediate::i32(*c as i128)),
            HirLiteral::Null => Operand::Const(Immediate::unit()),
        }
    }

    /// Emit an init call for a literal (allocate self, prepend &mut, call).
    pub(crate) fn emit_init_literal_call(
        &mut self,
        callee: Callee,
        args: Vec<(Operand, ArgMode)>,
        result_ty: TyId,
    ) -> Operand {
        let self_local = self.fresh_temp(result_ty);
        let mut call_args = vec![(Operand::Place(Place::local(self_local)), ArgMode::RefMut)];
        call_args.extend(args);
        self.emit_call(None, callee, call_args);
        Operand::Place(Place::local(self_local))
    }

    // === Array literal ===

    pub fn lower_array_literal(
        &mut self,
        expr_id: HirExprId,
        elements: &[HirExprId],
    ) -> Operand {
        let result_ty = self.resolve_expr_type(expr_id);

        if let Some(op) = self.try_array_literal_via_init(elements, result_ty) {
            return op;
        }

        // Fallback: raw ArrayLiteral rvalue
        let element_ty = if let MirTy::Named { type_args, .. } = self.ctx.module.ty_arena.get(result_ty) {
            type_args.first().copied().unwrap_or_else(|| self.ctx.module.ty_arena.error())
        } else {
            self.ctx.module.ty_arena.error()
        };

        let values: Vec<(Operand, UseMode)> = elements
            .iter()
            .map(|&e| {
                let op = self.lower_expr(e);
                let ty = self.resolve_expr_type(e);
                (op, self.use_mode_for(ty))
            })
            .collect();

        let dest = self.fresh_temp(result_ty);
        self.emit_assign(
            Place::local(dest),
            Rvalue::ArrayLiteral {
                element_ty,
                values,
            },
        );
        Operand::Place(Place::local(dest))
    }

    fn try_array_literal_via_init(
        &mut self,
        elements: &[HirExprId],
        result_ty: TyId,
    ) -> Option<Operand> {
        let (init_entity, element_ty, type_args) = self.resolve_array_literal_init(result_ty)?;

        let ptr_ty = self.ctx.module.ty_arena.pointer(element_ty);
        let i64_ty = self.ctx.module.ty_arena.i64();
        let unit_ty = self.ctx.module.ty_arena.unit();

        // StackAlloc
        let ptr_local = self.fresh_temp(ptr_ty);
        let count = Operand::Const(Immediate::i64(elements.len() as i128));
        self.emit_assign(
            Place::local(ptr_local),
            Rvalue::Op1 {
                op: Op::StackAlloc(element_ty),
                arg: count.clone(),
            },
        );

        // Element stride (bytes per element)
        let size_local = self.fresh_temp(i64_ty);
        self.emit_assign(
            Place::local(size_local),
            Rvalue::Op1 {
                op: Op::SizeOf(element_ty),
                arg: Operand::Const(Immediate::unit()),
            },
        );

        // Write elements
        for (i, &elem_expr) in elements.iter().enumerate() {
            let elem_val = self.lower_expr(elem_expr);
            let elem_ptr = if i == 0 {
                Operand::Place(Place::local(ptr_local))
            } else {
                // offset = i * size
                let offset_local = self.fresh_temp(i64_ty);
                self.emit_assign_op2(
                    Place::local(offset_local),
                    Op::Mul(IntBits::I64, Signedness::Signed),
                    Operand::Const(Immediate::i64(i as i128)),
                    Operand::Place(Place::local(size_local)),
                );
                let offset_ptr = self.fresh_temp(ptr_ty);
                self.emit_assign_op2(
                    Place::local(offset_ptr),
                    Op::PtrOffset,
                    Operand::Place(Place::local(ptr_local)),
                    Operand::Place(Place::local(offset_local)),
                );
                Operand::Place(Place::local(offset_ptr))
            };
            // PtrWrite
            let write_dest = self.fresh_temp(unit_ty);
            self.emit_assign_op2(
                Place::local(write_dest),
                Op::PtrWrite(element_ty),
                elem_ptr,
                elem_val,
            );
        }

        // Call init(ptr, count)
        self.ctx.register_name(init_entity);
        let callee = Callee::direct_with_args(init_entity, type_args, Some(result_ty));
        let self_local = self.fresh_temp(result_ty);
        let call_args = vec![
            (Operand::Place(Place::local(self_local)), ArgMode::RefMut),
            (Operand::Place(Place::local(ptr_local)), ArgMode::Copy),
            (count, ArgMode::Copy),
        ];
        self.emit_call(None, callee, call_args);
        Some(Operand::Place(Place::local(self_local)))
    }

    // === Dict literal ===

    pub fn lower_dict_literal(
        &mut self,
        expr_id: HirExprId,
        entries: &[HirDictEntry],
    ) -> Operand {
        let result_ty = self.resolve_expr_type(expr_id);

        if let Some(op) = self.try_dict_literal_via_init(entries, result_ty) {
            return op;
        }

        Operand::Const(Immediate::error())
    }

    fn try_dict_literal_via_init(
        &mut self,
        entries: &[HirDictEntry],
        result_ty: TyId,
    ) -> Option<Operand> {
        let (init_entity, pair_ty, type_args) = self.resolve_dict_literal_init(result_ty)?;

        let MirTy::Tuple(elem_tys) = self.ctx.module.ty_arena.get(pair_ty).clone() else {
            return None;
        };
        if elem_tys.len() != 2 {
            return None;
        }

        let ptr_ty = self.ctx.module.ty_arena.pointer(pair_ty);
        let i64_ty = self.ctx.module.ty_arena.i64();
        let unit_ty = self.ctx.module.ty_arena.unit();

        let ptr_local = self.fresh_temp(ptr_ty);
        let count = Operand::Const(Immediate::i64(entries.len() as i128));
        self.emit_assign(
            Place::local(ptr_local),
            Rvalue::Op1 {
                op: Op::StackAlloc(pair_ty),
                arg: count.clone(),
            },
        );

        let size_local = self.fresh_temp(i64_ty);
        self.emit_assign(
            Place::local(size_local),
            Rvalue::Op1 {
                op: Op::SizeOf(pair_ty),
                arg: Operand::Const(Immediate::unit()),
            },
        );

        for (i, entry) in entries.iter().enumerate() {
            let key = self.lower_expr(entry.key);
            let val = self.lower_expr(entry.value);
            let key_ty = self.resolve_expr_type(entry.key);
            let val_ty = self.resolve_expr_type(entry.value);

            let pair_dest = self.fresh_temp(pair_ty);
            self.emit_tuple(
                Place::local(pair_dest),
                vec![
                    (key, self.use_mode_for(key_ty)),
                    (val, self.use_mode_for(val_ty)),
                ],
            );

            let elem_ptr = if i == 0 {
                Operand::Place(Place::local(ptr_local))
            } else {
                let offset_local = self.fresh_temp(i64_ty);
                self.emit_assign_op2(
                    Place::local(offset_local),
                    Op::Mul(IntBits::I64, Signedness::Signed),
                    Operand::Const(Immediate::i64(i as i128)),
                    Operand::Place(Place::local(size_local)),
                );
                let offset_ptr = self.fresh_temp(ptr_ty);
                self.emit_assign_op2(
                    Place::local(offset_ptr),
                    Op::PtrOffset,
                    Operand::Place(Place::local(ptr_local)),
                    Operand::Place(Place::local(offset_local)),
                );
                Operand::Place(Place::local(offset_ptr))
            };

            let write_dest = self.fresh_temp(unit_ty);
            self.emit_assign_op2(
                Place::local(write_dest),
                Op::PtrWrite(pair_ty),
                elem_ptr,
                Operand::Place(Place::local(pair_dest)),
            );
        }

        self.ctx.register_name(init_entity);
        let callee = Callee::direct_with_args(init_entity, type_args, Some(result_ty));
        let self_local = self.fresh_temp(result_ty);
        let call_args = vec![
            (Operand::Place(Place::local(self_local)), ArgMode::RefMut),
            (Operand::Place(Place::local(ptr_local)), ArgMode::Copy),
            (count, ArgMode::Copy),
        ];
        self.emit_call(None, callee, call_args);
        Some(Operand::Place(Place::local(self_local)))
    }

    // === Init discovery helpers ===

    fn find_literal_protocol_init(
        &self,
        struct_entity: Entity,
        protocol: kestrel_hir::Builtin,
        predicate: impl Fn(&kestrel_ast_builder::Callable) -> bool,
    ) -> Option<Entity> {
        let proto_entity = self.ctx.query.query(kestrel_name_res::ResolveBuiltin {
            builtin: protocol,
            root: self.ctx.root,
        })?;
        kestrel_name_res::find_protocol_witness_init(
            &self.ctx.query,
            struct_entity,
            proto_entity,
            self.ctx.root,
            predicate,
        )
    }

    fn find_literal_init(
        &self,
        struct_entity: Entity,
        protocol: kestrel_hir::Builtin,
        label: &str,
    ) -> Option<Entity> {
        self.find_literal_protocol_init(struct_entity, protocol, |c| {
            c.params.len() == 1 && c.params[0].label.as_deref() == Some(label)
        })
    }

    fn find_null_literal_init(&self, struct_entity: Entity) -> Option<Entity> {
        self.find_literal_protocol_init(
            struct_entity,
            kestrel_hir::Builtin::ExpressibleByNullLiteral,
            |c| c.params.is_empty(),
        )
    }

    pub(crate) fn find_string_literal_init(&self, struct_entity: Entity) -> Option<Entity> {
        self.find_literal_protocol_init(
            struct_entity,
            kestrel_hir::Builtin::ExpressibleByStringLiteral,
            |c| c.params.len() == 2 && c.params[0].label.as_deref() == Some("stringLiteral"),
        )
    }

    fn resolve_init_param_type(&mut self, init_entity: Entity) -> Option<TyId> {
        let types = resolve_callable_types(self.ctx, init_entity);
        types.into_iter().next().flatten()
    }

    fn resolve_array_literal_init(&self, result_ty: TyId) -> Option<(Entity, TyId, Vec<TyId>)> {
        let MirTy::Named { entity, .. } = self.ctx.module.ty_arena.get(result_ty) else {
            return None;
        };
        let entity = *entity;
        let init_func = self.ctx.module.functions.iter().find(|f| {
            let kestrel_mir_2::item::function::FunctionKind::Initializer { parent } = f.kind else {
                return false;
            };
            self.init_parent_matches(parent, entity)
                && f.params.len() == 3
                && matches!(f.params[0].convention, ParamConvention::MutBorrow)
                && matches!(self.ctx.module.ty_arena.get(f.params[1].ty), MirTy::Pointer(_))
                && self.ctx.module.ty_arena.get(f.params[2].ty) == &MirTy::I64
        })?;

        let type_args = self.prepend_receiver_type_args(result_ty, vec![]);
        // Resolve element type from the pointer param
        let ptr_ty = init_func.params.get(1)?.ty;
        let element_ty = match self.ctx.module.ty_arena.get(ptr_ty) {
            MirTy::Pointer(inner) => *inner,
            _ => return None,
        };
        Some((init_func.entity, element_ty, type_args))
    }

    fn resolve_dict_literal_init(&self, result_ty: TyId) -> Option<(Entity, TyId, Vec<TyId>)> {
        let MirTy::Named { entity, .. } = self.ctx.module.ty_arena.get(result_ty) else {
            return None;
        };
        let entity = *entity;
        let init_func = self.ctx.module.functions.iter().find(|f| {
            let kestrel_mir_2::item::function::FunctionKind::Initializer { parent } = f.kind else {
                return false;
            };
            if !self.init_parent_matches(parent, entity) || f.params.len() != 3 {
                return false;
            }
            if !matches!(f.params[0].convention, ParamConvention::MutBorrow) {
                return false;
            }
            let MirTy::Pointer(inner) = self.ctx.module.ty_arena.get(f.params[1].ty) else {
                return false;
            };
            let MirTy::Tuple(elems) = self.ctx.module.ty_arena.get(*inner) else {
                return false;
            };
            elems.len() == 2 && self.ctx.module.ty_arena.get(f.params[2].ty) == &MirTy::I64
        })?;

        let type_args = self.prepend_receiver_type_args(result_ty, vec![]);
        let ptr_ty = init_func.params.get(1)?.ty;
        let pair_ty = match self.ctx.module.ty_arena.get(ptr_ty) {
            MirTy::Pointer(inner) => *inner,
            _ => return None,
        };
        Some((init_func.entity, pair_ty, type_args))
    }

    fn init_parent_matches(&self, init_parent: Entity, target_type: Entity) -> bool {
        if init_parent == target_type {
            return true;
        }
        if self.ctx.world.get::<NodeKind>(init_parent) != Some(&NodeKind::Extension) {
            return false;
        }
        self.ctx
            .query
            .query(kestrel_name_res::ExtensionTargetEntity {
                extension: init_parent,
                root: self.ctx.root,
            })
            .is_some_and(|target| target == target_type)
    }
}
