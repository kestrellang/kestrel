//! The owning closure tiers: a heap-boxed environment, shared or single-owner.
//!
//! - **`escaping`** — the `@builtin(.ClosureBox)` binding (`Shared`, i.e.
//!   `RcBox`, today — swappable in `std.memory.box_bindings`,
//!   docs/design/shared-box.md): many handles, many calls, the
//!   last release destroys the captures.
//! - **`consuming`** — the `@builtin(.ConsumingClosureBox)` binding
//!   (`Owned`, i.e. `OwnedBox`, today): ONE owner, ONE call. The box holds
//!   `Optional[E]`; the call swaps it for `.None` through
//!   `MutableIndirection.valueMutRef()` and runs on `E` in its own frame
//!   (where per-slot partial-move machinery applies). The release shim is the
//!   box's ordinary drop, so a closure that is never called drops `E` there
//!   and one that was called drops `.None`: every capture drops exactly once
//!   either way ([`OnceSlot`]).
//!
//! The compiler emits NO retain/release/allocate sequence of its own: it
//! instantiates the binding at the synthesized environment struct `E` and
//! touches it only through the binding's requirements, so editing the
//! binding alias retargets every boxing site.
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

use kestrel_ast_builder::Callable;
use kestrel_hecs::Entity;
use kestrel_hir::Builtin;
use kestrel_mir::body::OssaBody;
use kestrel_mir::callee::Callee;
use kestrel_mir::inst::{CallArg, InstKind, Instruction};
use kestrel_mir::item::function::{FunctionDef, FunctionKind, ParamDef};
use kestrel_mir::item::witness::WitnessMethodKey;
use kestrel_mir::terminator::{Terminator, TerminatorKind};
use kestrel_mir::value::{Ownership, RootProvenance, ValueDef};
use kestrel_mir::{FieldIdx, Immediate, MirTy, Op, ParamConvention, TyId, ValueId, VariantIdx};

use super::OssaBodyCtx;

/// One wrapper struct between a box handle and its raw pointer word.
#[derive(Clone)]
pub(crate) struct HandleLayer {
    /// The wrapper struct type.
    pub ty: TyId,
    /// The field leading down to the pointer.
    pub ptr_field: FieldIdx,
    /// Every field's type, substituted at `ty`. All but `ptr_field` are empty
    /// structs: dropped when the handle is forgotten down to its word, and
    /// rebuilt (they hold nothing) when the word becomes a handle again.
    pub fields: Vec<TyId>,
}

impl HandleLayer {
    /// The type one step further down: the next layer, or the raw pointer.
    fn inner_ty(layers: &[HandleLayer], i: usize, raw_ty: TyId) -> TyId {
        layers.get(i + 1).map_or(raw_ty, |l| l.ty)
    }
}

/// `layers` rebuilt around `raw`, innermost first, as plain instructions
/// (for the synthesized shims, which have no `OssaBodyCtx`).
fn wrap_raw_insts(
    body: &mut OssaBody,
    insts: &mut Vec<Instruction>,
    layers: &[HandleLayer],
    raw: ValueId,
) -> ValueId {
    let mut cur = raw;
    for layer in layers.iter().rev() {
        let fields = layer
            .fields
            .iter()
            .enumerate()
            .map(|(i, &ty)| {
                let idx = FieldIdx::new(i);
                if idx == layer.ptr_field {
                    return (idx, cur);
                }
                let empty = body.alloc_value(ValueDef::owned(ty));
                insts.push(Instruction::new(InstKind::Struct {
                    result: empty,
                    ty,
                    fields: vec![],
                }));
                (idx, empty)
            })
            .collect();
        let outer = body.alloc_value(ValueDef::owned(layer.ty));
        insts.push(Instruction::new(InstKind::Struct {
            result: outer,
            ty: layer.ty,
            fields,
        }));
        cur = outer;
    }
    cur
}

/// `handle` taken apart down to its raw word (the inverse of
/// [`wrap_raw_insts`]); the empty side fields are destroyed on the way.
fn forget_handle_insts(
    body: &mut OssaBody,
    insts: &mut Vec<Instruction>,
    layers: &[HandleLayer],
    raw_ty: TyId,
    handle: ValueId,
) -> ValueId {
    let mut cur = handle;
    for (i, layer) in layers.iter().enumerate() {
        let inner_ty = HandleLayer::inner_ty(layers, i, raw_ty);
        let results: Vec<ValueId> = layer
            .fields
            .iter()
            .enumerate()
            .map(|(f, &ty)| {
                let ty = if FieldIdx::new(f) == layer.ptr_field {
                    inner_ty
                } else {
                    ty
                };
                body.alloc_value(ValueDef::owned(ty))
            })
            .collect();
        insts.push(Instruction::new(InstKind::DestructureStruct {
            results: results.clone(),
            operand: cur,
        }));
        for (f, &v) in results.iter().enumerate() {
            if FieldIdx::new(f) != layer.ptr_field {
                insts.push(Instruction::new(InstKind::DestroyValue { operand: v }));
            }
        }
        cur = results[layer.ptr_field.index()];
    }
    cur
}

/// A resolved box binding (`ClosureBox` or `ConsumingClosureBox`), specialized at one payload
/// type. Every member is found BY REQUIREMENT NAME on the binding entity —
/// never by a hard-coded `RcBox` reference (plan D9).
pub(crate) struct BoxBinding {
    /// `Named{binding, [payload]}` — the handle type.
    pub handle_ty: TyId,
    /// Wrapper structs from the handle down to the raw machine pointer,
    /// OUTERMOST FIRST — e.g. `[RcBox[E, A], std.memory.Pointer[Sto]]`.
    /// A handle is required to be a chain of wrappers ending in a pointer,
    /// each holding one field on the way down plus only empty structs (a
    /// stateless allocator) beside it (shared-box.md "Constraints on a Box
    /// Implementation"); anything else has no one-word representation and
    /// the binding is rejected.
    pub wrappers: Vec<HandleLayer>,
    /// The raw machine pointer at the bottom of `wrappers`. This — NOT the
    /// handle struct, which codegen addresses by reference — is the word
    /// packed into the closure value.
    pub raw_ty: TyId,
    /// `init(consuming value: Target)`.
    pub init: Entity,
    /// SHARED box only: `sharedMutRef() -> &mutating Target` — borrow the
    /// payload in place.
    pub shared_mut_ref: Option<Entity>,
    /// CONSUMING box only: the `Optional[E]` the environment is boxed as,
    /// and how the one call reaches it in place.
    pub once: Option<OnceSlot>,
    /// Type args for a member call on the binding: `[payload]`.
    pub type_args: Vec<TyId>,
}

/// How a `consuming` closure's box holds its environment `E`: as
/// `Optional[E]`, so the one call can move `E` out and leave `.None` behind.
/// The box's ordinary drop then frees the block, dropping `E` only if the
/// closure was never called. That is the whole "taken" flag, and it needs
/// nothing from the box beyond `Box` and `MutableIndirection`.
#[derive(Clone)]
pub(crate) struct OnceSlot {
    /// `Optional[E]` — the box's payload type.
    pub opt_ty: TyId,
    /// `.Some` / `.None`, found by their `@builtin` case tags.
    pub some: VariantIdx,
    pub none: VariantIdx,
    /// `MutableIndirection.valueMutRef()`: its protocol and witness key.
    pub protocol: Entity,
    pub value_mut_ref: WitnessMethodKey,
}

/// Does `c` have the box binding's required initializer shape,
/// `init(consuming value: Target)` (`lang/std/memory/box.ks`)?
///
/// Three axes, all load-bearing: arity 1, NO label (the requirement's `value`
/// is a single-name parameter, so call sites are positional) and `consuming`
/// (the box takes ownership of the payload). `RcBox`'s other one-parameter
/// init — `init(inner inner: Pointer[RcBoxStorage[T]])` — fails on both of the
/// last two. `kestrel-analyze`'s conformance-completeness checker already
/// disambiguates exactly this pair by per-param label
/// (`conformance_completeness.rs::signatures_match`), so a shape that could
/// pick the wrong one here would already have failed E454/E458 in the stdlib.
///
/// Pure and ECS-free on purpose — this is the part worth unit-testing.
pub(crate) fn is_box_init_shape(c: &Callable) -> bool {
    matches!(c.params.as_slice(), [p] if p.label.is_none() && p.is_consuming)
}

/// Turn a requirement search into the single answer it must have, panicking
/// loudly otherwise.
///
/// Every caller reaches here only AFTER the box type itself resolved, so
/// "requirement missing" and "requirement ambiguous" both mean the stdlib
/// binding is broken — there is no legitimate `None` left to fall back on.
/// Returning `Option` here is what let a mis-selected member reach codegen as
/// a forged pointer; a `debug_assert!` would not do, because the corruption
/// this guards happens in release builds.
fn exactly_one_box_member<T: Copy>(matches: &[T], requirement: &str) -> T {
    match matches {
        [one] => *one,
        [] => panic!(
            "ICE: box binding is missing its required member `{requirement}` — \
             the `@builtin(.ClosureBox)` box resolved but does not implement \
             the requirement"
        ),
        many => panic!(
            "ICE: box binding has {} members matching `{requirement}` — the \
             requirement must select exactly one; picking arbitrarily silently \
             corrupts every boxed closure environment",
            many.len()
        ),
    }
}

impl OssaBodyCtx<'_, '_> {
    /// Resolve the shared-box binding at `payload_ty`, or `None` when the
    /// stdlib supplies no `@builtin(.ClosureBox)` binding (a `stdlib: false` test):
    /// the caller then falls back to the historical stack environment.
    pub(crate) fn resolve_box_binding(&mut self, payload_ty: TyId) -> Option<BoxBinding> {
        let mut b = self.resolve_box_common(Builtin::ClosureBox, Builtin::BoxInit, payload_ty)?;
        let entity = match self.ctx.module.ty_arena.get(b.handle_ty) {
            MirTy::Named { entity, .. } => *entity,
            _ => return None,
        };
        let shared_mut_ref = self.find_box_member(entity, Builtin::SharedBoxSharedMutRef);
        self.ctx.register_name(shared_mut_ref);
        b.shared_mut_ref = Some(shared_mut_ref);
        Some(b)
    }

    /// Resolve the `@builtin(.ConsumingClosureBox)` binding for environment
    /// `env_ty` — the `consuming` tier's container. The box holds
    /// `Optional[env_ty]` ([`OnceSlot`]); its release is the box's ordinary
    /// drop, so nothing here is specific to one box type.
    pub(crate) fn resolve_consuming_box_binding(&mut self, env_ty: TyId) -> Option<BoxBinding> {
        let resolve = |this: &Self, builtin| {
            this.ctx.query.query(kestrel_name_res::ResolveBuiltin {
                builtin,
                root: this.ctx.root,
            })
        };
        let optional = resolve(self, Builtin::OptionalEnum)?;
        self.ctx.register_name(optional);
        let opt_ty = self.ctx.module.ty_arena.named(optional, vec![env_ty]);
        let mut b =
            self.resolve_box_common(Builtin::ConsumingClosureBox, Builtin::BoxInit, opt_ty)?;
        // Past here the box resolved, so a missing piece is a broken std.
        let some = self.ctx.builtin_variant_idx(Builtin::OptionalSomeCase);
        let none = self.ctx.builtin_variant_idx(Builtin::OptionalNoneCase);
        let value_mut_ref = resolve(self, Builtin::MutableIndirectionValueMutRef);
        let (Some(some), Some(none), Some(value_mut_ref)) = (some, none, value_mut_ref) else {
            panic!(
                "ICE: `@builtin(.ConsumingClosureBox)` resolved but std lacks the \
                 `Optional` case tags or `MutableIndirection.valueMutRef()`"
            );
        };
        let protocol = self
            .ctx
            .is_protocol_method(value_mut_ref)
            .expect("ICE: `valueMutRef` is not a `MutableIndirection` member");
        self.ctx.register_name(protocol);
        b.once = Some(OnceSlot {
            opt_ty,
            some,
            none,
            protocol,
            value_mut_ref: self.ctx.witness_method_key(value_mut_ref),
        });
        Some(b)
    }

    /// The part every box binding shares: resolve the entity, intern the
    /// handle type, peel it to its raw pointer word and find `init`.
    fn resolve_box_common(
        &mut self,
        builtin: Builtin,
        init_tag: Builtin,
        payload_ty: TyId,
    ) -> Option<BoxBinding> {
        let entity = self.ctx.query.query(kestrel_name_res::ResolveBuiltin {
            builtin,
            root: self.ctx.root,
        })?;
        self.ctx.register_name(entity);

        let type_args = crate::ty::with_default_type_args(self.ctx, entity, vec![payload_ty]);
        let handle_ty = self.ctx.module.ty_arena.named(entity, type_args.clone());
        let (wrappers, raw_ty) = self.unwrap_handle_to_pointer(handle_ty)?;

        // The tagged init, never "a one-parameter init": `RcBox` also has a
        // private `init(inner inner: Pointer[Storage[T]])` adopting an
        // already-counted block; picking it stored the payload straight into
        // the handle field as a forged pointer (a silent SIGSEGV at every
        // escaping-closure call).
        let init = self.find_box_member(entity, init_tag);
        self.ctx.register_name(init);

        Some(BoxBinding {
            handle_ty,
            wrappers,
            raw_ty,
            init,
            shared_mut_ref: None,
            once: None,
            type_args,
        })
    }

    /// Peel wrapper structs off `handle_ty` until a raw `MirTy::Pointer` is
    /// reached, returning `(wrappers outermost-first, raw pointer type)`.
    /// `RcBox[E, A]` → `std.memory.Pointer[RcBoxStorage[E]]` →
    /// `MirTy::Pointer(RcBoxStorage[E])`. Each wrapper holds exactly one
    /// non-empty field, the next step down; its other fields must be empty
    /// structs (a stateless allocator), which add nothing to the word.
    pub(crate) fn unwrap_handle_to_pointer(
        &mut self,
        handle_ty: TyId,
    ) -> Option<(Vec<HandleLayer>, TyId)> {
        let mut wrappers = Vec::new();
        let mut cur = handle_ty;
        // Small bound: a handle is a thin wrapper, not a deep tower.
        for _ in 0..4 {
            let fields = match self.ctx.module.ty_arena.get(cur) {
                MirTy::Pointer(_) => return Some((wrappers, cur)),
                MirTy::Named { .. } => self.struct_field_tys_at(cur)?,
                _ => return None,
            };
            let mut non_empty = (0..fields.len()).filter(|&i| !self.is_empty_struct(fields[i]));
            let (Some(ptr), None) = (non_empty.next(), non_empty.next()) else {
                return None;
            };
            wrappers.push(HandleLayer {
                ty: cur,
                ptr_field: FieldIdx::new(ptr),
                fields: fields.clone(),
            });
            cur = fields[ptr];
        }
        None
    }

    /// The field types of struct type `ty`, substituted at its type args.
    fn struct_field_tys_at(&mut self, ty: TyId) -> Option<Vec<TyId>> {
        let MirTy::Named { entity, type_args } = self.ctx.module.ty_arena.get(ty).clone() else {
            return None;
        };
        let sdef = self.ctx.module.structs.get(&entity)?;
        let field_tys: Vec<TyId> = sdef.fields.iter().map(|f| f.ty).collect();
        let mut subst = kestrel_mir::SubstMap::new();
        for (tp, &arg) in sdef.type_params.iter().zip(type_args.iter()) {
            subst.type_params.insert(tp.entity, arg);
        }
        Some(
            field_tys
                .into_iter()
                .map(|f| kestrel_mir::substitute(&mut self.ctx.module.ty_arena, f, &subst))
                .collect(),
        )
    }

    /// A struct with no fields (`SystemAllocator`): no bytes, nothing owned.
    fn is_empty_struct(&self, ty: TyId) -> bool {
        match self.ctx.module.ty_arena.get(ty) {
            MirTy::Named { entity, .. } => self
                .ctx
                .module
                .structs
                .get(entity)
                .is_some_and(|s| s.fields.is_empty()),
            _ => false,
        }
    }

    /// The binding's member for the `@builtin`-tagged `tag`: the tagged
    /// entity itself when it sits on the binding (or an extension of it),
    /// else the ONE binding member carrying the tagged
    /// PROTOCOL requirement's witness key (`SharedBox.sharedMutRef`, whose
    /// implementation lives on whichever type is the swappable binding).
    ///
    /// Infallible by construction: by the time this runs the box TYPE is
    /// already resolved (the `ResolveBuiltin` query succeeded and the handle
    /// peeled to a raw pointer), so a missing tag or a missing / duplicated
    /// member is a broken stdlib, never the legitimate "no box available"
    /// case. See [`exactly_one_box_member`].
    pub(crate) fn find_box_member(&mut self, entity: Entity, tag: Builtin) -> Entity {
        let what = tag.name();
        let Some(requirement) = self.ctx.query.query(kestrel_name_res::ResolveBuiltin {
            builtin: tag,
            root: self.ctx.root,
        }) else {
            return exactly_one_box_member::<Entity>(&[], what);
        };
        // Collect ALL matches rather than taking the first: "first hit wins"
        // is what made the selection depend on declaration order (F4). The
        // tag sitting on the binding itself is its own (sole) witness.
        let matches = self.ctx.witness_candidates(entity, requirement);
        if matches.contains(&requirement) {
            return requirement;
        }
        exactly_one_box_member(&matches, what)
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
        // The `CallArg` list below is POSITIONAL and unvalidated: it hands the
        // payload over as the sole non-`self` argument. If the resolved init
        // ever had a different arity the extra/missing slot would be read as
        // garbage, so re-check the shape at the point that depends on it.
        debug_assert!(
            self.ctx
                .query
                .get::<Callable>(binding.init)
                .is_some_and(is_box_init_shape),
            "ICE: resolved box initializer is not `init(consuming value: Target)`; \
             the positional argument list below assumes exactly one payload param"
        );
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
        let layers = &binding.wrappers;
        let mut cur = handle;
        for (i, layer) in layers.iter().enumerate() {
            let inner_ty = HandleLayer::inner_ty(layers, i, binding.raw_ty);
            let results: Vec<ValueId> = layer
                .fields
                .iter()
                .enumerate()
                .map(|(f, &ty)| {
                    let ty = if FieldIdx::new(f) == layer.ptr_field {
                        inner_ty
                    } else {
                        ty
                    };
                    self.alloc_value(ty, Ownership::Owned)
                })
                .collect();
            self.consume(cur);
            self.push_inst(InstKind::DestructureStruct {
                results: results.clone(),
                operand: cur,
            });
            for (f, &v) in results.iter().enumerate() {
                self.track_owned(v);
                if FieldIdx::new(f) != layer.ptr_field {
                    self.emit_destroy_value(v);
                }
            }
            cur = results[layer.ptr_field.index()];
        }
        cur
    }

    /// Rebuild a handle value from a raw pointer word (the inverse of
    /// [`Self::emit_forget_handle`]). The result is @owned but carries NO new
    /// reference — the caller must either forget it again or destroy it
    /// exactly once against the +1 the word already stands for.
    pub(crate) fn emit_handle_from_raw(&mut self, binding: &BoxBinding, raw: ValueId) -> ValueId {
        let mut cur = raw;
        for layer in binding.wrappers.iter().rev() {
            let fields = layer
                .fields
                .iter()
                .enumerate()
                .map(|(i, &ty)| {
                    let idx = FieldIdx::new(i);
                    if idx == layer.ptr_field {
                        (idx, cur)
                    } else {
                        (idx, self.emit_struct(ty, vec![]))
                    }
                })
                .collect();
            cur = self.emit_struct(layer.ty, fields);
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

    /// Wrap a fresh `consuming` environment for its box: `.Some(env)`.
    pub(crate) fn emit_once_payload(&mut self, binding: &BoxBinding, env: ValueId) -> ValueId {
        let once = binding
            .once
            .as_ref()
            .expect("ICE: consuming box without its slot");
        self.emit_enum_variant(once.opt_ty, once.some, vec![env])
    }

    /// Move the environment OUT of a `consuming` closure's box, returning the
    /// @owned `E`. The box keeps `.None` in its place and stays alive: the
    /// closure value's release (the box's ordinary drop) frees it afterwards,
    /// dropping nothing, so every box is freed exactly once whether or not the
    /// closure was ever called.
    ///
    /// This is the `consuming` tier's prologue: the environment lands in the
    /// body's own frame, where the ORDINARY partial-move machinery gives each
    /// capture slot its own drop state (plan D5, "per-slot drop flags in the
    /// body frame").
    pub(crate) fn emit_consuming_take(
        &mut self,
        binding: &BoxBinding,
        raw: ValueId,
        env_ty: TyId,
    ) -> ValueId {
        let once = binding
            .once
            .clone()
            .expect("ICE: consuming box without its slot");
        // `valueMutRef()` is `mutating`: the conjured handle needs an address.
        let handle = self.emit_handle_from_raw(binding, raw);
        let ptr_ty = self.ctx.module.ty_arena.pointer(binding.handle_ty);
        let one = self.emit_literal(Immediate::i64(1));
        let handle_addr = self.emit_op1(Op::StackAlloc(binding.handle_ty), one, ptr_ty);
        self.emit_store_init(handle_addr, handle);
        let callee = Callee::Witness {
            protocol: once.protocol,
            method: once.value_mut_ref,
            self_type: binding.handle_ty,
            method_type_args: vec![],
        };
        let slot_ref = self.emit_call_returning(
            callee,
            vec![CallArg {
                value: handle_addr,
                convention: ParamConvention::MutBorrow,
            }],
            once.opt_ty,
        );
        let opt_ptr_ty = self.ctx.module.ty_arena.pointer(once.opt_ty);
        let slot = self.emit_op1(Op::RefToPtr, slot_ref, opt_ptr_ty);
        // Same reason as `emit_box_payload_ptr`: only the raw pointer is used
        // from here on, so the tracked ref ends now.
        self.end_ref_if_single_use(slot_ref);
        // Swap the payload for `.None`, then unwrap the `.Some` we took.
        let taken = self.emit_take(slot, once.opt_ty);
        let none = self.emit_enum_variant(once.opt_ty, once.none, Vec::new());
        self.emit_store_init(slot, none);
        let env = self.emit_destructure_enum(taken, once.some, &[env_ty])[0];
        // Give the conjured handle back to the word it came from — the closure
        // VALUE still owns the (now `.None`) box.
        let handle = self.emit_take(handle_addr, binding.handle_ty);
        let forgotten = self.emit_forget_handle(binding, handle);
        self.emit_destroy_value(forgotten);
        env
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

/// The `consuming` tier's shim pair: a NO-OP retain (a one-shot owner is
/// `not Copyable`, so no share path may ever be reachable — the word exists
/// only to keep the 4-word owning layout single-sourced through
/// `func_thick_words`) and the box's ordinary release, which drops whatever
/// the `Optional[E]` still holds (`.None` once the closure has been called).
pub(crate) fn synthesize_consuming_env_shims(
    ctx: &mut crate::LowerCtx<'_>,
    binding: &BoxBinding,
    base_name: &str,
    type_params: Vec<kestrel_mir::TypeParamDef>,
    type_args: Vec<TyId>,
) -> (Callee, Callee) {
    let (retain, _) = nop_shims(ctx);
    let release = build_shim(
        ctx,
        binding,
        binding.handle_ty,
        base_name,
        &type_params,
        false,
    );
    (retain, Callee::direct_with_args(release, type_args, None))
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
    let box_val = wrap_raw_insts(&mut body, &mut insts, &binding.wrappers, raw_param);

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
            let word =
                forget_handle_insts(&mut body, &mut insts, &binding.wrappers, binding.raw_ty, h);
            insts.push(Instruction::new(InstKind::DestroyValue { operand: word }));
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

// ===========================================================================
// Requirement-selection tests
// ===========================================================================
//
// These pin the two halves of F4 that are pure: the SHAPE predicate and the
// count→outcome decision. Both are deliberately ECS-free so the order
// experiment ("swap `RcBox`'s two inits") can be pinned here instead of by
// editing the shipped stdlib.

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast_builder::AstParam;

    fn param(label: Option<&str>, is_consuming: bool) -> AstParam {
        AstParam {
            label: label.map(str::to_string),
            name: "value".to_string(),
            ty: None,
            default_entity: None,
            pattern: None,
            is_mut: false,
            is_consuming,
            by_ref: false,
        }
    }

    fn callable(p: AstParam) -> Callable {
        Callable {
            params: vec![p],
            receiver: None,
        }
    }

    /// `RcBox`'s real pair: the public `init(consuming value: T)` and the
    /// private `init(inner inner: Pointer[RcBoxStorage[T]])`. Both are arity 1,
    /// so the old `params.len() == 1` predicate matched BOTH and silently took
    /// whichever came first. The shape predicate must pick the same one in
    /// either declaration order.
    #[test]
    fn box_init_shape_is_declaration_order_independent() {
        let public_init = callable(param(None, true));
        let private_init = callable(param(Some("inner"), false));

        for (order, candidates) in [
            (
                "declared order",
                vec![public_init.clone(), private_init.clone()],
            ),
            ("reversed", vec![private_init, public_init]),
        ] {
            let picked: Vec<usize> = candidates
                .iter()
                .enumerate()
                .filter(|(_, c)| is_box_init_shape(c))
                .map(|(i, _)| i)
                .collect();
            assert_eq!(picked.len(), 1, "{order}: expected exactly one match");
            let chosen = &candidates[exactly_one_box_member(&picked, "init")];
            assert!(
                chosen.params[0].label.is_none() && chosen.params[0].is_consuming,
                "{order}: selected the wrong init"
            );
        }

        // And the arity-only predicate the fix replaced does NOT discriminate —
        // this is the bug, stated as an assertion.
        assert!(callable(param(Some("inner"), false)).params.len() == 1);
    }

    #[test]
    fn two_shape_matching_inits_are_rejected_not_guessed() {
        let candidates = vec![callable(param(None, true)), callable(param(None, true))];
        let picked: Vec<usize> = candidates
            .iter()
            .enumerate()
            .filter(|(_, c)| is_box_init_shape(c))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(picked.len(), 2);

        let err = std::panic::catch_unwind(|| exactly_one_box_member(&picked, "init"))
            .expect_err("an ambiguous requirement must panic, not pick one");
        let msg = err
            .downcast_ref::<String>()
            .expect("ICE panics carry a formatted String");
        assert!(msg.contains("ICE"), "unexpected panic message: {msg}");
        assert!(msg.contains("2 members"), "unexpected panic message: {msg}");
    }

    #[test]
    fn a_missing_requirement_is_rejected_not_silently_skipped() {
        let candidates = vec![callable(param(Some("inner"), false))];
        let picked: Vec<usize> = candidates
            .iter()
            .enumerate()
            .filter(|(_, c)| is_box_init_shape(c))
            .map(|(i, _)| i)
            .collect();
        assert!(picked.is_empty());

        let err = std::panic::catch_unwind(|| exactly_one_box_member(&picked, "init"))
            .expect_err("a missing requirement must panic, not return None");
        let msg = err
            .downcast_ref::<String>()
            .expect("ICE panics carry a formatted String");
        assert!(msg.contains("ICE"), "unexpected panic message: {msg}");
        assert!(msg.contains("missing"), "unexpected panic message: {msg}");
    }
}
