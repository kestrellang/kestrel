use std::collections::{HashMap, HashSet};

use kestrel_hecs::Entity;

use crate::body::OssaBody;
use crate::callee::Callee;
use crate::inst::{CallArg, InstKind, Instruction};
use crate::item::CopyBehavior;
use crate::item::function::{FunctionDef, FunctionKind};
use crate::mono::types::{MonoFunction, MonoModule};
use crate::ty::{MirTy, ParamConvention};
use crate::value::{Ownership, ValueDef};
use crate::{MonoFuncId, TyId, ValueId};

/// Expand DestroyValue and CopyValue instructions in all monomorphized bodies.
///
/// After monomorphization, concrete types are known. This pass:
///
/// 1. Replaces `DestroyValue` on Named types with a call to their drop shim.
/// 2. Removes `DestroyValue` on non-Named types (trivial, no-op).
/// 3. Replaces `CopyValue` on Named types with clone if available.
/// 4. Removes `CopyValue` on non-Named types (trivial bitwise alias).
///
/// Must run after `monomorphize()` and before `verify_mono()`.
pub fn expand_destroy_copy(
    module: &mut MonoModule,
    generic_functions: &indexmap::IndexMap<Entity, FunctionDef>,
) {
    let shim_lookup = build_drop_shim_lookup(module, generic_functions);
    let clone_lookup = build_clone_lookup(module, generic_functions);

    // Map deinit/drop-shim entities → nominal. Suppresses DestroyValue→__drop$T
    // expansion only inside a type's own drop machinery (its `deinit` and
    // `__drop$T` shim), breaking the drop-recursion cycle. Ordinary methods —
    // including consuming methods that own and must drop `self` — are NOT in
    // this map, so their `destroy_value self` expands normally.
    let drop_impl_to_nominal = build_drop_impl_to_nominal_map(generic_functions);

    // Narrow map for the CopyValue→clone guard: only a type's own clone
    // implementation may suppress cloning a copy of itself (to avoid
    // recursion). Ordinary methods that copy `self` must still clone.
    let clone_impl_to_nominal = build_clone_impl_to_nominal_map(generic_functions);

    // Pre-intern Pointer(Named) types for cloneable types so the expand
    // pass can create BeginBorrow values without mutable arena access.
    for (nominal, type_args) in clone_lookup.keys() {
        if let Some(named_ty) = module.ty_arena.find(|t| {
            matches!(t, MirTy::Named { entity, type_args: ta } if *entity == *nominal && ta == type_args)
        }) {
            module.ty_arena.pointer(named_ty);
        }
    }

    // Collect not-Copyable type *instances* — CopyValue on these is a move,
    // not a copy. Keyed by (nominal, type_args), NOT by nominal alone:
    // conditional Copyable is per-instantiation (`Optional[Int64]` is Copyable
    // while `Optional[File]` is not), so collapsing to the nominal would poison
    // every monomorphization of a generic that is ever instantiated move-only.
    // That poisoning degrades a real CopyValue into a move-alias; when the
    // operand is a borrow of a place that is later mutated (the ubiquitous
    // `let x = self.field; self.field = ...; x` pattern in iterators/`take`),
    // the returned value observes the mutation → silent corruption.
    let not_copyable: HashSet<(Entity, Vec<TyId>)> = module
        .structs
        .values()
        .filter(|s| matches!(s.type_info.copy, CopyBehavior::None))
        .map(|s| (s.source, s.type_args.clone()))
        .chain(
            module
                .enums
                .values()
                .filter(|e| matches!(e.type_info.copy, CopyBehavior::None))
                .map(|e| (e.source, e.type_args.clone())),
        )
        .collect();

    for fi in 0..module.functions.len() {
        let source = module.functions[fi].source;
        // Per-instantiation drop-recursion guard: skip ONLY a DestroyValue of
        // this shim's exact monomorphic self type, reconstructed as
        // (nominal, this instance's type_args). Keying by nominal alone
        // collapsed every instantiation, so `__drop$Wrapper[Wrapper[T]]` also
        // skipped dropping its payload of type `Wrapper[T]` (a *different*
        // instantiation of the same generic) → the recursive drop chain
        // stopped and nested-enum payloads leaked. (AGENTS.md: key by
        // (Entity, type_args), never the nominal alone.)
        let skip_self = drop_impl_to_nominal
            .get(&source)
            .map(|&n| (n, module.functions[fi].type_args.clone()));
        let skip_clone_nominal = clone_impl_to_nominal.get(&source).copied();
        expand_function(
            &mut module.functions[fi],
            &module.ty_arena,
            &shim_lookup,
            &clone_lookup,
            skip_self.as_ref(),
            skip_clone_nominal,
            &not_copyable,
        );
    }
}

/// Maps (nominal_entity, type_args) -> MonoFuncId for drop shim dispatch.
type DropShimLookup = HashMap<(Entity, Vec<TyId>), MonoFuncId>;

/// Build func_entity → nominal for the *drop machinery only* — a type's
/// synthesized `__drop$T` shim and its user-written `deinit`.
///
/// DestroyValue/DestroyAddr on T is suppressed inside these (instead of
/// expanding to `call __drop$T`) to break the recursion
/// `DestroyValue(T)` → `__drop$T` → [drops self / its temporaries] →
/// `DestroyValue(T)` → … . It must NOT be suppressed in ordinary methods:
/// a consuming method like `consuming func destroy(self) {}` owns `self` and
/// has to drop it at end of body — using the broad "all methods of T" map here
/// silently dropped that `destroy_value self`, so the receiver leaked (its
/// `deinit` never ran). This mirrors the clone-side narrowing in
/// `build_clone_impl_to_nominal_map` (see the StringSlice.asSlice double-free).
fn build_drop_impl_to_nominal_map(
    generic_functions: &indexmap::IndexMap<Entity, FunctionDef>,
) -> HashMap<Entity, Entity> {
    let mut map = HashMap::new();
    for f in generic_functions.values() {
        match &f.kind {
            FunctionKind::Deinit { parent } | FunctionKind::DropShim { nominal: parent } => {
                map.insert(f.entity, *parent);
            },
            _ => {},
        }
    }
    map
}

/// Build func_entity → nominal for *clone implementations only* — the
/// synthesized `__clone$T` shim and the user-written `T.clone()` method.
///
/// CopyValue→clone expansion is suppressed inside a type's own clone
/// implementation, where expanding a copy of `T` would recurse
/// (`T.clone` → copy `T` → `T.clone` → …). It must NOT be suppressed in
/// ordinary methods: `asSlice() -> StringSlice { self }` copies `self` to
/// return it, and that copy has to clone so the refcount on the shared
/// `RcBox` is bumped. Using the broad `method_to_nominal` map there left the
/// returned slice as a bitwise alias with no refcount bump → the alias's
/// later release over-decremented the count → double-free of the storage.
fn build_clone_impl_to_nominal_map(
    generic_functions: &indexmap::IndexMap<Entity, FunctionDef>,
) -> HashMap<Entity, Entity> {
    let mut map = HashMap::new();
    for f in generic_functions.values() {
        match &f.kind {
            FunctionKind::CloneShim { nominal } => {
                map.insert(f.entity, *nominal);
            },
            FunctionKind::Method { parent, .. } if f.name.ends_with(".clone") => {
                map.insert(f.entity, *parent);
            },
            _ => {},
        }
    }
    map
}

/// Build (nominal_entity, type_args) → MonoFuncId for clone functions.
/// Finds both synthesized CloneShim functions and user-written .clone() methods
/// via FunctionKind matching.
fn build_clone_lookup(
    module: &MonoModule,
    generic_functions: &indexmap::IndexMap<Entity, FunctionDef>,
) -> DropShimLookup {
    // Map clone function entity → nominal parent.
    // Include ALL clone shims and user .clone() methods regardless of CopyBehavior.
    let mut clone_func_to_parent: HashMap<Entity, Entity> = HashMap::new();
    for f in generic_functions.values() {
        match &f.kind {
            FunctionKind::CloneShim { nominal } => {
                clone_func_to_parent.insert(f.entity, *nominal);
            },
            FunctionKind::Method { parent, .. } if f.name.ends_with(".clone") => {
                // Prefer the self-param nominal: an `extend`-defined `clone()`
                // doesn't reliably set `parent` to the extended type.
                let nominal = f
                    .clone_method_self_nominal(&module.ty_arena)
                    .unwrap_or(*parent);
                clone_func_to_parent.insert(f.entity, nominal);
            },
            _ => {},
        }
    }

    if std::env::var("KESTREL_DEBUG_CLONE").is_ok() {
        eprintln!(
            "[clone_lookup] clone_func_to_parent: {} entries",
            clone_func_to_parent.len()
        );
        for (func_entity, parent) in &clone_func_to_parent {
            let name = generic_functions
                .get(func_entity)
                .map(|f| f.name.as_str())
                .unwrap_or("?");
            let parent_name = module
                .entity_names
                .get(parent)
                .map(|s| s.as_str())
                .unwrap_or("?");
            eprintln!("  clone func: {name} → parent={parent_name}");
        }
    }

    // Collect instances that don't need clone shim calls:
    // Bitwise types (trivial copy) and not-Copyable types (move, never clone).
    // Keyed per (nominal, type_args), NOT by nominal alone: a conditionally-
    // Copyable container has both Bitwise and Clone instances (`Optional[Int64]`
    // vs `Optional[String]`). Collapsing to the nominal would let the Bitwise
    // instance drop the Clone instance's shim from the lookup → the Clone
    // instance falls back to a non-refcounting bit-copy → double-free.
    let skip_clone_instances: HashSet<(Entity, Vec<TyId>)> = module
        .structs
        .values()
        .filter(|s| matches!(s.type_info.copy, CopyBehavior::Bitwise | CopyBehavior::None))
        .map(|s| (s.source, s.type_args.clone()))
        .chain(
            module
                .enums
                .values()
                .filter(|e| matches!(e.type_info.copy, CopyBehavior::Bitwise | CopyBehavior::None))
                .map(|e| (e.source, e.type_args.clone())),
        )
        .collect();

    let mut lookup = DropShimLookup::new();
    for (mi, mf) in module.functions.iter().enumerate() {
        if let Some(&nominal) = clone_func_to_parent.get(&mf.source) {
            if skip_clone_instances.contains(&(nominal, mf.type_args.clone())) {
                if std::env::var("KESTREL_DEBUG_CLONE").is_ok() {
                    eprintln!(
                        "[clone_lookup] SKIPPED (bitwise/not-copyable): {} source={:?} nominal={:?} type_args={:?}",
                        mf.name, mf.source, nominal, mf.type_args
                    );
                }
                continue;
            }
            if std::env::var("KESTREL_DEBUG_CLONE").is_ok() {
                eprintln!(
                    "[clone_lookup] ADDED: {} source={:?} nominal={:?} type_args={:?}",
                    mf.name, mf.source, nominal, mf.type_args
                );
            }
            lookup.insert((nominal, mf.type_args.clone()), MonoFuncId::new(mi));
        }
    }

    if std::env::var("KESTREL_DEBUG_CLONE").is_ok() {
        eprintln!("[clone_lookup] final lookup: {} entries", lookup.len());
    }

    lookup
}

/// Scan the generic function list for DropShim functions, then find their
/// monomorphized counterparts in the MonoModule.
fn build_drop_shim_lookup(
    module: &MonoModule,
    generic_functions: &indexmap::IndexMap<Entity, FunctionDef>,
) -> DropShimLookup {
    let shim_to_nominal: HashMap<Entity, Entity> = generic_functions
        .values()
        .filter_map(|f| match &f.kind {
            FunctionKind::DropShim { nominal } => Some((f.entity, *nominal)),
            _ => None,
        })
        .collect();

    let mut lookup = DropShimLookup::new();

    for (mi, mf) in module.functions.iter().enumerate() {
        if let Some(&nominal) = shim_to_nominal.get(&mf.source) {
            lookup.insert((nominal, mf.type_args.clone()), MonoFuncId::new(mi));
        }
    }

    lookup
}

/// True when `(entity, type_args)` is exactly this shim's own monomorphic self
/// type — the one case where expanding `DestroyValue → __drop$Self` would
/// recurse. Compared per-instantiation (full type, not nominal alone) so a
/// payload that is a *different* instantiation of the same generic still drops.
fn is_drop_self(
    skip_self: Option<&(Entity, Vec<TyId>)>,
    entity: Entity,
    type_args: &[TyId],
) -> bool {
    matches!(skip_self, Some((e, args)) if *e == entity && args.as_slice() == type_args)
}

/// True when destroying a value of `ty` requires running a destructor: a Named
/// type with a registered drop shim, or a tuple with at least one element that
/// itself needs dropping. Mirrors the type kinds the destroy arms handle.
fn ty_needs_drop(
    ty_arena: &crate::ty::TyArena,
    shim_lookup: &DropShimLookup,
    ty: TyId,
) -> bool {
    match ty_arena.get(ty) {
        MirTy::Named { entity, type_args } => {
            shim_lookup.contains_key(&(*entity, type_args.clone()))
        },
        MirTy::Tuple(elems) => {
            let elems = elems.clone();
            elems
                .iter()
                .any(|&e| ty_needs_drop(ty_arena, shim_lookup, e))
        },
        _ => false,
    }
}

/// Emit the instructions that destroy `value` (of type `ty`) into `out`,
/// recursing through tuple members. Named-with-shim → consuming shim call;
/// tuple → `DestructureTuple` then each member destroyed in turn; trivial →
/// nothing. Tuples have no nominal entity, so they never get a `__drop$T`
/// shim — their members must be expanded inline here, or every resource-owning
/// tuple member (e.g. the `String`s in `Array[(String, String)]`) leaks. This
/// is the post-mono analog of a struct/enum drop shim's per-field drops.
fn emit_destroy_recursive(
    body: &mut OssaBody,
    ty_arena: &crate::ty::TyArena,
    shim_lookup: &DropShimLookup,
    skip_self: Option<&(Entity, Vec<TyId>)>,
    value: ValueId,
    ty: TyId,
    span: &Option<kestrel_span::Span>,
    out: &mut Vec<Instruction>,
) {
    match ty_arena.get(ty) {
        MirTy::Named { entity, type_args } => {
            let entity = *entity;
            let type_args = type_args.clone();
            if is_drop_self(skip_self, entity, &type_args) {
                return;
            }
            if let Some(&shim_id) = shim_lookup.get(&(entity, type_args)) {
                out.push(Instruction {
                    kind: InstKind::Call {
                        result: None,
                        callee: Callee::Resolved(shim_id),
                        args: vec![CallArg {
                            value,
                            convention: ParamConvention::Consuming,
                        }],
                    },
                    span: span.clone(),
                });
            }
        },
        MirTy::Tuple(elems) => {
            let elems = elems.clone();
            if !elems
                .iter()
                .any(|&e| ty_needs_drop(ty_arena, shim_lookup, e))
            {
                return;
            }
            let results: Vec<ValueId> = elems
                .iter()
                .map(|&e| body.alloc_value(ValueDef::owned(e)))
                .collect();
            out.push(Instruction {
                kind: InstKind::DestructureTuple {
                    results: results.clone(),
                    operand: value,
                },
                span: span.clone(),
            });
            for (i, &e) in elems.iter().enumerate() {
                emit_destroy_recursive(body, ty_arena, shim_lookup, skip_self, results[i], e, span, out);
            }
        },
        _ => {},
    }
}

/// Emit the instructions that deep-clone `operand` (a **@guaranteed** value of
/// type `ty`) into the @owned `result`, recursing through tuple members. This
/// is the copy-side mirror of [`emit_destroy_recursive`] and the inline analog
/// of a struct/enum clone shim:
/// - Named with a clone shim → `BeginBorrow` + `Call(clone)` + `EndBorrow`
///   (the exact form the `CopyValue` Named arm emits).
/// - Tuple → `TupleExtract` each element (a @guaranteed projection of the
///   @guaranteed operand), recursively clone it, then rebuild a `Tuple`.
/// - Trivial (primitive, or Named without a clone shim) → keep a `CopyValue`,
///   which codegen lowers to a correct bitwise load.
///
/// Without the tuple arm, `CopyValue` on a tuple is left as a bitwise alias
/// (codegen loads the ByRef), so a cloned `Array[(String, String)]` (COW
/// clone / grow via `read()`) aliases the inner `String`s without bumping their
/// refcount — which double-frees once the element destructors run.
fn emit_clone_recursive(
    body: &mut OssaBody,
    ty_arena: &crate::ty::TyArena,
    clone_lookup: &DropShimLookup,
    skip_clone_nominal: Option<Entity>,
    operand: ValueId,
    ty: TyId,
    result: ValueId,
    span: &Option<kestrel_span::Span>,
    out: &mut Vec<Instruction>,
) {
    match ty_arena.get(ty) {
        MirTy::Named { entity, type_args } => {
            let entity = *entity;
            let type_args = type_args.clone();
            if skip_clone_nominal != Some(entity)
                && let Some(&clone_id) = clone_lookup.get(&(entity, type_args))
            {
                let ptr_ty = ty_arena
                    .find(|t| matches!(t, MirTy::Pointer(p) if *p == ty))
                    .expect("Pointer type should be pre-interned for cloneable types");
                let borrow_val = body.alloc_value(ValueDef::guaranteed(ptr_ty, operand));
                out.push(Instruction::new(InstKind::BeginBorrow {
                    result: borrow_val,
                    operand,
                }));
                out.push(Instruction {
                    kind: InstKind::Call {
                        result: Some(result),
                        callee: Callee::Resolved(clone_id),
                        args: vec![CallArg {
                            value: borrow_val,
                            convention: ParamConvention::Borrow,
                        }],
                    },
                    span: span.clone(),
                });
                out.push(Instruction::new(InstKind::EndBorrow {
                    operand: borrow_val,
                }));
                return;
            }
            // Trivial Named (no clone shim): bitwise load is a correct copy.
            out.push(Instruction {
                kind: InstKind::CopyValue { result, operand },
                span: span.clone(),
            });
        },
        MirTy::Tuple(elems) => {
            let elems = elems.clone();
            let mut cloned: Vec<ValueId> = Vec::with_capacity(elems.len());
            for (i, &elem_ty) in elems.iter().enumerate() {
                let elem = body.alloc_value(ValueDef::guaranteed(elem_ty, operand));
                out.push(Instruction::new(InstKind::TupleExtract {
                    result: elem,
                    operand,
                    index: i as u32,
                }));
                let cl = body.alloc_value(ValueDef::owned(elem_ty));
                emit_clone_recursive(body, ty_arena, clone_lookup, skip_clone_nominal, elem, elem_ty, cl, span, out);
                cloned.push(cl);
            }
            out.push(Instruction::new(InstKind::Tuple {
                result,
                elements: cloned,
            }));
        },
        _ => {
            // Primitive: bitwise load is a correct copy.
            out.push(Instruction {
                kind: InstKind::CopyValue { result, operand },
                span: span.clone(),
            });
        },
    }
}

/// Expand DestroyValue/CopyValue in a single function body.
/// `skip_self`: if this function is the drop machinery (`deinit`/`__drop$T`) for
/// a specific monomorphic type, DestroyValue on *that exact type* is removed
/// instead of expanded, to avoid recursive drop shim → deinit → drop shim
/// cycles. Other types — including other instantiations of the same generic —
/// expand normally.
fn expand_function(
    func: &mut MonoFunction,
    ty_arena: &crate::ty::TyArena,
    shim_lookup: &DropShimLookup,
    clone_lookup: &DropShimLookup,
    skip_self: Option<&(Entity, Vec<TyId>)>,
    skip_clone_nominal: Option<Entity>,
    not_copyable: &HashSet<(Entity, Vec<TyId>)>,
) {
    let Some(body) = &mut func.body else { return };

    // value_remap tracks CopyValue removals: result -> operand
    let mut value_remap: HashMap<ValueId, ValueId> = HashMap::new();
    // Values that were moved via CopyValue on not-Copyable types.
    // DestroyValue on these is a no-op (ownership already transferred).
    let mut moved_values: HashSet<ValueId> = HashSet::new();

    for block_idx in 0..body.blocks.len() {
        let old_insts = std::mem::take(&mut body.blocks[block_idx].insts);
        let mut new_insts: Vec<Instruction> = Vec::with_capacity(old_insts.len());

        for inst in old_insts {
            match &inst.kind {
                InstKind::DestroyValue { operand } => {
                    let operand = *operand;
                    let remapped = remap_value(operand, &value_remap);

                    // Skip destroy on values that were moved (not-Copyable copy_value).
                    if moved_values.contains(&remapped) {
                        if std::env::var("KESTREL_DEBUG_CLONE").is_ok() {
                            eprintln!(
                                "[expand] SKIP destroy on moved value {remapped:?} (orig {operand:?}) in {}",
                                func.name
                            );
                        }
                        continue;
                    }

                    let vty = body.values[operand.index()].ty;

                    match ty_arena.get(vty) {
                        MirTy::Named { entity, type_args } => {
                            // Skip only this shim's own self type — expanding it
                            // would recurse into __drop$Self. A payload that is a
                            // different instantiation of the same generic still drops.
                            if is_drop_self(skip_self, *entity, type_args) {
                                continue;
                            }
                            let key = (*entity, type_args.clone());
                            if let Some(&shim_id) = shim_lookup.get(&key) {
                                new_insts.push(Instruction {
                                    kind: InstKind::Call {
                                        result: None,
                                        callee: Callee::Resolved(shim_id),
                                        args: vec![CallArg {
                                            value: remap_value(operand, &value_remap),
                                            convention: ParamConvention::Consuming,
                                        }],
                                    },
                                    span: inst.span.clone(),
                                });
                            }
                        },
                        // Tuples have no nominal drop shim — destructure and drop
                        // each member inline, recursing through nested tuples.
                        MirTy::Tuple(_) => {
                            let value = remap_value(operand, &value_remap);
                            emit_destroy_recursive(
                                body,
                                ty_arena,
                                shim_lookup,
                                skip_self,
                                value,
                                vty,
                                &inst.span,
                                &mut new_insts,
                            );
                        },
                        _ => {},
                    }
                },

                // DestroyAddr: load the value from the address, then drop it.
                // Expands to: take %tmp = *%addr; <destroy %tmp>. This is the
                // path `lang.drop_in_place` lowers to — so dropping array
                // elements (e.g. the `(String, String)` pairs in `Headers`)
                // flows through here.
                InstKind::DestroyAddr { address, ty } => {
                    let address = remap_value(*address, &value_remap);
                    let ty = *ty;
                    let span = inst.span.clone();

                    match ty_arena.get(ty) {
                        MirTy::Named { entity, type_args } => {
                            if !is_drop_self(skip_self, *entity, type_args) {
                                let key = (*entity, type_args.clone());
                                if let Some(&shim_id) = shim_lookup.get(&key) {
                                    let tmp = body.alloc_value(ValueDef::owned(ty));
                                    new_insts.push(Instruction {
                                        kind: InstKind::Take {
                                            result: tmp,
                                            address,
                                            ty,
                                        },
                                        span: span.clone(),
                                    });
                                    new_insts.push(Instruction {
                                        kind: InstKind::Call {
                                            result: None,
                                            callee: Callee::Resolved(shim_id),
                                            args: vec![CallArg {
                                                value: tmp,
                                                convention: ParamConvention::Consuming,
                                            }],
                                        },
                                        span,
                                    });
                                }
                            }
                        },
                        // Take the whole tuple by value, then destructure-and-drop
                        // its members. Skip when nothing needs dropping so a
                        // trivial tuple address isn't loaded for no reason.
                        MirTy::Tuple(_) => {
                            if ty_needs_drop(ty_arena, shim_lookup, ty) {
                                let tmp = body.alloc_value(ValueDef::owned(ty));
                                new_insts.push(Instruction {
                                    kind: InstKind::Take {
                                        result: tmp,
                                        address,
                                        ty,
                                    },
                                    span: span.clone(),
                                });
                                emit_destroy_recursive(
                                    body,
                                    ty_arena,
                                    shim_lookup,
                                    skip_self,
                                    tmp,
                                    ty,
                                    &span,
                                    &mut new_insts,
                                );
                            }
                        },
                        _ => {},
                    }
                },

                // StoreAssign: destroy the old value at the address, then store the new one.
                // Expands to: take %tmp = *%addr; call __drop$T(%tmp); store_init %addr, %new
                // For non-Named or trivial types, falls through to a plain store_init.
                InstKind::StoreAssign { address, value } => {
                    let address = remap_value(*address, &value_remap);
                    let value = remap_value(*value, &value_remap);
                    let addr_ty = body.values[address.index()].ty;
                    let span = inst.span.clone();

                    let mut expanded = false;
                    if let MirTy::Pointer(pointee) = ty_arena.get(addr_ty) {
                        let pointee = *pointee;
                        if let MirTy::Named { entity, type_args } = ty_arena.get(pointee) {
                            if !is_drop_self(skip_self, *entity, type_args) {
                                let key = (*entity, type_args.clone());
                                if let Some(&shim_id) = shim_lookup.get(&key) {
                                    let tmp = body.alloc_value(ValueDef::owned(pointee));
                                    new_insts.push(Instruction {
                                        kind: InstKind::Take {
                                            result: tmp,
                                            address,
                                            ty: pointee,
                                        },
                                        span: span.clone(),
                                    });
                                    new_insts.push(Instruction {
                                        kind: InstKind::Call {
                                            result: None,
                                            callee: Callee::Resolved(shim_id),
                                            args: vec![CallArg {
                                                value: tmp,
                                                convention: ParamConvention::Consuming,
                                            }],
                                        },
                                        span: span.clone(),
                                    });
                                    expanded = true;
                                }
                            }
                        }
                    }
                    new_insts.push(Instruction {
                        kind: if expanded {
                            InstKind::StoreInit { address, value }
                        } else {
                            InstKind::StoreAssign { address, value }
                        },
                        span,
                    });
                },

                InstKind::CopyValue { result, operand } => {
                    let result = *result;
                    let operand = *operand;

                    // Tuples have no nominal clone shim — deep-clone each member
                    // inline. The operand is @guaranteed in the real cases (a
                    // `PtrRead` result, as in `Pointer.read`, or a `StructExtract`
                    // field in a clone shim); leaving it as a bitwise alias would
                    // double-free the cloned members once their destructors run.
                    let (vty, is_guaranteed_tuple) = {
                        let vd = &body.values[operand.index()];
                        (
                            vd.ty,
                            vd.ownership == Ownership::Guaranteed
                                && matches!(ty_arena.get(vd.ty), MirTy::Tuple(_)),
                        )
                    };
                    if is_guaranteed_tuple {
                        let src = remap_value(operand, &value_remap);
                        emit_clone_recursive(
                            body,
                            ty_arena,
                            clone_lookup,
                            skip_clone_nominal,
                            src,
                            vty,
                            result,
                            &inst.span,
                            &mut new_insts,
                        );
                        continue;
                    }

                    let value_def = &body.values[operand.index()];

                    // Named type with a clone function → BeginBorrow + Call(clone) + EndBorrow
                    if let MirTy::Named { entity, type_args } = ty_arena.get(value_def.ty) {
                        // Skip ONLY inside this type's own clone implementation, to
                        // prevent T.clone → copy T → T.clone recursion. Other methods
                        // that copy a `T` (e.g. `asSlice() { self }`) must still clone
                        // so refcounted fields stay balanced (see double-free note).
                        if skip_clone_nominal != Some(*entity) {
                            let key = (*entity, type_args.clone());
                            if std::env::var("KESTREL_DEBUG_CLONE").is_ok() {
                                let found = clone_lookup.get(&key).is_some();
                                if !found {
                                    eprintln!(
                                        "[expand] CopyValue on Named {entity:?} — NOT in clone_lookup (type_args={:?})",
                                        type_args
                                    );
                                }
                            }
                            if let Some(&clone_id) = clone_lookup.get(&key) {
                                if std::env::var("KESTREL_DEBUG_CLONE").is_ok() {
                                    eprintln!(
                                        "[expand] CopyValue EXPANDED to clone call for {entity:?} in func {}",
                                        func.name
                                    );
                                }
                                let remapped_operand = remap_value(operand, &value_remap);

                                let ptr_ty = ty_arena
                                    .find(|t| matches!(t, MirTy::Pointer(p) if *p == value_def.ty))
                                    .expect(
                                        "Pointer type should be pre-interned for cloneable types",
                                    );
                                let borrow_val = body
                                    .alloc_value(ValueDef::guaranteed(ptr_ty, remapped_operand));

                                new_insts.push(Instruction::new(InstKind::BeginBorrow {
                                    result: borrow_val,
                                    operand: remapped_operand,
                                }));
                                new_insts.push(Instruction {
                                    kind: InstKind::Call {
                                        result: Some(result),
                                        callee: Callee::Resolved(clone_id),
                                        args: vec![CallArg {
                                            value: borrow_val,
                                            convention: ParamConvention::Borrow,
                                        }],
                                    },
                                    span: inst.span,
                                });
                                new_insts.push(Instruction::new(InstKind::EndBorrow {
                                    operand: borrow_val,
                                }));
                                continue;
                            }
                        }
                    }

                    // not-Copyable Named types: CopyValue is a move (alias).
                    // The source is marked as moved so DestroyValue becomes a no-op.
                    // Keyed per-instantiation: only THIS monomorphization's copy
                    // behavior matters (see `not_copyable` construction).
                    if let MirTy::Named { entity, type_args } = ty_arena.get(value_def.ty) {
                        if not_copyable.contains(&(*entity, type_args.clone())) {
                            let target = remap_value(operand, &value_remap);
                            if std::env::var("KESTREL_DEBUG_CLONE").is_ok() {
                                eprintln!(
                                    "[expand] MOVE (not-Copyable): {result:?} = copy_value {operand:?} → alias to {target:?} in {}",
                                    func.name
                                );
                            }
                            value_remap.insert(result, target);
                            moved_values.insert(target);
                            continue;
                        }
                    }

                    // @guaranteed operands are ByRef pointers — CopyValue must be
                    // preserved so codegen loads from the pointer (Option B invariant).
                    // Non-Named @owned types alias trivially.
                    if value_def.ownership == Ownership::Guaranteed {
                        let mut kept = inst;
                        remap_inst_operands(&mut kept.kind, &value_remap);
                        new_insts.push(kept);
                    } else if !matches!(ty_arena.get(value_def.ty), MirTy::Named { .. }) {
                        let target = remap_value(operand, &value_remap);
                        value_remap.insert(result, target);
                    } else {
                        let mut kept = inst;
                        remap_inst_operands(&mut kept.kind, &value_remap);
                        new_insts.push(kept);
                    }
                },

                _ => {
                    let mut kept = inst;
                    remap_inst_operands(&mut kept.kind, &value_remap);
                    new_insts.push(kept);
                },
            }
        }

        body.blocks[block_idx].insts = new_insts;
        remap_terminator(&mut body.blocks[block_idx].terminator.kind, &value_remap);
    }
}

/// Resolve a ValueId through the remap chain (handles transitive A->B->C).
fn remap_value(v: ValueId, remap: &HashMap<ValueId, ValueId>) -> ValueId {
    let mut current = v;
    while let Some(&target) = remap.get(&current) {
        current = target;
    }
    current
}

/// Replace all operand ValueIds in an instruction using the remap table.
fn remap_inst_operands(kind: &mut InstKind, remap: &HashMap<ValueId, ValueId>) {
    if remap.is_empty() {
        return;
    }

    match kind {
        InstKind::CopyValue { operand, .. }
        | InstKind::MoveValue { operand, .. }
        | InstKind::DestroyValue { operand }
        | InstKind::BeginBorrow { operand, .. }
        | InstKind::EndBorrow { operand }
        | InstKind::BeginMutBorrow { operand, .. }
        | InstKind::EndMutBorrow { operand }
        | InstKind::Discriminant { operand, .. }
        | InstKind::StructExtract { operand, .. }
        | InstKind::TupleExtract { operand, .. }
        | InstKind::EnumPayload { operand, .. }
        | InstKind::DestructureStruct { operand, .. }
        | InstKind::DestructureTuple { operand, .. }
        | InstKind::DestructureEnum { operand, .. } => {
            *operand = remap_value(*operand, remap);
        },

        InstKind::Load { address, .. } => {
            *address = remap_value(*address, remap);
        },
        InstKind::CopyAddr { address, .. }
        | InstKind::Take { address, .. }
        | InstKind::BeginBorrowAddr { address, .. }
        | InstKind::BeginMutBorrowAddr { address, .. }
        | InstKind::DestroyAddr { address, .. } => {
            *address = remap_value(*address, remap);
        },
        InstKind::StoreInit { address, value } | InstKind::StoreAssign { address, value } => {
            *address = remap_value(*address, remap);
            *value = remap_value(*value, remap);
        },

        InstKind::Op1 { arg, .. } => {
            *arg = remap_value(*arg, remap);
        },
        InstKind::Op2 { lhs, rhs, .. } => {
            *lhs = remap_value(*lhs, remap);
            *rhs = remap_value(*rhs, remap);
        },
        InstKind::Op3 { a, b, c, .. } => {
            *a = remap_value(*a, remap);
            *b = remap_value(*b, remap);
            *c = remap_value(*c, remap);
        },

        InstKind::Literal { .. } | InstKind::GlobalRef { .. } => {},

        InstKind::Struct { fields, .. } => {
            for (_, v) in fields.iter_mut() {
                *v = remap_value(*v, remap);
            }
        },
        InstKind::Tuple { elements, .. } | InstKind::Array { elements, .. } => {
            for v in elements.iter_mut() {
                *v = remap_value(*v, remap);
            }
        },
        InstKind::Enum { payload, .. } => {
            for v in payload.iter_mut() {
                *v = remap_value(*v, remap);
            }
        },

        InstKind::Call { args, callee, .. } => {
            for arg in args.iter_mut() {
                arg.value = remap_value(arg.value, remap);
            }
            match callee {
                Callee::Thin(v) | Callee::Thick(v) => {
                    *v = remap_value(*v, remap);
                },
                _ => {},
            }
        },
        InstKind::ApplyPartial {
            callee, captures, ..
        } => {
            for v in captures.iter_mut() {
                *v = remap_value(*v, remap);
            }
            match callee {
                Callee::Thin(v) | Callee::Thick(v) => {
                    *v = remap_value(*v, remap);
                },
                _ => {},
            }
        },

        InstKind::FieldAddr { base, .. } => {
            *base = remap_value(*base, remap);
        },

        InstKind::Uninit { .. } => {},
    }
}

/// Replace all operand ValueIds in a terminator using the remap table.
fn remap_terminator(
    kind: &mut crate::terminator::TerminatorKind,
    remap: &HashMap<ValueId, ValueId>,
) {
    use crate::terminator::TerminatorKind;

    if remap.is_empty() {
        return;
    }

    match kind {
        TerminatorKind::Return(v) => {
            *v = remap_value(*v, remap);
        },
        TerminatorKind::Jump { args, .. } => {
            for v in args.iter_mut() {
                *v = remap_value(*v, remap);
            }
        },
        TerminatorKind::Branch {
            condition,
            then_args,
            else_args,
            ..
        } => {
            *condition = remap_value(*condition, remap);
            for v in then_args.iter_mut() {
                *v = remap_value(*v, remap);
            }
            for v in else_args.iter_mut() {
                *v = remap_value(*v, remap);
            }
        },
        TerminatorKind::Switch {
            discriminant,
            cases,
        } => {
            *discriminant = remap_value(*discriminant, remap);
            for arm in cases.iter_mut() {
                for v in arm.args.iter_mut() {
                    *v = remap_value(*v, remap);
                }
            }
        },
        TerminatorKind::Panic(_) | TerminatorKind::Unreachable => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BasicBlock;
    use crate::body::OssaBody;
    use crate::inst::Instruction;
    use crate::item::TypeParamDef;
    use crate::item::function::{FunctionDef, FunctionKind};
    use crate::mono::types::{MonoFunction, MonoModule};
    use crate::terminator::{Terminator, TerminatorKind};
    use crate::ty::{ParamConvention, TyArena};
    use crate::value::ValueDef;
    use crate::{BlockId, Immediate, ValueId};
    use indexmap::IndexMap;

    fn entity(id: u32) -> Entity {
        Entity::from_raw(id)
    }

    fn make_module() -> MonoModule {
        MonoModule::new(TyArena::new())
    }

    fn make_body(insts: Vec<Instruction>, ret_val: ValueId, values: Vec<ValueDef>) -> OssaBody {
        let mut block = BasicBlock::new();
        block.insts = insts;
        block.terminator = Terminator::new(TerminatorKind::Return(ret_val));
        OssaBody {
            values,
            blocks: vec![block],
            entry: BlockId::new(0),
            param_count: 0,
        }
    }

    fn make_mono_func(
        name: &str,
        source: Entity,
        type_args: Vec<TyId>,
        ret: TyId,
        body: Option<OssaBody>,
    ) -> MonoFunction {
        MonoFunction {
            name: name.into(),
            source,
            type_args,
            self_type: None,
            params: vec![],
            ret,
            body,
            extern_info: None,
        }
    }

    #[test]
    fn destroy_value_on_none_removed() {
        let mut module = make_module();
        let i64_ty = module.ty_arena.i64();
        let unit = module.ty_arena.unit();
        let body = make_body(
            vec![
                Instruction::new(InstKind::Literal {
                    result: ValueId::new(1),
                    value: Immediate::i64(42),
                }),
                Instruction::new(InstKind::DestroyValue {
                    operand: ValueId::new(1),
                }),
            ],
            ValueId::new(0),
            vec![ValueDef::owned(unit), ValueDef::owned(i64_ty)],
        );
        module.add_function(make_mono_func("test", entity(1), vec![], unit, Some(body)));
        expand_destroy_copy(&mut module, &indexmap::IndexMap::new());
        let body = module.functions[0].body.as_ref().unwrap();
        assert_eq!(body.blocks[0].insts.len(), 1);
        assert!(matches!(
            body.blocks[0].insts[0].kind,
            InstKind::Literal { .. }
        ));
    }

    #[test]
    fn destroy_value_named_with_shim_becomes_call() {
        let mut module = make_module();
        let unit = module.ty_arena.unit();
        let named_ty = module.ty_arena.named(entity(10), vec![]);
        let body = make_body(
            vec![Instruction::new(InstKind::DestroyValue {
                operand: ValueId::new(1),
            })],
            ValueId::new(0),
            vec![ValueDef::owned(unit), ValueDef::owned(named_ty)],
        );
        let shim_body = make_body(vec![], ValueId::new(0), vec![ValueDef::owned(unit)]);
        module.add_function(make_mono_func(
            "__drop$MyStruct",
            entity(20),
            vec![],
            unit,
            Some(shim_body),
        ));
        module.add_function(make_mono_func("test", entity(1), vec![], unit, Some(body)));
        let mut generic_functions = indexmap::IndexMap::new();
        generic_functions.insert(
            entity(20),
            FunctionDef {
                entity: entity(20),
                name: "__drop$MyStruct".into(),
                kind: FunctionKind::DropShim {
                    nominal: entity(10),
                },
                type_params: vec![],
                params: vec![],
                ret: unit,
                where_clause: None,
                body: None,
                extern_info: None,
            },
        );
        expand_destroy_copy(&mut module, &generic_functions);
        let test_func = module.functions.iter().find(|f| f.name == "test").unwrap();
        let body = test_func.body.as_ref().unwrap();
        assert_eq!(body.blocks[0].insts.len(), 1);
        match &body.blocks[0].insts[0].kind {
            InstKind::Call {
                callee,
                args,
                result,
            } => {
                assert!(matches!(callee, Callee::Resolved(id) if id.index() == 0));
                assert_eq!(args.len(), 1);
                assert_eq!(args[0].value, ValueId::new(1));
                assert_eq!(args[0].convention, ParamConvention::Consuming);
                assert!(result.is_none());
            },
            other => panic!("expected Call, got {:?}", other),
        }
    }

    #[test]
    fn copy_value_on_none_removed_and_remapped() {
        let mut module = make_module();
        let i64_ty = module.ty_arena.i64();
        let unit = module.ty_arena.unit();
        let v3_ty = module.ty_arena.i64();
        let body = make_body(
            vec![
                Instruction::new(InstKind::Literal {
                    result: ValueId::new(1),
                    value: Immediate::i64(42),
                }),
                Instruction::new(InstKind::CopyValue {
                    result: ValueId::new(2),
                    operand: ValueId::new(1),
                }),
                Instruction::new(InstKind::Op1 {
                    result: ValueId::new(3),
                    op: crate::op::Op::Neg(crate::op::IntBits::I64),
                    arg: ValueId::new(2),
                }),
            ],
            ValueId::new(0),
            vec![
                ValueDef::owned(unit),
                ValueDef::owned(i64_ty),
                ValueDef::owned(i64_ty),
                ValueDef::owned(v3_ty),
            ],
        );
        module.add_function(make_mono_func("test", entity(1), vec![], unit, Some(body)));
        expand_destroy_copy(&mut module, &indexmap::IndexMap::new());
        let body = module.functions[0].body.as_ref().unwrap();
        assert_eq!(body.blocks[0].insts.len(), 2);
        match &body.blocks[0].insts[1].kind {
            InstKind::Op1 { arg, .. } => assert_eq!(*arg, ValueId::new(1)),
            other => panic!("expected Op1, got {:?}", other),
        }
    }

    #[test]
    fn copy_value_on_owned_kept() {
        let mut module = make_module();
        let unit = module.ty_arena.unit();
        let named_ty = module.ty_arena.named(entity(10), vec![]);
        let body = make_body(
            vec![Instruction::new(InstKind::CopyValue {
                result: ValueId::new(2),
                operand: ValueId::new(1),
            })],
            ValueId::new(0),
            vec![
                ValueDef::owned(unit),
                ValueDef::owned(named_ty),
                ValueDef::owned(named_ty),
            ],
        );
        module.add_function(make_mono_func("test", entity(1), vec![], unit, Some(body)));
        expand_destroy_copy(&mut module, &indexmap::IndexMap::new());
        let body = module.functions[0].body.as_ref().unwrap();
        assert_eq!(body.blocks[0].insts.len(), 1);
        assert!(matches!(
            body.blocks[0].insts[0].kind,
            InstKind::CopyValue { .. }
        ));
    }

    #[test]
    fn copy_value_transitive_remap() {
        let mut module = make_module();
        let i64_ty = module.ty_arena.i64();
        let unit = module.ty_arena.unit();
        let mut block = BasicBlock::new();
        block.insts = vec![
            Instruction::new(InstKind::Literal {
                result: ValueId::new(1),
                value: Immediate::i64(99),
            }),
            Instruction::new(InstKind::CopyValue {
                result: ValueId::new(2),
                operand: ValueId::new(1),
            }),
            Instruction::new(InstKind::CopyValue {
                result: ValueId::new(3),
                operand: ValueId::new(2),
            }),
        ];
        block.terminator = Terminator::new(TerminatorKind::Return(ValueId::new(3)));
        let body = OssaBody {
            values: vec![
                ValueDef::owned(unit),
                ValueDef::owned(i64_ty),
                ValueDef::owned(i64_ty),
                ValueDef::owned(i64_ty),
            ],
            blocks: vec![block],
            entry: BlockId::new(0),
            param_count: 0,
        };
        module.add_function(make_mono_func(
            "test",
            entity(1),
            vec![],
            i64_ty,
            Some(body),
        ));
        expand_destroy_copy(&mut module, &indexmap::IndexMap::new());
        let body = module.functions[0].body.as_ref().unwrap();
        assert_eq!(body.blocks[0].insts.len(), 1);
        match &body.blocks[0].terminator.kind {
            TerminatorKind::Return(v) => assert_eq!(*v, ValueId::new(1)),
            other => panic!("expected Return, got {:?}", other),
        }
    }

    #[test]
    fn destroy_value_generic_named_with_type_args() {
        let mut module = make_module();
        let unit = module.ty_arena.unit();
        let i64_ty = module.ty_arena.i64();
        let named_ty = module.ty_arena.named(entity(10), vec![i64_ty]);
        let body = make_body(
            vec![Instruction::new(InstKind::DestroyValue {
                operand: ValueId::new(1),
            })],
            ValueId::new(0),
            vec![ValueDef::owned(unit), ValueDef::owned(named_ty)],
        );
        let shim_body = make_body(vec![], ValueId::new(0), vec![ValueDef::owned(unit)]);
        module.add_function(make_mono_func(
            "__drop$Array_Int64",
            entity(20),
            vec![i64_ty],
            unit,
            Some(shim_body),
        ));
        module.add_function(make_mono_func("test", entity(1), vec![], unit, Some(body)));
        let mut generic_functions = indexmap::IndexMap::new();
        generic_functions.insert(
            entity(20),
            FunctionDef {
                entity: entity(20),
                name: "__drop$Array".into(),
                kind: FunctionKind::DropShim {
                    nominal: entity(10),
                },
                type_params: vec![TypeParamDef::new(entity(30), "T")],
                params: vec![],
                ret: unit,
                where_clause: None,
                body: None,
                extern_info: None,
            },
        );
        expand_destroy_copy(&mut module, &generic_functions);
        let test_func = module.functions.iter().find(|f| f.name == "test").unwrap();
        let body = test_func.body.as_ref().unwrap();
        assert_eq!(body.blocks[0].insts.len(), 1);
        assert!(matches!(
            &body.blocks[0].insts[0].kind,
            InstKind::Call { callee: Callee::Resolved(id), .. } if id.index() == 0
        ));
    }
}
