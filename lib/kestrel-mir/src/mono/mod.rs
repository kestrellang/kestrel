pub mod audit;
pub mod collect;
pub mod expand;
pub mod mangle;
pub mod types;
pub mod verify;
pub mod witness;

pub use collect::CollectionResult;
pub use types::{
    InstantiationKey, MonoEnum, MonoEnumCase, MonoField, MonoFunction, MonoModule, MonoParam,
    MonoStruct, MonoTypeKey,
};
pub use verify::{MonoVerifyError, MonoVerifyResult};
pub use witness::MonoError;

use std::borrow::Cow;
use std::collections::HashMap;

use indexmap::IndexMap;
use kestrel_copy_fold::{CopyLayer, CopySemantics, instance_semantics};
use kestrel_hecs::Entity;

use crate::body::OssaBody;
use crate::callee::Callee;
use crate::immediate::ImmediateKind;
use crate::inst::InstKind;
use crate::item::enum_def::EnumDef;
use crate::item::function::{FunctionDef, FunctionKind, WhereConstraint};
use crate::item::protocol::ProtocolDef;
use crate::item::struct_def::StructDef;
use crate::item::witness::WitnessDef;
use crate::item::{Layout, TargetConfig};
use crate::layout::StructLayout;
use crate::substitute::{SubstMap, substitute};
use crate::ty::{MirTy, TyArena};
use crate::value::Ownership;
use crate::{CopyBehavior, MirModule, MonoFuncId, TyId};

/// Check if a function needs self_type in its InstantiationKey.
///
/// Monomorphize a generic MirModule into a concrete MonoModule.
pub fn monomorphize(
    module: MirModule,
    target: &TargetConfig,
) -> Result<MonoModule, Vec<MonoError>> {
    // Destructure to split borrows: &mut ty_arena alongside &functions etc.
    let MirModule {
        name: _,
        functions,
        structs,
        enums,
        protocols,
        witnesses,
        statics,
        mut ty_arena,
        entity_names,
        copyable_protocol,
        cloneable_protocol,
        // Condition lowering is complete before mono: mir-lower has already
        // decided raw-branch vs `boolValue()` witness call, and the resulting
        // `Callee::Witness` is resolved by the ordinary witness machinery.
        boolean_conditional_protocol: _,
        bool_struct: _,
    } = module;

    // Phase 1: Instantiation discovery
    let CollectionResult {
        instantiations,
        witness_cache,
    } = collect::collect_all(
        &functions,
        &structs,
        &enums,
        &protocols,
        &witnesses,
        &mut ty_arena,
        &entity_names,
    )?;

    // Phase 2: Body monomorphization
    let mut mono_bodies: Vec<MonoBodyResult> = Vec::with_capacity(instantiations.len());

    let _ = witness_cache;
    for key in &instantiations {
        let result = monomorphize_body(
            &mut ty_arena,
            &functions,
            &protocols,
            &witnesses,
            &entity_names,
            key,
        );
        mono_bodies.push(result);
    }

    // Phase 3: ID assignment + callee rewriting
    let func_id_map: HashMap<InstantiationKey, MonoFuncId> = instantiations
        .iter()
        .enumerate()
        .map(|(i, key)| (key.clone(), MonoFuncId::new(i)))
        .collect();

    for body_result in mono_bodies.iter_mut() {
        rewrite_callees(body_result, &func_id_map, &functions);
    }

    // Phase 4: Type and layout resolution
    let (mono_structs, mono_enums) = resolve_types_and_layouts(
        &mut ty_arena,
        &structs,
        &enums,
        &witnesses,
        &mono_bodies,
        target,
        // Inert `Clone(_)` payload for escaping-closure values — matches the
        // pre-mono `ty_query` answer so the two sides of the mono boundary
        // agree (lockstep 1). Same lang item `ty_query::find_cloneable_protocol`
        // reads, so the two cannot drift.
        cloneable_protocol,
    );

    // Phase 5: Assembly
    let mut mono_module = MonoModule::new(ty_arena);

    for (i, key) in instantiations.iter().enumerate() {
        let body_result = &mono_bodies[i];

        // #127 backstop: an instantiation that violates an explicit
        // `T: Copyable` where-bound with a non-Copyable concrete type slipped
        // through the frontend (the Copyable-default substitution hole) — its
        // body bit-copies a move-only value, a double-free. Poison the body
        // with a Panic instead of hard-failing the build: such instantiations
        // are often statically reachable yet never executed (Array[T]'s clone
        // shim reaches `Pointer.read[T]`), so only actually running one traps.
        let poisoned_body = functions.get(&key.func_entity).and_then(|func| {
            let violation = violated_copyable_bound(
                func,
                key,
                &mut mono_module.ty_arena,
                &protocols,
                copyable_protocol,
                &witnesses,
                &mono_structs,
                &mono_enums,
            )?;
            let ty = describe_ty(&mono_module.ty_arena, &entity_names, violation);
            let fname = entity_names
                .get(&key.func_entity)
                .map(|s| s.as_str())
                .unwrap_or("<unknown>");
            Some(poison_body(
                body_result.body.as_ref()?,
                format!(
                    "cannot copy non-Copyable type '{ty}': '{fname}' requires \
                     'Copyable' (bound not satisfied by this instantiation)"
                ),
            ))
        });

        let func_name = entity_names
            .get(&key.func_entity)
            .map(|s| s.as_str())
            .unwrap_or("<unknown>");

        // Determine receiver convention for mangling
        let receiver = functions.get(&key.func_entity).and_then(|f| match &f.kind {
            FunctionKind::Method { receiver, .. } => Some(*receiver),
            _ => None,
        });

        // Safety net: resolve any residual projections in key type_args/self_type.
        // Phase 1 should produce fully-resolved keys, but deep_resolve catches
        // edge cases where substitute() couldn't resolve nested projections.
        let resolved_type_args: Vec<TyId> = key
            .type_args
            .iter()
            .map(|&ta| {
                collect::substitute_and_resolve(
                    &mut mono_module.ty_arena,
                    &witnesses,
                    ta,
                    &SubstMap::new(),
                )
            })
            .collect();
        let resolved_self = key.self_type.map(|st| {
            collect::substitute_and_resolve(
                &mut mono_module.ty_arena,
                &witnesses,
                st,
                &SubstMap::new(),
            )
        });

        let mangled_name = mangle::mangle_function(
            &mono_module.ty_arena,
            &entity_names,
            func_name,
            &resolved_type_args,
            resolved_self,
            &body_result.params,
            body_result.ret,
            receiver,
        );

        // ret_borrow is an ABI property of the DECLARED signature, so derive
        // it from the generic `FunctionDef.ret` (same arena — mono extends
        // it). Deriving from the substituted mono ret would wrongly flip a
        // `-> T` instance at `T = &U` to the borrow ABI (stage 2b mints such
        // instances): the body produces an owned pointer scalar there, while
        // a declared `-> &T` keeps `MirTy::Ref` through substitution either
        // way. Synthesized functions absent from `functions` (shims) can't
        // be ret_borrow; fall back to the substituted ret for them.
        let declared_ret = functions
            .get(&key.func_entity)
            .map(|f| f.ret)
            .unwrap_or(body_result.ret);
        let ret_borrow = matches!(
            crate::item::function::ret_convention(&mono_module.ty_arena, declared_ret),
            crate::item::function::RetConvention::RefBorrow { .. }
        );
        mono_module.add_function(MonoFunction {
            name: mangled_name,
            source: key.func_entity,
            type_args: resolved_type_args,
            self_type: resolved_self,
            params: body_result.params.clone(),
            ret: body_result.ret,
            body: poisoned_body.or_else(|| body_result.body.clone()),
            extern_info: body_result.extern_info.clone(),
            is_main: body_result.is_main,
            ret_borrow,
        });
    }

    mono_module.entity_names = entity_names;
    mono_module.structs = mono_structs;
    mono_module.enums = mono_enums;

    // Copy statics (statics aren't monomorphized — use StaticDef directly)
    for s in statics.values() {
        mono_module.statics.insert(s.entity, s.clone());
    }

    Ok(mono_module)
}

// -- #127 Copyable-bound backstop -------------------------------------------

/// Concrete type violating one of `func`'s explicit `T: Copyable` where-bounds
/// in this instantiation, if any. Copy class is read from the post-mono
/// `type_info` (the per-instantiation authority, same as `mono::audit`).
#[allow(clippy::too_many_arguments)]
fn violated_copyable_bound(
    func: &FunctionDef,
    key: &InstantiationKey,
    arena: &mut TyArena,
    protocols: &IndexMap<Entity, ProtocolDef>,
    copyable_protocol: Option<Entity>,
    witnesses: &[WitnessDef],
    mono_structs: &IndexMap<MonoTypeKey, MonoStruct>,
    mono_enums: &IndexMap<MonoTypeKey, MonoEnum>,
) -> Option<TyId> {
    let wc = func.where_clause.as_ref()?;
    let copyable_bounds: Vec<Entity> = wc
        .constraints
        .iter()
        .filter_map(|c| match c {
            WhereConstraint::Implements {
                type_param,
                protocol,
                ..
            } if copyable_protocol == Some(*protocol) => Some(*type_param),
            _ => None,
        })
        .collect();
    if copyable_bounds.is_empty() {
        return None;
    }
    let subst = collect::build_subst(
        func,
        &key.type_args,
        key.self_type,
        arena,
        protocols,
        witnesses,
    );
    for tp in copyable_bounds {
        let Some(&concrete) = subst.type_params.get(&tp) else {
            continue;
        };
        if mono_copy_is_none(arena, mono_structs, mono_enums, concrete) {
            return Some(concrete);
        }
    }
    None
}

/// True when `ty`'s resolved per-instantiation copy class is `None`
/// (move-only). Unknown/never-instantiated types conservatively read as
/// copyable — the backstop only fires on a certain violation.
fn mono_copy_is_none(
    arena: &TyArena,
    mono_structs: &IndexMap<MonoTypeKey, MonoStruct>,
    mono_enums: &IndexMap<MonoTypeKey, MonoEnum>,
    ty: TyId,
) -> bool {
    match arena.get(ty) {
        MirTy::Tuple(elems) => {
            let elems = elems.clone();
            elems
                .iter()
                .any(|&e| mono_copy_is_none(arena, mono_structs, mono_enums, e))
        },
        MirTy::Named { entity, type_args } => {
            let key = (*entity, type_args.clone());
            mono_structs
                .get(&key)
                .map(|s| s.type_info.copy == CopyBehavior::None)
                .or_else(|| {
                    mono_enums
                        .get(&key)
                        .map(|e| e.type_info.copy == CopyBehavior::None)
                })
                .unwrap_or(false)
        },
        _ => false,
    }
}

/// Replace a poisoned instantiation's body with a single Panic block, keeping
/// the parameter value slots (codegen binds arguments to values
/// `0..param_count`) so the signature stays ABI-valid.
fn poison_body(original: &OssaBody, message: String) -> OssaBody {
    let mut block = crate::block::BasicBlock::new();
    block.terminator =
        crate::terminator::Terminator::new(crate::terminator::TerminatorKind::Panic(message));
    OssaBody {
        values: original.values[..original.param_count].to_vec(),
        blocks: vec![block],
        entry: crate::BlockId::new(0),
        param_count: original.param_count,
        value_names: Default::default(),
    }
}

/// Minimal type rendering for the poison message (phase 5 has no assembled
/// MonoModule yet, so `verify::describe_mono_ty` isn't usable here).
fn describe_ty(arena: &TyArena, entity_names: &IndexMap<Entity, String>, ty: TyId) -> String {
    match arena.get(ty) {
        MirTy::Named { entity, type_args } => {
            let name = entity_names
                .get(entity)
                .cloned()
                .unwrap_or_else(|| format!("{entity:?}"));
            if type_args.is_empty() {
                name
            } else {
                let args: Vec<String> = type_args
                    .clone()
                    .iter()
                    .map(|&a| describe_ty(arena, entity_names, a))
                    .collect();
                format!("{}[{}]", name, args.join(", "))
            }
        },
        MirTy::Tuple(elems) => {
            let parts: Vec<String> = elems
                .clone()
                .iter()
                .map(|&e| describe_ty(arena, entity_names, e))
                .collect();
            format!("({})", parts.join(", "))
        },
        other => format!("{other:?}"),
    }
}

// -- Phase 2: Body monomorphization --

struct MonoBodyResult {
    body: Option<OssaBody>,
    params: Vec<MonoParam>,
    ret: TyId,
    extern_info: Option<crate::item::function::ExternInfo>,
    is_main: bool,
}

fn monomorphize_body(
    arena: &mut TyArena,
    functions: &IndexMap<Entity, FunctionDef>,
    protocols: &IndexMap<Entity, ProtocolDef>,
    witnesses: &[WitnessDef],
    entity_names: &IndexMap<Entity, String>,
    key: &InstantiationKey,
) -> MonoBodyResult {
    let func = functions
        .get(&key.func_entity)
        .expect("instantiation key must reference a valid function");

    let subst = collect::build_subst(
        func,
        &key.type_args,
        key.self_type,
        arena,
        protocols,
        witnesses,
    );

    // Substitute param and return types
    let params: Vec<MonoParam> = func
        .params
        .iter()
        .map(|p| {
            MonoParam::with_label(
                &p.name,
                substitute(arena, p.ty, &subst),
                p.convention,
                p.external_label.clone(),
            )
        })
        .collect();
    let ret = substitute(arena, func.ret, &subst);

    let extern_info = func.extern_info.clone();
    let is_main = func.is_main;

    let Some(body) = &func.body else {
        return MonoBodyResult {
            body: None,
            params,
            ret,
            extern_info,
            is_main,
        };
    };

    // Clone and substitute the body
    let mut mono_body = body.clone();
    // Substitute value types
    for value in &mut mono_body.values {
        value.ty = substitute(arena, value.ty, &subst);
    }

    // Substitute block param types
    for block in &mut mono_body.blocks {
        for param in &mut block.params {
            param.ty = substitute(arena, param.ty, &subst);
        }
    }

    // Walk instructions and substitute types
    for block in &mut mono_body.blocks {
        for inst in &mut block.insts {
            substitute_inst(
                arena,
                witnesses,
                protocols,
                functions,
                entity_names,
                &mut inst.kind,
                &subst,
                key.self_type,
            );
        }
        // No terminator substitution needed — MIR terminators carry ValueId only
    }

    // Re-derive ownership after type substitution. Guaranteed values keep their
    // ownership (they represent borrows); everything else becomes Owned.
    for value in &mut mono_body.values {
        if value.ownership != Ownership::Guaranteed {
            value.ownership = Ownership::Owned;
        }
    }
    for block in &mut mono_body.blocks {
        for param in &mut block.params {
            if param.ownership != Ownership::Guaranteed {
                param.ownership = Ownership::Owned;
            }
        }
    }

    // Resolve any AssociatedProjections that survived substitution. This
    // handles cases where substitute() replaces a TypeParam base to produce
    // a concrete-base projection that isn't in the SubstMap's assoc_types.
    let resolve = |arena: &mut TyArena, ty: TyId| -> TyId {
        collect::substitute_and_resolve(arena, witnesses, ty, &subst)
    };
    let params: Vec<MonoParam> = params
        .into_iter()
        .map(|mut p| {
            p.ty = resolve(arena, p.ty);
            p
        })
        .collect();
    let ret = resolve(arena, ret);
    for value in &mut mono_body.values {
        value.ty = resolve(arena, value.ty);
    }
    for block in &mut mono_body.blocks {
        for param in &mut block.params {
            param.ty = resolve(arena, param.ty);
        }
    }

    MonoBodyResult {
        body: Some(mono_body),
        params,
        ret,
        extern_info,
        is_main,
    }
}

/// Substitute types in a single instruction (the OSSA analogue of the
/// older place-based `substitute_rvalue` / `substitute_operand` /
/// `substitute_terminator` helpers).
fn substitute_inst(
    arena: &mut TyArena,
    witnesses: &[WitnessDef],
    protocols: &IndexMap<Entity, ProtocolDef>,
    functions: &IndexMap<Entity, FunctionDef>,
    entity_names: &IndexMap<Entity, String>,
    kind: &mut InstKind,
    subst: &SubstMap,
    parent_self: Option<TyId>,
) {
    // Embedded types must be `substitute_and_resolve`d, not just `substitute`d:
    // substitution replaces a projection's TypeParam base with a concrete type
    // but leaves the projection itself in place (it isn't keyed in the SubstMap's
    // assoc_types). Only `deep_resolve` runs the witness lookup that turns
    // `Array[Int64].TargetIterator.Item` into `Int64`. The value/block-param
    // pass at the call site re-resolves those, but instruction-embedded types
    // (enum/struct/array construction, addr ops) are only seen here — without
    // this, a surviving `AssociatedProjection` in e.g. an `Optional` enum payload
    // fails post-mono verify ("AssociatedProjection in Enum type").
    let resolve = |arena: &mut TyArena, ty: TyId| {
        collect::substitute_and_resolve(arena, witnesses, ty, subst)
    };
    for (_, callee) in kind.callees_mut() {
        substitute_callee_and_resolve(
            arena,
            witnesses,
            protocols,
            functions,
            entity_names,
            callee,
            subst,
            parent_self,
        );
    }

    match kind {
        // Memory access instructions with embedded type
        InstKind::CopyAddr { ty, .. }
        | InstKind::Take { ty, .. }
        | InstKind::BeginBorrowAddr { ty, .. }
        | InstKind::BeginMutBorrowAddr { ty, .. }
        | InstKind::DestroyAddr { ty, .. }
        | InstKind::FieldAddr { ty, .. }
        | InstKind::Uninit { ty, .. } => {
            *ty = resolve(arena, *ty);
        },

        // Aggregate construction
        InstKind::Struct { ty, .. } => {
            *ty = resolve(arena, *ty);
        },
        InstKind::Enum { enum_ty, .. } => {
            *enum_ty = resolve(arena, *enum_ty);
        },
        InstKind::Array { element_ty, .. } => {
            *element_ty = resolve(arena, *element_ty);
        },

        // Ops with embedded type
        InstKind::Op1 { op, .. } => {
            substitute_op_type(arena, witnesses, op, subst);
        },
        InstKind::Op2 { op, .. } => {
            substitute_op_type(arena, witnesses, op, subst);
        },
        InstKind::Op3 { op, .. } => {
            substitute_op_type(arena, witnesses, op, subst);
        },

        // Constants
        InstKind::Literal { value, .. } => {
            substitute_immediate(arena, witnesses, &mut value.kind, subst);
        },

        // All other InstKinds carry only ValueId — no substitution needed
        _ => {},
    }
}

fn substitute_immediate(
    arena: &mut TyArena,
    witnesses: &[WitnessDef],
    kind: &mut ImmediateKind,
    subst: &SubstMap,
) {
    let resolve = |arena: &mut TyArena, ty: TyId| {
        collect::substitute_and_resolve(arena, witnesses, ty, subst)
    };
    match kind {
        ImmediateKind::SizeOf(ty) | ImmediateKind::AlignOf(ty) | ImmediateKind::NullPtr(ty) => {
            *ty = resolve(arena, *ty);
        },
        ImmediateKind::FunctionRef {
            type_args,
            self_type,
            ..
        } => {
            for ta in type_args.iter_mut() {
                *ta = resolve(arena, *ta);
            }
            if let Some(st) = self_type {
                *st = resolve(arena, *st);
            }
        },
        _ => {},
    }
}

fn substitute_op_type(
    arena: &mut TyArena,
    witnesses: &[WitnessDef],
    op: &mut crate::op::Op,
    subst: &SubstMap,
) {
    // The variant list lives once, in `op::op_ty_variants!` — see the comment
    // there. Hand-rolling it here is what let `collect_named_types` drift out of
    // lockstep with this function (G2).
    let Some(ty) = crate::op::op_type_mut(op) else {
        return;
    };
    *ty = collect::substitute_and_resolve(arena, witnesses, *ty, subst);
}

fn substitute_callee_and_resolve(
    arena: &mut TyArena,
    witnesses: &[WitnessDef],
    protocols: &IndexMap<Entity, ProtocolDef>,
    functions: &IndexMap<Entity, FunctionDef>,
    entity_names: &IndexMap<Entity, String>,
    callee: &mut Callee,
    subst: &SubstMap,
    parent_self: Option<TyId>,
) {
    let resolved_witness = match callee {
        Callee::Direct {
            func,
            type_args,
            self_type,
        } => {
            for ta in type_args.iter_mut() {
                *ta = collect::substitute_and_resolve(arena, witnesses, *ta, subst);
            }
            if let Some(st) = self_type {
                *st = collect::substitute_and_resolve(arena, witnesses, *st, subst);
            }
            // Nested callees (closures/thunks) inherit parent's self_type
            // so rewrite_callee can look them up with the correct key.
            if self_type.is_none()
                && parent_self.is_some()
                && let Some(f) = functions.get(func)
                && matches!(
                    f.kind,
                    FunctionKind::Closure { .. }
                        | FunctionKind::ClosureCall { .. }
                        | FunctionKind::Thunk { .. }
                )
            {
                *self_type = parent_self;
            }
            None
        },
        Callee::Witness {
            protocol,
            method,
            self_type,
            method_type_args,
        } => {
            *self_type = collect::substitute_and_resolve(arena, witnesses, *self_type, subst);
            for ta in method_type_args.iter_mut() {
                *ta = collect::substitute_and_resolve(arena, witnesses, *ta, subst);
            }
            // Resolve witness to concrete function
            let witness_result = witness::resolve_witness_call(
                arena,
                witnesses,
                protocols,
                functions,
                entity_names,
                *protocol,
                method,
                *self_type,
                method_type_args,
            );
            witness_result.ok().map(|resolved| {
                InstantiationKey::new(resolved.func_entity, resolved.type_args, resolved.self_type)
            })
        },
        _ => None,
    };

    if let Some(target) = resolved_witness {
        *callee = Callee::Direct {
            func: target.func_entity,
            type_args: target.type_args,
            self_type: target.self_type,
        };
    }
}

// -- Phase 3: Callee rewriting --

fn rewrite_callees(
    body_result: &mut MonoBodyResult,
    func_id_map: &HashMap<InstantiationKey, MonoFuncId>,
    functions: &IndexMap<Entity, FunctionDef>,
) {
    let Some(body) = &mut body_result.body else {
        return;
    };

    for block in &mut body.blocks {
        for inst in &mut block.insts {
            for (_, callee) in inst.kind.callees_mut() {
                rewrite_callee(callee, func_id_map, functions);
            }
            match &mut inst.kind {
                InstKind::Literal { value, .. } => {
                    if let ImmediateKind::FunctionRef {
                        func,
                        type_args,
                        self_type,
                    } = &value.kind
                    {
                        let key = InstantiationKey::new(*func, type_args.clone(), *self_type);
                        if let Some(&mono_id) = func_id_map.get(&key) {
                            value.kind = ImmediateKind::MonoFunctionRef(mono_id);
                        }
                    }
                },
                _ => {},
            }
        }
    }
}

fn rewrite_callee(
    callee: &mut Callee,
    func_id_map: &HashMap<InstantiationKey, MonoFuncId>,
    functions: &IndexMap<Entity, FunctionDef>,
) {
    match callee {
        Callee::Direct {
            func,
            type_args,
            self_type,
        } => {
            // Mirror collection's arity normalization (collect::scan_callee) so
            // the lookup key matches the key the instance was enqueued under.
            let mut targs = type_args.clone();
            if let Some(f) = functions.get(&*func) {
                collect::normalize_direct_arity(&mut targs, f.type_params.len());
            }
            let key = InstantiationKey::new(*func, targs, *self_type);
            if let Some(&mono_id) = func_id_map.get(&key) {
                *callee = Callee::Resolved(mono_id);
            }
        },
        _ => {},
    }
}

// -- Phase 4: Type and layout resolution --

fn resolve_types_and_layouts(
    arena: &mut TyArena,
    structs: &IndexMap<Entity, StructDef>,
    enums: &IndexMap<Entity, EnumDef>,
    witnesses: &[WitnessDef],
    mono_bodies: &[MonoBodyResult],
    target: &TargetConfig,
    cloneable_proto: Option<Entity>,
) -> (
    IndexMap<MonoTypeKey, MonoStruct>,
    IndexMap<MonoTypeKey, MonoEnum>,
) {
    // Collect all concrete Named types from monomorphized bodies
    let mut concrete_types: IndexMap<(Entity, Vec<TyId>), ConcreteTypeKind> = IndexMap::new();

    for body_result in mono_bodies {
        if let Some(body) = &body_result.body {
            collect_named_types(arena, body, &mut concrete_types, structs, enums);
        }
    }

    // Compute layouts for concrete types (fixed-point loop)
    let mut mono_structs: IndexMap<MonoTypeKey, MonoStruct> = IndexMap::new();
    let mut mono_enums: IndexMap<MonoTypeKey, MonoEnum> = IndexMap::new();
    let mut layout_cache: HashMap<(Entity, Vec<TyId>), (u64, u64)> = HashMap::new();

    // Fixed-point: loop until no progress (handles dependency chains).
    //
    // Two things can advance a pass: a layout gets resolved (`progress`), or a
    // *field* type is discovered that no body ever mentioned (`discovered`).
    // The latter must be staged in a separate map because the `for` below holds
    // a shared borrow of `concrete_types` — before G2 that borrow made the loop
    // structurally unable to grow its own worklist, so a struct whose field
    // type was never seeded got `all_resolved = false` and was dropped from
    // `mono_structs` entirely, with no diagnostic and a pointer-sized fallback
    // layout at both backends.
    //
    // Termination: every pass either resolves a layout (finite: one per key) or
    // inserts a key that was not already in `concrete_types`. The key universe
    // is finite for any program mono can compile at all — unbounded field-type
    // growth needs polymorphic recursion, which is impossible by value and
    // already diverges in function collection when hidden behind a `Pointer`.
    loop {
        let mut progress = false;
        // Field types found this pass. Populated unconditionally — a field
        // whose own layout is still unknown is exactly the one we must seed.
        let mut discovered: IndexMap<(Entity, Vec<TyId>), ConcreteTypeKind> = IndexMap::new();

        for ((entity, type_args), kind) in &concrete_types {
            let cache_key = (*entity, type_args.clone());
            if layout_cache.contains_key(&cache_key) {
                continue;
            }

            match kind {
                ConcreteTypeKind::Struct(struct_entity) => {
                    let sdef = &structs[struct_entity];
                    let subst =
                        build_type_subst(sdef.type_params.iter().map(|tp| tp.entity), type_args);

                    let mut layout = StructLayout::new();
                    let mut all_resolved = true;
                    let mut fields = Vec::new();

                    for field in &sdef.fields {
                        let concrete_ty =
                            collect::substitute_and_resolve(arena, witnesses, field.ty, &subst);
                        fields.push(MonoField::new(&field.name, concrete_ty));
                        collect_named_type_from_ty(
                            arena,
                            concrete_ty,
                            &mut discovered,
                            structs,
                            enums,
                        );

                        if let Some((size, align)) =
                            mono_size_and_align(arena, concrete_ty, target, &layout_cache)
                        {
                            layout.append_field(StructLayout::scalar(size, align));
                        } else {
                            all_resolved = false;
                            break;
                        }
                    }

                    if all_resolved {
                        layout.pad_to_align();
                        layout_cache.insert(cache_key, (layout.size, layout.align));
                        let mut ms = MonoStruct::new(*entity, type_args.clone());
                        ms.fields = fields;
                        ms.type_info = sdef.type_info.clone();
                        ms.type_info.layout = Some(Layout::Struct(layout));
                        mono_structs.insert((*entity, type_args.clone()), ms);
                        progress = true;
                    }
                },
                ConcreteTypeKind::Enum(enum_entity) => {
                    let edef = &enums[enum_entity];
                    let subst =
                        build_type_subst(edef.type_params.iter().map(|tp| tp.entity), type_args);

                    let mut all_resolved = true;
                    let mut cases = Vec::new();
                    let mut variant_layouts = Vec::new();

                    for case in &edef.cases {
                        let mut case_layout = StructLayout::new();
                        let mut mono_fields = Vec::new();
                        for field in &case.payload_fields {
                            let concrete_ty =
                                collect::substitute_and_resolve(arena, witnesses, field.ty, &subst);
                            mono_fields.push(MonoField::new(&field.name, concrete_ty));
                            collect_named_type_from_ty(
                                arena,
                                concrete_ty,
                                &mut discovered,
                                structs,
                                enums,
                            );
                            if let Some((size, align)) =
                                mono_size_and_align(arena, concrete_ty, target, &layout_cache)
                            {
                                case_layout.append_field(StructLayout::scalar(size, align));
                            } else {
                                all_resolved = false;
                                break;
                            }
                        }
                        if !all_resolved {
                            break;
                        }
                        case_layout.pad_to_align();
                        variant_layouts.push(case_layout);
                        let mut mc = MonoEnumCase::new(&case.name, case.discriminant);
                        mc.payload_fields = mono_fields;
                        cases.push(mc);
                    }

                    if all_resolved {
                        let enum_layout = build_enum_layout(&variant_layouts, edef.cases.len());
                        layout_cache.insert(cache_key, (enum_layout.size, enum_layout.align));
                        let mut me = MonoEnum::new(
                            *entity,
                            type_args.clone(),
                            enum_layout.discriminant_width,
                        );
                        me.cases = cases;
                        me.type_info = edef.type_info.clone();
                        me.type_info.layout = Some(Layout::Enum(enum_layout.clone()));
                        mono_enums.insert((*entity, type_args.clone()), me);
                        progress = true;
                    }
                },
            }
        }

        // Legal only outside the shared-borrow iteration above.
        let mut added_new = false;
        for (key, kind) in discovered {
            if !concrete_types.contains_key(&key) {
                concrete_types.insert(key, kind);
                added_new = true;
            }
        }

        if !progress && !added_new {
            break;
        }
    }

    refine_mono_copy_behavior(
        arena,
        structs,
        enums,
        &mut mono_structs,
        &mut mono_enums,
        cloneable_proto,
    );

    (mono_structs, mono_enums)
}

/// Re-derive per-instantiation `copy` behavior for conditional containers.
///
/// `MonoStruct`/`MonoEnum.type_info.copy` is cloned from the *generic* def,
/// which for a conditional container (`: not Copyable` + `extend …: Copyable
/// where T: Copyable`) is `None`. For a concrete instantiation the type is
/// Copyable iff every gating arg (the positions in
/// `StructDef::conditionally_copyable`) is itself Copyable — so `Result[Int64,
/// Error]` becomes `Bitwise` while `Result[Array[Int64], E]` stays `None`.
/// The fold is the shared decision tree (`kestrel_copy_fold::instance_semantics`
/// via `MonoCopyLayer`) over already-mono types (args are concrete, so member
/// lookups hit the mono maps instead of threading a `where_clause`).
///
/// Inert for types with no gating positions. A fixed point handles nesting
/// (`Result[Optional[Int], E]`): each round refines types whose gating children
/// are already refined, until no `copy` field changes.
fn refine_mono_copy_behavior(
    arena: &TyArena,
    structs: &IndexMap<Entity, StructDef>,
    enums: &IndexMap<Entity, EnumDef>,
    mono_structs: &mut IndexMap<MonoTypeKey, MonoStruct>,
    mono_enums: &mut IndexMap<MonoTypeKey, MonoEnum>,
    cloneable_proto: Option<Entity>,
) {
    loop {
        let layer = MonoCopyLayer {
            arena,
            structs,
            enums,
            mono_structs,
            mono_enums,
            cloneable_proto,
        };
        // (key, is_struct, new copy) — collected read-only, applied after.
        let mut updates: Vec<(MonoTypeKey, bool, CopyBehavior)> = Vec::new();
        for (key, ms) in mono_structs.iter() {
            // Cheap filter: only conditional containers can change.
            if structs[&ms.source].conditionally_copyable.is_empty() {
                continue;
            }
            let want = instance_semantics(&layer, ms.source, &ms.type_args);
            if want != ms.type_info.copy {
                updates.push((key.clone(), true, want));
            }
        }
        for (key, me) in mono_enums.iter() {
            if enums[&me.source].conditionally_copyable.is_empty() {
                continue;
            }
            let want = instance_semantics(&layer, me.source, &me.type_args);
            if want != me.type_info.copy {
                updates.push((key.clone(), false, want));
            }
        }
        if updates.is_empty() {
            break;
        }
        for (key, is_struct, cb) in updates {
            if is_struct {
                if let Some(ms) = mono_structs.get_mut(&key) {
                    ms.type_info.copy = cb;
                }
            } else if let Some(me) = mono_enums.get_mut(&key) {
                me.type_info.copy = cb;
            }
        }
    }
}

/// `CopyLayer` over already-monomorphized `TyId`s — the mono refine pass's
/// hooks into the shared decision tree (`kestrel_copy_fold::instance_semantics`,
/// the single source of truth for per-instantiation copy semantics across
/// semantics / solver / analyze / MIR). Layer-specific plumbing: base + gating
/// come from the *generic* defs, member lookups hit the mono maps (no
/// `where_clause` — args are concrete).
struct MonoCopyLayer<'a> {
    arena: &'a TyArena,
    structs: &'a IndexMap<Entity, StructDef>,
    enums: &'a IndexMap<Entity, EnumDef>,
    mono_structs: &'a IndexMap<MonoTypeKey, MonoStruct>,
    mono_enums: &'a IndexMap<MonoTypeKey, MonoEnum>,
    /// Inert `Clone(_)` payload for escaping-closure values (the same protocol
    /// entity `ty_query`/`clone_shim` stamp). Never destructured.
    cloneable_proto: Option<Entity>,
}

impl CopyLayer for MonoCopyLayer<'_> {
    type Ty = TyId;
    type Sem = CopyBehavior;

    fn base_semantics(&self, entity: Entity) -> CopyBehavior {
        // Generic def `type_info.copy`; `None` by the ConditionalCopyableParams
        // invariant whenever gating is non-empty (the only case the refine
        // loop reaches), so the kernel always falls through to the gating fold
        // — exactly the former `conditional_copy` behavior.
        let copy = self
            .structs
            .get(&entity)
            .map(|s| s.type_info.copy.clone())
            .or_else(|| self.enums.get(&entity).map(|e| e.type_info.copy.clone()))
            .unwrap_or(CopyBehavior::Bitwise);
        // Checked form of the invariant above: a pass that flips a conditional
        // container's base away from None would silently skip the gating fold.
        debug_assert!(
            self.gating_positions(entity).is_empty() || matches!(copy, CopyBehavior::None),
            "conditional container's generic def must keep CopyBehavior::None"
        );
        copy
    }

    fn gating_positions(&self, entity: Entity) -> Cow<'_, [usize]> {
        Cow::Borrowed(
            self.structs
                .get(&entity)
                .map(|s| s.conditionally_copyable.as_slice())
                .or_else(|| {
                    self.enums
                        .get(&entity)
                        .map(|e| e.conditionally_copyable.as_slice())
                })
                .unwrap_or(&[]),
        )
    }

    fn member_semantics(&self, &ty: &TyId) -> CopyBehavior {
        // Stays per-layer: mono-map lookups, no where_clause.
        concrete_copy(
            self.arena,
            ty,
            self.mono_structs,
            self.mono_enums,
            self.cloneable_proto,
        )
    }

    fn sem_from_class(&self, entity: Entity, class: CopySemantics) -> CopyBehavior {
        match class {
            CopySemantics::Copyable => CopyBehavior::Bitwise,
            // Container entity payload — matches `ty_query`'s `MirCopyLayer`.
            CopySemantics::Cloneable => CopyBehavior::Clone(entity),
            CopySemantics::NotCopyable => CopyBehavior::None,
        }
    }
}

/// Copy behavior of an already-monomorphized concrete type. Named types are
/// looked up in the mono maps (their `copy` is correct, or being refined this
/// round); primitives/pointers/functions are bit-copyable; tuples use the
/// canonical fold (move-only dominates, else Clone if any element clones).
fn concrete_copy(
    arena: &TyArena,
    ty: TyId,
    mono_structs: &IndexMap<MonoTypeKey, MonoStruct>,
    mono_enums: &IndexMap<MonoTypeKey, MonoEnum>,
    cloneable_proto: Option<Entity>,
) -> CopyBehavior {
    match arena.get(ty) {
        MirTy::Named { entity, type_args } => {
            let key = (*entity, type_args.clone());
            mono_structs
                .get(&key)
                .map(|s| s.type_info.copy.clone())
                .or_else(|| mono_enums.get(&key).map(|e| e.type_info.copy.clone()))
                .unwrap_or(CopyBehavior::Bitwise)
        },
        // Canonical fold (copy-drift #4 resolved 2026-06-10): matches
        // ty_query's tuple rule — move-only element dominates, else first
        // Clone element decides (payload inert), else Bitwise. Previously any
        // non-Bitwise element classified the tuple move-only here, making the
        // two MIR passes disagree.
        MirTy::Tuple(elems) => {
            let mut first_clone = None;
            for &e in elems {
                match concrete_copy(arena, e, mono_structs, mono_enums, cloneable_proto) {
                    CopyBehavior::None => return CopyBehavior::None,
                    b @ CopyBehavior::Clone(_) if first_clone.is_none() => first_clone = Some(b),
                    _ => {},
                }
            }
            first_clone.unwrap_or(CopyBehavior::Bitwise)
        },
        // Explicit arm — the pre-mono `ty_query::copy_behavior` FuncThick arm's
        // post-mono twin (lockstep 1). Falling into the `_` catch-all below
        // would silently answer Bitwise for an owning closure and make the two
        // sides of the mono boundary disagree.
        MirTy::FuncThick { kind, .. } => match (kind, cloneable_proto) {
            (crate::ty::FnKind::Consuming, _) => CopyBehavior::None,
            (crate::ty::FnKind::Escaping, Some(p)) => CopyBehavior::Clone(p),
            _ => CopyBehavior::Bitwise,
        },
        _ => CopyBehavior::Bitwise,
    }
}

enum ConcreteTypeKind {
    Struct(Entity),
    Enum(Entity),
}

/// Walk an OssaBody and collect all concrete Named types.
fn collect_named_types(
    arena: &TyArena,
    body: &OssaBody,
    out: &mut IndexMap<(Entity, Vec<TyId>), ConcreteTypeKind>,
    structs: &IndexMap<Entity, StructDef>,
    enums: &IndexMap<Entity, EnumDef>,
) {
    // Walk value types
    for value in &body.values {
        collect_named_type_from_ty(arena, value.ty, out, structs, enums);
    }
    // Walk block param types
    for block in &body.blocks {
        for param in &block.params {
            collect_named_type_from_ty(arena, param.ty, out, structs, enums);
        }
    }
    // Walk instruction types (Struct.ty, Enum.enum_ty, Array.element_ty, Literal immediates)
    for block in &body.blocks {
        for inst in &block.insts {
            match &inst.kind {
                InstKind::Struct { ty, .. } => {
                    collect_named_type_from_ty(arena, *ty, out, structs, enums);
                },
                InstKind::Enum { enum_ty, .. } => {
                    collect_named_type_from_ty(arena, *enum_ty, out, structs, enums);
                },
                InstKind::Array { element_ty, .. } => {
                    collect_named_type_from_ty(arena, *element_ty, out, structs, enums);
                },
                InstKind::Literal { value, .. } => match &value.kind {
                    ImmediateKind::SizeOf(ty)
                    | ImmediateKind::AlignOf(ty)
                    | ImmediateKind::NullPtr(ty) => {
                        collect_named_type_from_ty(arena, *ty, out, structs, enums);
                    },
                    _ => {},
                },
                // Op type operands. Mirrors `substitute_op_type` via the one
                // canonical variant list in `op::op_ty_variants!`. `SizeOf`/
                // `AlignOf` are the ops whose type genuinely escapes — nothing
                // else in the body mentions it — but seeding all ten costs
                // nothing and keeps the two walks in lockstep by construction.
                InstKind::Op1 { op, .. } | InstKind::Op2 { op, .. } | InstKind::Op3 { op, .. } => {
                    if let Some(ty) = crate::op::op_type(op) {
                        collect_named_type_from_ty(arena, ty, out, structs, enums);
                    }
                },
                // CopyAddr/Take/BeginBorrowAddr/BeginMutBorrowAddr/DestroyAddr/FieldAddr/Uninit
                // carry ty but those are address types (Pointer), not Named
                _ => {},
            }
        }
    }
}

fn collect_named_type_from_ty(
    arena: &TyArena,
    ty: TyId,
    out: &mut IndexMap<(Entity, Vec<TyId>), ConcreteTypeKind>,
    structs: &IndexMap<Entity, StructDef>,
    enums: &IndexMap<Entity, EnumDef>,
) {
    match arena.get(ty) {
        MirTy::Named { entity, type_args } => {
            let entity = *entity;
            let type_args = type_args.clone();
            let key = (entity, type_args.clone());
            if !out.contains_key(&key) {
                if structs.contains_key(&entity) {
                    out.insert(key, ConcreteTypeKind::Struct(entity));
                } else if enums.contains_key(&entity) {
                    out.insert(key, ConcreteTypeKind::Enum(entity));
                }
            }
            // Recurse into type args
            for &arg in &type_args {
                collect_named_type_from_ty(arena, arg, out, structs, enums);
            }
        },
        MirTy::Pointer(inner) => {
            collect_named_type_from_ty(arena, *inner, out, structs, enums);
        },
        MirTy::Ref { pointee, .. } => {
            collect_named_type_from_ty(arena, *pointee, out, structs, enums);
        },
        MirTy::Tuple(elems) => {
            for &elem in elems {
                collect_named_type_from_ty(arena, elem, out, structs, enums);
            }
        },
        _ => {},
    }
}

fn build_type_subst(
    type_param_entities: impl Iterator<Item = Entity>,
    type_args: &[TyId],
) -> SubstMap {
    let mut subst = SubstMap::new();
    for (entity, &arg) in type_param_entities.zip(type_args.iter()) {
        subst.type_params.insert(entity, arg);
    }
    subst
}

// Reuse layout functions from passes/layout.rs
use crate::passes::layout::{build_enum_layout, primitive_size_and_align};

/// Compute size and alignment for a concrete type, looking up mono layouts.
fn mono_size_and_align(
    arena: &TyArena,
    ty: TyId,
    target: &TargetConfig,
    layout_cache: &HashMap<(Entity, Vec<TyId>), (u64, u64)>,
) -> Option<(u64, u64)> {
    if let Some(sa) = primitive_size_and_align(arena.get(ty), target) {
        return Some(sa);
    }
    match arena.get(ty) {
        MirTy::Tuple(elems) => {
            let elems = elems.clone();
            if elems.is_empty() {
                return Some((0, 1));
            }
            let mut layout = StructLayout::new();
            for elem in &elems {
                let (size, align) = mono_size_and_align(arena, *elem, target, layout_cache)?;
                layout.append_field(StructLayout::scalar(size, align));
            }
            layout.pad_to_align();
            Some((layout.size, layout.align))
        },

        MirTy::Named { entity, type_args } => {
            let key = (*entity, type_args.clone());
            layout_cache.get(&key).copied()
        },

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::{BasicBlock, BlockParam};
    use crate::body::OssaBody;
    use crate::callee::Callee;
    use crate::inst::{CallArg, InstKind, Instruction};
    use crate::item::TypeParamDef;
    use crate::item::function::{FunctionDef, FunctionKind, ParamDef};
    use crate::item::protocol::ProtocolDef;
    use crate::item::witness::{WitnessDef, WitnessMethodBinding};
    use crate::terminator::{Terminator, TerminatorKind};
    use crate::ty::ParamConvention;
    use crate::value::ValueDef;
    use crate::op::Op;
    use crate::{BlockId, ValueId, WitnessMethodKey};

    fn entity(id: u32) -> Entity {
        Entity::from_raw(id)
    }

    /// Build a single-block OssaBody with the given instructions, returning `ret_val`.
    fn make_body(insts: Vec<Instruction>, ret_val: ValueId, values: Vec<ValueDef>) -> OssaBody {
        let mut block = BasicBlock::new();
        block.insts = insts;
        block.terminator = Terminator::new(TerminatorKind::Return(ret_val));
        OssaBody {
            values,
            blocks: vec![block],
            entry: BlockId::new(0),
            param_count: 0,
            value_names: Default::default(),
        }
    }

    #[test]
    fn monomorphize_concrete_function() {
        let mut module = MirModule::new("test");
        let unit = module.ty_arena.unit();

        let ret_val = ValueId::new(0);
        let func = FunctionDef {
            entity: entity(1),
            name: "main".into(),
            kind: FunctionKind::Free,
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(vec![], ret_val, vec![ValueDef::owned(unit)])),
            extern_info: None,
            is_main: false,
            provides_protocol_default: false,
        };
        module.add_function(func);
        module.register_name(entity(1), "main");

        let target = TargetConfig::host_64();
        let mono = monomorphize(module, &target).unwrap();

        assert_eq!(mono.functions.len(), 1);
        assert!(mono.functions[0].body.is_some());
        assert!(mono.functions[0].name.starts_with("_K0"));
    }

    /// `struct Name[T] { a: T, b: T, c: T }` — three words wide at `T = i64`.
    fn three_word_generic(module: &mut MirModule, ent: u32, tp: u32, name: &str) -> Entity {
        let tp_ty = module.ty_arena.intern(MirTy::TypeParam(entity(tp)));
        let mut sdef = StructDef::new(entity(ent), name);
        sdef.type_params = vec![TypeParamDef::new(entity(tp), "T")];
        for f in ["a", "b", "c"] {
            sdef.add_field(crate::item::struct_def::FieldDef::new(f, tp_ty));
        }
        module.add_struct(sdef);
        module.register_name(entity(ent), name);
        entity(ent)
    }

    fn mono_struct_size(mono: &MonoModule, key: (Entity, Vec<TyId>)) -> Option<u64> {
        match &mono.structs.get(&key)?.type_info.layout {
            Some(Layout::Struct(sl)) => Some(sl.size),
            _ => None,
        }
    }

    /// G2, Op half — correlation-free. `collect_named_types` walked value types,
    /// block params, `Struct`/`Enum`/`Array` type fields and `Literal`
    /// immediates, but ZERO `Op` type operands, while `substitute_op_type`
    /// handled all ten. Only `SizeOf`/`AlignOf` genuinely escape; the other
    /// eight normally produce a `Pointer[T]` value that the `Pointer` recursion
    /// re-seeds — an incidental correlation, not an invariant.
    ///
    /// This test breaks the correlation on purpose: the `PtrCast` result value
    /// is typed `i64`, not `Pointer[Ghost[i64]]`, so the ONLY mention of
    /// `Ghost[i64]` anywhere in the body is the op operand.
    #[test]
    fn op_type_operand_seeds_named_type() {
        for op_of in [
            Op::SizeOf as fn(TyId) -> Op,
            Op::AlignOf as fn(TyId) -> Op,
            Op::PtrCast as fn(TyId) -> Op,
            Op::PtrTo as fn(TyId) -> Op,
        ] {
            let mut module = MirModule::new("test");
            let unit = module.ty_arena.unit();
            let i64_ty = module.ty_arena.i64();
            let ghost = three_word_generic(&mut module, 10, 11, "Ghost");
            let ghost_i64 = module.ty_arena.intern(MirTy::Named {
                entity: ghost,
                type_args: vec![i64_ty],
            });

            // main() { %1 = op[Ghost[i64]] %0; return %0 }
            // Note the deliberately non-`Pointer` result type on %1.
            let arg = ValueId::new(0);
            let result = ValueId::new(1);
            let body = make_body(
                vec![Instruction::new(InstKind::Op1 {
                    result,
                    op: op_of(ghost_i64),
                    arg,
                })],
                arg,
                vec![ValueDef::owned(unit), ValueDef::owned(i64_ty)],
            );
            module.add_function(FunctionDef {
                entity: entity(1),
                name: "main".into(),
                kind: FunctionKind::Free,
                type_params: vec![],
                params: vec![],
                ret: unit,
                where_clause: None,
                body: Some(body),
                extern_info: None,
                is_main: true,
                provides_protocol_default: false,
            });
            module.register_name(entity(1), "main");

            let mono = monomorphize(module, &TargetConfig::host_64()).unwrap();
            assert_eq!(
                mono_struct_size(&mono, (ghost, vec![i64_ty])),
                Some(24),
                "op operand did not seed Ghost[i64]",
            );
        }
    }

    /// G2, field half. `Inner[i64]` is reachable ONLY as a field type of
    /// `Outer[i64]`; `collect_named_type_from_ty` never descends into fields,
    /// and the layout fixed-point loop iterated its worklist by shared borrow so
    /// it could not add what it found there. `mono_size_and_align` then returned
    /// `None`, `all_resolved` went false, and the CONTAINING `Outer[i64]` was
    /// dropped from `mono_structs` too — silently.
    #[test]
    fn field_type_seeds_nested_generic() {
        let mut module = MirModule::new("test");
        let unit = module.ty_arena.unit();
        let i64_ty = module.ty_arena.i64();

        let inner = three_word_generic(&mut module, 20, 21, "Inner");
        let u_ty = module.ty_arena.intern(MirTy::TypeParam(entity(23)));
        let inner_u = module.ty_arena.intern(MirTy::Named {
            entity: inner,
            type_args: vec![u_ty],
        });

        // struct Outer[U] { i: Inner[U], x: U }
        let mut outer_def = StructDef::new(entity(22), "Outer");
        outer_def.type_params = vec![TypeParamDef::new(entity(23), "U")];
        outer_def.add_field(crate::item::struct_def::FieldDef::new("i", inner_u));
        outer_def.add_field(crate::item::struct_def::FieldDef::new("x", u_ty));
        let outer = module.add_struct(outer_def);
        module.register_name(entity(22), "Outer");

        let outer_i64 = module.ty_arena.intern(MirTy::Named {
            entity: outer,
            type_args: vec![i64_ty],
        });

        // A concrete function is an unconditional mono root, so its value seeds
        // `Outer[i64]` — and nothing else ever mentions `Inner[i64]`.
        let ret_val = ValueId::new(0);
        module.add_function(FunctionDef {
            entity: entity(1),
            name: "main".into(),
            kind: FunctionKind::Free,
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(
                vec![],
                ret_val,
                vec![ValueDef::owned(unit), ValueDef::owned(outer_i64)],
            )),
            extern_info: None,
            is_main: true,
            provides_protocol_default: false,
        });
        module.register_name(entity(1), "main");

        let mono = monomorphize(module, &TargetConfig::host_64()).unwrap();
        assert_eq!(
            mono_struct_size(&mono, (inner, vec![i64_ty])),
            Some(24),
            "field type Inner[i64] was never seeded",
        );
        assert_eq!(
            mono_struct_size(&mono, (outer, vec![i64_ty])),
            Some(32),
            "Outer[i64] was dropped because its field type was unresolved",
        );
    }

    #[test]
    fn monomorphize_generic_function_via_call() {
        let mut module = MirModule::new("test");
        let unit = module.ty_arena.unit();
        let i64_ty = module.ty_arena.i64();
        let tp_ty = module.ty_arena.intern(MirTy::TypeParam(entity(3)));

        // generic_fn[T](x: T) -> T
        let x_val = ValueId::new(0);
        let generic_fn = FunctionDef {
            entity: entity(2),
            name: "identity".into(),
            kind: FunctionKind::Free,
            type_params: vec![TypeParamDef::new(entity(3), "T")],
            params: vec![ParamDef::new("x", x_val, tp_ty, ParamConvention::Consuming)],
            ret: tp_ty,
            where_clause: None,
            body: Some({
                let mut body = OssaBody::new();
                // value 0: the parameter x
                body.alloc_value(ValueDef::owned(tp_ty));
                let entry = body.alloc_block();
                body.entry = entry;
                body.param_count = 1;
                // Entry block has x as a block param
                body.block_mut(entry).params.push(BlockParam {
                    value: x_val,
                    ty: tp_ty,
                    ownership: Ownership::Owned,
                });
                body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Return(x_val));
                body
            }),
            extern_info: None,
            is_main: false,
            provides_protocol_default: false,
        };

        // main() calls identity[Int64]
        let ret_val = ValueId::new(0);
        let arg_val = ValueId::new(1);
        let result_val = ValueId::new(2);
        let call_inst = Instruction::new(InstKind::Call {
            result: Some(result_val),
            callee: Callee::Direct {
                func: entity(2),
                type_args: vec![i64_ty],
                self_type: None,
            },
            args: vec![CallArg {
                value: arg_val,
                convention: ParamConvention::Consuming,
            }],
        });

        let main_fn = FunctionDef {
            entity: entity(1),
            name: "main".into(),
            kind: FunctionKind::Free,
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(
                vec![call_inst],
                ret_val,
                vec![
                    ValueDef::owned(unit),
                    ValueDef::owned(i64_ty),
                    ValueDef::owned(i64_ty),
                ],
            )),
            extern_info: None,
            is_main: false,
            provides_protocol_default: false,
        };

        module.add_function(main_fn);
        module.add_function(generic_fn);
        module.register_name(entity(1), "main");
        module.register_name(entity(2), "identity");

        let target = TargetConfig::host_64();
        let mono = monomorphize(module, &target).unwrap();

        // main + identity[Int64]
        assert_eq!(mono.functions.len(), 2);

        // The identity function should have concrete params
        let identity = mono
            .functions
            .iter()
            .find(|f| f.source == entity(2))
            .unwrap();
        assert_eq!(identity.params.len(), 1);
        assert_eq!(identity.params[0].ty, i64_ty);
        assert_eq!(identity.ret, i64_ty);

        // The call in main should be Resolved
        let main = mono
            .functions
            .iter()
            .find(|f| f.source == entity(1))
            .unwrap();
        let body = main.body.as_ref().unwrap();
        let call = &body.blocks[0].insts[0];
        match &call.kind {
            InstKind::Call { callee, .. } => {
                assert!(matches!(callee, Callee::Resolved(_)));
            },
            _ => panic!("expected call"),
        }
    }

    #[test]
    fn monomorphize_apply_partial_rewrites_every_callee() {
        let mut module = MirModule::new("test");
        let unit = module.ty_arena.unit();
        let i64_ty = module.ty_arena.i64();
        let maker_param = entity(20);
        let maker_param_ty = module.ty_arena.intern(MirTy::TypeParam(maker_param));

        let generic_stub = |entity, param, name: &str| {
            let mut body = OssaBody::new();
            let ret = body.alloc_value(ValueDef::owned(unit));
            let entry = body.alloc_block();
            body.entry = entry;
            body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Return(ret));
            FunctionDef {
                entity,
                name: name.into(),
                kind: FunctionKind::Free,
                type_params: vec![TypeParamDef::new(param, "T")],
                params: vec![],
                ret: unit,
                where_clause: None,
                body: Some(body),
                extern_info: None,
                is_main: false,
                provides_protocol_default: false,
            }
        };

        let target_entity = entity(2);
        let retain_entity = entity(3);
        let release_entity = entity(4);
        module.add_function(generic_stub(target_entity, entity(21), "target"));
        module.add_function(generic_stub(retain_entity, entity(22), "retain"));
        module.add_function(generic_stub(release_entity, entity(23), "release"));

        let result = ValueId::new(0);
        let maker = FunctionDef {
            entity: entity(5),
            name: "maker".into(),
            kind: FunctionKind::Free,
            type_params: vec![TypeParamDef::new(maker_param, "T")],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(
                vec![Instruction::new(InstKind::ApplyPartial {
                    result,
                    callee: Callee::direct_with_args(target_entity, vec![maker_param_ty], None),
                    captures: vec![],
                    retain: Some(Callee::direct_with_args(
                        retain_entity,
                        vec![maker_param_ty],
                        None,
                    )),
                    release: Some(Callee::direct_with_args(
                        release_entity,
                        vec![maker_param_ty],
                        None,
                    )),
                })],
                result,
                vec![ValueDef::owned(unit)],
            )),
            extern_info: None,
            is_main: false,
            provides_protocol_default: false,
        };
        module.add_function(maker);

        let main_ret = ValueId::new(0);
        module.add_function(FunctionDef {
            entity: entity(1),
            name: "main".into(),
            kind: FunctionKind::Free,
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(
                vec![Instruction::new(InstKind::Call {
                    result: None,
                    callee: Callee::direct_with_args(entity(5), vec![i64_ty], None),
                    args: vec![],
                })],
                main_ret,
                vec![ValueDef::owned(unit)],
            )),
            extern_info: None,
            is_main: false,
            provides_protocol_default: false,
        });

        let mono = monomorphize(module, &TargetConfig::host_64()).unwrap();
        let maker = mono
            .functions
            .iter()
            .find(|f| f.source == entity(5))
            .unwrap();
        let InstKind::ApplyPartial {
            callee,
            retain,
            release,
            ..
        } = &maker.body.as_ref().unwrap().blocks[0].insts[0].kind
        else {
            panic!("expected ApplyPartial");
        };
        assert!(matches!(callee, Callee::Resolved(_)));
        assert!(matches!(retain, Some(Callee::Resolved(_))));
        assert!(matches!(release, Some(Callee::Resolved(_))));
        for source in [target_entity, retain_entity, release_entity] {
            assert!(
                mono.functions
                    .iter()
                    .any(|f| f.source == source && f.type_args == vec![i64_ty])
            );
        }
    }

    #[test]
    fn monomorphize_apply_partial_resolves_every_witness_callee() {
        let mut module = MirModule::new("test");
        let unit = module.ty_arena.unit();
        let i64_ty = module.ty_arena.i64();
        let protocol_entity = entity(10);
        module.add_protocol(ProtocolDef::new(protocol_entity, "ClosureLifecycle"));

        let target_key = WitnessMethodKey::new("target", vec![]);
        let retain_key = WitnessMethodKey::new("retain", vec![]);
        let release_key = WitnessMethodKey::new("release", vec![]);
        let mut witness = WitnessDef::new(protocol_entity, i64_ty);
        for (key, function) in [
            (target_key.clone(), entity(2)),
            (retain_key.clone(), entity(3)),
            (release_key.clone(), entity(4)),
        ] {
            witness.add_method(WitnessMethodBinding::new(key, function, vec![]));
        }
        module.add_witness(witness);

        for (function, name) in [
            (entity(2), "target"),
            (entity(3), "retain"),
            (entity(4), "release"),
        ] {
            let ret = ValueId::new(0);
            module.add_function(FunctionDef {
                entity: function,
                name: name.into(),
                kind: FunctionKind::Free,
                type_params: vec![],
                params: vec![],
                ret: unit,
                where_clause: None,
                body: Some(make_body(vec![], ret, vec![ValueDef::owned(unit)])),
                extern_info: None,
                is_main: false,
                provides_protocol_default: false,
            });
        }

        let witness_callee = |method| Callee::Witness {
            protocol: protocol_entity,
            method,
            self_type: i64_ty,
            method_type_args: vec![],
        };
        let result = ValueId::new(0);
        module.add_function(FunctionDef {
            entity: entity(1),
            name: "main".into(),
            kind: FunctionKind::Free,
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(
                vec![Instruction::new(InstKind::ApplyPartial {
                    result,
                    callee: witness_callee(target_key),
                    captures: vec![],
                    retain: Some(witness_callee(retain_key)),
                    release: Some(witness_callee(release_key)),
                })],
                result,
                vec![ValueDef::owned(unit)],
            )),
            extern_info: None,
            is_main: false,
            provides_protocol_default: false,
        });
        module.register_name(protocol_entity, "ClosureLifecycle");

        let mono = monomorphize(module, &TargetConfig::host_64()).unwrap();
        let main = mono
            .functions
            .iter()
            .find(|f| f.source == entity(1))
            .unwrap();
        for (_, callee) in main.body.as_ref().unwrap().blocks[0].insts[0]
            .kind
            .callees()
        {
            assert!(matches!(callee, Callee::Resolved(_)));
        }
    }

    #[test]
    fn monomorphize_extern_function() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();
        let unit = module.ty_arena.unit();
        let ptr = module.ty_arena.pointer(unit);

        let func = FunctionDef {
            entity: entity(1),
            name: "malloc".into(),
            kind: FunctionKind::Free,
            type_params: vec![],
            params: vec![ParamDef::new(
                "size",
                ValueId::new(0),
                i64_ty,
                ParamConvention::Consuming,
            )],
            ret: ptr,
            where_clause: None,
            body: None,
            extern_info: Some(crate::item::function::ExternInfo {
                calling_convention: crate::item::function::CallingConvention::C,
                symbol_name: "malloc".into(),
            }),
            is_main: false,
            provides_protocol_default: false,
        };
        module.add_function(func);
        module.register_name(entity(1), "malloc");

        let target = TargetConfig::host_64();
        let mono = monomorphize(module, &target).unwrap();

        assert_eq!(mono.functions.len(), 1);
        assert!(mono.functions[0].body.is_none());
        assert!(mono.functions[0].extern_info.is_some());
    }
}
