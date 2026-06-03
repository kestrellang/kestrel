//! Literal lowering — primitives, strings, arrays, dicts.
//!
//! OSSA form:
//! - Returns `ValueId` instead of `Operand`
//! - Uses `emit_literal(imm)` for constants
//! - Init calls use StackAlloc (not Uninit) → `emit_call_void` → `emit_take`/`emit_load`
//! - Stack buffer ops use `emit_op1` / `emit_op2`

use kestrel_ast_builder::{Callable, NodeKind};
use kestrel_hecs::Entity;
use kestrel_hir::body::{HirDictEntry, HirExprId, HirLiteral};
use kestrel_mir::callee::Callee;
use kestrel_mir::inst::CallArg;
use kestrel_mir::item::function::FunctionKind;
use kestrel_mir::{
    Immediate, IntBits, MirTy, Op, Ownership, ParamConvention, Signedness, TyId, ValueId,
};

use super::OssaBodyCtx;
use crate::ty::resolve_callable_types;

impl OssaBodyCtx<'_, '_> {
    // ================================================================
    // Scalar / named-struct literal
    // ================================================================

    /// Lower a literal value. Named struct types go through protocol init
    /// dispatch; primitive types emit directly.
    pub fn lower_literal(&mut self, expr_id: HirExprId, lit: &HirLiteral) -> ValueId {
        let result_ty = self.resolve_expr_type(expr_id);

        // Named struct → wrap via literal protocol init
        if let MirTy::Named { entity, .. } = self.ctx.module.ty_arena.get(result_ty).clone() {
            match lit {
                HirLiteral::String { value: content, .. } => {
                    if let Some(init) = self.find_string_literal_init(entity) {
                        let ptr = self.emit_literal(Immediate::string_pointer(content.clone()));
                        let len = self.emit_literal(Immediate::i64(content.len() as i128));
                        self.ctx.register_name(init);
                        let callee = Callee::direct_with_args(init, vec![], None);
                        return self.emit_init_literal_call(callee, vec![ptr, len], result_ty);
                    }
                    return self.lower_literal_primitive(lit, result_ty);
                },
                HirLiteral::Null => {
                    if let Some(init) = self.find_null_literal_init(entity) {
                        self.ctx.register_name(init);
                        let type_args = self.prepend_receiver_type_args(result_ty, vec![]);
                        let callee = Callee::direct_with_args(init, type_args, None);
                        return self.emit_init_literal_call(callee, vec![], result_ty);
                    }
                    return self.lower_literal_primitive(lit, result_ty);
                },
                _ => {},
            }

            let (label, protocol) = match lit {
                HirLiteral::Bool(_) => (
                    "boolLiteral",
                    kestrel_hir::Builtin::ExpressibleByBoolLiteral,
                ),
                HirLiteral::Integer(_) => (
                    "intLiteral",
                    kestrel_hir::Builtin::ExpressibleByIntegerLiteral,
                ),
                HirLiteral::Float(_) => (
                    "floatLiteral",
                    kestrel_hir::Builtin::ExpressibleByFloatLiteral,
                ),
                HirLiteral::Char(_) => (
                    "charLiteral",
                    kestrel_hir::Builtin::ExpressibleByCharLiteral,
                ),
                _ => return self.lower_literal_primitive(lit, result_ty),
            };

            if let Some(init) = self.find_literal_init(entity, protocol, label) {
                let param_ty = self.resolve_init_param_type(init).unwrap_or(result_ty);
                let primitive = self.lower_literal_primitive(lit, param_ty);
                self.ctx.register_name(init);
                let callee = Callee::direct_with_args(init, vec![], None);
                return self.emit_init_literal_call(callee, vec![primitive], result_ty);
            }
        }

        self.lower_literal_primitive(lit, result_ty)
    }

    fn lower_literal_primitive(&mut self, lit: &HirLiteral, target_ty: TyId) -> ValueId {
        let imm = match lit {
            HirLiteral::Integer(v) => match self.ctx.module.ty_arena.get(target_ty) {
                MirTy::I8 => Immediate::i8(*v as i128),
                MirTy::I16 => Immediate::i16(*v as i128),
                MirTy::I32 => Immediate::i32(*v as i128),
                _ => Immediate::i64(*v as i128),
            },
            HirLiteral::Float(v) => match self.ctx.module.ty_arena.get(target_ty) {
                MirTy::F32 => Immediate::f32(*v),
                _ => Immediate::f64(*v),
            },
            HirLiteral::Bool(v) => Immediate::bool(*v),
            HirLiteral::String { value, .. } => Immediate::string(value.clone()),
            HirLiteral::Char(c) => Immediate::i32(*c as i128),
            HirLiteral::Null => Immediate::unit(),
        };
        self.emit_literal(imm)
    }

    /// Emit an init call for a literal: allocate self via Uninit, prepend
    /// &mut self, call the init, return the initialized value.
    pub(crate) fn emit_init_literal_call(
        &mut self,
        callee: Callee,
        args: Vec<ValueId>,
        result_ty: TyId,
    ) -> ValueId {
        // Allocate stack space via StackAlloc (not Uninit — Uninit triggers
        // sub-field tracking which the opaque init call can't satisfy).
        let ptr_ty = self.ctx.module.ty_arena.pointer(result_ty);
        let one = self.emit_literal(Immediate::i64(1));
        let self_ptr = self.emit_op1(Op::StackAlloc(result_ty), one, ptr_ty);
        // Build call args: &mut self (the pointer), then literal args
        let mut call_args = vec![CallArg {
            value: self_ptr,
            convention: ParamConvention::MutBorrow,
        }];
        for arg in args {
            call_args.push(self.prepare_call_arg(arg, ParamConvention::Borrow));
        }
        self.emit_call_void(callee, call_args);
        let ownership = self.ownership_for(result_ty);
        if ownership == Ownership::Owned {
            self.emit_take(self_ptr, result_ty)
        } else {
            self.emit_load(self_ptr, result_ty)
        }
    }

    // ================================================================
    // Array literal
    // ================================================================

    /// Lower `[a, b, c]` — either via the type's buffer-pointer init or as
    /// an error fallback.
    pub fn lower_array_literal(&mut self, expr_id: HirExprId, elements: &[HirExprId]) -> ValueId {
        let result_ty = self.resolve_expr_type(expr_id);

        if let Some(val) = self.try_array_literal_via_init(elements, result_ty) {
            return val;
        }

        // Fallback: no init found — emit error literal
        self.emit_literal(Immediate::error())
    }

    /// Stack-allocate a buffer, write elements, then call the `init(ptr, count)`
    /// initializer discovered from the Array type's protocol conformance.
    fn try_array_literal_via_init(
        &mut self,
        elements: &[HirExprId],
        result_ty: TyId,
    ) -> Option<ValueId> {
        let (init_entity, element_ty, type_args) = self.resolve_array_literal_init(result_ty)?;

        let ptr_ty = self.ctx.module.ty_arena.pointer(element_ty);
        let i64_ty = self.ctx.module.ty_arena.i64();

        // StackAlloc: allocate buffer for `elements.len()` items
        let count = self.emit_literal(Immediate::i64(elements.len() as i128));
        let ptr = self.emit_op1(Op::StackAlloc(element_ty), count, ptr_ty);

        // Element stride (bytes per element)
        let unit_val = self.emit_literal(Immediate::unit());
        let stride = self.emit_op1(Op::SizeOf(element_ty), unit_val, i64_ty);

        // Write each element into the buffer
        let unit_ty = self.ctx.module.ty_arena.unit();
        for (i, &elem_expr) in elements.iter().enumerate() {
            let elem_val = self.lower_expr(elem_expr);
            let elem_ptr = if i == 0 {
                ptr
            } else {
                // offset = i * stride
                let idx = self.emit_literal(Immediate::i64(i as i128));
                let byte_offset = self.emit_op2(
                    Op::Mul(IntBits::I64, Signedness::Signed),
                    idx,
                    stride,
                    i64_ty,
                );
                self.emit_op2(Op::PtrOffset, ptr, byte_offset, ptr_ty)
            };
            // PtrWrite: store element at the computed pointer (consumes elem_val)
            self.emit_op2(Op::PtrWrite(element_ty), elem_ptr, elem_val, unit_ty);
            self.consume(elem_val);
        }

        // Call init(&mut self, ptr, count) — StackAlloc (not Uninit) since
        // the opaque init call can't satisfy Uninit's sub-field tracking.
        self.ctx.register_name(init_entity);
        let callee = Callee::direct_with_args(init_entity, type_args, None);
        let ptr_ty = self.ctx.module.ty_arena.pointer(result_ty);
        let one = self.emit_literal(Immediate::i64(1));
        let self_val = self.emit_op1(Op::StackAlloc(result_ty), one, ptr_ty);
        let call_args = vec![
            CallArg {
                value: self_val,
                convention: ParamConvention::MutBorrow,
            },
            CallArg {
                value: ptr,
                convention: ParamConvention::Consuming,
            },
            CallArg {
                value: count,
                convention: ParamConvention::Consuming,
            },
        ];
        self.emit_call_void(callee, call_args);
        let ownership = self.ownership_for(result_ty);
        if ownership == Ownership::Owned {
            Some(self.emit_take(self_val, result_ty))
        } else {
            Some(self.emit_load(self_val, result_ty))
        }
    }

    // ================================================================
    // Dict literal
    // ================================================================

    /// Lower `[k1: v1, k2: v2]` — buffer-pointer init with `(K, V)` tuples.
    pub fn lower_dict_literal(&mut self, expr_id: HirExprId, entries: &[HirDictEntry]) -> ValueId {
        let result_ty = self.resolve_expr_type(expr_id);

        if let Some(val) = self.try_dict_literal_via_init(entries, result_ty) {
            return val;
        }

        self.emit_literal(Immediate::error())
    }

    fn try_dict_literal_via_init(
        &mut self,
        entries: &[HirDictEntry],
        result_ty: TyId,
    ) -> Option<ValueId> {
        let (init_entity, pair_ty, type_args) = self.resolve_dict_literal_init(result_ty)?;

        let MirTy::Tuple(elem_tys) = self.ctx.module.ty_arena.get(pair_ty).clone() else {
            return None;
        };
        if elem_tys.len() != 2 {
            return None;
        }

        let ptr_ty = self.ctx.module.ty_arena.pointer(pair_ty);
        let i64_ty = self.ctx.module.ty_arena.i64();

        // StackAlloc: buffer for `entries.len()` pair tuples
        let count = self.emit_literal(Immediate::i64(entries.len() as i128));
        let ptr = self.emit_op1(Op::StackAlloc(pair_ty), count, ptr_ty);

        // Pair stride
        let unit_val = self.emit_literal(Immediate::unit());
        let stride = self.emit_op1(Op::SizeOf(pair_ty), unit_val, i64_ty);

        let unit_ty = self.ctx.module.ty_arena.unit();
        for (i, entry) in entries.iter().enumerate() {
            let key = self.lower_expr(entry.key);
            let val = self.lower_expr(entry.value);

            // Build (key, val) tuple
            let pair = self.emit_tuple(pair_ty, vec![key, val]);

            let elem_ptr = if i == 0 {
                ptr
            } else {
                let idx = self.emit_literal(Immediate::i64(i as i128));
                let byte_offset = self.emit_op2(
                    Op::Mul(IntBits::I64, Signedness::Signed),
                    idx,
                    stride,
                    i64_ty,
                );
                self.emit_op2(Op::PtrOffset, ptr, byte_offset, ptr_ty)
            };

            // PtrWrite: store pair tuple at the computed pointer (consumes pair)
            self.emit_op2(Op::PtrWrite(pair_ty), elem_ptr, pair, unit_ty);
            self.consume(pair);
        }

        // Call init(&mut self, ptr, count) — StackAlloc (not Uninit) since
        // the opaque init call can't satisfy Uninit's sub-field tracking.
        self.ctx.register_name(init_entity);
        let callee = Callee::direct_with_args(init_entity, type_args, None);
        let ptr_ty = self.ctx.module.ty_arena.pointer(result_ty);
        let one = self.emit_literal(Immediate::i64(1));
        let self_val = self.emit_op1(Op::StackAlloc(result_ty), one, ptr_ty);
        let call_args = vec![
            CallArg {
                value: self_val,
                convention: ParamConvention::MutBorrow,
            },
            CallArg {
                value: ptr,
                convention: ParamConvention::Consuming,
            },
            CallArg {
                value: count,
                convention: ParamConvention::Consuming,
            },
        ];
        self.emit_call_void(callee, call_args);
        let ownership = self.ownership_for(result_ty);
        if ownership == Ownership::Owned {
            Some(self.emit_take(self_val, result_ty))
        } else {
            Some(self.emit_load(self_val, result_ty))
        }
    }

    // ================================================================
    // Init discovery helpers
    // ================================================================

    fn find_literal_protocol_init(
        &self,
        struct_entity: Entity,
        protocol: kestrel_hir::Builtin,
        predicate: impl Fn(&Callable) -> bool,
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

    /// Find the `init(&mut self, Pointer[T], Int64)` initializer for array
    /// literal lowering. Returns (init entity, element type, type args).
    fn resolve_array_literal_init(&self, result_ty: TyId) -> Option<(Entity, TyId, Vec<TyId>)> {
        let MirTy::Named { entity, .. } = self.ctx.module.ty_arena.get(result_ty) else {
            return None;
        };
        let entity = *entity;
        let init_func = self.ctx.module.functions.values().find(|f| {
            let FunctionKind::Initializer { parent } = f.kind else {
                return false;
            };
            self.init_parent_matches(parent, entity)
                && f.params.len() == 3
                && matches!(f.params[0].convention, ParamConvention::MutBorrow)
                && matches!(
                    self.ctx.module.ty_arena.get(f.params[1].ty),
                    MirTy::Pointer(_)
                )
                && self.ctx.module.ty_arena.get(f.params[2].ty) == &MirTy::I64
        })?;

        let type_args = self.prepend_receiver_type_args(result_ty, vec![]);
        // Extract element type from result_ty's type args (Array[T] → T),
        // not the init function's generic param which is unsubstituted.
        let element_ty = match self.ctx.module.ty_arena.get(result_ty) {
            MirTy::Named { type_args, .. } => type_args.first().copied(),
            _ => None,
        };
        let element_ty = element_ty?;
        Some((init_func.entity, element_ty, type_args))
    }

    /// Find the `init(&mut self, Pointer[(K,V)], Int64)` initializer for dict
    /// literal lowering. Returns (init entity, pair tuple type, type args).
    fn resolve_dict_literal_init(&mut self, result_ty: TyId) -> Option<(Entity, TyId, Vec<TyId>)> {
        let MirTy::Named { entity, .. } = self.ctx.module.ty_arena.get(result_ty) else {
            return None;
        };
        let entity = *entity;
        let init_func = self.ctx.module.functions.values().find(|f| {
            let FunctionKind::Initializer { parent } = f.kind else {
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
        // Build the concrete pair type from result_ty's type args (Dict[K, V] → (K, V)),
        // not the init function's generic param which is unsubstituted.
        let pair_ty = match self.ctx.module.ty_arena.get(result_ty) {
            MirTy::Named { type_args, .. } if type_args.len() >= 2 => {
                let k = type_args[0];
                let v = type_args[1];
                self.ctx.module.ty_arena.tuple(vec![k, v])
            },
            _ => return None,
        };
        Some((init_func.entity, pair_ty, type_args))
    }

    /// Check whether an init's parent matches the target type entity, handling
    /// extension-defined inits (where parent is the extension, not the type).
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
