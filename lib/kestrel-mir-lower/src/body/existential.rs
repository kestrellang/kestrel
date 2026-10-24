//! Lowering existential values: requirement calls through a function table
//! and the Box / Unsize / Upcast conversions (existentials plan Phase 10).
//! Table shapes and slot indices come from `crate::existential`.

use kestrel_hecs::Entity;
use kestrel_mir::callee::Callee;
use kestrel_mir::inst::CallArg;
use kestrel_mir::{MirTy, Op, ParamConvention, TyId, ValueId};

use super::OssaBodyCtx;
use crate::existential::{bounds_of, dispatch_path};

impl OssaBodyCtx<'_, '_> {
    /// A requirement call whose receiver is a raw existential (`args[0]`, a
    /// (data, table) value): load the thunk from the table and call it with
    /// the data word in place of the receiver. `None` when the requirement
    /// has no slot (not dispatchable — rejected upstream as E145).
    pub(crate) fn emit_dynamic_call(
        &mut self,
        requirement: Entity,
        receiver_ty: TyId,
        args: Vec<CallArg>,
        result_ty: TyId,
    ) -> Option<ValueId> {
        let bounds = bounds_of(self.ctx, receiver_ty);
        let (hops, slot) = dispatch_path(self.ctx, &bounds, requirement)?;
        kestrel_debug::ktrace!("vtable", "dynamic call slot {slot} via {hops:?}");
        let raw = self.raw_ptr_ty();
        let recv = args.first()?.value;
        let data = self.emit_op1(Op::FatWord(0), recv, raw);
        let table = self.table_at(recv, &hops);
        let mut params: Vec<(TyId, ParamConvention)> = vec![(raw, ParamConvention::Consuming)];
        params.extend(
            args[1..]
                .iter()
                .map(|a| (self.body.values[a.value.index()].ty, a.convention)),
        );
        let fn_ty = self.ctx.intern(MirTy::FuncThin {
            params,
            ret: result_ty,
        });
        let thunk = self.emit_op1(Op::FatWord(slot), table, fn_ty);
        self.emit_destroy_value(table);
        let mut call_args = vec![CallArg {
            value: data,
            convention: ParamConvention::Consuming,
        }];
        call_args.extend(args.into_iter().skip(1));
        let result = self.emit_call_returning(Callee::Thin(thunk), call_args, result_ty);
        self.emit_destroy_value(thunk);
        // The receiver view (a `pointeeRef`/`pointeeMutRef` result) was used
        // only for its words: end it now, as a direct call ends a borrowed
        // ref argument, so it can't reach a control-flow merge (E497).
        if self.ref_results.contains(&recv) && !self.ref_binding_vals.contains_key(&recv) {
            self.emit_end_borrow(recv);
        }
        Some(result)
    }

    /// The table of fat value `value`, then each word of `hops` in turn
    /// (sub-table pointers).
    fn table_at(&mut self, value: ValueId, hops: &[u32]) -> ValueId {
        let raw = self.raw_ptr_ty();
        let mut table = self.emit_op1(Op::FatWord(1), value, raw);
        for &hop in hops {
            let next = self.emit_op1(Op::FatWord(hop), table, raw);
            self.emit_destroy_value(table);
            table = next;
        }
        table
    }

    fn raw_ptr_ty(&mut self) -> TyId {
        let i8 = self.ctx.module.ty_arena.i8();
        self.ctx.module.ty_arena.pointer(i8)
    }
}

// ===== Conversions (existentials design §4) =====

impl OssaBodyCtx<'_, '_> {
    /// Apply a recorded existential conversion to `value` (of the
    /// conversion's `from` type), yielding the existential `to`.
    pub(crate) fn apply_existential_conversion(
        &mut self,
        conv: &kestrel_type_infer::result::ExistentialConversion,
        value: ValueId,
    ) -> ValueId {
        use kestrel_type_infer::result::ConversionKind;
        let from_ty = crate::ty::lower_resolved_ty(self.ctx, &conv.from);
        let to_ty = crate::ty::lower_resolved_ty(self.ctx, &conv.to);
        let MirTy::Named {
            entity: boxed,
            type_args,
        } = self.ctx.module.ty_arena.get(to_ty).clone()
        else {
            return value;
        };
        let Some(&raw) = type_args.first() else {
            return value;
        };
        let bounds = bounds_of(self.ctx, raw);
        match conv.kind {
            ConversionKind::Box => {
                let boxed_ty = self.ctx.module.ty_arena.named(boxed, vec![from_ty]);
                let Some(made) = self.emit_box_init(boxed_ty, value) else {
                    return value;
                };
                self.emit_unsize(made, boxed_ty, to_ty, boxed, &bounds, from_ty)
            },
            ConversionKind::Unsize => {
                let payload = match self.ctx.module.ty_arena.get(from_ty) {
                    MirTy::Named { type_args, .. } => type_args.first().copied(),
                    _ => None,
                };
                let Some(payload) = payload else {
                    return value;
                };
                self.emit_unsize(value, from_ty, to_ty, boxed, &bounds, payload)
            },
            ConversionKind::Upcast => {
                let from_raw = match self.ctx.module.ty_arena.get(from_ty) {
                    MirTy::Named { type_args, .. } => type_args.first().copied(),
                    _ => None,
                };
                let from_bounds = from_raw.map(|r| bounds_of(self.ctx, r)).unwrap_or_default();
                // Only a single target protocol has a table to move to; a
                // narrower composition would need a table built at runtime.
                let [target] = bounds.as_slice() else {
                    kestrel_debug::ktrace!("vtable", "upcast to a composition is not supported");
                    return value;
                };
                let Some(hops) = crate::existential::upcast_path(self.ctx, &from_bounds, *target)
                else {
                    return value;
                };
                self.rebox(value, from_ty, to_ty, &mut |this, field, _from, to| {
                    let raw_ptr = this.raw_ptr_ty();
                    let data = this.emit_op1(Op::FatWord(0), field, raw_ptr);
                    let table = this.table_at(field, &hops);
                    this.emit_destroy_value(field);
                    this.emit_fat(data, table, to)
                })
            },
        }
    }

    /// `B[T](value)` through the `ExistentialBox` requirement
    /// `init(consuming value: Target)`.
    fn emit_box_init(&mut self, boxed_ty: TyId, value: ValueId) -> Option<ValueId> {
        let protocol = self.ctx.query.query(kestrel_name_res::ResolveBuiltin {
            builtin: kestrel_hir::Builtin::ExistentialBox,
            root: self.ctx.root,
        })?;
        let init = self
            .ctx
            .world
            .children_of(protocol)
            .iter()
            .copied()
            .find(|&c| {
                self.ctx.world.get::<kestrel_ast_builder::NodeKind>(c)
                    == Some(&kestrel_ast_builder::NodeKind::Initializer)
            })?;
        self.ctx.register_name(protocol);
        let key = self.ctx.witness_method_key(init);
        // An init call writes through a `mutating self` slot, like
        // `emit_init_call`.
        let ptr_ty = self.ctx.module.ty_arena.pointer(boxed_ty);
        let one = self.emit_literal(kestrel_mir::Immediate::i64(1));
        let slot = self.emit_op1(Op::StackAlloc(boxed_ty), one, ptr_ty);
        let arg = self.prepare_call_arg(value, ParamConvention::Consuming);
        self.emit_call_void(
            Callee::Witness {
                protocol,
                method: key,
                self_type: boxed_ty,
                method_type_args: Vec::new(),
            },
            vec![
                CallArg {
                    value: slot,
                    convention: ParamConvention::MutBorrow,
                },
                arg,
            ],
        );
        Some(self.emit_take(slot, boxed_ty))
    }

    /// `B[T]` → `B[rawany P]`: every pointer field that changes type gains
    /// the `(B, T, P)` table word.
    fn emit_unsize(
        &mut self,
        value: ValueId,
        from_ty: TyId,
        to_ty: TyId,
        boxed: Entity,
        bounds: &[Entity],
        payload: TyId,
    ) -> ValueId {
        let template = crate::existential::template_for(self.ctx, boxed, bounds);
        let raw_ptr = self.raw_ptr_ty();
        let table = self.emit_literal(kestrel_mir::Immediate::new(
            kestrel_mir::ImmediateKind::VTable { template, payload },
        ));
        let result = self.rebox(value, from_ty, to_ty, &mut |this, field, _from, to| {
            let data = this.emit_op1(Op::PtrCast(raw_ptr), field, raw_ptr);
            this.emit_destroy_value(field);
            let table = this.emit_copy_value(table);
            this.emit_fat(data, table, to)
        });
        self.emit_destroy_value(table);
        result
    }

    /// Rebuild `value: from_ty` as `to_ty`, the same struct at a different
    /// payload: fields whose type doesn't change move over; a pointer field
    /// that changes goes through `on_pointer`; a struct field recurses.
    fn rebox(
        &mut self,
        value: ValueId,
        from_ty: TyId,
        to_ty: TyId,
        on_pointer: &mut dyn FnMut(&mut Self, ValueId, TyId, TyId) -> ValueId,
    ) -> ValueId {
        if from_ty == to_ty {
            return value;
        }
        let arena = &self.ctx.module.ty_arena;
        match (arena.get(from_ty).clone(), arena.get(to_ty).clone()) {
            (MirTy::Pointer(_), MirTy::Pointer(_)) => on_pointer(self, value, from_ty, to_ty),
            (
                MirTy::Named {
                    entity,
                    type_args: from_args,
                },
                MirTy::Named {
                    type_args: to_args, ..
                },
            ) => {
                let Some(def) = self.ctx.module.structs.get(&entity).cloned() else {
                    return value;
                };
                let field_tys = |this: &mut Self, args: &[TyId]| -> Vec<TyId> {
                    let mut subst = kestrel_mir::substitute::SubstMap::new();
                    for (tp, &a) in def.type_params.iter().zip(args) {
                        subst.type_params.insert(tp.entity, a);
                    }
                    def.fields
                        .iter()
                        .map(|f| {
                            kestrel_mir::substitute::substitute(
                                &mut this.ctx.module.ty_arena,
                                f.ty,
                                &subst,
                            )
                        })
                        .collect()
                };
                let from_fields = field_tys(self, &from_args);
                let to_fields = field_tys(self, &to_args);
                let parts = self.emit_destructure_struct(value, &from_fields);
                let fields: Vec<(kestrel_mir::FieldIdx, ValueId)> = parts
                    .into_iter()
                    .enumerate()
                    .map(|(i, part)| {
                        let v = self.rebox(part, from_fields[i], to_fields[i], on_pointer);
                        (kestrel_mir::FieldIdx::new(i), v)
                    })
                    .collect();
                self.emit_struct(to_ty, fields)
            },
            _ => value,
        }
    }

    /// The fat value `(data, table)` of type `ty`.
    fn emit_fat(&mut self, data: ValueId, table: ValueId, ty: TyId) -> ValueId {
        let raw_ptr = self.raw_ptr_ty();
        let pair_ty = self.ctx.module.ty_arena.tuple(vec![raw_ptr, raw_ptr]);
        let pair = self.emit_tuple(pair_ty, vec![data, table]);
        self.emit_op1(Op::FatPtr(ty), pair, ty)
    }
}

// ===== Consuming requirements (existentials design §3) =====

impl OssaBodyCtx<'_, '_> {
    /// The receiver of a `consuming` requirement called on an existential:
    /// the box is taken by value and viewed through `pointeeRef`; the
    /// consuming thunk then moves the payload out of the view. Returns the
    /// view argument and `(box, borrow)` for
    /// [`Self::emit_release_consumed_box`]. `None` for any other call.
    pub(crate) fn existential_consuming_receiver(
        &mut self,
        receiver_expr: kestrel_hir::body::HirExprId,
        peels: &[kestrel_type_infer::result::IndirectionPeel],
        conv: ParamConvention,
    ) -> Option<(CallArg, (ValueId, ValueId))> {
        let [peel] = peels else {
            return None;
        };
        if conv != ParamConvention::Consuming {
            return None;
        }
        let target = crate::ty::lower_resolved_ty(self.ctx, &peel.target);
        if !matches!(
            self.ctx.module.ty_arena.get(target),
            MirTy::RawExistential { .. }
        ) {
            return None;
        }
        let box_val = self.lower_expr_for_consuming(receiver_expr);
        let box_ty = self.body.value(box_val).ty;
        let borrow = self.emit_begin_borrow(box_val);
        self.ctx.register_name(peel.read_method);
        let type_args = self.prepend_receiver_type_args(box_ty, vec![]);
        let view = self.emit_call_returning(
            Callee::direct_with_args(peel.read_method, type_args, None),
            vec![CallArg {
                value: borrow,
                convention: ParamConvention::Borrow,
            }],
            target,
        );
        Some((
            CallArg {
                value: view,
                convention: ParamConvention::Consuming,
            },
            (box_val, borrow),
        ))
    }

    /// After a consuming requirement moved the payload out of `box_val`:
    /// end the box's box with `consumePointee` and a closure that does
    /// nothing (the payload is already gone), which frees the storage without
    /// running the payload's deinit.
    pub(crate) fn emit_release_consumed_box(&mut self, box_val: ValueId, borrow: ValueId) {
        self.emit_end_borrow(borrow);
        let box_ty = self.body.value(box_val).ty;
        let Some(raw) = (match self.ctx.module.ty_arena.get(box_ty) {
            MirTy::Named { type_args, .. } => type_args.first().copied(),
            _ => None,
        }) else {
            return;
        };
        let Some(protocol) = self.ctx.query.query(kestrel_name_res::ResolveBuiltin {
            builtin: kestrel_hir::Builtin::ConsumingIndirection,
            root: self.ctx.root,
        }) else {
            return;
        };
        let Some(requirement) = self
            .ctx
            .world
            .children_of(protocol)
            .iter()
            .copied()
            .find(|&c| {
                self.ctx
                    .world
                    .get::<kestrel_ast_builder::Name>(c)
                    .map(|n| n.0.as_str())
                    == Some("consumePointee")
            })
        else {
            return;
        };
        self.ctx.register_name(protocol);
        let key = self.ctx.witness_method_key(requirement);
        let raw_ptr = self.ctx.module.ty_arena.pointer(raw);
        let unit = self.ctx.module.ty_arena.unit();
        let closure_ty = self.ctx.intern(MirTy::FuncThick {
            kind: kestrel_mir::FnKind::Normal,
            params: vec![(raw_ptr, ParamConvention::Consuming)],
            ret: unit,
        });
        let discard = crate::existential::discard_fn(self.ctx);
        let body = self.emit_apply_partial(
            Callee::direct_with_args(discard, vec![raw], None),
            vec![],
            closure_ty,
        );
        let borrow_body = self.emit_begin_borrow(body);
        self.emit_call_returning(
            Callee::Witness {
                protocol,
                method: key,
                self_type: box_ty,
                method_type_args: vec![unit],
            },
            vec![
                CallArg {
                    value: box_val,
                    convention: ParamConvention::Consuming,
                },
                CallArg {
                    value: borrow_body,
                    convention: ParamConvention::Borrow,
                },
            ],
            unit,
        );
        self.emit_end_borrow(borrow_body);
        self.emit_destroy_value(body);
    }
}

// ===== Type tests and downcasts (existentials design §8, §8b) =====

impl OssaBodyCtx<'_, '_> {
    /// The box's payload as a raw existential view (`Indirection.pointeeRef`),
    /// or `None` when `box_ty` is not a box over a raw existential.
    fn emit_existential_view(&mut self, boxed: ValueId, box_ty: TyId) -> Option<(ValueId, TyId)> {
        let MirTy::Named { type_args, .. } = self.ctx.module.ty_arena.get(box_ty) else {
            return None;
        };
        let raw = *type_args.first()?;
        if !matches!(
            self.ctx.module.ty_arena.get(raw),
            MirTy::RawExistential { .. }
        ) {
            return None;
        }
        let protocol = self.ctx.query.query(kestrel_name_res::ResolveBuiltin {
            builtin: kestrel_hir::Builtin::Indirection,
            root: self.ctx.root,
        })?;
        let requirement = self.protocol_requirement(protocol, "pointeeRef")?;
        self.ctx.register_name(protocol);
        let method = self.ctx.witness_method_key(requirement);
        let view = self.emit_call_returning(
            Callee::Witness {
                protocol,
                method,
                self_type: box_ty,
                method_type_args: Vec::new(),
            },
            vec![CallArg {
                value: boxed,
                convention: ParamConvention::Borrow,
            }],
            raw,
        );
        Some((view, raw))
    }

    /// End a view from [`Self::emit_existential_view`] once only its words
    /// were read.
    fn end_existential_view(&mut self, view: ValueId) {
        if self.ref_results.contains(&view) && !self.ref_binding_vals.contains_key(&view) {
            self.emit_end_borrow(view);
        }
    }

    /// `protocol`'s requirement named `name`.
    fn protocol_requirement(&self, protocol: Entity, name: &str) -> Option<Entity> {
        self.ctx
            .world
            .children_of(protocol)
            .iter()
            .copied()
            .find(|&c| {
                self.ctx
                    .world
                    .get::<kestrel_ast_builder::Name>(c)
                    .map(|n| n.0.as_str())
                    == Some(name)
            })
    }

    /// Bool: is the payload of `boxed` a `target`? Payloads are told apart by
    /// their table: tables are interned per (shape, payload), so the box's
    /// table is `target`'s exactly when the payload is a `target`. A target
    /// that can't conform to the bounds (E149) is a constant miss.
    pub(crate) fn emit_payload_is(
        &mut self,
        boxed: ValueId,
        box_ty: TyId,
        target: &kestrel_type_infer::result::ResolvedTy,
    ) -> ValueId {
        let bool_ty = self.ctx.module.ty_arena.bool();
        let boxed_entity = match self.ctx.module.ty_arena.get(box_ty) {
            MirTy::Named { entity, .. } => Some(*entity),
            _ => None,
        };
        let Some(((view, raw), boxed_entity)) =
            self.emit_existential_view(boxed, box_ty).zip(boxed_entity)
        else {
            return self.emit_literal(kestrel_mir::immediate::Immediate::bool(false));
        };
        let bounds = bounds_of(self.ctx, raw);
        if kestrel_type_infer::type_test_blocker(&self.ctx.query, target, &bounds, self.ctx.root)
            .is_some()
        {
            self.end_existential_view(view);
            return self.emit_literal(kestrel_mir::immediate::Immediate::bool(false));
        }
        let raw_ptr = self.raw_ptr_ty();
        let table = self.emit_op1(Op::FatWord(1), view, raw_ptr);
        self.end_existential_view(view);
        let target = crate::ty::lower_resolved_ty(self.ctx, target);
        let template = crate::existential::template_for(self.ctx, boxed_entity, &bounds);
        let expected = self.emit_literal(kestrel_mir::immediate::Immediate::new(
            kestrel_mir::immediate::ImmediateKind::VTable {
                template,
                payload: target,
            },
        ));
        let i64_ty = self.ctx.module.ty_arena.i64();
        let actual_addr = self.emit_op1(Op::PtrToAddress, table, i64_ty);
        let expected_addr = self.emit_op1(Op::PtrToAddress, expected, i64_ty);
        self.emit_destroy_value(table);
        self.emit_destroy_value(expected);
        self.emit_op2(
            Op::Eq(kestrel_mir::IntBits::I64),
            actual_addr,
            expected_addr,
            bool_ty,
        )
    }

    /// The payload of `boxed`, known to be a `target`, borrowed in place.
    pub(crate) fn emit_payload_view(
        &mut self,
        boxed: ValueId,
        box_ty: TyId,
        target: TyId,
    ) -> ValueId {
        let Some((view, _)) = self.emit_existential_view(boxed, box_ty) else {
            return self.emit_literal(kestrel_mir::immediate::Immediate::unit());
        };
        let raw_ptr = self.raw_ptr_ty();
        let data = self.emit_op1(Op::FatWord(0), view, raw_ptr);
        self.end_existential_view(view);
        let target_ptr = self.ctx.module.ty_arena.pointer(target);
        let ptr = self.emit_op1(Op::PtrCast(target), data, target_ptr);
        self.emit_begin_borrow_addr(ptr, target)
    }

    /// `TakePayload(x)`: consume the box `x`, whose payload a preceding `is`
    /// test proved is a `T`, and return the payload. `consumePointee` frees the
    /// box; its body moves the payload out (`__existential.take[T]`).
    pub(crate) fn lower_take_payload(
        &mut self,
        expr_id: kestrel_hir::body::HirExprId,
        operand: kestrel_hir::body::HirExprId,
    ) -> ValueId {
        let target = self.resolve_expr_type(expr_id);
        let boxed = self.lower_expr_for_consuming(operand);
        let box_ty = self.body.value(boxed).ty;
        let raw = match self.ctx.module.ty_arena.get(box_ty) {
            MirTy::Named { type_args, .. } => type_args.first().copied(),
            _ => None,
        };
        let protocol = self.ctx.query.query(kestrel_name_res::ResolveBuiltin {
            builtin: kestrel_hir::Builtin::ConsumingIndirection,
            root: self.ctx.root,
        });
        let requirement = protocol.and_then(|p| self.protocol_requirement(p, "consumePointee"));
        let (Some(raw), Some(protocol), Some(requirement)) = (raw, protocol, requirement) else {
            return self.emit_literal(kestrel_mir::immediate::Immediate::unit());
        };
        self.ctx.register_name(protocol);
        let method = self.ctx.witness_method_key(requirement);
        let raw_ptr = self.ctx.module.ty_arena.pointer(raw);
        let closure_ty = self.ctx.intern(MirTy::FuncThick {
            kind: kestrel_mir::FnKind::Normal,
            params: vec![(raw_ptr, ParamConvention::Consuming)],
            ret: target,
        });
        let take = crate::existential::take_fn(self.ctx);
        let body = self.emit_apply_partial(
            Callee::direct_with_args(take, vec![raw, target], None),
            vec![],
            closure_ty,
        );
        let borrow_body = self.emit_begin_borrow(body);
        let payload = self.emit_call_returning(
            Callee::Witness {
                protocol,
                method,
                self_type: box_ty,
                method_type_args: vec![target],
            },
            vec![
                CallArg {
                    value: boxed,
                    convention: ParamConvention::Consuming,
                },
                CallArg {
                    value: borrow_body,
                    convention: ParamConvention::Borrow,
                },
            ],
            target,
        );
        self.emit_end_borrow(borrow_body);
        self.emit_destroy_value(body);
        payload
    }
}
