//! Literal lowering — primitives, strings, arrays, dicts.
//!
//! OSSA form:
//! - Returns `ValueId` instead of `Operand`
//! - Uses `emit_literal(imm)` for constants
//! - Init calls use StackAlloc (not Uninit) → `emit_call_void` → `emit_take`/`emit_load`
//! - Stack buffer ops use `emit_op1` / `emit_op2`

use kestrel_hecs::Entity;
use kestrel_hir::Builtin;
use kestrel_hir::body::{HirDictEntry, HirExprId, HirLiteral};
use kestrel_mir::callee::Callee;
use kestrel_mir::inst::CallArg;
use kestrel_mir::item::witness::WitnessMethodKey;
use kestrel_mir::mono::collect::substitute_and_resolve;
use kestrel_mir::substitute::SubstMap;
use kestrel_mir::{
    Immediate, IntBits, MirTy, Op, Ownership, ParamConvention, Signedness, TyId, ValueId,
};
use kestrel_name_res::{ConformingProtocols, ResolveBuiltin};
use kestrel_reporting::{Diagnostic, Label};

use super::OssaBodyCtx;
use crate::ty::resolve_callable_types;

/// A literal protocol's requirement init, ready to call for one literal type.
///
/// Found through the requirement's `@builtin` tag and called as a
/// `Callee::Witness`, so the conformer's init is whatever witness lowering
/// bound for that conformance — the table every protocol call dispatches
/// through. Literal lowering never picks an init by name, labels or shape.
pub(crate) struct LiteralInit {
    protocol: Entity,
    key: WitnessMethodKey,
    /// The requirement's param types with the protocol's `Self` replaced by the
    /// literal's type and associated types resolved through the witness table
    /// (`lang.ptr[Element]` → `lang.ptr[J]`); still projections for a generic type.
    params: Vec<TyId>,
}

impl OssaBodyCtx<'_, '_> {
    // ================================================================
    // Scalar / named-struct literal
    // ================================================================

    /// Lower a literal value. A type that conforms to the literal's protocol
    /// (or a type parameter bounded by it) builds through the requirement init;
    /// primitive types take the literal as an immediate.
    pub fn lower_literal(&mut self, expr_id: HirExprId, lit: &HirLiteral) -> ValueId {
        let result_ty = self.resolve_expr_type(expr_id);
        let builtin = match lit {
            HirLiteral::Bool(_) => Builtin::BoolLiteralInit,
            HirLiteral::Integer(_) => Builtin::IntegerLiteralInit,
            HirLiteral::Float(_) => Builtin::FloatLiteralInit,
            HirLiteral::Char { .. } => Builtin::CharLiteralInit,
            HirLiteral::String { .. } => Builtin::StringLiteralInit,
            HirLiteral::Null => Builtin::NullLiteralInit,
        };
        let Some(init) = self.literal_init(builtin, result_ty) else {
            return self.lower_literal_primitive(lit, result_ty);
        };
        let args = match lit {
            HirLiteral::String { value, .. } => self.string_literal_args(value),
            HirLiteral::Null => vec![],
            _ => {
                // Scalar requirements take one raw primitive (`lang.i64`, …).
                let Some(&param_ty) = init.params.first() else {
                    return self.lower_literal_primitive(lit, result_ty);
                };
                vec![self.lower_literal_primitive(lit, param_ty)]
            },
        };
        self.emit_literal_init_call(&init, args, result_ty)
    }

    /// Build a value of `ty` from the text of a string literal through
    /// `ExpressibleByStringLiteral` — `None` when `ty` takes a raw string.
    pub(crate) fn emit_string_literal_value(&mut self, text: &str, ty: TyId) -> Option<ValueId> {
        let init = self.literal_init(Builtin::StringLiteralInit, ty)?;
        let args = self.string_literal_args(text);
        Some(self.emit_literal_init_call(&init, args, ty))
    }

    /// Wrap a raw `lang.i64` into `ty` through `ExpressibleByIntegerLiteral` —
    /// `None` when `ty` is itself primitive.
    pub(crate) fn emit_integer_literal_value(&mut self, raw: ValueId, ty: TyId) -> Option<ValueId> {
        let init = self.literal_init(Builtin::IntegerLiteralInit, ty)?;
        Some(self.emit_literal_init_call(&init, vec![raw], ty))
    }

    /// The `(pointer, byte length)` pair a string-literal init takes.
    fn string_literal_args(&mut self, text: &str) -> Vec<ValueId> {
        let ptr = self.emit_literal(Immediate::string_pointer(text.to_string()));
        let len = self.emit_literal(Immediate::i64(text.len() as i128));
        vec![ptr, len]
    }

    fn lower_literal_primitive(&mut self, lit: &HirLiteral, target_ty: TyId) -> ValueId {
        let imm = match lit {
            HirLiteral::Integer(v) => match self.ctx.module.ty_arena.get(target_ty) {
                MirTy::I8 => Immediate::i8(*v),
                MirTy::I16 => Immediate::i16(*v),
                MirTy::I32 => Immediate::i32(*v),
                _ => Immediate::i64(*v),
            },
            HirLiteral::Float(v) => match self.ctx.module.ty_arena.get(target_ty) {
                MirTy::F32 => Immediate::f32(*v),
                _ => Immediate::f64(*v),
            },
            HirLiteral::Bool(v) => Immediate::bool(*v),
            HirLiteral::String { value, .. } => Immediate::string(value.clone()),
            HirLiteral::Char { value: c, .. } => Immediate::i32(*c as i128),
            HirLiteral::Null => Immediate::unit(),
        };
        self.emit_literal(imm)
    }

    /// Call a literal requirement init: stack-allocate `self`, pass it as
    /// `&mut self` ahead of `args` (borrowed), and take the initialized value.
    fn emit_literal_init_call(
        &mut self,
        init: &LiteralInit,
        args: Vec<ValueId>,
        result_ty: TyId,
    ) -> ValueId {
        let callee = self.literal_init_callee(init, result_ty);
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
        let init = self.literal_init(Builtin::ArrayLiteralInit, result_ty)?;
        let ptr_param = *init.params.first()?;
        let MirTy::Pointer(element_ty) = *self.ctx.module.ty_arena.get(ptr_param) else {
            return None;
        };

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
            // Stage 1.5: a ref element decays to an owned copy (the buffer
            // owns its elements); the copy ends the ref's borrow.
            let elem_val = self.decay_if_ref(elem_val);
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
        let callee = self.literal_init_callee(&init, result_ty);
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
        let init = self.literal_init(Builtin::DictionaryLiteralInit, result_ty)?;
        let ptr_param = *init.params.first()?;
        let MirTy::Pointer(pair_ty) = *self.ctx.module.ty_arena.get(ptr_param) else {
            return None;
        };

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
            // Stage 1.5: ref entries decay to owned copies (the dict owns
            // its keys and values).
            let key = self.decay_if_ref(key);
            let val = self.lower_expr(entry.value);
            let val = self.decay_if_ref(val);

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
        let callee = self.literal_init_callee(&init, result_ty);
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
    // Literal requirement inits
    // ================================================================

    /// The requirement init a literal of type `ty` builds through, or `None`
    /// when `ty` takes the literal as an immediate (`lang.i64`, tuples, …).
    ///
    /// A named type qualifies when it conforms to the requirement's protocol.
    /// A type parameter or projection always does — the literal type-checked,
    /// so its bound supplies the witness at mono.
    pub(crate) fn literal_init(&mut self, builtin: Builtin, ty: TyId) -> Option<LiteralInit> {
        let named = match self.ctx.module.ty_arena.get(ty) {
            MirTy::Named { entity, .. } => Some(*entity),
            MirTy::TypeParam(_) | MirTy::AssociatedProjection { .. } => None,
            _ => return None,
        };
        let requirement = self.ctx.query.query(ResolveBuiltin {
            builtin,
            root: self.ctx.root,
        })?;
        let protocol = self.ctx.is_protocol_method(requirement)?;
        if let Some(entity) = named {
            let conforms = self
                .ctx
                .query
                .query(ConformingProtocols {
                    entity,
                    root: self.ctx.root,
                })
                .contains(&protocol);
            if !conforms {
                return None;
            }
        }
        let key = self.ctx.witness_method_key(requirement);
        self.ctx.register_name(protocol);

        // The protocol's `Self` is `TypeParam(protocol)`.
        let mut subst = SubstMap::new();
        subst.type_params.insert(protocol, ty);
        let params = resolve_callable_types(self.ctx, requirement)
            .into_iter()
            .map(|param| {
                let module = &mut self.ctx.module;
                param.map(|p| {
                    substitute_and_resolve(&mut module.ty_arena, &module.witnesses, p, &subst)
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(LiteralInit {
            protocol,
            key,
            params,
        })
    }

    /// The witness call for `init` on a literal of type `self_type`.
    fn literal_init_callee(&mut self, init: &LiteralInit, self_type: TyId) -> Callee {
        Callee::Witness {
            protocol: init.protocol,
            method: init.key.clone(),
            self_type,
            method_type_args: vec![],
        }
    }
}
