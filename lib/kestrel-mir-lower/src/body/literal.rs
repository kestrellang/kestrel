//! Literal lowering — primitives, strings, arrays, dicts.
//!
//! OSSA form:
//! - Returns `ValueId` instead of `Operand`
//! - Uses `emit_literal(imm)` for constants
//! - Init calls use StackAlloc (not Uninit) → `emit_call_void` → `emit_take`/`emit_load`
//! - Stack buffer ops use `emit_op1` / `emit_op2`

use kestrel_ast_builder::{Callable, Name, NodeKind};
use kestrel_hecs::Entity;
use kestrel_hir::body::{HirDictEntry, HirExprId, HirLiteral};
use kestrel_mir::callee::Callee;
use kestrel_mir::inst::CallArg;
use kestrel_mir::item::function::FunctionKind;
use kestrel_mir::{
    Immediate, IntBits, MirTy, Op, Ownership, ParamConvention, Signedness, TyId, ValueId,
};
use kestrel_name_res::{ProtocolAssociatedTypes, ResolveBuiltin, extensions::ExtensionsFor};
use kestrel_reporting::{Diagnostic, Label};

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

        // No usable initializer was resolved. Rather than silently emitting an
        // error literal (which miscompiles to a zero value), surface it.
        self.emit_unlowerable_literal(
            expr_id,
            result_ty,
            "array literal",
            "`init(_arrayLiteralPointer: lang.ptr[Element], _arrayLiteralCount: lang.i64)`",
            "_ExpressibleByArrayLiteral",
        )
    }

    /// Emit a diagnostic for a collection literal whose target type has no
    /// usable `_ExpressibleBy*Literal` initializer, then return an error
    /// literal so lowering can continue (the accumulated error aborts the
    /// build). Suppressed when `result_ty` is already an error — that means an
    /// earlier phase reported the real cause and a second diagnostic would just
    /// be cascade noise.
    fn emit_unlowerable_literal(
        &mut self,
        expr_id: HirExprId,
        result_ty: TyId,
        what: &str,
        init_sig: &str,
        protocol: &str,
    ) -> ValueId {
        if !matches!(self.ctx.module.ty_arena.get(result_ty), MirTy::Error) {
            let span = super::expr_span(&self.hir, expr_id);
            let ty_str = kestrel_mir::display::ty_to_string(result_ty, &self.ctx.module);
            self.ctx.query.accumulate(
                Diagnostic::error()
                    .with_message(format!("cannot lower {what} to type `{ty_str}`"))
                    .with_labels(vec![
                        Label::primary(span.file_id, span.range()).with_message(format!(
                            "no usable `{protocol}` initializer was resolved"
                        )),
                    ])
                    .with_notes(vec![format!(
                        "`{ty_str}` must conform to `{protocol}` and provide {init_sig}"
                    )]),
            );
        }
        self.emit_literal(Immediate::error())
    }

    /// Hard-error for an expression that type-checked but hit a hole in MIR
    /// lowering (an unresolved callee, an unhandled `Def` kind, ...). Returns an
    /// error literal so lowering continues; the accumulated error aborts the
    /// build. Suppressed when the expression's own type is already an error —
    /// that means an earlier phase reported the real cause and a second
    /// diagnostic would just be cascade noise.
    pub(crate) fn emit_lowering_gap(
        &mut self,
        expr_id: HirExprId,
        message: impl Into<String>,
    ) -> ValueId {
        let result_ty = self.resolve_expr_type(expr_id);
        if !matches!(self.ctx.module.ty_arena.get(result_ty), MirTy::Error) {
            let span = super::expr_span(&self.hir, expr_id);
            self.ctx.query.accumulate(
                Diagnostic::error()
                    .with_message(message.into())
                    .with_labels(vec![
                        Label::primary(span.file_id, span.range())
                            .with_message("could not be lowered to MIR"),
                    ]),
            );
        }
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

        self.emit_unlowerable_literal(
            expr_id,
            result_ty,
            "dictionary literal",
            "`init(_dictionaryLiteralPointer: lang.ptr[(Key, Value)], _dictionaryLiteralCount: lang.i64)`",
            "_ExpressibleByDictionaryLiteral",
        )
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
    fn resolve_array_literal_init(&mut self, result_ty: TyId) -> Option<(Entity, TyId, Vec<TyId>)> {
        let MirTy::Named { entity, .. } = self.ctx.module.ty_arena.get(result_ty) else {
            return None;
        };
        let entity = *entity;
        let init_entity = self
            .ctx
            .module
            .functions
            .values()
            .find(|f| {
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
            })?
            .entity;

        let type_args = self.prepend_receiver_type_args(result_ty, vec![]);
        // The element type is the first generic arg for a generic conformer
        // (Array[T] → T), or the `Element` associated type for a non-generic
        // one (`struct MyList: _ExpressibleByArrayLiteral` with
        // `type Element = Int64`). Not the init's own param, which is the
        // unsubstituted generic param for `Array`.
        let element_ty = match self.ctx.module.ty_arena.get(result_ty) {
            MirTy::Named { type_args, .. } => type_args.first().copied(),
            _ => None,
        };
        let element_ty = match element_ty {
            Some(ty) => ty,
            None => self.resolve_literal_assoc_type(
                entity,
                kestrel_hir::Builtin::InternalExpressibleByArrayLiteral,
                "Element",
            )?,
        };
        Some((init_entity, element_ty, type_args))
    }

    /// Find the `init(&mut self, Pointer[(K,V)], Int64)` initializer for dict
    /// literal lowering. Returns (init entity, pair tuple type, type args).
    fn resolve_dict_literal_init(&mut self, result_ty: TyId) -> Option<(Entity, TyId, Vec<TyId>)> {
        let MirTy::Named { entity, .. } = self.ctx.module.ty_arena.get(result_ty) else {
            return None;
        };
        let entity = *entity;
        let init_entity = self
            .ctx
            .module
            .functions
            .values()
            .find(|f| {
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
            })?
            .entity;

        let type_args = self.prepend_receiver_type_args(result_ty, vec![]);
        // The (Key, Value) pair comes from result_ty's generic args
        // (Dict[K, V] → (K, V)) for a generic conformer, or the `Key`/`Value`
        // associated types for a non-generic one — not the init's own param,
        // which is the unsubstituted generic pair for `Dictionary`.
        let kv = match self.ctx.module.ty_arena.get(result_ty) {
            MirTy::Named { type_args, .. } if type_args.len() >= 2 => {
                Some((type_args[0], type_args[1]))
            },
            _ => None,
        };
        let (k, v) = match kv {
            Some(kv) => kv,
            None => {
                let proto = kestrel_hir::Builtin::InternalExpressibleByDictionaryLiteral;
                let k = self.resolve_literal_assoc_type(entity, proto, "Key")?;
                let v = self.resolve_literal_assoc_type(entity, proto, "Value")?;
                (k, v)
            },
        };
        let pair_ty = self.ctx.module.ty_arena.tuple(vec![k, v]);
        Some((init_entity, pair_ty, type_args))
    }

    /// Resolve a literal protocol's associated type (e.g. `Element` for
    /// `_ExpressibleByArrayLiteral`, or `Key`/`Value` for the dictionary
    /// variant) on a non-generic conforming type, by reading its `type X = ...`
    /// binding from the type body or one of its extensions. Returns `None` if
    /// the protocol, the associated member, or the binding can't be found.
    fn resolve_literal_assoc_type(
        &mut self,
        type_entity: Entity,
        protocol: kestrel_hir::Builtin,
        assoc_name: &str,
    ) -> Option<TyId> {
        let proto_entity = self.ctx.query.query(ResolveBuiltin {
            builtin: protocol,
            root: self.ctx.root,
        })?;
        let assoc_entity = self
            .ctx
            .query
            .query(ProtocolAssociatedTypes {
                protocol: proto_entity,
                root: self.ctx.root,
            })
            .into_iter()
            .find(|m| {
                self.ctx.world.get::<Name>(m.entity).map(|n| n.0.as_str()) == Some(assoc_name)
            })?
            .entity;

        // The binding lives either in the type body (`struct S { type X = ... }`)
        // or in a conforming extension (`extend S: P { type X = ... }`).
        if let Some(ty) =
            crate::items::witness_lower::find_associated_type(self.ctx, type_entity, assoc_entity)
        {
            return Some(ty);
        }
        let extensions = self.ctx.query.query(ExtensionsFor {
            target: type_entity,
            root: self.ctx.root,
        });
        for ext in extensions {
            if let Some(ty) =
                crate::items::witness_lower::find_associated_type(self.ctx, ext, assoc_entity)
            {
                return Some(ty);
            }
        }
        None
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
