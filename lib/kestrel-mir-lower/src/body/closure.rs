//! Closure lowering — env struct, synthetic call function, ApplyPartial.
//!
//! Captures are decided by the post-inference `ClosureCaptures` query (see
//! kestrel-type-infer/src/captures.rs) — the single source of truth. This
//! module only *emits* that decision; it does not recompute captures.
//!
//! Two capture tiers, selected by the closure type's [`FnKind`]:
//!
//! **View tier (`Normal` / `Mutating`)** — the shipped default. EVERY captured
//! place is captured by ADDRESS: the env field is `Pointer[T]` and holds a
//! pointer into the enclosing frame. The body binds each capture the way a
//! `MutBorrow` param is bound, so reads load through the pointer AT EACH USE
//! (a write between calls — or reentrantly during one — is observed) and a
//! `mutating` body's writes store back to the original. The env owns nothing:
//! nothing is copied, cloned, moved or dropped, and the closure value stays
//! frame-bound (E494). See docs/design/closures.md §"How it is captured".
//!
//! **Owning tier (`Consuming` / `Escaping`)** — the env struct is BY VALUE
//! throughout (bit-copy a Copyable capture, `clone()` a Cloneable one, MOVE a
//! non-Copyable one) and lives on the heap in a box (`closure_box.rs`), so the
//! value may leave the frame. The two owning kinds differ only in the box and
//! how the body reaches the environment:
//! - `Escaping` — the SHARED box. The body borrows the payload in place
//!   (`sharedMutRef()`) and binds each capture to its FIELD ADDRESS inside the
//!   shared storage, so state persists across calls and aliases observe it.
//! - `Consuming` — the UNIQUE box. The ONE call takes the whole environment
//!   out (`takeValue()`) and DESTRUCTURES it into per-capture @owned locals in
//!   the body's frame; each is then an ordinary local, which is what lets the
//!   body move a capture out while the rest drop normally.
//!
//! A module with no box binding (a `stdlib: false` fixture) falls back to the
//! historical stack-env snapshot path further down:
//! - a **whole local** (`Whole`) — captured by value if *duplicable* (Copyable
//!   or Cloneable: the env owns a bitwise copy / clone and the source stays
//!   live, so capturing an `RcBox` bumps its refcount), else by reference (a
//!   `Pointer[T]` to a stack snapshot, for move-only `not Copyable` types whose
//!   sole owner is moved into the env);
//! - a **projected place** (`Place`, e.g. `self.cap`) — always a Copyable,
//!   read-only field captured by value. Non-Copyable or written places fall
//!   back to whole-local capture (so behavior never regresses), because the
//!   by-reference projection of an arbitrary place isn't expressible yet.
//!
//! OSSA notes:
//! - Everything is ValueId, no Place/Operand/Rvalue.
//! - Whole-local captures bind into `local_map`; projected captures bind into
//!   `place_capture_map`, consulted when the body reads/borrows `self.cap`.
//! - Parent materializes a projected snapshot capture by walking the access
//!   chain from the real receiver (`StructExtract`), then copying the Copyable
//!   field — the receiver itself is never duplicated. A projected VIEW capture
//!   projects the field's ADDRESS instead (`FieldAddr`), never a value.

use std::collections::HashMap;
use std::mem;

use kestrel_hir::body::{HirBlock, HirClosureParam, HirExpr, HirExprId};
use kestrel_hir::res::LocalId as HirLocalId;
use kestrel_mir::body::OssaBody;
use kestrel_mir::callee::Callee;
use kestrel_mir::item::function::{FunctionDef, FunctionKind, ParamDef};
use kestrel_mir::item::struct_def::{FieldDef, StructDef};
use kestrel_mir::value::{Ownership, RootProvenance, ValueDef};
use kestrel_mir::{FieldIdx, FnKind, Immediate, MirTy, Op, ParamConvention, TyId, ValueId};
use kestrel_type_infer::captures::{CaptureKind, CapturedPlace};

use super::{LocalBinding, LoopInfo, OssaBodyCtx, PlaceCapture, ScopeFrame};

/// One resolved capture slot for a closure, after applying copyability to the
/// abstract `ClosureCaptures` plan.
enum CaptureSlot {
    /// Capture the whole local (by value if Copyable, else by reference).
    Whole(HirLocalId),
    /// Capture a Copyable, read-only projected place (e.g. `self.cap`) by value.
    Place(CapturedPlace),
}

/// Saved parent state during closure body lowering.
struct SavedState {
    body: OssaBody,
    current_block: Option<kestrel_mir::BlockId>,
    local_map: HashMap<HirLocalId, LocalBinding>,
    place_capture_map: HashMap<kestrel_type_infer::captures::PlaceKey, PlaceCapture>,
    loop_stack: Vec<LoopInfo>,
    scope_stack: Vec<ScopeFrame>,
    tracker: super::LiveTracker,
    func_entity: kestrel_hecs::Entity,
    temp_counter: u32,
    body_context: super::BodyContext,
    /// Per-body value-rename map — value IDs index the parent body's arena, so
    /// it must be swapped out for the closure's separate arena (otherwise a
    /// stale forwarding entry resolves into the wrong arena: out-of-bounds).
    value_forwarding: HashMap<ValueId, ValueId>,
    /// ret_borrow is per-body: a closure can never ret_borrow (E491), so it
    /// lowers with `false` and the parent's flag is restored after.
    ret_borrow: bool,
    /// Per-body value ids — same swap rationale as `value_forwarding`.
    ref_results: std::collections::HashSet<ValueId>,
    /// Named ref bindings are per-body too (value ids + the closure's own
    /// locals). A PARENT binding may now be captured (E212 retired): the view
    /// tier hands its target address to the env, so inside the body the
    /// capture is an ordinary `Var` place and none of this per-body budget
    /// machinery applies to it — which is exactly why the maps still swap.
    ref_binding_vals: HashMap<ValueId, kestrel_hir::res::LocalId>,
    ref_binding_remaining: HashMap<kestrel_hir::res::LocalId, usize>,
    /// Derived-address anchors are per-body value ids — same swap rationale
    /// as `value_forwarding`.
    addr_anchors: HashMap<ValueId, ValueId>,
}

impl OssaBodyCtx<'_, '_> {
    pub fn lower_closure_expr(
        &mut self,
        expr_id: HirExprId,
        params: &[HirClosureParam],
        body: &HirBlock,
    ) -> ValueId {
        let closure_ty = self.resolve_expr_type(expr_id);

        // Captures decided post-inference; we only apply copyability here.
        let slots = self.plan_slots(expr_id);

        let closure_idx = self.ctx.closure_counter;
        self.ctx.closure_counter += 1;
        let parent_name = self
            .ctx
            .module
            .functions
            .get(&self.func_entity)
            .map(|f| f.name.clone())
            .unwrap_or_default();
        let closure_name = format!("{}.closure.{}", parent_name, closure_idx);

        // Determine the closure KIND, param types, per-param conventions, and
        // return type from the closure's function type. A `MutBorrow` param is
        // bound by-reference. An unresolved type falls back to the view tier
        // (the default kind) — that is also today's single tier.
        let (fn_kind, param_tys, param_convs, ret_ty) =
            match self.ctx.module.ty_arena.get(closure_ty) {
                MirTy::FuncThick { kind, params, ret } => {
                    let tys = params.iter().map(|(ty, _)| *ty).collect::<Vec<_>>();
                    let convs = params.iter().map(|(_, c)| *c).collect::<Vec<_>>();
                    (*kind, tys, convs, *ret)
                },
                _ => {
                    let p: Vec<TyId> = params
                        .iter()
                        .map(|p| self.resolve_local_type(p.local))
                        .collect();
                    let convs = vec![ParamConvention::Consuming; p.len()];
                    let unit = self.ctx.module.ty_arena.unit();
                    (FnKind::Normal, p, convs, unit)
                },
            };
        // View tier: every capture is an address into this frame.
        let is_view = fn_kind.is_view();
        // Boxed (owning) tier: the env is an OWNED struct moved into a box at
        // creation; the closure value carries the handle plus the type-erased
        // retain/release shims. `escaping` boxes into the SHARED binding
        // (`@builtin(.SharedBox)`), `consuming` into the UNIQUE one
        // (`@builtin(.UniqueBox)`). A module with no such binding (a
        // `stdlib: false` test) falls back to the historical stack env —
        // unsound to return, but the escape check still rejects that, so
        // nothing silently miscompiles.
        let boxed = fn_kind.is_boxed();
        let unique = boxed && !fn_kind.is_shared();

        // Per-slot capture type (whole-local type, or projected place type).
        let slot_tys: Vec<TyId> = slots.iter().map(|s| self.slot_ty(s)).collect();

        // Create env struct for captures
        let env_struct_entity = if !slots.is_empty() {
            let env_struct_name = format!("{}.env", closure_name);
            let env_entity = self.ctx.next_synthetic_entity();
            self.ctx.module.register_name(env_entity, &env_struct_name);

            let mut env_def = StructDef::new(env_entity, &env_struct_name);
            env_def.type_params = self
                .ctx
                .module
                .functions
                .get(&self.func_entity)
                .map(|f| f.type_params.clone())
                .unwrap_or_default();
            for (i, &cap_ty) in slot_tys.iter().enumerate() {
                // View tier: EVERY field is a `Pointer[T]` into the frame.
                // Boxed tier (both owning kinds): EVERY field is BY VALUE — the
                // environment owns its snapshots (bit-copy / clone / move per
                // the D4 owning table) and is destroyed exactly once, by the
                // shared box's last release or the unique box's `destroy()`.
                let field_ty = if boxed {
                    cap_ty
                } else if is_view || !self.captures_by_value(cap_ty) {
                    self.ctx.module.ty_arena.pointer(cap_ty)
                } else {
                    cap_ty
                };
                env_def.add_field(FieldDef::new(format!("cap{i}"), field_ty));
            }
            if boxed {
                // A boxed environment is MOVE-ONLY: it owns its snapshots
                // (including moved-in `not Copyable` captures) and is never
                // duplicated — sharing goes through the box HANDLE, not the
                // payload. Leaving it `Bitwise` would trip Inv-3b the moment a
                // non-Copyable capture lands in it, and would license a bitwise
                // duplicate of an owned environment.
                env_def.type_info.copy = kestrel_mir::CopyBehavior::None;
            }
            let entity = env_def.entity;
            self.ctx.module.add_struct(env_def);
            Some(entity)
        } else {
            None
        };

        // Create closure function def
        let closure_entity = self.ctx.next_synthetic_entity();
        self.ctx.module.register_name(closure_entity, &closure_name);

        let mut func_def = FunctionDef::new(closure_entity, &closure_name, ret_ty);
        func_def.type_params = self
            .ctx
            .module
            .functions
            .get(&self.func_entity)
            .map(|f| f.type_params.clone())
            .unwrap_or_default();
        func_def.kind = if let Some(env_entity) = env_struct_entity {
            FunctionKind::ClosureCall {
                env_struct: env_entity,
            }
        } else {
            FunctionKind::Closure {
                parent_func: self.func_entity,
            }
        };

        // Build closure body
        let mut closure_body = OssaBody::new();

        // Env parameter — first value in the closure body.
        // Type is Pointer[EnvStruct] or Pointer[Unit] for no-capture closures;
        // at the boxed tier it is the BOX HANDLE's raw pointer word (word 1 of
        // the closure value), and the body projects the payload through the
        // binding's `sharedMutRef()`.
        let env_named_ty = env_struct_entity.map(|env_entity| {
            let tp_entities: Vec<kestrel_hecs::Entity> = self
                .ctx
                .module
                .functions
                .get(&self.func_entity)
                .map(|f| f.type_params.iter().map(|tp| tp.entity).collect())
                .unwrap_or_default();
            let env_type_args: Vec<TyId> = tp_entities
                .iter()
                .map(|&e| self.ctx.intern(MirTy::TypeParam(e)))
                .collect();
            self.ctx.module.ty_arena.named(env_entity, env_type_args)
        });
        // Resolved once and reused for the body prologue, the creation site and
        // the shims, so all three agree on the same binding instantiation.
        let binding = match (boxed, env_named_ty) {
            (true, Some(env_named)) if unique => self.resolve_unique_box_binding(env_named),
            (true, Some(env_named)) => self.resolve_box_binding(env_named),
            _ => None,
        };
        let env_ty = match (&binding, env_named_ty) {
            (Some(b), _) => b.raw_ty,
            (None, Some(named)) => self.ctx.module.ty_arena.pointer(named),
            (None, None) => {
                let unit = self.ctx.module.ty_arena.unit();
                self.ctx.module.ty_arena.pointer(unit)
            },
        };

        // Env param is the first ValueId (index 0) in the closure body
        let env_val =
            closure_body.alloc_value(ValueDef::owned(env_ty).with_root(RootProvenance::Param(0)));
        func_def.params.push(ParamDef::new(
            "env",
            env_val,
            env_ty,
            ParamConvention::Consuming,
        ));
        closure_body.param_count += 1;

        // Closure params — sequential ValueIds after env
        let mut closure_local_map: HashMap<HirLocalId, LocalBinding> = HashMap::new();
        for (i, cp) in params.iter().enumerate() {
            let ty = param_tys
                .get(i)
                .copied()
                .unwrap_or_else(|| self.ctx.module.ty_arena.error());
            let conv = param_convs
                .get(i)
                .copied()
                .unwrap_or(ParamConvention::Consuming);
            let name = &self.hir.locals[cp.local].name;
            match conv {
                ParamConvention::MutBorrow => {
                    // By-reference param: arrives as an address (ByRef ABI),
                    // bound as a mutable place so `x`/`x.field` writes lower
                    // in place through the borrow (mirrors `mutating self`,
                    // body/mod.rs MutBorrow arm).
                    let val = closure_body.alloc_value(ValueDef {
                        ty,
                        ownership: Ownership::Guaranteed,
                        borrow_source: None,
                        root: RootProvenance::Param((i + 1) as u32),
                        span: None,
                    });
                    closure_body.value_names.insert(val, name.clone());
                    func_def
                        .params
                        .push(ParamDef::new(name, val, ty, ParamConvention::MutBorrow));
                    closure_local_map.insert(cp.local, LocalBinding::Var(val));
                },
                _ => {
                    // Consuming (default) / Borrow: keep the @owned SSA binding
                    // (unchanged from pre-#106 behavior).
                    let val = closure_body.alloc_value(
                        ValueDef::owned(ty).with_root(RootProvenance::Param((i + 1) as u32)),
                    );
                    closure_body.value_names.insert(val, name.clone());
                    func_def
                        .params
                        .push(ParamDef::new(name, val, ty, ParamConvention::Consuming));
                    closure_local_map.insert(cp.local, LocalBinding::Ssa(val));
                },
            }
            closure_body.param_count += 1;
        }

        // Entry block
        let entry_block = closure_body.alloc_block();
        closure_body.entry = entry_block;

        // Save parent state
        let saved = SavedState {
            body: mem::replace(&mut self.body, closure_body),
            current_block: self.current_block.take(),
            local_map: mem::replace(&mut self.local_map, closure_local_map),
            place_capture_map: mem::take(&mut self.place_capture_map),
            loop_stack: mem::take(&mut self.loop_stack),
            scope_stack: mem::take(&mut self.scope_stack),
            tracker: mem::replace(&mut self.tracker, super::LiveTracker::from_live(&[])),
            func_entity: self.func_entity,
            temp_counter: self.temp_counter,
            body_context: std::mem::replace(&mut self.body_context, super::BodyContext::Normal),
            value_forwarding: mem::take(&mut self.value_forwarding),
            ret_borrow: mem::replace(&mut self.ret_borrow, false),
            ref_results: mem::take(&mut self.ref_results),
            ref_binding_vals: mem::take(&mut self.ref_binding_vals),
            ref_binding_remaining: mem::take(&mut self.ref_binding_remaining),
            addr_anchors: mem::take(&mut self.addr_anchors),
        };
        self.current_block = Some(entry_block);
        self.temp_counter = 0;
        // body_context already set to Normal by the mem::replace above
        self.push_scope();

        // Materialize captured slots from the env struct.
        //
        // By-value captures (duplicable: Copyable or Cloneable): env field is T
        // — extract the value directly (copy/clone of the @guaranteed field).
        // By-ref captures (move-only whole locals): env field is Pointer[T] —
        // extract the pointer, then load through it to get the T value.
        // BOXED TIER: the env param is the box handle's raw word. Project the
        // payload IN PLACE through the binding's `sharedMutRef()` (borrow,
        // never consume — one call must not release a multi-call environment)
        // and bind each capture to its FIELD ADDRESS inside the shared
        // storage. That is the reference semantics `escaping` marks: writes
        // persist across calls and every alias observes them.
        if let (Some(b), Some(env_named), true) = (&binding, env_named_ty, unique) {
            // UNIQUE (`consuming`) TIER: the ONE call owns the environment.
            // Move it out of the box and DESTRUCTURE it into per-capture @owned
            // locals in THIS frame — that is what makes the body "the one place
            // allowed to move captures out": each slot is now an ordinary local
            // with its own move/drop state, so moving one out and dropping the
            // rest is the existing partial-move machinery, not a special case.
            // The (now empty) block is reclaimed by the value's release shim.
            let binding = b;
            let env = self.emit_unique_take(binding, env_val, env_named);
            let fields = self.emit_destructure_struct(env, &slot_tys);
            for (i, slot) in slots.iter().enumerate() {
                match slot {
                    CaptureSlot::Whole(root) => {
                        self.local_map.insert(*root, LocalBinding::Ssa(fields[i]));
                    },
                    CaptureSlot::Place(cp) => {
                        self.place_capture_map
                            .insert(cp.key.clone(), PlaceCapture::Value(fields[i]));
                    },
                }
            }
        } else if let (Some(b), Some(env_named)) = (&binding, env_named_ty) {
            let binding = b;
            let payload_ptr = self.emit_box_payload_ptr(binding, env_val, env_named);
            for (i, slot) in slots.iter().enumerate() {
                let addr = self.emit_field_addr(payload_ptr, env_named, FieldIdx::new(i));
                match slot {
                    CaptureSlot::Whole(root) => {
                        self.local_map.insert(*root, LocalBinding::Var(addr));
                    },
                    CaptureSlot::Place(cp) => {
                        self.place_capture_map
                            .insert(cp.key.clone(), PlaceCapture::Addr(addr));
                    },
                }
            }
        } else if env_struct_entity.is_some() {
            let env_struct_ty = match self.ctx.module.ty_arena.get(env_ty) {
                MirTy::Pointer(inner) => *inner,
                _ => unreachable!("env_ty must be Pointer[EnvStruct]"),
            };

            // Borrow the env struct *in place* through the env pointer for
            // multi-field extraction. We must not load it by value and destroy
            // it: a by-value capture stores an @owned (cloned) field in the env,
            // and destroying a per-call snapshot of the env would drop that
            // field on every call, under-counting the refcount. Each extract
            // from the borrow is @guaranteed; we copy/clone to get @owned.
            let env_borrow = self.emit_begin_borrow_addr(env_val, env_struct_ty);

            for (i, slot) in slots.iter().enumerate() {
                let cap_ty = slot_tys[i];
                let is_ref_capture = is_view || !self.captures_by_value(cap_ty);
                let field_ty = if is_ref_capture {
                    self.ctx.module.ty_arena.pointer(cap_ty)
                } else {
                    cap_ty
                };

                // Extract from borrow → @guaranteed, then copy → @owned
                // (a `Pointer[T]` field copies bitwise — no clone, no move).
                let field_val = self.emit_struct_extract(env_borrow, FieldIdx::new(i), field_ty);
                let owned_field = self.emit_copy_value(field_val);

                // VIEW TIER: bind the frame ADDRESS, exactly like a MutBorrow
                // param — every read loads through it at its use site and every
                // write stores back. The old eager `emit_load` prologue took a
                // per-call snapshot, which could not observe a write that
                // happened after the call started (reentrancy) or between calls.
                // Protocol-`Self` keeps its borrow-alias binding: its abstract
                // type has no loadable/copyable representation here.
                if is_view && !self.is_protocol_self(cap_ty) {
                    match slot {
                        CaptureSlot::Whole(root) => {
                            self.local_map.insert(*root, LocalBinding::Var(owned_field));
                        },
                        CaptureSlot::Place(cp) => {
                            self.place_capture_map
                                .insert(cp.key.clone(), PlaceCapture::Addr(owned_field));
                        },
                    }
                    continue;
                }

                let value = if is_ref_capture {
                    if self.is_protocol_self(cap_ty) {
                        // Borrow-alias capture (non-escaping protocol `Self`):
                        // borrow through the env pointer; never load-and-own,
                        // which would drop the aliased receiver and desync the
                        // conformer's refcount.
                        self.emit_begin_borrow_addr(owned_field, cap_ty)
                    } else {
                        let loaded = self.emit_load(owned_field, cap_ty);
                        self.emit_destroy_value(owned_field);
                        loaded
                    }
                } else {
                    owned_field
                };

                match slot {
                    CaptureSlot::Whole(root) => {
                        self.local_map.insert(*root, LocalBinding::Ssa(value));
                    },
                    CaptureSlot::Place(cp) => {
                        self.place_capture_map
                            .insert(cp.key.clone(), PlaceCapture::Value(value));
                    },
                }
            }

            self.emit_end_borrow(env_borrow);
        }

        // Lower closure body
        let body_val = self.lower_hir_block(body);
        if !self.is_terminated() {
            // A closure can never ret_borrow (E491), so a @guaranteed tail
            // (e.g. a ref-returning call `{ b.peek() }`) must be copied out to
            // an @owned value — refs do not cross the closure boundary (#195).
            // `prepare_return_value` performs that copy for non-ret_borrow
            // bodies (here `self.ret_borrow` is always false — set by the
            // SavedState mem::replace above).
            let body_val = self.prepare_return_value(body_val);
            self.destroy_scope_except(&[body_val]);
            // A guarded destroy in the exit renames threaded values.
            let body_val = self.resolve_value(body_val);
            self.emit_ret(body_val);
        }

        // Extract closure body and restore parent
        let completed_body = mem::replace(&mut self.body, saved.body);
        self.current_block = saved.current_block;
        self.local_map = saved.local_map;
        self.place_capture_map = saved.place_capture_map;
        self.loop_stack = saved.loop_stack;
        self.scope_stack = saved.scope_stack;
        self.tracker = saved.tracker;
        self.func_entity = saved.func_entity;
        self.temp_counter = saved.temp_counter;
        self.body_context = saved.body_context;
        self.value_forwarding = saved.value_forwarding;
        self.ret_borrow = saved.ret_borrow;
        self.ref_results = saved.ref_results;
        self.ref_binding_vals = saved.ref_binding_vals;
        self.ref_binding_remaining = saved.ref_binding_remaining;
        self.addr_anchors = saved.addr_anchors;

        // Attach body and register function
        func_def.body = Some(completed_body);
        self.ctx.module.add_function(func_def);

        // Emit ApplyPartial in parent scope — materialize each capture.
        let mut captures: Vec<ValueId> = Vec::new();
        for (i, slot) in slots.iter().enumerate() {
            let cap = if is_view {
                self.materialize_view_capture(slot, slot_tys[i])
            } else if boxed {
                self.materialize_owning_capture(slot, slot_tys[i])
            } else {
                self.materialize_capture(slot, slot_tys[i])
            };
            captures.push(cap);
        }

        // The closure inherits the parent's type params (same entities), so the
        // partial application binds them by identity. Carrying these type args
        // lets monomorphization resolve the closure/thunk to the correct
        // instance — without them every `read[T]` collapsed to the first thunk.
        let parent_tp_entities: Vec<kestrel_hecs::Entity> = self
            .ctx
            .module
            .functions
            .get(&self.func_entity)
            .map(|f| f.type_params.iter().map(|tp| tp.entity).collect())
            .unwrap_or_default();
        let type_args: Vec<TyId> = parent_tp_entities
            .iter()
            .map(|&e| self.ctx.intern(MirTy::TypeParam(e)))
            .collect();
        let callee = Callee::direct_with_args(closure_entity, type_args.clone(), None);

        if !boxed {
            return self.emit_apply_partial(callee, captures, closure_ty);
        }

        // BOXED TIER creation: build the owned environment, move it into the
        // shared box and hand the raw handle word to `ApplyPartial` as word 1.
        // A capture-free escaping value never allocates: null handle, shared
        // no-op shims.
        let (handle, shims) = match (&binding, env_named_ty) {
            (Some(b), Some(env_named)) => {
                let fields: Vec<(FieldIdx, ValueId)> = captures
                    .iter()
                    .enumerate()
                    .map(|(i, &v)| (FieldIdx::new(i), v))
                    .collect();
                let env = self.emit_struct(env_named, fields);
                let handle = self.emit_box_env(b, env);
                let type_params = self
                    .ctx
                    .module
                    .functions
                    .get(&self.func_entity)
                    .map(|f| f.type_params.clone())
                    .unwrap_or_default();
                let shims = if unique {
                    super::closure_box::synthesize_unique_env_shims(
                        self.ctx,
                        b,
                        &closure_name,
                        type_params,
                        type_args,
                    )
                } else {
                    super::closure_box::synthesize_env_shims(
                        self.ctx,
                        b,
                        env_named,
                        &closure_name,
                        type_params,
                        type_args,
                    )
                };
                (vec![handle], shims)
            },
            // No captures (or no binding): the environment is nothing at all.
            _ => {
                for &c in &captures {
                    self.emit_destroy_value(c);
                }
                (Vec::new(), super::closure_box::nop_shims(self.ctx))
            },
        };
        self.emit_apply_partial_shimmed(callee, handle, closure_ty, Some(shims))
    }

    // === Capture planning ===

    /// Translate the abstract capture plan into concrete slots, applying
    /// copyability (only the MIR layer knows it). A projected place is kept as
    /// a by-value place capture only when it is Copyable and read-only;
    /// otherwise the whole root is captured (preserving historical behavior).
    fn plan_slots(&mut self, closure_id: HirExprId) -> Vec<CaptureSlot> {
        let plan: Vec<CapturedPlace> = self.captures.get(closure_id).to_vec();

        // First pass: which roots must be captured whole.
        let mut whole_roots: std::collections::HashSet<HirLocalId> =
            std::collections::HashSet::new();
        for cp in &plan {
            if cp.key.is_whole() {
                whole_roots.insert(cp.key.root);
                continue;
            }
            let pty = self.resolve_expr_type(cp.repr);
            // A read-only projected place is kept as a by-value place-capture
            // when it is *duplicable* (Copyable or Cloneable) — we clone/copy
            // just the field and borrow the receiver, never duplicating it.
            // Gating on `is_copy_type` (Bitwise only) here would force a Clone
            // field to fall back to whole-receiver capture, which can wrongly
            // consume a move-only receiver. See [[closure_cloneable_capture_clone]].
            if !(self.captures_by_value(pty) && cp.kind == CaptureKind::Read) {
                whole_roots.insert(cp.key.root);
            }
        }

        // Second pass: build ordered, deduplicated slots (plan is already in a
        // deterministic order).
        let mut slots = Vec::new();
        let mut added_whole: std::collections::HashSet<HirLocalId> =
            std::collections::HashSet::new();
        for cp in plan {
            let root = cp.key.root;
            if !cp.key.is_whole() && !whole_roots.contains(&root) {
                slots.push(CaptureSlot::Place(cp));
            } else if added_whole.insert(root) {
                slots.push(CaptureSlot::Whole(root));
            }
        }
        slots
    }

    fn slot_ty(&mut self, slot: &CaptureSlot) -> TyId {
        match slot {
            CaptureSlot::Whole(root) => self.resolve_local_type(*root),
            CaptureSlot::Place(cp) => self.resolve_expr_type(cp.repr),
        }
    }

    // === View-tier capture materialization (parent side) ===

    /// Materialize a VIEW capture: an @owned `Pointer[cap_ty]` aliasing the
    /// source's storage in THIS frame. Nothing is copied, cloned, moved or
    /// consumed — a view env owns nothing, so a `clone()` here would leak
    /// (nothing ever releases it) and a `Take` would kill the source (#177).
    fn materialize_view_capture(&mut self, slot: &CaptureSlot, cap_ty: TyId) -> ValueId {
        // A protocol-`Self` capture has no addressable representation at its
        // abstract type; keep its historical borrow-alias slot.
        if self.is_protocol_self(cap_ty) {
            return self.materialize_capture(slot, cap_ty);
        }
        match slot {
            CaptureSlot::Whole(root) => self.view_addr_of_local(*root, cap_ty),
            CaptureSlot::Place(cp) => self.view_addr_of_place(cp, cap_ty),
        }
    }

    /// The frame address of a whole-local view capture. The result is a fresh
    /// @owned `Pointer[cap_ty]` — `ApplyPartial` consumes its captures, so the
    /// source binding's own address value must never be handed over directly.
    fn view_addr_of_local(&mut self, root: HirLocalId, cap_ty: TyId) -> ValueId {
        match self.local_map.get(&root).copied() {
            // Every `let`/`var` (uniform binding, #107) and every MutBorrow
            // param lives at an address — alias it. This is the true view:
            // later writes through the binding are seen by the closure, and a
            // `mutating` closure's writes land here. A `Pointer[T]` copy is
            // bitwise: it duplicates the pointer, never the pointee.
            Some(LocalBinding::Var(addr)) => {
                let addr = self.whole_slot_addr(addr);
                self.emit_value_use(addr)
            },
            // A NAMED REF BINDING (`let r = &x;`) is already a place: its
            // value is an in-place borrow of the referent's storage, so the
            // view captures THE TARGET, not a snapshot of it. `PtrTo` on the
            // borrow is the canonical "address of this @guaranteed place"
            // form (`whole_slot_addr`), and it is what makes a captured `r`
            // observe later writes to `x` — the behavior that let E212
            // ("closures cannot capture reference bindings") retire.
            // Anchoring the bits instead, as the generic SSA path below does,
            // silently froze the pointee at capture time.
            Some(LocalBinding::Ssa(v)) if self.ref_binding_vals.contains_key(&v) => {
                let addr = self.whole_slot_addr(v);
                self.emit_value_use(addr)
            },
            // Non-addressable SSA binding (a consuming/borrowing param, a
            // pattern binding). It is IMMUTABLE for its whole extent, so
            // materializing its bits ONCE into a frame slot is observationally
            // identical to a live view — and unlike a copy/clone it neither
            // takes ownership nor bumps a refcount. An @owned source is
            // borrowed first so the store never consumes it.
            Some(LocalBinding::Ssa(v)) => self.anchor_value_in_slot(v, cap_ty),
            None => {
                let v = self.map_local(root);
                self.anchor_value_in_slot(v, cap_ty)
            },
        }
    }

    /// The frame address of a projected view capture (`self.n`, `p.a`).
    fn view_addr_of_place(&mut self, cp: &CapturedPlace, cap_ty: TyId) -> ValueId {
        // Address route: project the field's address off an addressable base
        // (`try_field_addr_chain` is the shared place walk).
        if let Some(addr) = self.try_field_addr_chain(cp.repr) {
            return addr;
        }
        // Base isn't addressable (a borrowed receiver, an rvalue): fall back to
        // anchoring the projected VALUE once. `lower_place_value_parent`
        // projects @guaranteed views out of the receiver without duplicating it.
        let v = self.lower_place_value_parent(cp.repr);
        self.anchor_value_in_slot(v, cap_ty)
    }

    /// Materialize `v`'s bits into a fresh frame slot and return the slot
    /// address. Never consumes, copies (clones) or moves `v`: an @owned value
    /// is borrowed for the store, a @guaranteed one is stored directly
    /// (`emit_store_init_borrowed` is the no-CopyValue store).
    fn anchor_value_in_slot(&mut self, v: ValueId, cap_ty: TyId) -> ValueId {
        let ptr_ty = self.ctx.module.ty_arena.pointer(cap_ty);
        let one = self.emit_literal(Immediate::i64(1));
        let addr = self.emit_op1(Op::StackAlloc(cap_ty), one, ptr_ty);
        if self.body.value(v).ownership == Ownership::Owned {
            let borrow = self.emit_begin_borrow(v);
            self.emit_store_init_borrowed(addr, borrow);
            self.emit_end_borrow(borrow);
        } else {
            self.emit_store_init_borrowed(addr, v);
        }
        addr
    }

    /// Materialize an OWNING capture (the D4 owning table) as a VALUE for the
    /// boxed environment struct: bit-copy a Copyable place, `clone()` a
    /// Cloneable one, MOVE a non-Copyable one out of the frame.
    ///
    /// The move is the MIR half of a decision the front end already recorded
    /// (`move_tracking` marks an owning capture as a move of the root, so a
    /// later use is E500 rather than an OSSA ICE — #177 symmetry). Rejections
    /// (ref-carrying values, protocol `Self`) are the analyzer's job; if one
    /// reaches here it degrades to a bit-copy rather than crashing.
    fn materialize_owning_capture(&mut self, slot: &CaptureSlot, cap_ty: TyId) -> ValueId {
        let CaptureSlot::Whole(root) = slot else {
            // Projected place: copy/clone just the field, never the receiver.
            return self.materialize_capture(slot, cap_ty);
        };
        let root = *root;
        let mir_val = self.map_local(root);
        if self.captures_by_value(cap_ty) {
            return if self.is_var_local(&root) {
                self.emit_copy_addr(mir_val, cap_ty)
            } else {
                self.emit_value_use(mir_val)
            };
        }
        // Move-only: the environment becomes the sole owner.
        if self.is_var_local(&root) && self.body.value(mir_val).ownership == Ownership::Owned {
            let value = self.emit_take(mir_val, cap_ty);
            self.set_var_init(root, super::VarInit::DefUninit);
            if let Some(flag) = self.var_flag(root) {
                self.store_drop_flag(flag, false);
            }
            return value;
        }
        if self.body.value(mir_val).ownership == Ownership::Owned {
            return self.emit_move_value(mir_val);
        }
        // A @guaranteed (borrowed) non-Copyable place cannot be owned. This is
        // the "non-owned non-Copyable" rejection; the analyzer reports it, so
        // keep the historical aliasing behavior rather than ICE here.
        self.emit_value_use(mir_val)
    }

    /// Materialize a capture in the parent scope for packing into the env.
    fn materialize_capture(&mut self, slot: &CaptureSlot, cap_ty: TyId) -> ValueId {
        match slot {
            CaptureSlot::Whole(root) => self.materialize_whole(*root, cap_ty),
            CaptureSlot::Place(cp) => {
                // By-value projection of a Copyable field — copy the field,
                // never the receiver.
                let v = self.lower_place_value_parent(cp.repr);
                if self.body.value(v).ownership == kestrel_mir::value::Ownership::Guaranteed {
                    self.emit_copy_value(v)
                } else {
                    v
                }
            },
        }
    }

    /// A whole-local capture is by value when the type is *duplicable* — either
    /// Copyable (bitwise) or Cloneable (has a clone shim). The env then owns a
    /// copy/clone and the source local stays live (capturing an `RcBox` bumps
    /// its refcount, leaving the original usable). Only move-only `not Copyable`
    /// types — and a protocol's type-erased `Self` (see `is_protocol_self`) —
    /// are captured by reference.
    fn captures_by_value(&self, cap_ty: TyId) -> bool {
        !self.is_non_copyable(cap_ty) && !self.is_protocol_self(cap_ty)
    }

    /// True when `cap_ty` is a protocol's `Self` type — `MirTy::TypeParam(e)`
    /// with `e` a protocol entity (see `ty::build_self_type`). Type-erased: a
    /// closure's `ApplyPartial` carries no `self_type`, so `copy_value` at the
    /// abstract type has no clone shim → captured by reference instead.
    fn is_protocol_self(&self, cap_ty: TyId) -> bool {
        match self.ctx.module.ty_arena.get(cap_ty) {
            MirTy::TypeParam(e) => {
                self.ctx.world.get::<kestrel_ast_builder::NodeKind>(*e)
                    == Some(&kestrel_ast_builder::NodeKind::Protocol)
            },
            _ => false,
        }
    }

    fn materialize_whole(&mut self, root: HirLocalId, cap_ty: TyId) -> ValueId {
        let mir_val = self.map_local(root);
        if self.captures_by_value(cap_ty) {
            // By-value capture: snapshot the value (bitwise copy or clone).
            // emit_value_use → emit_copy_value emits a CopyValue, which the
            // expand pass turns into the type's clone shim for Cloneable Named
            // types — the source stays live. Var locals are address-based, so
            // load (and clone) the value from the address instead.
            if self.is_var_local(&root) {
                self.emit_copy_addr(mir_val, cap_ty)
            } else {
                self.emit_value_use(mir_val)
            }
        } else {
            // Ref capture: materialize the value into a stack slot and capture
            // the slot address (Pointer[cap_ty], matching the env field). How we
            // fill the slot depends on the captured value's ownership:
            //   - Var local   → load through its address.
            //   - @owned      → MOVE into the slot; the env now owns the value.
            //                   This is how an escaping closure keeps a
            //                   non-Copyable capture (e.g. a comparator) alive:
            //                   escaping closure params are `consuming`, so they
            //                   arrive @owned and are moved, never aliased.
            //   - @guaranteed → borrow-capture: store the borrowed bits directly
            //                   (no CopyValue — copying a non-Copyable @thick
            //                   value is illegal). The slot aliases the borrow's
            //                   storage. Sound only for a non-escaping closure
            //                   (e.g. the `and`/`or` short-circuit thunk that
            //                   captures a called-not-stored predicate); the
            //                   convention guarantees an escaping closure's
            //                   captures are @owned and take the move branch.
            let ptr_ty = self.ctx.module.ty_arena.pointer(cap_ty);
            let one = self.emit_literal(Immediate::i64(1));
            let addr = self.emit_op1(Op::StackAlloc(cap_ty), one, ptr_ty);
            if self.is_var_local(&root) {
                // A non-Copyable local in an @owned slot (every let/var,
                // #107) is MOVED into the env — the frontend records the
                // capture as a move of the outer local (E500 on later use).
                // Take + mark the slot empty; a bitwise `copy_addr` would
                // leave the slot's scope-exit destroy live and double-free.
                // A @guaranteed slot (`mutating self`) or a protocol-Self
                // capture still snapshots the bits.
                let slot_owned = self.body.value(mir_val).ownership == Ownership::Owned;
                if slot_owned && self.is_non_copyable(cap_ty) {
                    let value = self.emit_take(mir_val, cap_ty);
                    self.set_var_init(root, super::VarInit::DefUninit);
                    if let Some(flag) = self.var_flag(root) {
                        self.store_drop_flag(flag, false);
                    }
                    self.emit_store_init(addr, value);
                } else {
                    let value = self.emit_copy_addr(mir_val, cap_ty);
                    self.emit_store_init(addr, value);
                }
            } else if self.body.value(mir_val).ownership == Ownership::Owned {
                let value = self.emit_move_value(mir_val);
                self.emit_store_init(addr, value);
            } else {
                self.emit_store_init_borrowed(addr, mir_val);
            }
            addr
        }
    }

    /// Project a place value from the *real* receiver in the parent scope
    /// (walking `StructExtract`/`TupleExtract`), bypassing the body-side
    /// capture interception. Used only for by-value projected captures.
    fn lower_place_value_parent(&mut self, repr: HirExprId) -> ValueId {
        match self.hir.exprs[repr].clone() {
            HirExpr::Local(..) => self.lower_expr_for_borrow(repr),
            HirExpr::Field { base, name, .. } => {
                let base_val = self.lower_place_value_parent(base);
                let base_ty = self.resolve_expr_type(base);
                let result_ty = self.resolve_expr_type(repr);
                let struct_entity = match self.ctx.module.ty_arena.get(base_ty) {
                    MirTy::Named { entity, .. } => Some(*entity),
                    _ => None,
                };
                let field_idx = struct_entity
                    .and_then(|se| self.ctx.resolve_field_idx(se, name.as_str_or_empty()))
                    .unwrap_or_else(|| FieldIdx::new(0));
                self.emit_struct_extract(base_val, field_idx, result_ty)
            },
            HirExpr::TupleIndex { base, index, .. } => {
                let base_val = self.lower_place_value_parent(base);
                let result_ty = self.resolve_expr_type(repr);
                self.emit_tuple_extract(base_val, index, result_ty)
            },
            _ => self.lower_expr(repr),
        }
    }
}
