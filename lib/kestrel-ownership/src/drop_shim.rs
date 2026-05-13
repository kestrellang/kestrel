//! Drop-shim synthesis — generate one `__drop$T(self: T)` function per
//! non-trivial nominal type.
//!
//! ## Why shims?
//!
//! Without shims, every drop site has to inline the user `deinit` call
//! plus the structural field-drop sequence — and for enums, a `Switch`
//! on the discriminant. That's CFG surgery at every drop site,
//! splattered across the program. With shims, the drop logic for type
//! `T` is generated *once* into a synthesized function and every
//! drop site emits a single `Call(__drop$T, ...)`. All the CFG
//! complexity lives inside the shim, where new basic blocks naturally
//! fit.
//!
//! This is the same model as Rust's `core::ptr::drop_in_place::<T>`.
//!
//! ## Receiver shape
//!
//! - User `deinit` is `&var Self` — borrows; can read/mutate fields
//!   but cannot move them out.
//! - **Shim is `consuming Self`** — takes ownership at the type
//!   level. Drop sites pass `Move(p)` so the dataflow correctly
//!   kills the path. Inside the shim, fields are decomposed by
//!   projection-move into per-field temps before being dropped, so
//!   each field's path is killed before the corresponding recursive
//!   shim call.
//!
//! The two signatures are different on purpose: the user-facing one
//! gives ergonomics (no move-out, value still valid); the
//! compiler-internal one gives semantic correctness (drop site sees
//! the place consumed).
//!
//! ## Body shape
//!
//! For struct `T { f0: F0, f1: F1, ... }`:
//!
//! ```text
//! __drop$T(self: T):
//!   bb0:
//!     Call(T.user_deinit, [RefMut(self)])    // if T has user deinit
//!     %_f0 = move self.f0                     // for each non-trivial F_i
//!     Call(__drop$F0, [Move(%_f0)])
//!     %_f1 = move self.f1
//!     Call(__drop$F1, [Move(%_f1)])
//!     ...
//!     return ()
//! ```
//!
//! For enum `T { Case0(F0), Case1, Case2(F2) }`:
//!
//! ```text
//! __drop$T(self: T):
//!   bb0:
//!     Call(T.user_deinit, [RefMut(self)])    // if T has user deinit
//!     switch self.discriminant {
//!       Case0 => bb_c0
//!       Case1 => bb_c1
//!       Case2 => bb_c2
//!     }
//!   bb_c0:
//!     %_f = move self.cases.Case0.0
//!     Call(__drop$F0, [Move(%_f)])
//!     jump bb_return
//!   bb_c1:
//!     jump bb_return                          // no payload
//!   bb_c2:
//!     %_f = move self.cases.Case2.0
//!     Call(__drop$F2, [Move(%_f)])
//!     jump bb_return
//!   bb_return:
//!     return ()
//! ```
//!
//! Tuples are not nominal — they have no Entity — so they don't get
//! shims. The shim for a struct containing a tuple field
//! `f: (A, B)` inlines the tuple decomposition: `%_f = move self.f`,
//! then `Call(__drop$A, [Move(%_f.0)])`, `Call(__drop$B, [Move(%_f.1)])`.

use std::collections::{HashMap, HashSet};

use kestrel_hecs::Entity;
use kestrel_mir::{
    BasicBlock, Callee, CopyBehavior, EnumDef, FunctionDef, FunctionKind, LocalDef, LocalId,
    MirBody, MirModule, MirTy, ParamDef, Place, Statement, StatementKind, StructDef, SwitchCase,
    Terminator, TypeParamDef, Value,
};

/// Map from nominal-type entity → synthesized `__drop$T` function entity.
/// `drop_expand` consults this to find the right shim to call at each
/// drop site.
pub type ShimMap = HashMap<Entity, Entity>;

/// Synthesize a drop shim for every non-trivial nominal type in the
/// module. Returns the `nominal → shim_entity` map. Trivial types
/// (no user deinit, no non-trivial fields) get no shim — drop_expand
/// removes their `Drop` statements outright.
pub fn run(module: &mut MirModule) -> ShimMap {
    let mut shim_map: ShimMap = HashMap::new();

    // First pass: decide which nominals need shims. This is a fixed
    // point — a struct needs a shim if any of its fields needs a shim
    // (or it has a user deinit). We iterate until stable.
    let needed = collect_needed_shims(module);

    // Reserve entity IDs and names up front so cross-shim references
    // (one shim's body calling another's) can resolve immediately.
    let next_entity_seed = module.functions.len() as u32;
    let mut nominal_to_shim: HashMap<Entity, Entity> = HashMap::new();
    for (i, nominal) in needed.iter().enumerate() {
        let shim_entity =
            Entity::from_raw(u32::MAX / 2 + 0x40000 + next_entity_seed + i as u32);
        let name = format!("__drop${}", lookup_nominal_name(*nominal, module));
        module.register_name(shim_entity, &name);
        nominal_to_shim.insert(*nominal, shim_entity);
        shim_map.insert(*nominal, shim_entity);
    }

    // Second pass: build each shim's body.
    for nominal in &needed {
        let shim_entity = nominal_to_shim[nominal];
        let shim = build_shim(*nominal, shim_entity, module, &nominal_to_shim);
        module.add_function(shim);
    }

    shim_map
}

/// Compute the fixed-point set of nominals that need drop shims. A
/// nominal needs a shim if it has a user `deinit` or any of its
/// fields / case payloads is itself non-trivial. The fixed-point
/// catches transitive dependencies (struct A wraps struct B which
/// wraps File — both A and B need shims).
fn collect_needed_shims(module: &MirModule) -> Vec<Entity> {
    let mut needed: HashSet<Entity> = HashSet::new();
    loop {
        let before = needed.len();
        for s in &module.structs {
            if needed.contains(&s.entity) {
                continue;
            }
            if s.deinit_behavior.user_method.is_some()
                || s.fields
                    .iter()
                    .any(|f| ty_needs_drop_work(&f.ty, module, &needed))
            {
                needed.insert(s.entity);
            }
        }
        for e in &module.enums {
            if needed.contains(&e.entity) {
                continue;
            }
            let any_case_needs = e.cases.iter().any(|c| {
                let payload = &module.structs[c.payload_struct.index()];
                payload
                    .fields
                    .iter()
                    .any(|f| ty_needs_drop_work(&f.ty, module, &needed))
            });
            if e.deinit_behavior.user_method.is_some() || any_case_needs {
                needed.insert(e.entity);
            }
        }
        if needed.len() == before {
            break;
        }
    }
    needed.into_iter().collect()
}

/// True if `ty` needs ANY drop work — recursively. Used during the
/// fixed-point above: `known_nontrivial` is the set we've decided so
/// far; an as-yet-undecided nominal that's in there counts as
/// non-trivial. Bitwise types short-circuit to false.
fn ty_needs_drop_work(ty: &MirTy, module: &MirModule, known_nontrivial: &HashSet<Entity>) -> bool {
    match ty {
        MirTy::Named { entity, .. } => {
            if known_nontrivial.contains(entity) {
                return true;
            }
            // Direct lookup: if user deinit, trivially needs work.
            if let Some(s) = module.structs.iter().find(|s| s.entity == *entity) {
                if s.deinit_behavior.user_method.is_some() {
                    return true;
                }
                return s
                    .fields
                    .iter()
                    .any(|f| ty_needs_drop_work(&f.ty, module, known_nontrivial));
            }
            if let Some(e) = module.enums.iter().find(|e| e.entity == *entity) {
                if e.deinit_behavior.user_method.is_some() {
                    return true;
                }
                return e.cases.iter().any(|c| {
                    let payload = &module.structs[c.payload_struct.index()];
                    payload
                        .fields
                        .iter()
                        .any(|f| ty_needs_drop_work(&f.ty, module, known_nontrivial))
                });
            }
            // Primitives + lang types — copy_behavior tells us. Bitwise
            // → no drop. None → tracked-affine; if no def found at all
            // we conservatively say "no work" (unresolved types skip).
            ty.copy_behavior(module) == CopyBehavior::None
        },
        MirTy::Tuple(elems) => elems
            .iter()
            .any(|t| ty_needs_drop_work(t, module, known_nontrivial)),
        // Refs and pointers are Bitwise — never drop their referent.
        MirTy::Ref(_)
        | MirTy::RefMut(_)
        | MirTy::Pointer(_)
        | MirTy::FuncThin { .. }
        | MirTy::FuncThick { .. } => false,
        // TypeParam / SelfType / AssociatedProjection — conservatively
        // assume might need work (the monomorphizer decides at codegen).
        // Returning true here means a generic struct wrapping a `T`
        // gets a shim even when T might be trivial — that's the right
        // behavior; the per-instantiation shim's body will conditionally
        // call __drop$T which itself might be trivial.
        MirTy::TypeParam(_) | MirTy::SelfType | MirTy::AssociatedProjection { .. } => true,
        // Primitives — no work.
        _ => false,
    }
}

fn lookup_nominal_name(entity: Entity, module: &MirModule) -> String {
    if let Some(s) = module.structs.iter().find(|s| s.entity == entity) {
        return s.name.clone();
    }
    if let Some(e) = module.enums.iter().find(|e| e.entity == entity) {
        return e.name.clone();
    }
    format!("entity_{:?}", entity)
}

/// Build the FunctionDef for `__drop$<nominal>`. Generic over the
/// nominal's own type-params so codegen monomorphization works the
/// same way as any other generic function.
fn build_shim(
    nominal: Entity,
    shim_entity: Entity,
    module: &MirModule,
    shim_map: &HashMap<Entity, Entity>,
) -> FunctionDef {
    // Struct or enum?
    if let Some(s) = module.structs.iter().find(|s| s.entity == nominal) {
        return build_struct_shim(s, shim_entity, module, shim_map);
    }
    if let Some(e) = module.enums.iter().find(|e| e.entity == nominal) {
        return build_enum_shim(e, shim_entity, module, shim_map);
    }
    panic!("nominal entity {:?} not found in module", nominal);
}

fn build_struct_shim(
    s: &StructDef,
    shim_entity: Entity,
    module: &MirModule,
    shim_map: &HashMap<Entity, Entity>,
) -> FunctionDef {
    let name = format!("__drop${}", s.name);
    let mut def = FunctionDef::new(shim_entity, &name, MirTy::unit());
    def.type_params = s.type_params.clone();
    def.kind = FunctionKind::Free;

    let type_args: Vec<MirTy> = s
        .type_params
        .iter()
        .map(|tp| MirTy::TypeParam(tp.entity))
        .collect();
    let self_ty = MirTy::Named {
        entity: s.entity,
        type_args: type_args.clone(),
    };

    let mut body = MirBody::new();
    let self_local = body.add_local(LocalDef::new("self", self_ty.clone()));
    def.params
        .push(ParamDef::new("self", self_local, self_ty.clone()));
    body.param_count += 1;

    let mut entry = BasicBlock::new();
    emit_struct_drop_sequence(
        &mut entry,
        &mut body,
        Place::local(self_local),
        s,
        &type_args,
        module,
        shim_map,
    );
    entry.terminator = Terminator::ret(Value::Const(kestrel_mir::Immediate::unit()));

    let entry_id = body.add_block(entry);
    body.entry = entry_id;
    def.body = Some(body);
    def
}

/// Emit the struct's drop-statement sequence into `block`. Used both
/// for shim bodies and for inline expansion of tuple-typed fields.
fn emit_struct_drop_sequence(
    block: &mut BasicBlock,
    body: &mut MirBody,
    place: Place,
    s: &StructDef,
    type_args: &[MirTy],
    module: &MirModule,
    shim_map: &HashMap<Entity, Entity>,
) {
    // Step 1: user deinit (if any), on a mutable borrow.
    if let Some(user_method) = s.deinit_behavior.user_method {
        let callee = Callee::method(
            user_method,
            type_args.to_vec(),
            MirTy::Named {
                entity: s.entity,
                type_args: type_args.to_vec(),
            },
        );
        block.stmts.push(Statement::new(StatementKind::Call {
            dest: None,
            callee,
            args: vec![Value::RefMut(place.clone())],
        }));
    }

    // Step 2: structural field drops in declaration order. Each
    // non-trivial field gets projection-moved into a temp local and
    // then handed to the appropriate `__drop$Field` shim.
    let subst = build_subst(&s.type_params, type_args);
    for field in &s.fields {
        let field_ty = substitute(&field.ty, &subst);
        if !ty_actually_needs_drop(&field_ty, module) {
            continue;
        }
        let field_place = Place::Field {
            parent: Box::new(place.clone()),
            name: field.name.clone(),
        };
        emit_drop_for(block, body, field_place, &field_ty, module, shim_map);
    }
}

/// Emit `Drop` for a place of the given type. Dispatches on whether
/// the type is nominal (shim call), tuple (inline decomposition), or
/// trivial (skip).
fn emit_drop_for(
    block: &mut BasicBlock,
    body: &mut MirBody,
    place: Place,
    ty: &MirTy,
    module: &MirModule,
    shim_map: &HashMap<Entity, Entity>,
) {
    match ty {
        MirTy::Named { entity, type_args } => {
            let Some(shim_entity) = shim_map.get(entity).copied() else {
                // Nominal with no shim — it's trivial. Nothing to do.
                return;
            };
            // Project-move the place into a fresh temp so the recursive
            // shim consumes it cleanly. This also keeps dataflow
            // accurate: the field path is killed at the move.
            let temp = body.add_local(LocalDef::new("_drop_arg", ty.clone()));
            block.stmts.push(Statement::new(StatementKind::Assign {
                dest: Place::local(temp),
                rvalue: kestrel_mir::Rvalue::Move(place),
            }));
            let callee = Callee::direct_generic(shim_entity, type_args.clone());
            block.stmts.push(Statement::new(StatementKind::Call {
                dest: None,
                callee,
                args: vec![Value::Move(Place::local(temp))],
            }));
        },
        MirTy::Tuple(elems) => {
            // Tuples have no nominal entity to attach a shim to;
            // decompose inline and recurse on each element. Each
            // element gets its own projection-move into a temp.
            for (i, elem_ty) in elems.iter().enumerate() {
                if !ty_actually_needs_drop(elem_ty, module) {
                    continue;
                }
                let elem_place = Place::Index {
                    parent: Box::new(place.clone()),
                    index: i,
                };
                emit_drop_for(block, body, elem_place, elem_ty, module, shim_map);
            }
        },
        // TypeParam, SelfType, AssociatedProjection — could be
        // non-trivial under monomorphization, so we *would* call a
        // generic shim if we knew which one. Today the type-param's
        // copy semantics aren't enough to pick a shim; conservatively
        // skip. This is the same gap as elsewhere — generic non-Copy
        // params don't get their fields drop-checked at the MIR
        // level. Tracked as a follow-up.
        _ => {},
    }
}

/// Cheap check: would `ty` produce any actual drop statements? Avoids
/// emitting `_t = move ...` + `Call` pairs for types that aren't in
/// the shim map (trivial nominals, primitives, etc).
fn ty_actually_needs_drop(ty: &MirTy, module: &MirModule) -> bool {
    match ty {
        MirTy::Named { entity, .. } => {
            // Look at the type's own behavior. Non-`None` means Copy or
            // Cloneable, both of which skip drops. `None` means affine
            // and might need a shim.
            ty.copy_behavior(module) == CopyBehavior::None
        },
        MirTy::Tuple(elems) => elems.iter().any(|t| ty_actually_needs_drop(t, module)),
        _ => false,
    }
}

fn build_enum_shim(
    e: &EnumDef,
    shim_entity: Entity,
    module: &MirModule,
    shim_map: &HashMap<Entity, Entity>,
) -> FunctionDef {
    let name = format!("__drop${}", e.name);
    let mut def = FunctionDef::new(shim_entity, &name, MirTy::unit());
    def.type_params = e.type_params.clone();
    def.kind = FunctionKind::Free;

    let type_args: Vec<MirTy> = e
        .type_params
        .iter()
        .map(|tp| MirTy::TypeParam(tp.entity))
        .collect();
    let self_ty = MirTy::Named {
        entity: e.entity,
        type_args: type_args.clone(),
    };

    let mut body = MirBody::new();
    let self_local = body.add_local(LocalDef::new("self", self_ty.clone()));
    def.params
        .push(ParamDef::new("self", self_local, self_ty.clone()));
    body.param_count += 1;

    // Order matters: the cranelift codegen treats the first MIR block
    // (BlockId(0)) as the function-entry — it's where parameters live and
    // execution begins. Build the entry block first with a placeholder
    // terminator, then fill in the actual switch after the per-case
    // block IDs are known. (Previously the entry was added last, ending
    // up as BlockId(3) for a 2-variant enum, and cranelift's verifier
    // tripped on the empty BlockId(0) — "terminator before end of
    // block3" via the mismatched function-entry slot.)
    let entry_id = body.add_block(BasicBlock::new());
    body.entry = entry_id;

    // bb_return is the join block; jumped to from each per-case block.
    let mut return_block = BasicBlock::new();
    return_block.terminator = Terminator::ret(Value::Const(kestrel_mir::Immediate::unit()));
    let return_id = body.add_block(return_block);

    // Per-case blocks. With Stage 4 root-folding, projecting `self.Variant.f`
    // for each non-trivial field is enough to give each case its own drop
    // sequence; the case block jumps to `return_id` when done.
    let subst = build_subst(&e.type_params, &type_args);
    let mut case_blocks: Vec<(SwitchCase, kestrel_mir::BlockId)> = Vec::new();
    for case in &e.cases {
        let payload = &module.structs[case.payload_struct.index()];
        let mut block = BasicBlock::new();
        let downcast = Place::Downcast {
            parent: Box::new(Place::local(self_local)),
            variant: case.name.clone(),
        };
        for field in &payload.fields {
            let field_ty = substitute(&field.ty, &subst);
            if !ty_actually_needs_drop(&field_ty, module) {
                continue;
            }
            let field_place = Place::Field {
                parent: Box::new(downcast.clone()),
                name: field.name.clone(),
            };
            emit_drop_for(&mut block, &mut body, field_place, &field_ty, module, shim_map);
        }
        block.terminator = Terminator::jump(return_id);
        let block_id = body.add_block(block);
        case_blocks.push((SwitchCase::Variant(case.name.clone()), block_id));
    }

    // Now fill in the entry block: user deinit (if any), then switch on
    // discriminant. We have all the case block IDs ready.
    let entry = body.block_mut(entry_id);
    if let Some(user_method) = e.deinit_behavior.user_method {
        let callee = Callee::method(
            user_method,
            type_args.to_vec(),
            MirTy::Named {
                entity: e.entity,
                type_args: type_args.to_vec(),
            },
        );
        entry.stmts.push(Statement::new(StatementKind::Call {
            dest: None,
            callee,
            args: vec![Value::RefMut(Place::local(self_local))],
        }));
    }
    entry.terminator = Terminator::switch(Place::local(self_local), case_blocks);

    def.body = Some(body);
    def
}

fn build_subst(params: &[TypeParamDef], args: &[MirTy]) -> HashMap<Entity, MirTy> {
    params
        .iter()
        .zip(args.iter())
        .map(|(p, a)| (p.entity, a.clone()))
        .collect()
}

fn substitute(ty: &MirTy, subst: &HashMap<Entity, MirTy>) -> MirTy {
    match ty {
        MirTy::TypeParam(e) => subst.get(e).cloned().unwrap_or_else(|| ty.clone()),
        MirTy::Ref(inner) => MirTy::Ref(Box::new(substitute(inner, subst))),
        MirTy::RefMut(inner) => MirTy::RefMut(Box::new(substitute(inner, subst))),
        MirTy::Pointer(inner) => MirTy::Pointer(Box::new(substitute(inner, subst))),
        MirTy::Tuple(elems) => MirTy::Tuple(elems.iter().map(|t| substitute(t, subst)).collect()),
        MirTy::Named { entity, type_args } => MirTy::Named {
            entity: *entity,
            type_args: type_args.iter().map(|t| substitute(t, subst)).collect(),
        },
        MirTy::FuncThin { params, ret } => MirTy::FuncThin {
            params: params.iter().map(|t| substitute(t, subst)).collect(),
            ret: Box::new(substitute(ret, subst)),
        },
        MirTy::FuncThick { params, ret } => MirTy::FuncThick {
            params: params.iter().map(|t| substitute(t, subst)).collect(),
            ret: Box::new(substitute(ret, subst)),
        },
        _ => ty.clone(),
    }
}
