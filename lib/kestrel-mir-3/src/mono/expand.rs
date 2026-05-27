use std::collections::{HashMap, HashSet};

use kestrel_hecs::Entity;

use crate::callee::Callee;
use crate::inst::{CallArg, InstKind, Instruction};
use crate::item::function::{FunctionDef, FunctionKind};
use crate::item::{CopyBehavior, DropBehavior};
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
    generic_functions: &[FunctionDef],
) {
    let shim_lookup = build_drop_shim_lookup(module, generic_functions);
    let clone_lookup = build_clone_lookup(module, generic_functions);

    // Map method/deinit entities → nominal for types with drop/clone.
    // Prevents recursive cycles: DestroyValue on T inside T's methods
    // won't expand to Call(__drop$T), and CopyValue on T inside T's
    // methods won't expand to Call(T.clone).
    let method_to_nominal = build_method_to_nominal_map(module, generic_functions);

    // Pre-intern Pointer(Named) types for cloneable types so the expand
    // pass can create BeginBorrow values without mutable arena access.
    for (nominal, type_args) in clone_lookup.keys() {
        if let Some(named_ty) = module.ty_arena.find(|t| {
            matches!(t, MirTy::Named { entity, type_args: ta } if *entity == *nominal && ta == type_args)
        }) {
            module.ty_arena.pointer(named_ty);
        }
    }

    for fi in 0..module.functions.len() {
        let source = module.functions[fi].source;
        let skip_nominal = method_to_nominal.get(&source).copied();
        expand_function(&mut module.functions[fi], &module.ty_arena, &shim_lookup, &clone_lookup, skip_nominal);
    }
}

/// Maps (nominal_entity, type_args) -> MonoFuncId for drop shim dispatch.
type DropShimLookup = HashMap<(Entity, Vec<TyId>), MonoFuncId>;

/// Build func_entity → nominal_entity for methods of droppable/cloneable types.
/// Prevents recursive expansion cycles: DestroyValue on T inside T's methods
/// won't expand to Call(__drop$T), and CopyValue on T inside T's methods
/// won't expand to Call(T.clone).
fn build_method_to_nominal_map(
    module: &MonoModule,
    generic_functions: &[FunctionDef],
) -> HashMap<Entity, Entity> {
    // Collect nominal entities that have drop shims or clone behavior
    let mut relevant: HashSet<Entity> = HashSet::new();
    for s in &module.structs {
        if s.type_info.drop != DropBehavior::None || matches!(s.type_info.copy, CopyBehavior::Clone(_)) {
            relevant.insert(s.source);
        }
    }
    for e in &module.enums {
        if e.type_info.drop != DropBehavior::None {
            relevant.insert(e.source);
        }
    }

    let mut map = HashMap::new();
    for f in generic_functions {
        let parent = match &f.kind {
            FunctionKind::Method { parent, .. }
            | FunctionKind::Deinit { parent }
            | FunctionKind::Initializer { parent }
            | FunctionKind::StaticMethod { parent }
            | FunctionKind::DropShim { nominal: parent }
            | FunctionKind::CloneShim { nominal: parent } => Some(*parent),
            _ => None,
        };
        if let Some(p) = parent {
            if relevant.contains(&p) {
                map.insert(f.entity, p);
            }
        }
    }
    map
}

/// Build (nominal_entity, type_args) → MonoFuncId for clone functions.
/// Finds both synthesized CloneShim functions and user-written .clone() methods
/// via FunctionKind matching.
fn build_clone_lookup(
    module: &MonoModule,
    generic_functions: &[FunctionDef],
) -> DropShimLookup {
    // Map clone function entity → nominal parent.
    // Include ALL clone shims and user .clone() methods regardless of CopyBehavior.
    let mut clone_func_to_parent: HashMap<Entity, Entity> = HashMap::new();
    for f in generic_functions {
        match &f.kind {
            FunctionKind::CloneShim { nominal } => {
                clone_func_to_parent.insert(f.entity, *nominal);
            }
            FunctionKind::Method { parent, .. } if f.name.ends_with(".clone") => {
                clone_func_to_parent.insert(f.entity, *parent);
            }
            _ => {}
        }
    }

    if std::env::var("KESTREL_DEBUG_CLONE").is_ok() {
        eprintln!("[clone_lookup] clone_func_to_parent: {} entries", clone_func_to_parent.len());
        for (func_entity, parent) in &clone_func_to_parent {
            let name = generic_functions.iter().find(|f| f.entity == *func_entity).map(|f| f.name.as_str()).unwrap_or("?");
            let parent_name = module.entity_names.get(parent).map(|s| s.as_str()).unwrap_or("?");
            eprintln!("  clone func: {name} → parent={parent_name}");
        }
    }

    // Collect entities of Bitwise types — these don't need clone shim calls
    let bitwise_nominals: HashSet<Entity> = module.structs.iter()
        .filter(|s| matches!(s.type_info.copy, CopyBehavior::Bitwise))
        .map(|s| s.source)
        .chain(module.enums.iter()
            .filter(|e| matches!(e.type_info.copy, CopyBehavior::Bitwise))
            .map(|e| e.source))
        .collect();

    let mut lookup = DropShimLookup::new();
    for (mi, mf) in module.functions.iter().enumerate() {
        if let Some(&nominal) = clone_func_to_parent.get(&mf.source) {
            if bitwise_nominals.contains(&nominal) {
                if std::env::var("KESTREL_DEBUG_CLONE").is_ok() {
                    eprintln!("[clone_lookup] SKIPPED (bitwise): {} source={:?} nominal={:?} type_args={:?}", mf.name, mf.source, nominal, mf.type_args);
                }
                continue;
            }
            if std::env::var("KESTREL_DEBUG_CLONE").is_ok() {
                eprintln!("[clone_lookup] ADDED: {} source={:?} nominal={:?} type_args={:?}", mf.name, mf.source, nominal, mf.type_args);
            }
            lookup.insert(
                (nominal, mf.type_args.clone()),
                MonoFuncId::new(mi),
            );
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
    generic_functions: &[FunctionDef],
) -> DropShimLookup {
    let shim_to_nominal: HashMap<Entity, Entity> = generic_functions
        .iter()
        .filter_map(|f| match &f.kind {
            FunctionKind::DropShim { nominal } => Some((f.entity, *nominal)),
            _ => None,
        })
        .collect();

    let mut lookup = DropShimLookup::new();

    for (mi, mf) in module.functions.iter().enumerate() {
        if let Some(&nominal) = shim_to_nominal.get(&mf.source) {
            lookup.insert(
                (nominal, mf.type_args.clone()),
                MonoFuncId::new(mi),
            );
        }
    }

    lookup
}

/// Expand DestroyValue/CopyValue in a single function body.
/// `skip_destroy_nominal`: if this function is a deinit for some nominal type,
/// DestroyValue on that type is removed instead of expanded to avoid
/// recursive drop shim → deinit → drop shim cycles.
fn expand_function(
    func: &mut MonoFunction,
    ty_arena: &crate::ty::TyArena,
    shim_lookup: &DropShimLookup,
    clone_lookup: &DropShimLookup,
    skip_nominal: Option<Entity>,
) {
    let Some(body) = &mut func.body else { return };

    // value_remap tracks CopyValue removals: result -> operand
    let mut value_remap: HashMap<ValueId, ValueId> = HashMap::new();

    for block_idx in 0..body.blocks.len() {
        let old_insts = std::mem::take(&mut body.blocks[block_idx].insts);
        let mut new_insts: Vec<Instruction> = Vec::with_capacity(old_insts.len());

        for inst in old_insts {
            match &inst.kind {
                InstKind::DestroyValue { operand } => {
                    let operand = *operand;
                    let value_def = &body.values[operand.index()];

                    match ty_arena.get(value_def.ty) {
                        MirTy::Named { entity, type_args } => {
                            // Skip if we're inside a method of this type — prevents
                            // drop shim → deinit → method → drop shim recursion.
                            if skip_nominal == Some(*entity) {
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
                                    span: inst.span,
                                });
                            }
                        }
                        _ => {}
                    }
                }

                // DestroyAddr: load the value from the address, then call the drop shim.
                // Expands to: take %tmp = *%addr; call __drop$T(%tmp)
                InstKind::DestroyAddr { address, ty } => {
                    let address = remap_value(*address, &value_remap);
                    let ty = *ty;
                    let span = inst.span.clone();

                    if let MirTy::Named { entity, type_args } = ty_arena.get(ty) {
                        if skip_nominal != Some(*entity) {
                            let key = (*entity, type_args.clone());
                            if let Some(&shim_id) = shim_lookup.get(&key) {
                                let tmp = body.alloc_value(ValueDef::owned(ty));
                                new_insts.push(Instruction {
                                    kind: InstKind::Take { result: tmp, address, ty },
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
                    }
                }

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
                            if skip_nominal != Some(*entity) {
                                let key = (*entity, type_args.clone());
                                if let Some(&shim_id) = shim_lookup.get(&key) {
                                    let tmp = body.alloc_value(ValueDef::owned(pointee));
                                    new_insts.push(Instruction {
                                        kind: InstKind::Take { result: tmp, address, ty: pointee },
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
                        kind: if expanded { InstKind::StoreInit { address, value } }
                              else { InstKind::StoreAssign { address, value } },
                        span,
                    });
                }

                InstKind::CopyValue { result, operand } => {
                    let result = *result;
                    let operand = *operand;
                    let value_def = &body.values[operand.index()];

                    // Named type with a clone function → BeginBorrow + Call(clone) + EndBorrow
                    if let MirTy::Named { entity, type_args } = ty_arena.get(value_def.ty) {
                        // Skip inside methods of this type to prevent recursion
                        if skip_nominal != Some(*entity) {
                            let key = (*entity, type_args.clone());
                            if std::env::var("KESTREL_DEBUG_CLONE").is_ok() {
                                let found = clone_lookup.get(&key).is_some();
                                if !found {
                                    eprintln!("[expand] CopyValue on Named {entity:?} — NOT in clone_lookup (type_args={:?})", type_args);
                                }
                            }
                            if let Some(&clone_id) = clone_lookup.get(&key) {
                                let remapped_operand = remap_value(operand, &value_remap);

                                let ptr_ty = ty_arena.find(|t| matches!(t, MirTy::Pointer(p) if *p == value_def.ty))
                                    .expect("Pointer type should be pre-interned for cloneable types");
                                let borrow_val = body.alloc_value(ValueDef::guaranteed(ptr_ty, remapped_operand));

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
                }

                _ => {
                    let mut kept = inst;
                    remap_inst_operands(&mut kept.kind, &value_remap);
                    new_insts.push(kept);
                }
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
        }

        InstKind::Load { address, .. } => {
            *address = remap_value(*address, remap);
        }
        InstKind::CopyAddr { address, .. }
        | InstKind::Take { address, .. }
        | InstKind::BeginBorrowAddr { address, .. }
        | InstKind::BeginMutBorrowAddr { address, .. }
        | InstKind::DestroyAddr { address, .. } => {
            *address = remap_value(*address, remap);
        }
        InstKind::StoreInit { address, value }
        | InstKind::StoreAssign { address, value } => {
            *address = remap_value(*address, remap);
            *value = remap_value(*value, remap);
        }

        InstKind::Op1 { arg, .. } => {
            *arg = remap_value(*arg, remap);
        }
        InstKind::Op2 { lhs, rhs, .. } => {
            *lhs = remap_value(*lhs, remap);
            *rhs = remap_value(*rhs, remap);
        }
        InstKind::Op3 { a, b, c, .. } => {
            *a = remap_value(*a, remap);
            *b = remap_value(*b, remap);
            *c = remap_value(*c, remap);
        }

        InstKind::Literal { .. } | InstKind::GlobalRef { .. } => {}

        InstKind::Struct { fields, .. } => {
            for (_, v) in fields.iter_mut() {
                *v = remap_value(*v, remap);
            }
        }
        InstKind::Tuple { elements, .. }
        | InstKind::Array { elements, .. } => {
            for v in elements.iter_mut() {
                *v = remap_value(*v, remap);
            }
        }
        InstKind::Enum { payload, .. } => {
            for v in payload.iter_mut() {
                *v = remap_value(*v, remap);
            }
        }

        InstKind::Call { args, callee, .. } => {
            for arg in args.iter_mut() {
                arg.value = remap_value(arg.value, remap);
            }
            match callee {
                Callee::Thin(v) | Callee::Thick(v) => {
                    *v = remap_value(*v, remap);
                }
                _ => {}
            }
        }
        InstKind::ApplyPartial { captures, .. } => {
            for v in captures.iter_mut() {
                *v = remap_value(*v, remap);
            }
        }

        InstKind::FieldAddr { base, .. } => {
            *base = remap_value(*base, remap);
        }

        InstKind::Uninit { .. } => {}
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
        }
        TerminatorKind::Jump { args, .. } => {
            for v in args.iter_mut() {
                *v = remap_value(*v, remap);
            }
        }
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
        }
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
        }
        TerminatorKind::Panic(_) | TerminatorKind::Unreachable => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BasicBlock;
    use crate::body::OssaBody;
    use crate::inst::Instruction;
    use crate::item::function::{FunctionDef, FunctionKind};
    use crate::item::TypeParamDef;
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
        MonoModule::new(TyArena::new(), IndexMap::new())
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
                Instruction::new(InstKind::Literal { result: ValueId::new(1), value: Immediate::i64(42) }),
                Instruction::new(InstKind::DestroyValue { operand: ValueId::new(1) }),
            ],
            ValueId::new(0),
            vec![ValueDef::owned(unit), ValueDef::owned(i64_ty)],
        );
        module.add_function(make_mono_func("test", entity(1), vec![], unit, Some(body)));
        expand_destroy_copy(&mut module, &[]);
        let body = module.functions[0].body.as_ref().unwrap();
        assert_eq!(body.blocks[0].insts.len(), 1);
        assert!(matches!(body.blocks[0].insts[0].kind, InstKind::Literal { .. }));
    }

    #[test]
    fn destroy_value_named_with_shim_becomes_call() {
        let mut module = make_module();
        let unit = module.ty_arena.unit();
        let named_ty = module.ty_arena.named(entity(10), vec![]);
        let body = make_body(
            vec![Instruction::new(InstKind::DestroyValue { operand: ValueId::new(1) })],
            ValueId::new(0),
            vec![ValueDef::owned(unit), ValueDef::owned(named_ty)],
        );
        let shim_body = make_body(vec![], ValueId::new(0), vec![ValueDef::owned(unit)]);
        module.add_function(make_mono_func("__drop$MyStruct", entity(20), vec![], unit, Some(shim_body)));
        module.add_function(make_mono_func("test", entity(1), vec![], unit, Some(body)));
        let generic_functions = vec![FunctionDef {
            entity: entity(20),
            name: "__drop$MyStruct".into(),
            kind: FunctionKind::DropShim { nominal: entity(10) },
            type_params: vec![], params: vec![], ret: unit,
            where_clause: None, body: None, extern_info: None,
        }];
        expand_destroy_copy(&mut module, &generic_functions);
        let body = module.functions[1].body.as_ref().unwrap();
        assert_eq!(body.blocks[0].insts.len(), 1);
        match &body.blocks[0].insts[0].kind {
            InstKind::Call { callee, args, result } => {
                assert!(matches!(callee, Callee::Resolved(id) if id.index() == 0));
                assert_eq!(args.len(), 1);
                assert_eq!(args[0].value, ValueId::new(1));
                assert_eq!(args[0].convention, ParamConvention::Consuming);
                assert!(result.is_none());
            }
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
                Instruction::new(InstKind::Literal { result: ValueId::new(1), value: Immediate::i64(42) }),
                Instruction::new(InstKind::CopyValue { result: ValueId::new(2), operand: ValueId::new(1) }),
                Instruction::new(InstKind::Op1 {
                    result: ValueId::new(3),
                    op: crate::op::Op::Neg(crate::op::IntBits::I64),
                    arg: ValueId::new(2),
                }),
            ],
            ValueId::new(0),
            vec![ValueDef::owned(unit), ValueDef::owned(i64_ty), ValueDef::owned(i64_ty), ValueDef::owned(v3_ty)],
        );
        module.add_function(make_mono_func("test", entity(1), vec![], unit, Some(body)));
        expand_destroy_copy(&mut module, &[]);
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
            vec![Instruction::new(InstKind::CopyValue { result: ValueId::new(2), operand: ValueId::new(1) })],
            ValueId::new(0),
            vec![ValueDef::owned(unit), ValueDef::owned(named_ty), ValueDef::owned(named_ty)],
        );
        module.add_function(make_mono_func("test", entity(1), vec![], unit, Some(body)));
        expand_destroy_copy(&mut module, &[]);
        let body = module.functions[0].body.as_ref().unwrap();
        assert_eq!(body.blocks[0].insts.len(), 1);
        assert!(matches!(body.blocks[0].insts[0].kind, InstKind::CopyValue { .. }));
    }

    #[test]
    fn copy_value_transitive_remap() {
        let mut module = make_module();
        let i64_ty = module.ty_arena.i64();
        let unit = module.ty_arena.unit();
        let mut block = BasicBlock::new();
        block.insts = vec![
            Instruction::new(InstKind::Literal { result: ValueId::new(1), value: Immediate::i64(99) }),
            Instruction::new(InstKind::CopyValue { result: ValueId::new(2), operand: ValueId::new(1) }),
            Instruction::new(InstKind::CopyValue { result: ValueId::new(3), operand: ValueId::new(2) }),
        ];
        block.terminator = Terminator::new(TerminatorKind::Return(ValueId::new(3)));
        let body = OssaBody {
            values: vec![ValueDef::owned(unit), ValueDef::owned(i64_ty), ValueDef::owned(i64_ty), ValueDef::owned(i64_ty)],
            blocks: vec![block],
            entry: BlockId::new(0),
            param_count: 0,
        };
        module.add_function(make_mono_func("test", entity(1), vec![], i64_ty, Some(body)));
        expand_destroy_copy(&mut module, &[]);
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
            vec![Instruction::new(InstKind::DestroyValue { operand: ValueId::new(1) })],
            ValueId::new(0),
            vec![ValueDef::owned(unit), ValueDef::owned(named_ty)],
        );
        let shim_body = make_body(vec![], ValueId::new(0), vec![ValueDef::owned(unit)]);
        module.add_function(make_mono_func("__drop$Array_Int64", entity(20), vec![i64_ty], unit, Some(shim_body)));
        module.add_function(make_mono_func("test", entity(1), vec![], unit, Some(body)));
        let generic_functions = vec![FunctionDef {
            entity: entity(20),
            name: "__drop$Array".into(),
            kind: FunctionKind::DropShim { nominal: entity(10) },
            type_params: vec![TypeParamDef::new(entity(30), "T")],
            params: vec![], ret: unit,
            where_clause: None, body: None, extern_info: None,
        }];
        expand_destroy_copy(&mut module, &generic_functions);
        let body = module.functions[1].body.as_ref().unwrap();
        assert_eq!(body.blocks[0].insts.len(), 1);
        assert!(matches!(
            &body.blocks[0].insts[0].kind,
            InstKind::Call { callee: Callee::Resolved(id), .. } if id.index() == 0
        ));
    }
}
