//! Boxed cases of an `indirect enum` in body lowering.
//!
//! `enum_lower` stores each recursive case as ONE physical payload field,
//! `OwnedBox[P]`, where `P` is the lone declared field's type or the tuple of
//! all of them (`EnumCaseDef::indirect`). Everything below body lowering —
//! layout, drop and clone shims, mono, codegen — sees only that box.
//!
//! Body lowering keeps speaking the DECLARED (logical) payload, and the three
//! enum choke points translate at the boundary:
//! - `emit_enum_variant` packs the logical fields and boxes them
//!   (`OwnedBox(consuming:)`) — [`OssaBodyCtx::emit_indirect_box`];
//! - `emit_destructure_enum` moves the box out and takes the payload back
//!   (`takeValue()`, which frees the block) — [`OssaBodyCtx::emit_indirect_unbox`];
//! - `emit_enum_payload` views the payload IN PLACE through the box's
//!   pointer, a scope-tracked address borrow like an `is T` payload view —
//!   [`OssaBodyCtx::emit_indirect_view`].
//!
//! The box is found through `@builtin(.OwnedBox)` and its members by shape,
//! never by a hard-coded path, like the closure-box bindings.

use kestrel_ast_builder::NodeKind;
use kestrel_hecs::Entity;
use kestrel_mir::callee::Callee;
use kestrel_mir::inst::{CallArg, InstKind};
use kestrel_mir::value::Ownership;
use kestrel_mir::{FieldIdx, Immediate, MirTy, Op, ParamConvention, TyId, ValueId, VariantIdx};

use super::OssaBodyCtx;
use super::closure_box::is_box_init_shape;

/// A boxed case of an `indirect enum`, substituted at one enum instance.
pub(crate) struct IndirectCase {
    /// The declared payload types, in order.
    pub logical_tys: Vec<TyId>,
    /// What the box holds: the lone field, or the tuple of all of them.
    pub payload_ty: TyId,
    /// `OwnedBox[payload_ty]` — the case's one physical field.
    pub box_ty: TyId,
    /// `payload_ty` is a tuple (two or more declared fields).
    pub is_tuple: bool,
}

impl OssaBodyCtx<'_, '_> {
    /// The boxed layout of `variant` of `enum_ty`, or `None` when the case is
    /// stored inline (every case of an ordinary enum).
    pub(crate) fn indirect_case(
        &mut self,
        enum_ty: TyId,
        variant: VariantIdx,
    ) -> Option<IndirectCase> {
        let MirTy::Named { entity, type_args } = self.ctx.module.ty_arena.get(enum_ty) else {
            return None;
        };
        let (entity, type_args) = (*entity, type_args.clone());
        let edef = self.ctx.module.enums.get(&entity)?;
        let ind = edef.cases.get(variant.index())?.indirect.as_ref()?;
        let raw: Vec<TyId> = ind.logical_fields.iter().map(|f| f.ty).collect();
        let (payload_ty, box_ty, is_tuple) = (ind.payload_ty, ind.box_ty, ind.is_tuple());
        let mut subst = kestrel_mir::SubstMap::new();
        for (tp, &arg) in edef.type_params.iter().zip(type_args.iter()) {
            subst.type_params.insert(tp.entity, arg);
        }
        let arena = &mut self.ctx.module.ty_arena;
        Some(IndirectCase {
            logical_tys: raw
                .iter()
                .map(|&t| kestrel_mir::substitute(arena, t, &subst))
                .collect(),
            payload_ty: kestrel_mir::substitute(arena, payload_ty, &subst),
            box_ty: kestrel_mir::substitute(arena, box_ty, &subst),
            is_tuple,
        })
    }

    /// Pack the logical `payload` fields and box them: `OwnedBox(consuming:
    /// (f0, f1, …))`. Returns the @owned box — the case's physical payload.
    pub(crate) fn emit_indirect_box(
        &mut self,
        case: &IndirectCase,
        payload: Vec<ValueId>,
    ) -> ValueId {
        let packed = if case.is_tuple {
            self.emit_tuple(case.payload_ty, payload)
        } else {
            payload
                .into_iter()
                .next()
                .expect("ICE: boxed enum case with no payload value")
        };
        let owned_box = self.owned_box_entity(case.box_ty);
        let init = self.find_box_member(
            owned_box,
            NodeKind::Initializer,
            "init(consuming value: T)",
            |c, _| is_box_init_shape(c),
        );
        self.ctx.register_name(init);
        // `emit_init_literal_call` shape: a stack slot for `self`, the init
        // writes it, `Take` moves the finished box out.
        let ptr_ty = self.ctx.module.ty_arena.pointer(case.box_ty);
        let one = self.emit_literal(Immediate::i64(1));
        let self_ptr = self.emit_op1(Op::StackAlloc(case.box_ty), one, ptr_ty);
        let arg = self.prepare_call_arg(packed, ParamConvention::Consuming);
        self.emit_call_void(
            Callee::direct_with_args(init, vec![case.payload_ty], None),
            vec![
                CallArg {
                    value: self_ptr,
                    convention: ParamConvention::MutBorrow,
                },
                arg,
            ],
        );
        self.emit_take(self_ptr, case.box_ty)
    }

    /// Consume an @owned box and move its payload out (`takeValue()`, which
    /// frees the block), split back into the @owned logical fields.
    pub(crate) fn emit_indirect_unbox(&mut self, case: &IndirectCase, boxed: ValueId) -> Vec<ValueId> {
        let owned_box = self.owned_box_entity(case.box_ty);
        let take_value =
            self.find_box_member(owned_box, NodeKind::Function, "takeValue()", |c, name| {
                name == "takeValue" && c.params.is_empty()
            });
        self.ctx.register_name(take_value);
        let payload = self.emit_call_returning(
            Callee::direct_with_args(take_value, vec![case.payload_ty], Some(case.box_ty)),
            vec![CallArg {
                value: boxed,
                convention: ParamConvention::Consuming,
            }],
            case.payload_ty,
        );
        if case.is_tuple {
            self.emit_destructure_tuple(payload, &case.logical_tys)
        } else {
            vec![payload]
        }
    }

    /// A @guaranteed view of the payload of `box_view` (a @guaranteed
    /// `OwnedBox[P]`), read in place through the box's pointer. The view is
    /// a scope-tracked address borrow, like an `is T` payload view
    /// (`emit_payload_view`): a call to `pointeeRef()` would instead yield a
    /// single-use ref result that may not live across a match's branches.
    pub(crate) fn emit_indirect_view(&mut self, box_view: ValueId, case: &IndirectCase) -> ValueId {
        let (field, field_ty) = self.box_pointer_field(case);
        let mut cur = self.alloc_guaranteed(field_ty, box_view);
        self.push_inst(InstKind::StructExtract {
            result: cur,
            operand: box_view,
            field,
        });
        // `Pointer[P]` → … → the raw machine pointer.
        let (wrappers, raw_ty) = self
            .unwrap_handle_to_pointer(field_ty)
            .expect("ICE: OwnedBox pointer field does not peel to a raw pointer");
        for i in 0..wrappers.len() {
            let next_ty = wrappers.get(i + 1).copied().unwrap_or(raw_ty);
            let next = self.alloc_guaranteed(next_ty, cur);
            self.push_inst(InstKind::StructExtract {
                result: next,
                operand: cur,
                field: FieldIdx::new(0),
            });
            cur = next;
        }
        // A @guaranteed scalar is its storage's ADDRESS in codegen; the borrow
        // needs the pointer word itself, so load it (a trivial copy). The
        // view borrows from the box view, not from that temporary word.
        let word = self.emit_copy_value(cur);
        self.addr_anchors.insert(word, box_view);
        let view = self.emit_begin_borrow_addr(word, case.payload_ty);
        debug_assert_eq!(self.body.value(view).ownership, Ownership::Guaranteed);
        view
    }

    /// The `OwnedBox` entity of a box type.
    fn owned_box_entity(&self, box_ty: TyId) -> Entity {
        match self.ctx.module.ty_arena.get(box_ty) {
            MirTy::Named { entity, .. } => *entity,
            _ => panic!("ICE: indirect enum box type is not a nominal OwnedBox"),
        }
    }

    /// The box's one field that peels to a raw pointer to the payload
    /// (`ptr: Pointer[T]`), substituted at `case.box_ty`.
    fn box_pointer_field(&mut self, case: &IndirectCase) -> (FieldIdx, TyId) {
        let owned_box = self.owned_box_entity(case.box_ty);
        let (fields, tps) = {
            let sdef = self
                .ctx
                .module
                .structs
                .get(&owned_box)
                .expect("ICE: OwnedBox has no struct definition");
            let fields: Vec<TyId> = sdef.fields.iter().map(|f| f.ty).collect();
            (fields, sdef.type_params.clone())
        };
        let mut subst = kestrel_mir::SubstMap::new();
        if let Some(tp) = tps.first() {
            subst.type_params.insert(tp.entity, case.payload_ty);
        }
        let target = self.ctx.module.ty_arena.pointer(case.payload_ty);
        for (i, ty) in fields.into_iter().enumerate() {
            let ty = kestrel_mir::substitute(&mut self.ctx.module.ty_arena, ty, &subst);
            if self
                .unwrap_handle_to_pointer(ty)
                .is_some_and(|(_, raw)| raw == target)
            {
                return (FieldIdx::new(i), ty);
            }
        }
        panic!("ICE: OwnedBox has no field pointing at its payload")
    }
}
