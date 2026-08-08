//! The owning closure tiers: a heap-boxed environment, shared or unique.
//!
//! - **`escaping`** — the `@builtin(.SharedBox)` binding (`RcBox` today,
//!   swappable — docs/design/shared-box.md): many handles, many calls, the
//!   last release destroys the captures.
//! - **`consuming`** — the `@builtin(.UniqueBox)` binding: ONE owner, ONE
//!   call. The call takes the environment out of the box into the callee's
//!   frame (where per-slot partial-move machinery applies) and the release
//!   shim reclaims the block; a closure that is never called releases with
//!   the environment still inside, so every capture drops exactly once either
//!   way. The shared box cannot serve here: its release drops the WHOLE
//!   payload, double-freeing slots the one-shot body moved out (plan D5).
//!
//! The compiler emits NO retain/release/allocate sequence of its own: it
//! instantiates the binding at the synthesized environment struct `E` and
//! touches it only through the binding's requirements, so a single lang-item
//! swap retargets every boxing site.
//!
//! Value shape (4 words, `passes/layout.rs` + both codegen `ty.rs`):
//! `{fn_ptr, env_handle, retain_fn, release_fn}`. The two shim pointers are
//! per-`E` monomorphized functions whose bodies are ORDINARY MIR calling the
//! box API; only the dispatch (load the word, call it indirectly) is
//! compiler-emitted, in `mono/expand.rs`. That is what lets a type-erased
//! `escaping (…) -> Int64` value clone and drop without naming `E`.
//!
//! Capture-free escaping values carry a null handle and a shared NO-OP shim
//! pair, so they never allocate and the erased dispatch needs no null check
//! (post-mono block splitting is not available in `expand`). The plan's
//! "null/nop shims" wording covers this — see the E2a report.

use kestrel_ast_builder::{Callable, NodeKind};
use kestrel_hecs::Entity;
use kestrel_mir::body::OssaBody;
use kestrel_mir::callee::Callee;
use kestrel_mir::inst::{CallArg, InstKind, Instruction};
use kestrel_mir::item::function::{FunctionDef, FunctionKind, ParamDef};
use kestrel_mir::terminator::{Terminator, TerminatorKind};
use kestrel_mir::value::{Ownership, RootProvenance, ValueDef};
use kestrel_mir::{FieldIdx, Immediate, MirTy, Op, ParamConvention, TyId, ValueId};

use super::OssaBodyCtx;

/// The resolved `@builtin(.SharedBox)` binding, specialized at one payload
/// type. Every member is found BY REQUIREMENT NAME on the binding entity —
/// never by a hard-coded `RcBox` reference (plan D9).
pub(crate) struct BoxBinding {
    /// `Named{binding, [payload]}` — the handle type.
    pub handle_ty: TyId,
    /// Single-field wrapper types from the handle down to the raw machine
    /// pointer, OUTERMOST FIRST — e.g. `[RcBox[E], std.memory.Pointer[Sto]]`.
    /// A handle is required to be a chain of single-field wrappers ending in a
    /// pointer (shared-box.md "Constraints on a Box Implementation"); anything
    /// else has no one-word representation and the binding is rejected.
    pub wrappers: Vec<TyId>,
    /// The raw machine pointer at the bottom of `wrappers`. This — NOT the
    /// handle struct, which codegen addresses by reference — is the word
    /// packed into the closure value.
    pub raw_ty: TyId,
    /// `init(consuming value: Target)`.
    pub init: Entity,
    /// SHARED box only: `sharedMutRef() -> &mutating Target` — borrow the
    /// payload in place.
    pub shared_mut_ref: Option<Entity>,
    /// UNIQUE box only: `takeValue() -> T` — move the payload out, leaving the
    /// (now empty) allocation for `destroy` to reclaim.
    pub take_value: Option<Entity>,
    /// UNIQUE box only: `destroy()` — drop any still-present payload and free
    /// the allocation. The single reclamation point of a unique box.
    pub destroy: Option<Entity>,
    /// Type args for a member call on the binding: `[payload]`.
    pub type_args: Vec<TyId>,
}

impl OssaBodyCtx<'_, '_> {
    /// Resolve the shared-box binding at `payload_ty`, or `None` when the
    /// stdlib supplies no `@builtin(.SharedBox)` type (a `stdlib: false` test):
    /// the caller then falls back to the historical stack environment.
    pub(crate) fn resolve_box_binding(&mut self, payload_ty: TyId) -> Option<BoxBinding> {
        let mut b = self.resolve_box_common(kestrel_hir::Builtin::SharedBox, payload_ty)?;
        let entity = match self.ctx.module.ty_arena.get(b.handle_ty) {
            MirTy::Named { entity, .. } => *entity,
            _ => return None,
        };
        let shared_mut_ref = self.find_box_member(entity, NodeKind::Function, |c, name| {
            name == "sharedMutRef" && c.params.is_empty()
        })?;
        self.ctx.register_name(shared_mut_ref);
        b.shared_mut_ref = Some(shared_mut_ref);
        Some(b)
    }

    /// Resolve the `@builtin(.UniqueBox)` binding at `payload_ty` — the
    /// `consuming` tier's container. Same shape as [`Self::resolve_box_binding`],
    /// different requirements: a unique box is taken from (once) or destroyed
    /// (once), never shared.
    pub(crate) fn resolve_unique_box_binding(&mut self, payload_ty: TyId) -> Option<BoxBinding> {
        let mut b = self.resolve_box_common(kestrel_hir::Builtin::UniqueBox, payload_ty)?;
        let entity = match self.ctx.module.ty_arena.get(b.handle_ty) {
            MirTy::Named { entity, .. } => *entity,
            _ => return None,
        };
        let take_value = self.find_box_member(entity, NodeKind::Function, |c, name| {
            name == "takeValue" && c.params.is_empty()
        })?;
        let destroy = self.find_box_member(entity, NodeKind::Function, |c, name| {
            name == "destroy" && c.params.is_empty()
        })?;
        self.ctx.register_name(take_value);
        self.ctx.register_name(destroy);
        b.take_value = Some(take_value);
        b.destroy = Some(destroy);
        Some(b)
    }

    /// The part every box binding shares: resolve the entity, intern the
    /// handle type, peel it to its raw pointer word and find `init`.
    fn resolve_box_common(
        &mut self,
        builtin: kestrel_hir::Builtin,
        payload_ty: TyId,
    ) -> Option<BoxBinding> {
        let entity = self.ctx.query.query(kestrel_name_res::ResolveBuiltin {
            builtin,
            root: self.ctx.root,
        })?;
        self.ctx.register_name(entity);

        let type_args = vec![payload_ty];
        let handle_ty = self.ctx.module.ty_arena.named(entity, type_args.clone());
        let (wrappers, raw_ty) = self.unwrap_handle_to_pointer(handle_ty)?;

        let init =
            self.find_box_member(entity, NodeKind::Initializer, |c, _| c.params.len() == 1)?;
        self.ctx.register_name(init);

        Some(BoxBinding {
            handle_ty,
            wrappers,
            raw_ty,
            init,
            shared_mut_ref: None,
            take_value: None,
            destroy: None,
            type_args,
        })
    }

    /// Peel single-field wrapper structs off `handle_ty` until a raw
    /// `MirTy::Pointer` is reached, returning `(wrappers outermost-first, raw
    /// pointer type)`. `RcBox[E]` → `std.memory.Pointer[RcBoxStorage[E]]` →
    /// `MirTy::Pointer(RcBoxStorage[E])`.
    fn unwrap_handle_to_pointer(&mut self, handle_ty: TyId) -> Option<(Vec<TyId>, TyId)> {
        let mut wrappers = Vec::new();
        let mut cur = handle_ty;
        // Small bound: a handle is a thin wrapper, not a deep tower.
        for _ in 0..4 {
            match self.ctx.module.ty_arena.get(cur) {
                MirTy::Pointer(_) => return Some((wrappers, cur)),
                MirTy::Named { entity, type_args } => {
                    let (entity, type_args): (kestrel_hecs::Entity, Vec<TyId>) =
                        (*entity, type_args.clone());
                    let sdef = self.ctx.module.structs.get(&entity)?;
                    if sdef.fields.len() != 1 {
                        return None;
                    }
                    let (field_ty, tps) = (sdef.fields[0].ty, sdef.type_params.clone());
                    let mut subst = kestrel_mir::SubstMap::new();
                    for (tp, &arg) in tps.iter().zip(type_args.iter()) {
                        subst.type_params.insert(tp.entity, arg);
                    }
                    wrappers.push(cur);
                    cur = kestrel_mir::substitute(&mut self.ctx.module.ty_arena, field_ty, &subst);
                },
                _ => return None,
            }
        }
        None
    }

    /// Find a member of the binding by requirement shape, searching the type
    /// entity itself and every extension of it (`sharedMutRef` and the
    /// `SharedBox` conformance live in an extension).
    fn find_box_member(
        &mut self,
        entity: Entity,
        kind: NodeKind,
        pred: impl Fn(&Callable, &str) -> bool,
    ) -> Option<Entity> {
        let mut parents = vec![entity];
        parents.extend(
            self.ctx
                .query
                .query(kestrel_name_res::extensions::ExtensionsFor {
                    target: entity,
                    root: self.ctx.root,
                }),
        );
        for parent in parents {
            for &child in self.ctx.query.children_of(parent).iter() {
                if self.ctx.query.get::<NodeKind>(child) != Some(&kind) {
                    continue;
                }
                let name = self
                    .ctx
                    .query
                    .get::<kestrel_ast_builder::Name>(child)
                    .map(|n| n.0.clone())
                    .unwrap_or_default();
                if self
                    .ctx
                    .query
                    .get::<Callable>(child)
                    .is_some_and(|c| pred(c, &name))
                {
                    return Some(child);
                }
            }
        }
        None
    }

    /// Move `env` (an @owned environment struct) into managed storage and
    /// return the RAW HANDLE WORD — the pointer that becomes word 1 of the
    /// closure value.
    ///
    /// `DestructureStruct` is what transfers the handle OUT of the linear
    /// ownership system: it consumes the box value and yields its POD pointer
    /// field, so no release runs here. The closure value now owns that +1, and
    /// the release shim reconstitutes a handle from the same word to give it
    /// back.
    pub(crate) fn emit_box_env(&mut self, binding: &BoxBinding, env: ValueId) -> ValueId {
        // `Box(consuming: env)` — the `emit_init_literal_call` shape, but the
        // payload argument is CONSUMING (the box takes ownership).
        let ptr_ty = self.ctx.module.ty_arena.pointer(binding.handle_ty);
        let one = self.emit_literal(Immediate::i64(1));
        let self_ptr = self.emit_op1(Op::StackAlloc(binding.handle_ty), one, ptr_ty);
        let payload = self.prepare_call_arg(env, ParamConvention::Consuming);
        let callee = Callee::direct_with_args(binding.init, binding.type_args.clone(), None);
        self.emit_call_void(
            callee,
            vec![
                CallArg {
                    value: self_ptr,
                    convention: ParamConvention::MutBorrow,
                },
                payload,
            ],
        );
        let box_val = self.emit_take(self_ptr, binding.handle_ty);
        self.emit_forget_handle(binding, box_val)
    }

    /// Consume a handle value without releasing it, yielding its raw pointer
    /// word. See [`Self::emit_box_env`] for why this is the right primitive.
    pub(crate) fn emit_forget_handle(&mut self, binding: &BoxBinding, handle: ValueId) -> ValueId {
        let mut cur = handle;
        for i in 0..binding.wrappers.len() {
            let inner_ty = binding
                .wrappers
                .get(i + 1)
                .copied()
                .unwrap_or(binding.raw_ty);
            let inner = self.alloc_value(inner_ty, Ownership::Owned);
            self.consume(cur);
            self.push_inst(InstKind::DestructureStruct {
                results: vec![inner],
                operand: cur,
            });
            self.track_owned(inner);
            cur = inner;
        }
        cur
    }

    /// Rebuild a handle value from a raw pointer word (the inverse of
    /// [`Self::emit_forget_handle`]). The result is @owned but carries NO new
    /// reference — the caller must either forget it again or destroy it
    /// exactly once against the +1 the word already stands for.
    pub(crate) fn emit_handle_from_raw(&mut self, binding: &BoxBinding, raw: ValueId) -> ValueId {
        let mut cur = raw;
        for &wrapper in binding.wrappers.iter().rev() {
            cur = self.emit_struct(wrapper, vec![(FieldIdx::new(0), cur)]);
        }
        cur
    }

    /// `Pointer[E]` to the payload inside the shared storage, borrowed in
    /// place through `sharedMutRef()` — the interior-mutability primitive the
    /// design names. NEVER consumes the handle: one call must not release a
    /// multi-call environment (plan D5, "Escaping env parameter ownership").
    pub(crate) fn emit_box_payload_ptr(
        &mut self,
        binding: &BoxBinding,
        raw: ValueId,
        payload_ty: TyId,
    ) -> ValueId {
        let handle = self.emit_handle_from_raw(binding, raw);
        let borrow = self.emit_begin_borrow(handle);
        let callee = Callee::direct_with_args(
            binding
                .shared_mut_ref
                .expect("ICE: shared-box binding without `sharedMutRef`"),
            binding.type_args.clone(),
            Some(binding.handle_ty),
        );
        let payload_ref = self.emit_call_returning(
            callee,
            vec![CallArg {
                value: borrow,
                convention: ParamConvention::Borrow,
            }],
            payload_ty,
        );
        let ptr_ty = self.ctx.module.ty_arena.pointer(payload_ty);
        let payload_ptr = self.emit_op1(Op::RefToPtr, payload_ref, ptr_ty);
        // END THE REF HERE. `sharedMutRef()` is a ret-borrow call, so its
        // result is a scope-TRACKED single-use reference; `RefToPtr` has
        // already turned it into a raw `Pointer[E]`, which is what every
        // capture address is derived from, so nothing downstream reads the ref
        // itself. Leaving it tracked makes it live at the FIRST terminator of
        // the body — so any `if`/`match` in an escaping closure that reads a
        // capture reported a false E497 ("a reference cannot stay live across
        // a control-flow merge"). The prologue must leave NO tracked ref
        // behind: the pointer it yields is not a reference and crosses merges
        // freely. Ended inside-out (payload before the handle borrow it is
        // rooted at).
        self.end_ref_if_single_use(payload_ref);
        self.emit_end_borrow(borrow);
        // Give the conjured handle back to the word it came from.
        let forgotten = self.emit_forget_handle(binding, handle);
        self.emit_destroy_value(forgotten);
        payload_ptr
    }

    /// Move the payload OUT of a unique box, returning the @owned value. The
    /// allocation stays alive and marked empty; the closure value's release
    /// shim (`destroy`) reclaims it afterwards, so every unique box is freed
    /// exactly once whether or not it was ever called.
    ///
    /// This is the `consuming` tier's prologue: the environment lands in the
    /// body's own frame, where the ORDINARY partial-move machinery gives each
    /// capture slot its own drop state (plan D5, "per-slot drop flags in the
    /// body frame").
    pub(crate) fn emit_unique_take(
        &mut self,
        binding: &BoxBinding,
        raw: ValueId,
        payload_ty: TyId,
    ) -> ValueId {
        let handle = self.emit_handle_from_raw(binding, raw);
        let borrow = self.emit_begin_borrow(handle);
        let callee = Callee::direct_with_args(
            binding
                .take_value
                .expect("ICE: unique-box binding without `takeValue`"),
            binding.type_args.clone(),
            Some(binding.handle_ty),
        );
        let payload = self.emit_call_returning(
            callee,
            vec![CallArg {
                value: borrow,
                convention: ParamConvention::Borrow,
            }],
            payload_ty,
        );
        self.emit_end_borrow(borrow);
        // Give the conjured handle back to the word it came from — the closure
        // VALUE still owns the (now empty) allocation.
        let forgotten = self.emit_forget_handle(binding, handle);
        self.emit_destroy_value(forgotten);
        payload
    }
}

// ===========================================================================
// Per-environment retain / release shims
// ===========================================================================

/// Synthesize the type-erased share/release pair for one environment type.
///
/// Both take the raw handle word and are ordinary MIR: `retain` conjures a
/// handle, `CopyValue`s it (the ordinary clone machinery → the box's
/// `clone()`) and forgets both, netting one extra reference; `release`
/// conjures a handle and destroys it (drop shim → `deinit` → release).
///
/// Returns `(retain, release)` callees ready for `ApplyPartial`. They are new
/// mono roots — `mono/collect.rs` scans them off the instruction.
pub(crate) fn synthesize_env_shims(
    ctx: &mut crate::LowerCtx<'_>,
    binding: &BoxBinding,
    payload_ty: TyId,
    base_name: &str,
    type_params: Vec<kestrel_mir::TypeParamDef>,
    type_args: Vec<TyId>,
) -> (Callee, Callee) {
    let retain = build_shim(ctx, binding, payload_ty, base_name, &type_params, true);
    let release = build_shim(ctx, binding, payload_ty, base_name, &type_params, false);
    (
        Callee::direct_with_args(retain, type_args.clone(), None),
        Callee::direct_with_args(release, type_args, None),
    )
}

/// The `consuming` tier's shim pair: a NO-OP retain (a unique one-shot owner
/// is `not Copyable`, so no share path may ever be reachable — the word exists
/// only to keep the 4-word owning layout single-sourced through
/// `func_thick_words`) and a real release that calls the unique box's
/// `destroy()`.
///
/// `destroy()` is the box's single reclamation point: it drops the payload
/// when the closure was never called and skips it when the call already took
/// the environment out, then frees the block either way.
pub(crate) fn synthesize_unique_env_shims(
    ctx: &mut crate::LowerCtx<'_>,
    binding: &BoxBinding,
    base_name: &str,
    type_params: Vec<kestrel_mir::TypeParamDef>,
    type_args: Vec<TyId>,
) -> (Callee, Callee) {
    let (retain, _) = nop_shims(ctx);
    let release = build_unique_release_shim(ctx, binding, base_name, &type_params);
    (retain, Callee::direct_with_args(release, type_args, None))
}

fn build_unique_release_shim(
    ctx: &mut crate::LowerCtx<'_>,
    binding: &BoxBinding,
    base_name: &str,
    type_params: &[kestrel_mir::TypeParamDef],
) -> Entity {
    let entity = ctx.next_synthetic_entity();
    let name = format!("{base_name}.release");
    ctx.module.register_name(entity, &name);

    let unit_ty = ctx.module.ty_arena.unit();
    let mut body = OssaBody::new();

    let raw_param = body.alloc_value(
        ValueDef::owned(binding.raw_ty).with_root(kestrel_mir::value::RootProvenance::Param(0)),
    );
    body.param_count = 1;
    let entry = body.alloc_block();
    body.entry = entry;

    let mut insts: Vec<Instruction> = Vec::new();
    let owned = |body: &mut OssaBody, ty: TyId| body.alloc_value(ValueDef::owned(ty));

    // Reconstitute the handle from the word, then `destroy()` it.
    let mut cur = raw_param;
    for &wrapper in binding.wrappers.iter().rev() {
        let outer = body.alloc_value(ValueDef::owned(wrapper));
        insts.push(Instruction::new(InstKind::Struct {
            result: outer,
            ty: wrapper,
            fields: vec![(FieldIdx::new(0), cur)],
        }));
        cur = outer;
    }
    let box_val = cur;
    let borrow = body.alloc_value(ValueDef::guaranteed(binding.handle_ty, box_val));
    insts.push(Instruction::new(InstKind::BeginBorrow {
        result: borrow,
        operand: box_val,
    }));
    insts.push(Instruction::new(InstKind::Call {
        result: None,
        callee: Callee::direct_with_args(
            binding
                .destroy
                .expect("ICE: unique-box binding without `destroy`"),
            binding.type_args.clone(),
            Some(binding.handle_ty),
        ),
        args: vec![CallArg {
            value: borrow,
            convention: ParamConvention::Borrow,
        }],
    }));
    insts.push(Instruction::new(InstKind::EndBorrow { operand: borrow }));
    // The handle itself owns nothing once `destroy()` has run — forget it back
    // down to the POD word rather than destroying a dangling box value.
    let mut cur = box_val;
    for i in 0..binding.wrappers.len() {
        let inner_ty = binding
            .wrappers
            .get(i + 1)
            .copied()
            .unwrap_or(binding.raw_ty);
        let inner = owned(&mut body, inner_ty);
        insts.push(Instruction::new(InstKind::DestructureStruct {
            results: vec![inner],
            operand: cur,
        }));
        cur = inner;
    }
    insts.push(Instruction::new(InstKind::DestroyValue { operand: cur }));

    let unit_val = owned(&mut body, unit_ty);
    insts.push(Instruction::new(InstKind::Literal {
        result: unit_val,
        value: Immediate::unit(),
    }));
    body.block_mut(entry).insts = insts;
    body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Return(unit_val));

    let mut func = FunctionDef::new(entity, &name, unit_ty);
    func.type_params = type_params.to_vec();
    func.kind = FunctionKind::Free;
    func.params.push(ParamDef::new(
        "handle",
        raw_param,
        binding.raw_ty,
        ParamConvention::Consuming,
    ));
    func.body = Some(body);
    ctx.module.add_function(func);
    entity
}

fn build_shim(
    ctx: &mut crate::LowerCtx<'_>,
    binding: &BoxBinding,
    _payload_ty: TyId,
    base_name: &str,
    type_params: &[kestrel_mir::TypeParamDef],
    is_retain: bool,
) -> Entity {
    let entity = ctx.next_synthetic_entity();
    let name = format!(
        "{base_name}.{}",
        if is_retain { "retain" } else { "release" }
    );
    ctx.module.register_name(entity, &name);

    let unit_ty = ctx.module.ty_arena.unit();
    let mut body = OssaBody::new();

    // The raw handle word arrives by value (a POD pointer).
    let raw_param = body.alloc_value(
        ValueDef::owned(binding.raw_ty).with_root(kestrel_mir::value::RootProvenance::Param(0)),
    );
    body.param_count = 1;
    let entry = body.alloc_block();
    body.entry = entry;

    let mut insts: Vec<Instruction> = Vec::new();
    let owned = |body: &mut OssaBody, ty: TyId| body.alloc_value(ValueDef::owned(ty));

    // %box = Handle { … { raw } } — conjured, standing for the reference the
    // word already holds.
    let wrap = |body: &mut OssaBody, insts: &mut Vec<Instruction>, raw: ValueId| {
        let mut cur = raw;
        for &wrapper in binding.wrappers.iter().rev() {
            let outer = body.alloc_value(ValueDef::owned(wrapper));
            insts.push(Instruction::new(InstKind::Struct {
                result: outer,
                ty: wrapper,
                fields: vec![(FieldIdx::new(0), cur)],
            }));
            cur = outer;
        }
        cur
    };
    let box_val = wrap(&mut body, &mut insts, raw_param);

    if is_retain {
        // Share: CopyValue routes through the ordinary clone machinery (the
        // box's own `clone()`), so the refcount discipline stays in Kestrel.
        let borrow = body.alloc_value(ValueDef::guaranteed(binding.handle_ty, box_val));
        insts.push(Instruction::new(InstKind::BeginBorrow {
            result: borrow,
            operand: box_val,
        }));
        let extra = owned(&mut body, binding.handle_ty);
        insts.push(Instruction::new(InstKind::CopyValue {
            result: extra,
            operand: borrow,
        }));
        insts.push(Instruction::new(InstKind::EndBorrow { operand: borrow }));
        // Forget BOTH handles: the conjured one never owned anything, and the
        // fresh one IS the extra reference we are handing to the copy.
        for h in [box_val, extra] {
            let mut cur = h;
            for i in 0..binding.wrappers.len() {
                let inner_ty = binding
                    .wrappers
                    .get(i + 1)
                    .copied()
                    .unwrap_or(binding.raw_ty);
                let inner = owned(&mut body, inner_ty);
                insts.push(Instruction::new(InstKind::DestructureStruct {
                    results: vec![inner],
                    operand: cur,
                }));
                cur = inner;
            }
            insts.push(Instruction::new(InstKind::DestroyValue { operand: cur }));
        }
    } else {
        // Release: hand the word's reference back. The drop shim runs the
        // box's `deinit`, which destroys the payload at the last release —
        // "captures released exactly once" falls out of ordinary drop rules.
        insts.push(Instruction::new(InstKind::DestroyValue {
            operand: box_val,
        }));
    }

    let unit_val = owned(&mut body, unit_ty);
    insts.push(Instruction::new(InstKind::Literal {
        result: unit_val,
        value: Immediate::unit(),
    }));
    body.block_mut(entry).insts = insts;
    body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Return(unit_val));

    let mut func = FunctionDef::new(entity, &name, unit_ty);
    func.type_params = type_params.to_vec();
    func.kind = FunctionKind::Free;
    func.params.push(ParamDef::new(
        "handle",
        raw_param,
        binding.raw_ty,
        ParamConvention::Consuming,
    ));
    func.body = Some(body);
    ctx.module.add_function(func);
    entity
}

/// The shared no-op shim used by capture-free escaping values (null handle,
/// nothing to share or release). Synthesized once per module.
pub(crate) fn nop_shims(ctx: &mut crate::LowerCtx<'_>) -> (Callee, Callee) {
    if let Some(e) = ctx.escaping_nop_shim {
        return (
            Callee::direct_with_args(e, Vec::new(), None),
            Callee::direct_with_args(e, Vec::new(), None),
        );
    }
    let entity = {
        let e = ctx.next_synthetic_entity();
        let name = "__closure.escaping.nop";
        ctx.module.register_name(e, name);
        let unit_ty = ctx.module.ty_arena.unit();
        let handle_ty = ctx.module.ty_arena.pointer(unit_ty);

        let mut body = OssaBody::new();
        let param =
            body.alloc_value(ValueDef::owned(handle_ty).with_root(RootProvenance::Param(0)));
        body.param_count = 1;
        let entry = body.alloc_block();
        body.entry = entry;
        let unit_val = body.alloc_value(ValueDef::owned(unit_ty));
        body.block_mut(entry).insts = vec![Instruction::new(InstKind::Literal {
            result: unit_val,
            value: Immediate::unit(),
        })];
        body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Return(unit_val));

        let mut func = FunctionDef::new(e, name, unit_ty);
        func.kind = FunctionKind::Free;
        func.params.push(ParamDef::new(
            "handle",
            param,
            handle_ty,
            ParamConvention::Consuming,
        ));
        func.body = Some(body);
        ctx.module.add_function(func);
        e
    };
    ctx.escaping_nop_shim = Some(entity);
    (
        Callee::direct_with_args(entity, Vec::new(), None),
        Callee::direct_with_args(entity, Vec::new(), None),
    )
}
