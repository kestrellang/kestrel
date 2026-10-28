//! Existential function tables and thunks (existentials plan Phase 10).
//!
//! Lowering builds, on demand:
//! - **thunks**: one generic function per requirement, over the payload type
//!   (the requirement's own protocol entity is the type param, so `Self` and
//!   `Self.Item` in its signature need no substitution). It takes the data
//!   word, re-types it as `Pointer[Self]`, and calls the witness;
//! - **a drop thunk**: `drop_in_place` on the payload;
//! - **templates** (`kestrel_mir::item::vtable`): one per box and protocol,
//!   one per box and composition.
//!
//! Mono instantiates a template at the payload wherever a `VTable` literal
//! names it. The word layout and the dispatch path of a requirement both come
//! from [`ancestors`] and [`kestrel_semantics::ExistentialLayout`], so a slot
//! index can't drift between the table and its callers.

use std::collections::HashMap;

use kestrel_ast_builder::Callable;
use kestrel_hecs::Entity;
use kestrel_mir::block::BasicBlock;
use kestrel_mir::body::OssaBody;
use kestrel_mir::callee::Callee;
use kestrel_mir::immediate::Immediate;
use kestrel_mir::inst::{CallArg, InstKind, Instruction};
use kestrel_mir::item::vtable::{VTABLE_HEADER_WORDS, VTableEntry, VTableTemplate};
use kestrel_mir::item::{FunctionDef, FunctionKind, ParamDef, TypeParamDef};
use kestrel_mir::terminator::{Terminator, TerminatorKind};
use kestrel_mir::value::{Ownership, RootProvenance, ValueDef};
use kestrel_mir::{MirTy, Op, ParamConvention, TyId, ValueId};

use crate::LowerCtx;
use crate::items::function_sig::{param_convention, receiver_convention};

/// Per-module caches: every thunk and template is built once.
#[derive(Default)]
pub struct ExistentialState {
    drop_thunk: Option<Entity>,
    discard_fn: Option<Entity>,
    take_fn: Option<Entity>,
    thunks: HashMap<Entity, Entity>,
    protocol_templates: HashMap<(Entity, Entity), Entity>,
    composition_templates: HashMap<(Entity, Vec<Entity>), Entity>,
}

/// The declared ancestors of `protocol` (transitively, through `: Parent`
/// edges only), ordered by qualified name. A single-protocol table holds one
/// pointer per ancestor, in this order, right after the header. A name, not
/// the entity index, so the layout doesn't depend on entity numbering.
pub(crate) fn ancestors(ctx: &LowerCtx, protocol: Entity) -> Vec<Entity> {
    let mut out: Vec<Entity> = kestrel_semantics::bound_closure(&ctx.query, &[protocol], ctx.root)
        .into_iter()
        .filter(|&p| p != protocol)
        .collect();
    out.sort_by_cached_key(|&p| kestrel_ast_builder::qualified_name(ctx.world, p));
    out
}

/// `protocol`'s own requirements that get a table slot, in slot order.
fn slots(ctx: &LowerCtx, protocol: Entity) -> Vec<Entity> {
    ctx.query
        .query(kestrel_semantics::ExistentialLayout {
            protocol,
            root: ctx.root,
        })
        .slots()
        .collect()
}

/// The bounds of a raw existential, canonical order.
pub(crate) fn bounds_of(ctx: &LowerCtx, raw: TyId) -> Vec<Entity> {
    match ctx.module.ty_arena.get(raw) {
        MirTy::RawExistential { bounds, .. } => bounds.iter().map(|(p, _)| *p).collect(),
        _ => Vec::new(),
    }
}

/// Where requirement `requirement` lives in a table for `bounds`: the words
/// to load in turn (each yields a sub-table), then the slot word.
pub(crate) fn dispatch_path(
    ctx: &LowerCtx,
    bounds: &[Entity],
    requirement: Entity,
) -> Option<(Vec<u32>, u32)> {
    let owner = ctx.world.parent_of(requirement)?;
    let mut hops = Vec::new();
    let protocol = match bounds {
        [single] => *single,
        _ => {
            let i = bounds
                .iter()
                .position(|&b| b == owner || ancestors(ctx, b).contains(&owner))?;
            hops.push(VTABLE_HEADER_WORDS + i as u32);
            bounds[i]
        },
    };
    if owner != protocol {
        let i = ancestors(ctx, protocol).iter().position(|&a| a == owner)?;
        hops.push(VTABLE_HEADER_WORDS + i as u32);
    }
    let slot = slots(ctx, owner).iter().position(|&r| r == requirement)?;
    let slot = VTABLE_HEADER_WORDS + ancestors(ctx, owner).len() as u32 + slot as u32;
    Some((hops, slot))
}

/// The path from a table for `from` to the table for the single protocol
/// `to` (an upcast): the words to load in turn. `None` when `to` isn't a
/// bound or an ancestor of one.
pub(crate) fn upcast_path(ctx: &LowerCtx, from: &[Entity], to: Entity) -> Option<Vec<u32>> {
    let mut hops = Vec::new();
    let protocol = match from {
        [single] => *single,
        _ => {
            let i = from
                .iter()
                .position(|&b| b == to || ancestors(ctx, b).contains(&to))?;
            hops.push(VTABLE_HEADER_WORDS + i as u32);
            from[i]
        },
    };
    if protocol != to {
        let i = ancestors(ctx, protocol).iter().position(|&a| a == to)?;
        hops.push(VTABLE_HEADER_WORDS + i as u32);
    }
    Some(hops)
}

/// The template for an existential with `bounds` in box `boxed`.
pub(crate) fn template_for(ctx: &mut LowerCtx, boxed: Entity, bounds: &[Entity]) -> Entity {
    if let [single] = bounds {
        return protocol_template(ctx, boxed, *single);
    }
    let key = (boxed, bounds.to_vec());
    if let Some(&t) = ctx.existentials.composition_templates.get(&key) {
        return t;
    }
    let mut entries = header(ctx, bounds);
    for &b in bounds {
        entries.push(VTableEntry::Table(protocol_template(ctx, boxed, b)));
    }
    let names: Vec<String> = bounds.iter().map(|&b| ctx.register_name(b)).collect();
    let entity = ctx.next_synthetic_entity();
    // The box is part of the name: tables are per (box, payload, bounds).
    let name = format!("__vtable.{}.{}", ctx.register_name(boxed), names.join("+"));
    add_template(ctx, entity, name, entries);
    ctx.existentials.composition_templates.insert(key, entity);
    entity
}

fn protocol_template(ctx: &mut LowerCtx, boxed: Entity, protocol: Entity) -> Entity {
    if let Some(&t) = ctx.existentials.protocol_templates.get(&(boxed, protocol)) {
        return t;
    }
    let entity = ctx.next_synthetic_entity();
    // Register before building ancestors: a refinement cycle (rejected
    // upstream) must not recurse forever.
    ctx.existentials
        .protocol_templates
        .insert((boxed, protocol), entity);
    let mut entries = header(ctx, &[protocol]);
    for a in ancestors(ctx, protocol) {
        entries.push(VTableEntry::Table(protocol_template(ctx, boxed, a)));
    }
    for requirement in slots(ctx, protocol) {
        entries.push(VTableEntry::Func(requirement_thunk(
            ctx,
            protocol,
            requirement,
        )));
    }
    let name = format!(
        "__vtable.{}.{}",
        ctx.register_name(boxed),
        ctx.register_name(protocol)
    );
    add_template(ctx, entity, name, entries);
    entity
}

fn add_template(ctx: &mut LowerCtx, entity: Entity, name: String, entries: Vec<VTableEntry>) {
    ctx.module.register_name(entity, name.clone());
    ctx.module.vtable_templates.insert(
        entity,
        VTableTemplate {
            entity,
            name,
            entries,
        },
    );
}

/// The four header words every table starts with. The clone word is filled
/// only when the bounds refine `Cloneable`, so every payload has a clone.
fn header(ctx: &mut LowerCtx, bounds: &[Entity]) -> Vec<VTableEntry> {
    let cloneable = ctx.module.cloneable_protocol.is_some_and(|c| {
        kestrel_semantics::bound_closure(&ctx.query, bounds, ctx.root).contains(&c)
    });
    let clone = match cloneable.then(|| clone_thunk(ctx)).flatten() {
        Some(thunk) => VTableEntry::Func(thunk),
        None => VTableEntry::Null,
    };
    vec![
        VTableEntry::Func(drop_thunk(ctx)),
        VTableEntry::Size,
        VTableEntry::Align,
        clone,
    ]
}

/// `fn clone[S](src: Pointer[i8], dst: Pointer[i8])`: write a clone of the
/// payload at `src` into the uninitialized `dst`, through `S: Cloneable`.
/// `None` without a stdlib (no `Cloneable`).
pub(crate) fn clone_thunk(ctx: &mut LowerCtx) -> Option<Entity> {
    if let Some(e) = ctx.module.existential_clone_thunk {
        return Some(e);
    }
    let cloneable = ctx.module.cloneable_protocol?;
    let requirement = ctx
        .world
        .children_of(cloneable)
        .iter()
        .copied()
        .find(|&c| {
            ctx.world
                .get::<kestrel_ast_builder::Name>(c)
                .map(|n| n.0.as_str())
                == Some("clone")
        })?;
    ctx.register_name(cloneable);
    let key = ctx.witness_method_key(requirement);
    let entity = ctx.next_synthetic_entity();
    let param = ctx.next_synthetic_entity();
    ctx.module.register_name(param, "Payload");
    let payload = ctx.module.ty_arena.intern(MirTy::TypeParam(param));
    let payload_ptr = ctx.module.ty_arena.pointer(payload);
    let unit = ctx.module.ty_arena.unit();
    let mut b = ThunkBuilder::new(ctx);
    let src = b.data_param();
    let dst = b.param(b.data_ty, ParamConvention::Consuming);
    let src_ptr = b.op1(Op::PtrCast(payload_ptr), src, payload_ptr);
    let view = b.body.alloc_value(ValueDef::guaranteed(payload, src_ptr));
    b.insts.push(Instruction::new(InstKind::BeginBorrowAddr {
        result: view,
        address: src_ptr,
        ty: payload,
    }));
    let copy = b.body.alloc_value(ValueDef::owned(payload));
    b.insts.push(Instruction::new(InstKind::Call {
        result: Some(copy),
        callee: Callee::Witness {
            protocol: cloneable,
            method: key,
            self_type: payload,
            method_type_args: Vec::new(),
        },
        args: vec![CallArg {
            value: view,
            convention: ParamConvention::Borrow,
        }],
    }));
    b.insts
        .push(Instruction::new(InstKind::EndBorrow { operand: view }));
    let dst_ptr = b.op1(Op::PtrCast(payload_ptr), dst, payload_ptr);
    b.insts.push(Instruction::new(InstKind::StoreInit {
        address: dst_ptr,
        value: copy,
    }));
    for v in [src_ptr, dst_ptr, src, dst] {
        b.insts
            .push(Instruction::new(InstKind::DestroyValue { operand: v }));
    }
    let ret = b.literal(Immediate::unit(), unit);
    let data_ty = b.data_ty;
    b.extra_params.push(ParamDef::new(
        "dst",
        dst,
        data_ty,
        ParamConvention::Consuming,
    ));
    let tps = vec![TypeParamDef::new(param, "Payload")];
    b.finish(ctx, entity, "__existential.clone", tps, unit, ret);
    ctx.module.existential_clone_thunk = Some(entity);
    Some(entity)
}

/// `fn drop[S](data: Pointer[i8])`: drop the payload in place.
fn drop_thunk(ctx: &mut LowerCtx) -> Entity {
    if let Some(e) = ctx.existentials.drop_thunk {
        return e;
    }
    let entity = ctx.next_synthetic_entity();
    let param = ctx.next_synthetic_entity();
    ctx.module.register_name(entity, "__existential.drop");
    ctx.module.register_name(param, "Payload");
    let payload = ctx.module.ty_arena.intern(MirTy::TypeParam(param));
    let payload_ptr = ctx.module.ty_arena.pointer(payload);
    let unit = ctx.module.ty_arena.unit();
    let mut b = ThunkBuilder::new(ctx);
    let data = b.data_param();
    let ptr = b.op1(Op::PtrCast(payload_ptr), data, payload_ptr);
    b.insts.push(Instruction::new(InstKind::DestroyAddr {
        address: ptr,
        ty: payload,
        user_deinit: false,
    }));
    b.insts
        .push(Instruction::new(InstKind::DestroyValue { operand: ptr }));
    b.insts
        .push(Instruction::new(InstKind::DestroyValue { operand: data }));
    let ret = b.literal(Immediate::unit(), unit);
    b.finish(
        ctx,
        entity,
        "__existential.drop",
        vec![TypeParamDef::new(param, "Payload")],
        unit,
        ret,
    );
    ctx.existentials.drop_thunk = Some(entity);
    entity
}

/// `fn discard[S](p: Pointer[S])`: ignore the pointer. The body handed to
/// `consumePointee` once a consuming requirement already moved the payload
/// out.
pub(crate) fn discard_fn(ctx: &mut LowerCtx) -> Entity {
    if let Some(e) = ctx.existentials.discard_fn {
        return e;
    }
    let entity = ctx.next_synthetic_entity();
    let param = ctx.next_synthetic_entity();
    ctx.module.register_name(param, "Payload");
    let payload = ctx.module.ty_arena.intern(MirTy::TypeParam(param));
    let ptr_ty = ctx.module.ty_arena.pointer(payload);
    let unit = ctx.module.ty_arena.unit();
    let mut body = OssaBody::new();
    let p = body.alloc_value(ValueDef::owned(ptr_ty).with_root(RootProvenance::Param(0)));
    body.param_count = 1;
    let ret = body.alloc_value(ValueDef::owned(unit));
    let entry = body.alloc_block();
    body.entry = entry;
    let block = body.block_mut(entry);
    block.insts = vec![
        Instruction::new(InstKind::DestroyValue { operand: p }),
        Instruction::new(InstKind::Literal {
            result: ret,
            value: Immediate::unit(),
        }),
    ];
    block.terminator = Terminator::new(TerminatorKind::Return(ret));
    let mut func = FunctionDef::new(entity, "__existential.discard", unit);
    func.type_params = vec![TypeParamDef::new(param, "Payload")];
    func.kind = FunctionKind::Free;
    func.params
        .push(ParamDef::new("p", p, ptr_ty, ParamConvention::Consuming));
    func.body = Some(body);
    ctx.module.register_name(entity, "__existential.discard");
    ctx.module.add_function(func);
    ctx.existentials.discard_fn = Some(entity);
    entity
}

/// `fn take[S, T](p: Pointer[S]) -> T`: move a `T` out of the payload at
/// `p` (an `is T` test proved the payload's type). The body handed to
/// `consumePointee` by `consuming x as? T`.
pub(crate) fn take_fn(ctx: &mut LowerCtx) -> Entity {
    if let Some(e) = ctx.existentials.take_fn {
        return e;
    }
    let entity = ctx.next_synthetic_entity();
    let source = ctx.next_synthetic_entity();
    let target = ctx.next_synthetic_entity();
    ctx.module.register_name(source, "Payload");
    ctx.module.register_name(target, "Target");
    let source_ty = ctx.module.ty_arena.intern(MirTy::TypeParam(source));
    let target_ty = ctx.module.ty_arena.intern(MirTy::TypeParam(target));
    let source_ptr = ctx.module.ty_arena.pointer(source_ty);
    let target_ptr = ctx.module.ty_arena.pointer(target_ty);
    let mut body = OssaBody::new();
    let p = body.alloc_value(ValueDef::owned(source_ptr).with_root(RootProvenance::Param(0)));
    body.param_count = 1;
    let cast = body.alloc_value(ValueDef::owned(target_ptr));
    let value = body.alloc_value(ValueDef::owned(target_ty));
    let entry = body.alloc_block();
    body.entry = entry;
    let block = body.block_mut(entry);
    block.insts = vec![
        Instruction::new(InstKind::Op1 {
            result: cast,
            op: Op::PtrCast(target_ty),
            arg: p,
        }),
        Instruction::new(InstKind::Take {
            result: value,
            address: cast,
            ty: target_ty,
            independent: true,
        }),
        Instruction::new(InstKind::DestroyValue { operand: cast }),
        Instruction::new(InstKind::DestroyValue { operand: p }),
    ];
    block.terminator = Terminator::new(TerminatorKind::Return(value));
    let mut func = FunctionDef::new(entity, "__existential.take", target_ty);
    func.type_params = vec![
        TypeParamDef::new(source, "Payload"),
        TypeParamDef::new(target, "Target"),
    ];
    func.kind = FunctionKind::Free;
    func.params.push(ParamDef::new(
        "p",
        p,
        source_ptr,
        ParamConvention::Consuming,
    ));
    func.body = Some(body);
    ctx.module.register_name(entity, "__existential.take");
    ctx.module.add_function(func);
    ctx.existentials.take_fn = Some(entity);
    entity
}

/// The thunk for `requirement` of `protocol`: `fn [Self](data, args…) -> R`
/// calling `Self`'s witness. Shared by every box (a consuming requirement
/// never gets a table slot through a box that can't give up its payload).
fn requirement_thunk(ctx: &mut LowerCtx, protocol: Entity, requirement: Entity) -> Entity {
    if let Some(&t) = ctx.existentials.thunks.get(&requirement) {
        return t;
    }
    let entity = ctx.next_synthetic_entity();
    let req_name = ctx.register_name(requirement);
    let name = format!("{req_name}.__thunk");
    ctx.register_name(protocol);
    let self_ty = ctx.module.ty_arena.intern(MirTy::TypeParam(protocol));
    let self_ptr = ctx.module.ty_arena.pointer(self_ty);
    let ret = crate::ty::resolve_callable_return_type(ctx, requirement);
    let param_tys = crate::ty::resolve_callable_types(ctx, requirement);
    let callable = ctx.world.get::<Callable>(requirement).cloned();
    let receiver = callable
        .as_ref()
        .and_then(|c| c.receiver.as_ref())
        .map(receiver_convention)
        .unwrap_or(ParamConvention::Borrow);
    let key = ctx.witness_method_key(requirement);

    let mut b = ThunkBuilder::new(ctx);
    let data = b.data_param();
    let mut forwarded: Vec<(String, TyId, ParamConvention, ValueId, Option<String>)> = Vec::new();
    if let Some(c) = &callable {
        for (i, p) in c.params.iter().enumerate() {
            let ty = param_tys
                .get(i)
                .copied()
                .flatten()
                .unwrap_or_else(|| ctx.module.ty_arena.error());
            let conv = param_convention(p, false);
            let v = b.param(ty, conv);
            forwarded.push((p.name.clone(), ty, conv, v, p.label.clone()));
        }
    }
    let ptr = b.op1(Op::PtrCast(self_ptr), data, self_ptr);
    // The receiver the witness expects: a borrow of the payload, its address
    // for `mutating`, or the payload itself moved out for `consuming` (the
    // call site then frees the box through `consumePointee`).
    let (recv, borrowed) = match receiver {
        ParamConvention::MutBorrow => (ptr, None),
        ParamConvention::Consuming => {
            let v = b.body.alloc_value(ValueDef::owned(self_ty));
            b.insts.push(Instruction::new(InstKind::Take {
                result: v,
                address: ptr,
                ty: self_ty,
                independent: true,
            }));
            (v, None)
        },
        _ => {
            let r = b.body.alloc_value(ValueDef::guaranteed(self_ty, ptr));
            b.insts.push(Instruction::new(InstKind::BeginBorrowAddr {
                result: r,
                address: ptr,
                ty: self_ty,
            }));
            (r, Some(r))
        },
    };
    let mut args = vec![CallArg {
        value: recv,
        convention: receiver,
    }];
    args.extend(forwarded.iter().map(|(_, _, conv, v, _)| CallArg {
        value: *v,
        convention: *conv,
    }));
    let result = b.body.alloc_value(ValueDef::owned(ret));
    b.insts.push(Instruction::new(InstKind::Call {
        result: Some(result),
        callee: Callee::Witness {
            protocol,
            method: key,
            self_type: self_ty,
            method_type_args: Vec::new(),
        },
        args,
    }));
    if let Some(r) = borrowed {
        b.insts
            .push(Instruction::new(InstKind::EndBorrow { operand: r }));
    }
    b.insts
        .push(Instruction::new(InstKind::DestroyValue { operand: ptr }));
    b.insts
        .push(Instruction::new(InstKind::DestroyValue { operand: data }));
    let mut params_extra: Vec<ParamDef> = forwarded
        .into_iter()
        .map(|(n, ty, conv, v, label)| ParamDef::with_label(&n, v, ty, conv, label))
        .collect();
    let tps = vec![TypeParamDef::new(protocol, "Self")];
    b.extra_params.append(&mut params_extra);
    b.finish(ctx, entity, &name, tps, ret, result);
    ctx.existentials.thunks.insert(requirement, entity);
    entity
}

/// Hand-builds a single-block thunk body. The first param is always the
/// payload's data word (`Pointer[i8]`, by value).
struct ThunkBuilder {
    body: OssaBody,
    insts: Vec<Instruction>,
    data_ty: TyId,
    data: Option<ValueId>,
    extra_params: Vec<ParamDef>,
}

impl ThunkBuilder {
    fn new(ctx: &mut LowerCtx) -> Self {
        let i8 = ctx.module.ty_arena.i8();
        Self {
            body: OssaBody::new(),
            insts: Vec::new(),
            data_ty: ctx.module.ty_arena.pointer(i8),
            data: None,
            extra_params: Vec::new(),
        }
    }

    fn data_param(&mut self) -> ValueId {
        let v = self
            .body
            .alloc_value(ValueDef::owned(self.data_ty).with_root(RootProvenance::Param(0)));
        self.body.param_count = 1;
        self.data = Some(v);
        v
    }

    /// A forwarded parameter (allocated right after the data word).
    fn param(&mut self, ty: TyId, conv: ParamConvention) -> ValueId {
        let index = self.body.param_count;
        let ownership = match conv {
            ParamConvention::Consuming => Ownership::Owned,
            _ => Ownership::Guaranteed,
        };
        let v = self.body.alloc_value(ValueDef {
            ty,
            ownership,
            borrow_source: None,
            root: RootProvenance::Param(index as u32),
            span: None,
        });
        self.body.param_count += 1;
        v
    }

    fn op1(&mut self, op: Op, arg: ValueId, ty: TyId) -> ValueId {
        let result = self.body.alloc_value(ValueDef::owned(ty));
        self.insts
            .push(Instruction::new(InstKind::Op1 { result, op, arg }));
        result
    }

    fn literal(&mut self, value: Immediate, ty: TyId) -> ValueId {
        let result = self.body.alloc_value(ValueDef::owned(ty));
        self.insts
            .push(Instruction::new(InstKind::Literal { result, value }));
        result
    }

    fn finish(
        mut self,
        ctx: &mut LowerCtx,
        entity: Entity,
        name: &str,
        type_params: Vec<TypeParamDef>,
        ret_ty: TyId,
        ret: ValueId,
    ) {
        let entry = self.body.alloc_block();
        self.body.entry = entry;
        let block: &mut BasicBlock = self.body.block_mut(entry);
        block.insts = std::mem::take(&mut self.insts);
        block.terminator = Terminator::new(TerminatorKind::Return(ret));
        let mut func = FunctionDef::new(entity, name, ret_ty);
        func.type_params = type_params;
        func.kind = FunctionKind::Free;
        func.params.push(ParamDef::new(
            "data",
            self.data.expect("thunk without a data param"),
            self.data_ty,
            ParamConvention::Consuming,
        ));
        func.params.append(&mut self.extra_params);
        func.body = Some(self.body);
        ctx.module.register_name(entity, name);
        ctx.module.add_function(func);
    }
}

// ===== `any P: P` witnesses (existentials design §6) =====

/// For every box over a raw existential the module mentions
/// (`B[rawany P]`), a witness of each protocol `Q` in the bounds' refinement
/// closure whose self-conformance holds: each requirement forwards to the
/// payload — peel the box (`pointeeRef`, or `pointeeMutRef` for `mutating`)
/// and call through the table. This is what lets a generic `T: P` (and
/// operators / `for-in`) take an existential with `T = any P`.
pub(crate) fn lower_self_conformance_witnesses(ctx: &mut LowerCtx) {
    let mut done: std::collections::HashSet<TyId> = Default::default();
    loop {
        let boxes: Vec<(TyId, Entity, TyId)> = (0..ctx.module.ty_arena.len())
            .map(TyId::new)
            .filter(|ty| !done.contains(ty))
            .filter_map(|ty| match ctx.module.ty_arena.get(ty) {
                MirTy::Named { entity, type_args } => match type_args.as_slice() {
                    [raw]
                        if matches!(
                            ctx.module.ty_arena.get(*raw),
                            MirTy::RawExistential { .. }
                        ) =>
                    {
                        Some((ty, *entity, *raw))
                    },
                    _ => None,
                },
                _ => None,
            })
            .collect();
        if boxes.is_empty() {
            return;
        }
        for (box_ty, boxed, raw) in boxes {
            done.insert(box_ty);
            box_witnesses(ctx, box_ty, boxed, raw);
        }
    }
}

fn box_witnesses(ctx: &mut LowerCtx, box_ty: TyId, boxed: Entity, raw: TyId) {
    let MirTy::RawExistential { bindings, .. } = ctx.module.ty_arena.get(raw).clone() else {
        return;
    };
    let bounds = bounds_of(ctx, raw);
    let pinned: Vec<Entity> = bindings.iter().map(|(a, _)| *a).collect();
    // The box type's own type params are the witness pattern's wildcards.
    let mut params = Vec::new();
    collect_type_params(&ctx.module.ty_arena, box_ty, &mut params);
    for protocol in kestrel_semantics::bound_closure(&ctx.query, &bounds, ctx.root) {
        let needs = kestrel_semantics::self_conformance(&ctx.query, protocol, &pinned, ctx.root);
        if needs.blocker.is_some() {
            continue;
        }
        let mut witness = kestrel_mir::item::WitnessDef::new(protocol, box_ty);
        witness.type_bindings = bindings
            .iter()
            .filter(|(a, _)| ctx.world.parent_of(*a) == Some(protocol))
            .copied()
            .collect();
        let requirements: Vec<Entity> = ctx
            .query
            .query(kestrel_semantics::ExistentialLayout {
                protocol,
                root: ctx.root,
            })
            .slots()
            .collect();
        for requirement in requirements {
            let Some(func) = forwarder(ctx, box_ty, boxed, protocol, requirement, &params) else {
                continue;
            };
            let key = ctx.witness_method_key(requirement);
            let type_args: Vec<TyId> = params
                .iter()
                .map(|&p| ctx.module.ty_arena.intern(MirTy::TypeParam(p)))
                .collect();
            witness
                .methods
                .push(kestrel_mir::item::WitnessMethodBinding::new(
                    key, func, type_args,
                ));
        }
        // Protocol extension defaults bind directly, with the box as `Self`
        // — as for any conformer.
        let members = ctx.query.query(kestrel_name_res::ProtocolMembers {
            protocol,
            root: ctx.root,
        });
        for m in members
            .iter()
            .filter(|m| m.extension.is_some() && m.declaring_protocol == protocol)
        {
            ctx.register_name(m.entity);
            let key = ctx.witness_method_key(m.entity);
            let type_args: Vec<TyId> = params
                .iter()
                .map(|&p| ctx.module.ty_arena.intern(MirTy::TypeParam(p)))
                .collect();
            witness
                .methods
                .push(kestrel_mir::item::WitnessMethodBinding::new(
                    key, m.entity, type_args,
                ));
        }
        kestrel_debug::ktrace!(
            "vtable",
            "self-conformance witness {:?}: {:?} ({} methods)",
            protocol,
            box_ty,
            witness.methods.len()
        );
        ctx.module.witnesses.push(witness);
    }
}

fn collect_type_params(arena: &kestrel_mir::TyArena, ty: TyId, out: &mut Vec<Entity>) {
    match arena.get(ty) {
        MirTy::TypeParam(p) => {
            if !out.contains(p) {
                out.push(*p);
            }
        },
        MirTy::Named { type_args, .. } => {
            for &a in type_args {
                collect_type_params(arena, a, out);
            }
        },
        MirTy::RawExistential { bounds, bindings } => {
            for (_, args) in bounds {
                for &a in args {
                    collect_type_params(arena, a, out);
                }
            }
            for (_, t) in bindings {
                collect_type_params(arena, *t, out);
            }
        },
        MirTy::Pointer(p) | MirTy::Ref { pointee: p, .. } => collect_type_params(arena, *p, out),
        MirTy::Tuple(elems) => {
            for &e in elems {
                collect_type_params(arena, e, out);
            }
        },
        _ => {},
    }
}

/// `fn requirement(self: B[rawany P], args…) -> R` forwarding through the
/// payload's table. `None` when the box lacks the accessor it needs.
fn forwarder(
    ctx: &mut LowerCtx,
    box_ty: TyId,
    boxed: Entity,
    protocol: Entity,
    requirement: Entity,
    type_params: &[Entity],
) -> Option<Entity> {
    use kestrel_hir::body::HirBody;
    use kestrel_type_infer::captures::ClosureCaptureMap;
    let callable = ctx.world.get::<Callable>(requirement).cloned()?;
    let receiver = callable
        .receiver
        .as_ref()
        .map(receiver_convention)
        .unwrap_or(ParamConvention::Borrow);
    // A consuming requirement goes through `ConsumingIndirection`
    // (existentials plan Phase 10), not a borrow of the payload.
    if receiver == ParamConvention::Consuming {
        return None;
    }
    let accessor = if receiver == ParamConvention::MutBorrow {
        "pointeeMutRef"
    } else {
        "pointeeRef"
    };
    let peel = ctx
        .query
        .query(kestrel_name_res::TypeMembersByName {
            type_entity: boxed,
            name: accessor.to_string(),
            context: boxed,
            root: ctx.root,
        })
        .first()?
        .entity;
    let raw = match ctx.module.ty_arena.get(box_ty) {
        MirTy::Named { type_args, .. } => *type_args.first()?,
        _ => return None,
    };
    // `Self` in the requirement is the box.
    let mut subst = kestrel_mir::substitute::SubstMap::new();
    subst.type_params.insert(protocol, box_ty);
    let ret = crate::ty::resolve_callable_return_type(ctx, requirement);
    let ret = kestrel_mir::substitute::substitute(&mut ctx.module.ty_arena, ret, &subst);
    let arg_tys: Vec<TyId> = crate::ty::resolve_callable_types(ctx, requirement)
        .into_iter()
        .map(|t| {
            let t = t.unwrap_or_else(|| ctx.module.ty_arena.error());
            kestrel_mir::substitute::substitute(&mut ctx.module.ty_arena, t, &subst)
        })
        .collect();

    let entity = ctx.next_synthetic_entity();
    let name = format!("{}.__existential", ctx.register_name(requirement));
    ctx.module.register_name(entity, name.clone());
    ctx.register_name(peel);
    let mut func = FunctionDef::new(entity, &name, ret);
    func.kind = FunctionKind::Free;
    func.type_params = type_params
        .iter()
        .map(|&p| {
            TypeParamDef::new(
                p,
                ctx.module.entity_names.get(&p).cloned().unwrap_or_default(),
            )
        })
        .collect();
    func.params
        .push(ParamDef::new("self", ValueId::new(0), box_ty, receiver));
    let mut convs = vec![receiver];
    for (i, p) in callable.params.iter().enumerate() {
        let conv = param_convention(p, false);
        convs.push(conv);
        func.params.push(ParamDef::with_label(
            &p.name,
            ValueId::new(i + 1),
            arg_tys[i],
            conv,
            p.label.clone(),
        ));
    }
    ctx.module.add_function(func);

    let empty = HirBody::empty();
    let captures = std::sync::Arc::new(ClosureCaptureMap::default());
    let mut b = crate::body::OssaBodyCtx::new(ctx, &empty, None, captures, entity, false);
    let entry = b.new_block();
    b.body.entry = entry;
    b.current_block = Some(entry);
    b.push_scope();
    b.body.param_count = convs.len();
    let mut values = Vec::new();
    for (i, &conv) in convs.iter().enumerate() {
        let ty = if i == 0 { box_ty } else { arg_tys[i - 1] };
        let ownership = match conv {
            ParamConvention::Consuming => Ownership::Owned,
            _ => Ownership::Guaranteed,
        };
        values.push(b.body.alloc_value(ValueDef {
            ty,
            ownership,
            borrow_source: None,
            root: RootProvenance::Param(i as u32),
            span: None,
        }));
    }
    let view = b.emit_call_returning(
        Callee::direct_with_args(peel, vec![raw], None),
        vec![CallArg {
            value: values[0],
            convention: receiver,
        }],
        raw,
    );
    let mut args = vec![CallArg {
        value: view,
        convention: receiver,
    }];
    args.extend(values[1..].iter().zip(&convs[1..]).map(|(&v, &c)| CallArg {
        value: v,
        convention: c,
    }));
    let result = b.emit_dynamic_call(requirement, raw, args, ret)?;
    b.emit_ret(result);
    let body = b.body;
    if let Some(f) = ctx.module.functions.get_mut(&entity) {
        f.body = Some(body);
    }
    Some(entity)
}
