use std::collections::{HashMap, HashSet};

use kestrel_debug::ktrace;
use kestrel_hecs::Entity;

use crate::block::BlockParam;
use crate::body::OssaBody;
use crate::inst::{InstKind, Instruction};
use crate::item::CopyBehavior;
use crate::item::function::{FunctionDef, FunctionKind, ParamDef};
use crate::item::witness::{WitnessDef, WitnessMethodBinding, WitnessMethodKey};
use crate::terminator::{SwitchArm, Terminator, TerminatorKind};
use crate::ty::TyArena;
use crate::ty::{MirTy, ParamConvention};
use crate::ty_query::find_cloneable_protocol;
use crate::value::{Ownership, RootProvenance, ValueDef};
use crate::{FieldIdx, MirModule, TyId, ValueId, VariantIdx};

/// Synthesize `__clone$T` functions for all structs/enums that aren't `not Copyable`.
/// Skips types that already have a user-written `.clone()` via a Cloneable witness.
/// Registers each synthesized shim as a Cloneable witness.
///
/// Exception: a *conditionally* Copyable container (`: not Copyable` + `extend …:
/// Copyable where T: Copyable`) has a generic base of `None`, but its concrete
/// instances may be `Clone` (e.g. `Optional[String]`). Such a type needs a
/// generic `__clone$T` shim so that those instances get a real clone (the per-
/// instance `CopyValue` in its body resolves to a bitwise copy or a payload
/// clone during expand). We synthesize the shim but leave the generic base
/// `None` and register no unconditional witness — the conformance stays
/// per-instance via the stdlib's `where`-gated extension. (See the
/// `Optional[String]` if-let double-free: an unsynthesized shim left the
/// scrutinee copy as a non-refcounting bit-copy that was then double-dropped.)
pub fn synthesize_clone_shims(module: &mut MirModule, next_entity: &mut u32) {
    let Some(cloneable_proto) = find_cloneable_protocol(module) else {
        return;
    };

    // Pre-intern Pointer(Named(entity)) for all candidate types (needed by BeginBorrow)
    for s in module.structs.values() {
        if s.type_info.copy == CopyBehavior::None && s.conditionally_copyable.is_empty() {
            continue;
        }
        let tp_ty_ids: Vec<TyId> = s
            .type_params
            .iter()
            .map(|tp| module.ty_arena.intern(MirTy::TypeParam(tp.entity)))
            .collect();
        let named_ty = module.ty_arena.intern(MirTy::Named {
            entity: s.entity,
            type_args: tp_ty_ids,
        });
        module.ty_arena.pointer(named_ty);
    }
    for e in module.enums.values() {
        if e.type_info.copy == CopyBehavior::None && e.conditionally_copyable.is_empty() {
            continue;
        }
        let tp_ty_ids: Vec<TyId> = e
            .type_params
            .iter()
            .map(|tp| module.ty_arena.intern(MirTy::TypeParam(tp.entity)))
            .collect();
        let named_ty = module.ty_arena.intern(MirTy::Named {
            entity: e.entity,
            type_args: tp_ty_ids,
        });
        module.ty_arena.pointer(named_ty);
    }

    // Pre-intern i32 for enum discriminants
    module.ty_arena.i32();

    // Nominal → its user-written `Cloneable.clone`, keyed off a *declared*
    // conformance and nothing else. This is the ONLY place that decision is
    // made; the result is stored in `type_info.clone_impl` below and every later
    // pass reads it rather than re-deriving it (F7).
    //
    // Must run before the shim witnesses are registered below, or the shims
    // would read back as user clones.
    let clone_key = clone_witness_key(module, cloneable_proto);
    let user_clone_funcs = collect_user_clone_funcs(module, cloneable_proto, &clone_key);
    let has_user_clone: HashSet<Entity> = user_clone_funcs.keys().copied().collect();

    // Build worklist: all structs/enums that don't already have a user clone.
    // Skip closure envs and types with unresolvable fields.
    let closure_env_entities: HashSet<Entity> = module
        .functions
        .values()
        .filter_map(|f| match &f.kind {
            FunctionKind::ClosureCall { env_struct } => Some(*env_struct),
            _ => None,
        })
        .collect();

    // A conditionally-Copyable container has a generic base of `None` but needs
    // a shim for its `Clone` instances — don't let the `None` base skip it.
    let mut worklist: Vec<Entity> = Vec::new();
    for s in module.structs.values() {
        if has_user_clone.contains(&s.entity)
            || closure_env_entities.contains(&s.entity)
            || (s.type_info.copy == CopyBehavior::None && s.conditionally_copyable.is_empty())
            || has_unresolvable_fields_struct(s, &module.ty_arena)
        {
            continue;
        }
        worklist.push(s.entity);
    }
    for e in module.enums.values() {
        if has_user_clone.contains(&e.entity)
            || (e.type_info.copy == CopyBehavior::None && e.conditionally_copyable.is_empty())
            || has_unresolvable_fields_enum(e, &module.ty_arena)
        {
            continue;
        }
        worklist.push(e.entity);
    }

    // Nominal → its synthesized shim.
    //
    // `IndexMap`, not `HashMap`: this map is iterated below to append witnesses
    // via `module.add_witness`, which is a plain `Vec::push`. `module.witnesses`
    // order is load-bearing — `mono::witness::select_most_specific` walks
    // candidates greedily from index 0 and keeps the first of any tied pair, so
    // a reordered tail silently changes which witness a genuinely-incomparable
    // overlap resolves to. That is unobservable today only because every shim
    // witness's `implementing_type` is a distinct nominal, i.e. no two of them
    // can ever tie — one invariant deep.
    //
    // Insertion order is `worklist` order: `module.structs` then
    // `module.enums`, both `IndexMap`s in declaration order. Drained front to
    // back (not `pop()`) so the witness tail and the shim entity numbering both
    // follow declaration order rather than its reverse.
    let mut shim_map: indexmap::IndexMap<Entity, Entity> = indexmap::IndexMap::new();

    for type_entity in worklist {
        if shim_map.contains_key(&type_entity) {
            continue;
        }

        let shim_entity = Entity::from_raw(*next_entity);
        *next_entity += 1;

        let Some(func) = generate_clone_shim(module, type_entity, shim_entity) else {
            continue;
        };

        let name = func.name.clone();
        module.register_name(shim_entity, &name);
        module.functions.insert(shim_entity, func);
        shim_map.insert(type_entity, shim_entity);
    }

    // Store the decision: nominal → the function that clones it. Every later
    // pass reads this instead of re-deriving it (F7). `user_clone_funcs` is
    // chained second so a user `clone()` overwrites a shim for the same nominal:
    // a hand-written clone is always a safe superset of the memberwise one,
    // whereas picking the shim instead can skip a retain and double-free.
    let clone_impls: HashMap<Entity, Entity> = shim_map
        .iter()
        .chain(user_clone_funcs.iter())
        .map(|(&nominal, &func)| (nominal, func))
        .collect();
    for (nominal, func) in clone_impls {
        if let Some(s) = module.structs.get_mut(&nominal) {
            s.type_info.clone_impl = Some(func);
        } else if let Some(e) = module.enums.get_mut(&nominal) {
            e.type_info.clone_impl = Some(func);
        }
    }

    // Register each shim as a Cloneable witness. A conditionally-Copyable
    // container is skipped: its conformance is per-instance (the stdlib's
    // `where T: Copyable` extension), so an unconditional witness here would
    // wrongly claim e.g. `Optional[File]: Cloneable`. The implicit-copy path
    // finds the shim by `FunctionKind::CloneShim`, not via this witness.
    for (&type_entity, &shim_entity) in &shim_map {
        if is_conditional_container(module, type_entity) {
            continue;
        }
        let func = &module.functions[&shim_entity];
        let tp_ty_ids: Vec<TyId> = func
            .type_params
            .iter()
            .map(|tp| {
                module
                    .ty_arena
                    .find(|t| matches!(t, MirTy::TypeParam(e) if *e == tp.entity))
                    .expect("TypeParam should be interned")
            })
            .collect();
        let self_ty = module
            .ty_arena
            .find(|t| {
                matches!(t, MirTy::Named { entity, type_args }
                if *entity == type_entity && *type_args == tp_ty_ids)
            })
            .expect("Named type should be interned");

        let mut witness = WitnessDef::new(cloneable_proto, self_ty);
        witness.add_method(WitnessMethodBinding::new(
            clone_key.clone(),
            shim_entity,
            tp_ty_ids,
        ));
        module.add_witness(witness);
    }

    // Mark `CopyBehavior::Clone` on every shim'd or user-`clone()` type whose
    // copy really needs a deep clone (see [`DeepCloneQuery`]); the rest stay
    // `Bitwise` and a copy is a memcpy.
    //
    // User-`clone()` types are skipped by shim synthesis above, so without
    // this they'd keep their front-end `Bitwise` and the expand pass would
    // degrade a `CopyValue` on one to a bitwise alias instead of a clone,
    // corrupting a heap-backed value stored in a generic container (e.g.
    // quill's `Value` as a `Dictionary[String, Value]` value: its bucket clone
    // shim bit-copied the `Value`, aliasing the inner String).
    //
    // A conditionally-Copyable container keeps its generic `None` base: its
    // per-instance behavior is derived later by `refine_mono_copy_behavior`
    // (`Optional[String]` → Clone, `Optional[Int64]` → Bitwise, `Optional[File]`
    // → None). Overwriting the base to Clone here would break that invariant.
    //
    // Every decision is made before any is written, so the query reads only
    // the front end's `type_info.copy`.
    let needs_clone: Vec<Entity> = {
        let mut query = DeepCloneQuery::new(module, &has_user_clone, &closure_env_entities);
        shim_map
            .keys()
            .chain(user_clone_funcs.keys())
            .copied()
            .filter(|&e| !is_conditional_container(module, e))
            .filter(|&e| query.nominal_needs_clone(e))
            .collect()
    };
    for type_entity in needs_clone {
        if let Some(s) = module.structs.get_mut(&type_entity) {
            s.type_info.copy = CopyBehavior::Clone(cloneable_proto);
        } else if let Some(e) = module.enums.get_mut(&type_entity) {
            e.type_info.copy = CopyBehavior::Clone(cloneable_proto);
        }
    }
}

/// The witness-method key for `Cloneable`'s sole requirement, taken from the
/// **protocol declaration** rather than written out here.
///
/// Both sides of the Cloneable witness table go through this: the shim
/// registration that writes a binding and the scan that reads one. Two
/// independently-typed `"clone"` literals is exactly the pair that can drift,
/// and a drift would silently stop matching user clones (F7). Sourcing the name
/// from `lang/std`'s declaration also means renaming the requirement there
/// propagates here instead of quietly un-matching — the same reason
/// `MirModule::cloneable_protocol` exists instead of a `name.ends_with` test.
///
/// Falls back to the literal only if the protocol didn't lower with exactly one
/// requirement, which would mean `Cloneable`'s shape changed out from under this
/// pass; `ktrace` says so rather than failing silently.
fn clone_witness_key(module: &MirModule, cloneable_proto: Entity) -> WitnessMethodKey {
    if let Some(proto) = module.protocols.get(&cloneable_proto)
        && let [only] = proto.methods.as_slice()
    {
        return WitnessMethodKey::simple(&only.name);
    }
    ktrace!(
        "clone-shim",
        "Cloneable did not lower with exactly one requirement (got {:?}) — \
         falling back to the literal `clone` key",
        module.protocols.get(&cloneable_proto).map(|p| p
            .methods
            .iter()
            .map(|m| m.name.as_str())
            .collect::<Vec<_>>())
    );
    WitnessMethodKey::simple("clone")
}

/// Nominal → its user-written `Cloneable.clone` implementation.
///
/// **A declared `Cloneable` conformance is the only thing that counts.** A method
/// merely *named* `clone()` does not make a type Cloneable and must not be
/// treated as one — that is duck typing in a nominal protocol system, and it
/// silently forced `CopyBehavior::Clone` (and implicit-copy routing) onto any
/// type with an unrelated `clone()`.
///
/// This used to fall back to scanning function names for a `.clone` suffix,
/// justified by a hedged comment claiming an `extend`-defined `clone()` doesn't
/// always surface a witness. That was a misdiagnosis: instrumenting the fallback
/// showed it fired for exactly one type in the entire stdlib —
/// `std.result.Optional`, whose `clone()` lived in a bare `extend Optional[T] {}`
/// with **no conformance clause at all**. Nothing failed to lower; nothing was
/// declared. Declaring `extend Optional[T]: Cloneable where T: Copyable` made the
/// witness appear and the fallback dead, and it is now gone.
///
/// Call this before registering synthesized shims as witnesses — afterwards the
/// scan would report the shims as user clones.
fn collect_user_clone_funcs(
    module: &MirModule,
    cloneable_proto: Entity,
    clone_key: &WitnessMethodKey,
) -> HashMap<Entity, Entity> {
    let mut map: HashMap<Entity, Entity> = HashMap::new();
    for w in module
        .witnesses
        .iter()
        .filter(|w| w.protocol == cloneable_proto)
    {
        let MirTy::Named { entity, .. } = module.ty_arena.get(w.implementing_type) else {
            continue;
        };
        // Full-key equality, matching `mono::witness`. Comparing `.name` alone
        // would be a partial-key match: two bindings differing only in labels
        // would resolve by `Vec` order.
        if let Some(binding) = w.methods.iter().find(|m| m.key == *clone_key) {
            map.insert(*entity, binding.func);
        }
    }
    map
}

/// True for a `: not Copyable` type that is *conditionally* Copyable (has gating
/// positions). Such a type's generic base is `None`, but concrete instances may
/// be `Clone`/`Bitwise` — so it gets a shim while keeping its `None` base and no
/// unconditional Cloneable witness.
fn is_conditional_container(module: &MirModule, entity: Entity) -> bool {
    if let Some(s) = module.structs.get(&entity) {
        return !s.conditionally_copyable.is_empty();
    }
    if let Some(e) = module.enums.get(&entity) {
        return !e.conditionally_copyable.is_empty();
    }
    false
}

fn generate_clone_shim(
    module: &MirModule,
    type_entity: Entity,
    shim_entity: Entity,
) -> Option<FunctionDef> {
    if let Some(s) = module.structs.get(&type_entity) {
        return Some(generate_struct_clone_shim(module, s, shim_entity));
    }
    if let Some(e) = module.enums.get(&type_entity) {
        return Some(generate_enum_clone_shim(module, e, shim_entity));
    }
    None
}

fn generate_struct_clone_shim(
    module: &MirModule,
    struct_def: &crate::item::struct_def::StructDef,
    shim_entity: Entity,
) -> FunctionDef {
    let name = format!("__clone${}", struct_def.name);

    let tp_ty_ids: Vec<TyId> = struct_def
        .type_params
        .iter()
        .map(|tp| {
            module
                .ty_arena
                .find(|t| matches!(t, MirTy::TypeParam(e) if *e == tp.entity))
                .expect("TypeParam should be interned")
        })
        .collect();
    let self_ty = module
        .ty_arena
        .find(|t| {
            matches!(t, MirTy::Named { entity, type_args }
            if *entity == struct_def.entity && *type_args == tp_ty_ids)
        })
        .expect("struct type should be interned");

    let mut body = OssaBody::new();

    // Self param: @guaranteed T (Borrow convention).
    // StructExtract on @guaranteed → @guaranteed fields, CopyValue → @owned clones.
    let self_val = body.alloc_value(ValueDef {
        ty: self_ty,
        ownership: Ownership::Guaranteed,
        borrow_source: None,
        root: RootProvenance::derived(),
        span: None,
    });
    body.param_count = 1;

    let entry = body.alloc_block();
    body.entry = entry;
    body.block_mut(entry).params.push(BlockParam {
        value: self_val,
        ty: self_ty,
        ownership: Ownership::Guaranteed,
    });

    let mut insts = Vec::new();

    let mut cloned_fields: Vec<(FieldIdx, ValueId)> = Vec::new();
    for (i, field_def) in struct_def.fields.iter().enumerate() {
        let fi = FieldIdx::new(i);
        let field_val = body.alloc_value(ValueDef::guaranteed(field_def.ty, self_val));
        insts.push(Instruction::new(InstKind::StructExtract {
            result: field_val,
            operand: self_val,
            field: fi,
        }));
        let cloned_val = body.alloc_value(ValueDef::owned(field_def.ty));
        insts.push(Instruction::new(InstKind::CopyValue {
            result: cloned_val,
            operand: field_val,
        }));
        cloned_fields.push((fi, cloned_val));
    }

    let result_val = body.alloc_value(ValueDef::owned(self_ty));
    insts.push(Instruction::new(InstKind::Struct {
        result: result_val,
        ty: self_ty,
        fields: cloned_fields,
    }));

    body.block_mut(entry).insts = insts;
    body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Return(result_val));

    let mut func = FunctionDef::new(shim_entity, name, self_ty);
    func.kind = FunctionKind::CloneShim {
        nominal: struct_def.entity,
    };
    func.type_params = struct_def.type_params.clone();
    func.params.push(ParamDef::new(
        "self",
        self_val,
        self_ty,
        ParamConvention::Borrow,
    ));
    func.body = Some(body);
    func
}

fn generate_enum_clone_shim(
    module: &MirModule,
    enum_def: &crate::item::enum_def::EnumDef,
    shim_entity: Entity,
) -> FunctionDef {
    let name = format!("__clone${}", enum_def.name);

    let tp_ty_ids: Vec<TyId> = enum_def
        .type_params
        .iter()
        .map(|tp| {
            module
                .ty_arena
                .find(|t| matches!(t, MirTy::TypeParam(e) if *e == tp.entity))
                .expect("TypeParam should be interned")
        })
        .collect();
    let self_ty = module
        .ty_arena
        .find(|t| {
            matches!(t, MirTy::Named { entity, type_args }
            if *entity == enum_def.entity && *type_args == tp_ty_ids)
        })
        .expect("enum type should be interned");

    let i32_ty = module
        .ty_arena
        .find(|t| matches!(t, MirTy::I32))
        .unwrap_or_else(|| {
            module
                .ty_arena
                .find(|t| matches!(t, MirTy::I8))
                .expect("integer type should be interned")
        });

    let mut body = OssaBody::new();

    // Self param: @guaranteed T (Borrow convention)
    let self_val = body.alloc_value(ValueDef {
        ty: self_ty,
        ownership: Ownership::Guaranteed,
        borrow_source: None,
        root: RootProvenance::derived(),
        span: None,
    });
    body.param_count = 1;

    let entry = body.alloc_block();
    body.entry = entry;
    body.block_mut(entry).params.push(BlockParam {
        value: self_val,
        ty: self_ty,
        ownership: Ownership::Guaranteed,
    });

    let mut entry_insts = Vec::new();

    // Discriminant — non-consuming read on @guaranteed
    let disc_val = body.alloc_value(ValueDef::owned(i32_ty));
    entry_insts.push(Instruction::new(InstKind::Discriminant {
        result: disc_val,
        operand: self_val,
    }));

    // Per-variant blocks — EnumPayload on @guaranteed, CopyValue to clone.
    //
    // Each arm returns its result itself rather than jumping to a shared exit
    // block. A shared exit joins every arm's aggregate through one block
    // param (a phi of slot addresses in codegen), which keeps every arm's
    // stack slot alive at once: an 84-case enum got a 5 KB frame. Returning
    // per arm leaves each slot local to its arm, so the backend builds the
    // result straight into the return slot.
    let mut switch_arms = Vec::new();

    for (case_idx, case_def) in enum_def.cases.iter().enumerate() {
        let variant_idx = VariantIdx::new(case_idx);
        let variant_block = body.alloc_block();

        // Forward self (@guaranteed) and disc (@owned) as block args
        let variant_self = body.alloc_value(ValueDef {
            ty: self_ty,
            ownership: Ownership::Guaranteed,
            borrow_source: None,
            root: RootProvenance::derived(),
            span: None,
        });
        let variant_disc = body.alloc_value(ValueDef::owned(i32_ty));
        body.block_mut(variant_block).params.push(BlockParam {
            value: variant_self,
            ty: self_ty,
            ownership: Ownership::Guaranteed,
        });
        body.block_mut(variant_block).params.push(BlockParam {
            value: variant_disc,
            ty: i32_ty,
            ownership: Ownership::Owned,
        });

        let mut variant_insts = Vec::new();
        variant_insts.push(Instruction::new(InstKind::DestroyValue {
            operand: variant_disc,
        }));

        // EnumPayload on @guaranteed → @guaranteed projections
        let mut cloned_payloads = Vec::new();
        for (fi, pf) in case_def.payload_fields.iter().enumerate() {
            let field_idx = FieldIdx::new(fi);
            let payload_val = body.alloc_value(ValueDef::guaranteed(pf.ty, variant_self));
            variant_insts.push(Instruction::new(InstKind::EnumPayload {
                result: payload_val,
                operand: variant_self,
                variant: variant_idx,
                field: field_idx,
            }));
            let cloned_val = body.alloc_value(ValueDef::owned(pf.ty));
            variant_insts.push(Instruction::new(InstKind::CopyValue {
                result: cloned_val,
                operand: payload_val,
            }));
            cloned_payloads.push(cloned_val);
        }

        // Construct cloned enum variant
        let cloned_enum = body.alloc_value(ValueDef::owned(self_ty));
        variant_insts.push(Instruction::new(InstKind::Enum {
            result: cloned_enum,
            enum_ty: self_ty,
            variant: variant_idx,
            payload: cloned_payloads,
        }));

        body.block_mut(variant_block).insts = variant_insts;
        body.block_mut(variant_block).terminator =
            Terminator::new(TerminatorKind::Return(cloned_enum));

        switch_arms.push(SwitchArm {
            pattern: crate::SwitchCase::Variant(variant_idx),
            target: variant_block,
            args: vec![self_val, disc_val],
        });
    }

    // Wildcard block — CopyValue of entire @guaranteed self
    let wildcard_block = body.alloc_block();
    let wildcard_self = body.alloc_value(ValueDef {
        ty: self_ty,
        ownership: Ownership::Guaranteed,
        borrow_source: None,
        root: RootProvenance::derived(),
        span: None,
    });
    let wildcard_disc = body.alloc_value(ValueDef::owned(i32_ty));
    body.block_mut(wildcard_block).params.push(BlockParam {
        value: wildcard_self,
        ty: self_ty,
        ownership: Ownership::Guaranteed,
    });
    body.block_mut(wildcard_block).params.push(BlockParam {
        value: wildcard_disc,
        ty: i32_ty,
        ownership: Ownership::Owned,
    });
    let mut wildcard_insts = Vec::new();
    wildcard_insts.push(Instruction::new(InstKind::DestroyValue {
        operand: wildcard_disc,
    }));
    let wildcard_copy = body.alloc_value(ValueDef::owned(self_ty));
    wildcard_insts.push(Instruction::new(InstKind::CopyValue {
        result: wildcard_copy,
        operand: wildcard_self,
    }));
    body.block_mut(wildcard_block).insts = wildcard_insts;
    body.block_mut(wildcard_block).terminator =
        Terminator::new(TerminatorKind::Return(wildcard_copy));
    switch_arms.push(SwitchArm {
        pattern: crate::SwitchCase::Wildcard,
        target: wildcard_block,
        args: vec![self_val, disc_val],
    });

    body.block_mut(entry).insts = entry_insts;
    body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Switch {
        discriminant: disc_val,
        cases: switch_arms,
    });

    let mut func = FunctionDef::new(shim_entity, name, self_ty);
    func.kind = FunctionKind::CloneShim {
        nominal: enum_def.entity,
    };
    func.type_params = enum_def.type_params.clone();
    func.params.push(ParamDef::new(
        "self",
        self_val,
        self_ty,
        ParamConvention::Borrow,
    ));
    func.body = Some(body);
    func
}

fn ty_contains_unresolvable(ty: TyId, arena: &TyArena) -> bool {
    match arena.get(ty) {
        MirTy::AssociatedProjection { .. } => true,
        MirTy::Named { type_args, .. } => type_args
            .iter()
            .any(|&t| ty_contains_unresolvable(t, arena)),
        MirTy::Pointer(inner) => ty_contains_unresolvable(*inner, arena),
        MirTy::Tuple(elems) => elems.iter().any(|&t| ty_contains_unresolvable(t, arena)),
        // A closure is cloned by retaining its environment whatever its
        // signature, so a projection in it (`escaping (I.A) -> Bool`) does
        // not block the shim.
        _ => false,
    }
}

fn has_unresolvable_fields_struct(s: &crate::item::struct_def::StructDef, arena: &TyArena) -> bool {
    s.fields
        .iter()
        .any(|f| ty_contains_unresolvable(f.ty, arena))
}

fn has_unresolvable_fields_enum(e: &crate::item::enum_def::EnumDef, arena: &TyArena) -> bool {
    e.cases.iter().any(|c| {
        c.payload_fields
            .iter()
            .any(|f| ty_contains_unresolvable(f.ty, arena))
    })
}

/// Why a type needs a deep clone: unconditionally, or only when one of its
/// own type args does. The second form exists for conditionally-Copyable
/// containers, whose copy class is decided per instance (`Optional[Int64]` is
/// bitwise, `Optional[String]` clones) — see `refine_mono_copy_behavior`.
#[derive(Clone, Debug, Default, PartialEq)]
struct CloneNeed {
    always: bool,
    /// Type-param positions whose arg needing a clone makes this type need one.
    params: Vec<usize>,
}

impl CloneNeed {
    const ALWAYS: CloneNeed = CloneNeed {
        always: true,
        params: Vec::new(),
    };

    fn merge(&mut self, other: CloneNeed) {
        if self.always {
            return;
        }
        if other.always {
            *self = CloneNeed::ALWAYS;
            return;
        }
        for p in other.params {
            if !self.params.contains(&p) {
                self.params.push(p);
            }
        }
    }
}

/// Decides whether copying a type takes a deep clone (`CopyBehavior::Clone`)
/// or is a plain bit copy. This is the one place that decision is made for
/// structs and enums; it is written into `type_info.copy` by
/// [`synthesize_clone_shims`] and read by everything after.
///
/// A named field is looked through, not assumed to need a clone: a payload of
/// `Value` (a one-`UInt64` struct) or a payload-free enum is plain bits, so a
/// type built only from such fields stays `Bitwise` and a copy is a memcpy.
/// What does need a clone: a shared (`escaping`) closure, a type with a
/// user-written `clone()` (the refcounted stdlib types: `RcBox`, `CowBox`,
/// `String`, `Array`, …), a non-conditional generic's type param, and anything
/// this pass can't see through. Results are memoized per nominal; a type
/// reached again while its own fields are being scanned is a cycle and counts
/// as needing a clone (a recursive type holds itself through an indirection).
struct DeepCloneQuery<'m> {
    module: &'m MirModule,
    user_clone: &'m HashSet<Entity>,
    closure_envs: &'m HashSet<Entity>,
    memo: HashMap<Entity, CloneNeed>,
}

impl<'m> DeepCloneQuery<'m> {
    fn new(
        module: &'m MirModule,
        user_clone: &'m HashSet<Entity>,
        closure_envs: &'m HashSet<Entity>,
    ) -> Self {
        DeepCloneQuery {
            module,
            user_clone,
            closure_envs,
            memo: HashMap::new(),
        }
    }

    /// Does copying a value of nominal `entity` (generic base) need a clone?
    fn nominal_needs_clone(&mut self, entity: Entity) -> bool {
        self.nominal(entity).always
    }

    fn nominal(&mut self, entity: Entity) -> CloneNeed {
        if let Some(need) = self.memo.get(&entity) {
            return need.clone();
        }
        // Cycle guard: seen again before its fields are done → needs a clone.
        self.memo.insert(entity, CloneNeed::ALWAYS);
        let need = self.compute_nominal(entity);
        ktrace!(
            "clone-shim",
            "deep-clone need of '{}': {:?}",
            self.module
                .entity_names
                .get(&entity)
                .map(|s| s.as_str())
                .unwrap_or("?"),
            need
        );
        self.memo.insert(entity, need.clone());
        need
    }

    fn compute_nominal(&mut self, entity: Entity) -> CloneNeed {
        let module = self.module;
        let (type_params, fields, info, conditional): (_, Vec<TyId>, _, _) =
            if let Some(s) = module.structs.get(&entity) {
                let fields = s.fields.iter().map(|f| f.ty).collect();
                (
                    &s.type_params,
                    fields,
                    &s.type_info,
                    &s.conditionally_copyable,
                )
            } else if let Some(e) = module.enums.get(&entity) {
                let fields = e
                    .cases
                    .iter()
                    .flat_map(|c| c.payload_fields.iter().map(|f| f.ty))
                    .collect();
                (
                    &e.type_params,
                    fields,
                    &e.type_info,
                    &e.conditionally_copyable,
                )
            } else {
                // Not a struct or enum this pass can see into.
                return CloneNeed::ALWAYS;
            };

        let is_conditional = !conditional.is_empty();
        let opaque = self.closure_envs.contains(&entity)
            || fields
                .iter()
                .any(|&t| ty_contains_unresolvable(t, &module.ty_arena))
            // Already a clone type (front end), or not copyable at all.
            || matches!(info.copy, CopyBehavior::Clone(_))
            // Something to drop means something a bit copy would share.
            || (info.drop != crate::DropBehavior::None && !is_conditional)
            || (info.copy == CopyBehavior::None && !is_conditional)
            // A Cloneable-only grant is never a bit copy.
            || conditional.clone_only;
        if opaque {
            return CloneNeed::ALWAYS;
        }

        // A user `clone()` is a promise the copy runs it. Kept to the old
        // shallow rule (any named field) so a hand-written clone on a type
        // with only primitive fields stays ignored, exactly as before.
        if self.user_clone.contains(&entity) && !is_conditional {
            let shallow = fields
                .iter()
                .any(|&t| ty_has_named_field(&module.ty_arena, t));
            return CloneNeed {
                always: shallow,
                params: Vec::new(),
            };
        }

        let params: Vec<Entity> = type_params.iter().map(|tp| tp.entity).collect();
        let mut need = CloneNeed::default();
        for ty in fields {
            need.merge(self.ty_need(ty, &params));
            if need.always {
                return need;
            }
        }
        // A non-conditional generic has one copy class for every instance,
        // so a type-param field means "may hold something that clones".
        if !is_conditional && !need.params.is_empty() {
            return CloneNeed::ALWAYS;
        }
        need
    }

    /// What a field of type `ty` needs, inside a nominal whose type params
    /// are `params`.
    fn ty_need(&mut self, ty: TyId, params: &[Entity]) -> CloneNeed {
        let arena = &self.module.ty_arena;
        match arena.get(ty).clone() {
            MirTy::TypeParam(p) => match params.iter().position(|&q| q == p) {
                Some(i) => CloneNeed {
                    always: false,
                    params: vec![i],
                },
                None => CloneNeed::ALWAYS,
            },
            MirTy::Named { entity, type_args } => {
                let inner = self.nominal(entity);
                if inner.always {
                    return inner;
                }
                let mut need = CloneNeed::default();
                for i in inner.params {
                    match type_args.get(i) {
                        Some(&arg) => need.merge(self.ty_need(arg, params)),
                        None => return CloneNeed::ALWAYS,
                    }
                }
                need
            },
            // The `String`-field route for closures (plan D6, lockstep 8): a
            // struct holding an `escaping` closure field must get a clone shim
            // and `type_info.copy = Clone(cloneable_proto)`, so copying the
            // struct SHARES the environment handle (a retain) instead of
            // bit-copying it. `is_shared`, NOT `is_boxed`: a `consuming`
            // closure is NotCopyable, not Cloneable — it makes its container
            // non-Copyable (`copy_behavior` answers `None`, which the aggregate
            // fold propagates) instead of giving it a clone shim that has
            // nothing to clone.
            MirTy::FuncThick { kind, .. } if kind.is_shared() => CloneNeed::ALWAYS,
            // A tuple has no nominal of its own: a struct whose only
            // non-trivial field is `(String, String)` still needs a clone, and
            // its `CopyValue` is deep-cloned element-wise during expand.
            MirTy::Tuple(elems) => {
                let mut need = CloneNeed::default();
                for e in elems {
                    need.merge(self.ty_need(e, params));
                }
                need
            },
            MirTy::AssociatedProjection { .. } => CloneNeed::ALWAYS,
            _ => CloneNeed::default(),
        }
    }
}

/// The old shallow rule, kept for user-`clone()` types: any named or
/// type-param field (or a tuple or shared closure holding one).
fn ty_has_named_field(arena: &TyArena, ty: TyId) -> bool {
    match arena.get(ty) {
        MirTy::Named { .. } | MirTy::TypeParam(_) => true,
        MirTy::FuncThick { kind, .. } => kind.is_shared(),
        MirTy::Tuple(elems) => elems.clone().iter().any(|&e| ty_has_named_field(arena, e)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MirModule;
    use crate::item::TypeInfo;
    use crate::item::protocol::{ProtocolDef, ProtocolMethodDef};
    use crate::item::struct_def::{FieldDef, StructDef};

    /// Build a module with a `Cloneable` protocol and `count` shim-eligible
    /// structs named `S0..S{count-1}`, declared in that order. Each has one
    /// `Named` field so it gets a clone shim.
    fn module_with_cloneable_structs(count: u32) -> MirModule {
        let mut module = MirModule::new("test");
        let unit = module.ty_arena.tuple(vec![]);

        let cloneable = Entity::from_raw(1);
        let mut proto = ProtocolDef::new(cloneable, "Cloneable");
        proto
            .methods
            .push(ProtocolMethodDef::new("clone", vec![], unit));
        module.add_protocol(proto);
        module.cloneable_protocol = Some(cloneable);

        // A payload nominal so every struct has a field that needs deep cloning.
        let payload = Entity::from_raw(2);
        module.register_name(payload, "Payload");
        let payload_ty = module.ty_arena.named(payload, vec![]);

        for i in 0..count {
            let e = Entity::from_raw(100 + i);
            let name = format!("S{i}");
            module.register_name(e, &name);
            module.ty_arena.named(e, vec![]);
            let mut def = StructDef::new(e, &name);
            def.add_field(FieldDef::new("payload", payload_ty));
            def.type_info = TypeInfo::bitwise();
            module.add_struct(def);
        }

        module
    }

    /// The witnesses appended by this pass must follow `module.structs`
    /// declaration order.
    ///
    /// Asserting the *specific* order, not just "the same every run", is the
    /// point: `module.witnesses` is a plain `Vec` and
    /// `mono::witness::select_most_specific` resolves an incomparable overlap
    /// to whichever candidate comes first in it. A `HashMap` here (F43d) was
    /// deterministic-per-process at best and reordered the tail between runs;
    /// a merely-deterministic-but-arbitrary order would pass a "same twice"
    /// check while still being the wrong answer.
    #[test]
    fn shim_witnesses_are_appended_in_module_struct_order() {
        let mut module = module_with_cloneable_structs(8);
        let struct_order: Vec<Entity> = module.structs.keys().copied().collect();

        let mut next_entity = 1000;
        synthesize_clone_shims(&mut module, &mut next_entity);

        let witness_order: Vec<Entity> = module
            .witnesses
            .iter()
            .map(|w| match module.ty_arena.get(w.implementing_type) {
                MirTy::Named { entity, .. } => *entity,
                other => panic!("shim witness on a non-nominal type: {other:?}"),
            })
            .collect();

        assert_eq!(
            witness_order, struct_order,
            "clone-shim witnesses must be appended in `module.structs` order — \
             witness position decides which candidate an incomparable overlap \
             resolves to (see mono::witness::select_most_specific)"
        );
    }

    /// Two independent builds of the same module produce byte-identical
    /// witness tails. Complements the order assertion above: that one pins
    /// *which* order, this one pins that the order is a function of the module
    /// and nothing else.
    #[test]
    fn shim_witness_order_is_reproducible() {
        let run = || {
            let mut module = module_with_cloneable_structs(8);
            let mut next_entity = 1000;
            synthesize_clone_shims(&mut module, &mut next_entity);
            module
                .witnesses
                .iter()
                .map(|w| format!("{:?}", module.ty_arena.get(w.implementing_type)))
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    // ── Deep-clone decision ────────────────────────────────────────────────

    use crate::item::enum_def::{EnumCaseDef, EnumDef};

    /// A module with only the `Cloneable` protocol registered.
    fn module_with_cloneable() -> MirModule {
        let mut module = MirModule::new("test");
        let unit = module.ty_arena.tuple(vec![]);
        let cloneable = Entity::from_raw(1);
        let mut proto = ProtocolDef::new(cloneable, "Cloneable");
        proto
            .methods
            .push(ProtocolMethodDef::new("clone", vec![], unit));
        module.add_protocol(proto);
        module.cloneable_protocol = Some(cloneable);
        module
    }

    fn add_struct(module: &mut MirModule, raw: u32, name: &str, fields: &[TyId]) -> TyId {
        let e = Entity::from_raw(raw);
        module.register_name(e, name);
        let ty = module.ty_arena.named(e, vec![]);
        let mut def = StructDef::new(e, name);
        for (i, &f) in fields.iter().enumerate() {
            def.add_field(FieldDef::new(format!("f{i}"), f));
        }
        def.type_info = TypeInfo::bitwise();
        module.add_struct(def);
        ty
    }

    /// An enum whose case `i` carries `cases[i]` as its payload.
    fn add_enum(module: &mut MirModule, raw: u32, name: &str, cases: &[&[TyId]]) -> TyId {
        let e = Entity::from_raw(raw);
        module.register_name(e, name);
        let ty = module.ty_arena.named(e, vec![]);
        let mut def = EnumDef::new(e, name);
        for (i, payload) in cases.iter().enumerate() {
            let fields = payload
                .iter()
                .enumerate()
                .map(|(j, &t)| FieldDef::new(format!("p{j}"), t))
                .collect();
            def.add_case(EnumCaseDef::with_payload(format!("C{i}"), i as u32, fields));
        }
        def.type_info = TypeInfo::bitwise();
        module.add_enum(def);
        ty
    }

    fn copy_of(module: &MirModule, raw: u32) -> CopyBehavior {
        let e = Entity::from_raw(raw);
        module
            .structs
            .get(&e)
            .map(|s| s.type_info.copy.clone())
            .or_else(|| module.enums.get(&e).map(|e| e.type_info.copy.clone()))
            .expect("nominal exists")
    }

    /// A field whose type is itself plain bits doesn't make its container a
    /// clone type: `enum E { case A(Value) case B(Int64) }` stays `Bitwise`,
    /// so a copy is a memcpy and never calls `__clone$E`.
    #[test]
    fn plain_bits_named_fields_stay_bitwise() {
        let mut module = module_with_cloneable();
        let i64_ty = module.ty_arena.i64();
        let value = add_struct(&mut module, 10, "Value", &[i64_ty]);
        let bin_op = add_enum(&mut module, 11, "BinOp", &[&[], &[]]);
        let e = add_enum(
            &mut module,
            12,
            "E",
            &[&[value], &[i64_ty], &[bin_op, i64_ty]],
        );
        add_struct(&mut module, 13, "Holder", &[value, e]);

        let mut next_entity = 1000;
        synthesize_clone_shims(&mut module, &mut next_entity);

        for raw in [10, 11, 12, 13] {
            assert_eq!(
                copy_of(&module, raw),
                CopyBehavior::Bitwise,
                "nominal {raw}"
            );
        }
    }

    /// A clone type nested any depth down still makes every container a
    /// clone type.
    #[test]
    fn nested_clone_field_makes_containers_clone() {
        let mut module = module_with_cloneable();
        let i64_ty = module.ty_arena.i64();
        // Stands in for `String`: the front end already decided it clones.
        let string = add_struct(&mut module, 10, "Str", &[i64_ty]);
        module
            .structs
            .get_mut(&Entity::from_raw(10))
            .unwrap()
            .type_info
            .copy = CopyBehavior::Clone(Entity::from_raw(10));
        let inner = add_struct(&mut module, 11, "Inner", &[i64_ty, string]);
        let tuple = module.ty_arena.tuple(vec![i64_ty, inner]);
        add_enum(&mut module, 12, "Outer", &[&[i64_ty], &[tuple]]);

        let mut next_entity = 1000;
        synthesize_clone_shims(&mut module, &mut next_entity);

        let proto = module.cloneable_protocol.unwrap();
        assert_eq!(copy_of(&module, 11), CopyBehavior::Clone(proto));
        assert_eq!(copy_of(&module, 12), CopyBehavior::Clone(proto));
    }

    /// A type this pass can't see into (no struct or enum def) is assumed to
    /// need a clone.
    #[test]
    fn unknown_nominal_field_needs_clone() {
        let mut module = module_with_cloneable();
        let opaque = module.ty_arena.named(Entity::from_raw(99), vec![]);
        add_struct(&mut module, 10, "HoldsOpaque", &[opaque]);

        let mut next_entity = 1000;
        synthesize_clone_shims(&mut module, &mut next_entity);

        let proto = module.cloneable_protocol.unwrap();
        assert_eq!(copy_of(&module, 10), CopyBehavior::Clone(proto));
    }

    /// Types that reach themselves count as needing a clone.
    #[test]
    fn cycle_counts_as_needing_clone() {
        let mut module = module_with_cloneable();
        let a = Entity::from_raw(10);
        let b = Entity::from_raw(11);
        module.register_name(a, "A");
        module.register_name(b, "B");
        let a_ty = module.ty_arena.named(a, vec![]);
        let b_ty = module.ty_arena.named(b, vec![]);
        let mut a_def = StructDef::new(a, "A");
        a_def.add_field(FieldDef::new("b", b_ty));
        a_def.type_info = TypeInfo::bitwise();
        module.add_struct(a_def);
        let mut b_def = StructDef::new(b, "B");
        b_def.add_field(FieldDef::new("a", a_ty));
        b_def.type_info = TypeInfo::bitwise();
        module.add_struct(b_def);

        let mut next_entity = 1000;
        synthesize_clone_shims(&mut module, &mut next_entity);

        let proto = module.cloneable_protocol.unwrap();
        assert_eq!(copy_of(&module, 10), CopyBehavior::Clone(proto));
        assert_eq!(copy_of(&module, 11), CopyBehavior::Clone(proto));
    }

    /// Every arm of an enum clone shim returns its own result. A shared exit
    /// block would join all the arms' aggregates through one block param,
    /// which in codegen keeps one stack slot per arm alive at once.
    #[test]
    fn enum_shim_arms_return_directly() {
        let mut module = module_with_cloneable();
        let i64_ty = module.ty_arena.i64();
        let shared = module.ty_arena.intern(MirTy::FuncThick {
            kind: crate::ty::FnKind::Escaping,
            params: vec![],
            ret: i64_ty,
        });
        add_enum(&mut module, 10, "E", &[&[i64_ty], &[shared], &[]]);

        let mut next_entity = 1000;
        synthesize_clone_shims(&mut module, &mut next_entity);

        let shim = module.enums[&Entity::from_raw(10)]
            .type_info
            .clone_impl
            .expect("E gets a clone shim");
        let body = module.functions[&shim].body.as_ref().unwrap();
        for block in &body.blocks[1..] {
            assert!(
                matches!(block.terminator.kind, TerminatorKind::Return(_)),
                "every arm returns directly, got {:?}",
                block.terminator.kind
            );
        }
        assert_eq!(
            copy_of(&module, 10),
            CopyBehavior::Clone(module.cloneable_protocol.unwrap())
        );
    }
}
