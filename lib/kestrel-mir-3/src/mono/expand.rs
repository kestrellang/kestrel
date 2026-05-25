use std::collections::HashMap;

use kestrel_hecs::Entity;

use crate::callee::Callee;
use crate::inst::{CallArg, InstKind, Instruction};
use crate::item::function::{FunctionDef, FunctionKind};
use crate::mono::types::{MonoFunction, MonoModule};
use crate::ty::{MirTy, ParamConvention};
use crate::value::Ownership;
use crate::{MonoFuncId, TyId, ValueId};

/// Expand DestroyValue and CopyValue instructions in all monomorphized bodies.
///
/// After monomorphization, some values that were generic `T` are now concrete
/// trivial types with `Ownership::None`. The OSSA verifier rejects both
/// `DestroyValue` and `CopyValue` on `@none` values, so this pass:
///
/// 1. Removes `DestroyValue` on `@none` values.
/// 2. Replaces `DestroyValue` on Named types with a call to their drop shim.
/// 3. Removes `DestroyValue` on types without drop shims (FuncThick, etc.).
/// 4. Removes `CopyValue` on `@none` values and remaps `result -> operand`.
///
/// Must run after `monomorphize()` and before `verify_mono()`.
pub fn expand_destroy_copy(
    module: &mut MonoModule,
    generic_functions: &[FunctionDef],
) {
    // Build a lookup from (nominal_entity, type_args) -> MonoFuncId for drop shims.
    // For each MonoFunction whose source entity corresponds to a DropShim in the
    // generic module, record which nominal type it drops.
    let shim_lookup = build_drop_shim_lookup(module, generic_functions);

    for fi in 0..module.functions.len() {
        expand_function(&mut module.functions[fi], &module.ty_arena, &shim_lookup);
    }
}

/// Maps (nominal_entity, type_args) -> MonoFuncId for drop shim dispatch.
type DropShimLookup = HashMap<(Entity, Vec<TyId>), MonoFuncId>;

/// Scan the generic function list for DropShim functions, then find their
/// monomorphized counterparts in the MonoModule.
fn build_drop_shim_lookup(
    module: &MonoModule,
    generic_functions: &[FunctionDef],
) -> DropShimLookup {
    // Map shim_entity -> nominal_entity from the generic module
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
            // The shim's type_args correspond to the Named type's type_args
            lookup.insert(
                (nominal, mf.type_args.clone()),
                MonoFuncId::new(mi),
            );
        }
    }

    lookup
}

/// Expand DestroyValue/CopyValue in a single function body.
fn expand_function(
    func: &mut MonoFunction,
    ty_arena: &crate::ty::TyArena,
    shim_lookup: &DropShimLookup,
) {
    let Some(body) = &mut func.body else { return };

    // value_remap tracks CopyValue removals: result -> operand
    let mut value_remap: HashMap<ValueId, ValueId> = HashMap::new();

    for block in &mut body.blocks {
        let mut new_insts: Vec<Instruction> = Vec::with_capacity(block.insts.len());

        for inst in block.insts.drain(..) {
            match &inst.kind {
                InstKind::DestroyValue { operand } => {
                    let operand = *operand;
                    let value_def = &body.values[operand.index()];

                    if value_def.ownership == Ownership::None {
                        // Trivial type after mono — just drop the instruction
                        continue;
                    }

                    // Owned value — check if it's a Named type with a drop shim
                    match ty_arena.get(value_def.ty) {
                        MirTy::Named { entity, type_args } => {
                            let key = (*entity, type_args.clone());
                            if let Some(&shim_id) = shim_lookup.get(&key) {
                                // Replace with Call(drop_shim)
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
                            // Named type without a drop shim — remove silently
                        }
                        // FuncThick, Tuple, etc. with @owned but no shim — remove
                        _ => {}
                    }
                }

                InstKind::CopyValue { result, operand } => {
                    let result = *result;
                    let operand = *operand;
                    let value_def = &body.values[operand.index()];

                    if value_def.ownership == Ownership::None {
                        // Trivially copyable after mono — remove and remap
                        let target = remap_value(operand, &value_remap);
                        value_remap.insert(result, target);
                        continue;
                    }

                    // Non-trivial CopyValue — keep as-is (clone expansion is separate)
                    let mut kept = inst;
                    remap_inst_operands(&mut kept.kind, &value_remap);
                    new_insts.push(kept);
                }

                _ => {
                    // All other instructions: apply value remapping and keep
                    let mut kept = inst;
                    remap_inst_operands(&mut kept.kind, &value_remap);
                    new_insts.push(kept);
                }
            }
        }

        block.insts = new_insts;

        // Remap ValueIds in the terminator
        remap_terminator(&mut block.terminator.kind, &value_remap);
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

        InstKind::Literal { .. } | InstKind::GlobalRef { .. } => {
            // No operand ValueIds
        }

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
            // Also remap indirect callee values (Thin/Thick)
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

        InstKind::Uninit { .. } => {
            // No operand ValueIds
        }
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

    /// Build a single-block body with the given instructions, returning `ret_val`.
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

    // -- Test 1: DestroyValue on @none value is removed --

    #[test]
    fn destroy_value_on_none_removed() {
        let mut module = make_module();
        let i64_ty = module.ty_arena.i64();
        let unit = module.ty_arena.unit();

        // v0 = unit (ret), v1 = i64 @none (will be destroyed)
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
            vec![
                ValueDef::none(unit),
                ValueDef::none(i64_ty), // @none — trivial type
            ],
        );

        module.add_function(make_mono_func("test", entity(1), vec![], unit, Some(body)));

        let generic_functions: Vec<FunctionDef> = vec![];
        expand_destroy_copy(&mut module, &generic_functions);

        // DestroyValue should be gone, only the Literal remains
        let body = module.functions[0].body.as_ref().unwrap();
        assert_eq!(body.blocks[0].insts.len(), 1);
        assert!(matches!(body.blocks[0].insts[0].kind, InstKind::Literal { .. }));
    }

    // -- Test 2: DestroyValue on Named type with shim becomes Call --

    #[test]
    fn destroy_value_named_with_shim_becomes_call() {
        let mut module = make_module();
        let unit = module.ty_arena.unit();

        // A Named type "MyStruct" (entity 10)
        let named_ty = module.ty_arena.named(entity(10), vec![]);

        // v0 = unit (ret), v1 = MyStruct @owned
        let body = make_body(
            vec![
                Instruction::new(InstKind::DestroyValue {
                    operand: ValueId::new(1),
                }),
            ],
            ValueId::new(0),
            vec![
                ValueDef::none(unit),
                ValueDef::owned(named_ty), // @owned Named type
            ],
        );

        // Add the drop shim as a MonoFunction (source = entity(20), the shim func)
        let shim_body = make_body(vec![], ValueId::new(0), vec![ValueDef::none(unit)]);
        let shim_func = make_mono_func("__drop$MyStruct", entity(20), vec![], unit, Some(shim_body));
        module.add_function(shim_func); // MonoFuncId(0)

        // Add the function under test
        module.add_function(make_mono_func("test", entity(1), vec![], unit, Some(body)));
        // MonoFuncId(1) is the test function

        // Generic module: entity(20) is a DropShim for entity(10)
        let generic_functions = vec![
            FunctionDef {
                entity: entity(20),
                name: "__drop$MyStruct".into(),
                kind: FunctionKind::DropShim { nominal: entity(10) },
                type_params: vec![],
                params: vec![],
                ret: unit,
                where_clause: None,
                body: None,
                extern_info: None,
            },
        ];

        expand_destroy_copy(&mut module, &generic_functions);

        // The DestroyValue should have become a Call to MonoFuncId(0)
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

    // -- Test 3: CopyValue on @none is removed, result remapped --

    #[test]
    fn copy_value_on_none_removed_and_remapped() {
        let mut module = make_module();
        let i64_ty = module.ty_arena.i64();
        let unit = module.ty_arena.unit();

        // v0 = unit (ret), v1 = i64 @none (src), v2 = i64 @none (copy result)
        // The CopyValue copies v1 -> v2, then v2 is used in an Op1.
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
                // Use v2 in an Op1 — after removal, this should reference v1
                Instruction::new(InstKind::Op1 {
                    result: ValueId::new(3),
                    op: crate::op::Op::Neg(crate::op::IntBits::I64),
                    arg: ValueId::new(2),
                }),
            ],
            ValueId::new(0),
            vec![
                ValueDef::none(unit),
                ValueDef::none(i64_ty),
                ValueDef::none(i64_ty), // copy result, also @none
                ValueDef::none(v3_ty),
            ],
        );

        module.add_function(make_mono_func("test", entity(1), vec![], unit, Some(body)));

        let generic_functions: Vec<FunctionDef> = vec![];
        expand_destroy_copy(&mut module, &generic_functions);

        let body = module.functions[0].body.as_ref().unwrap();
        // CopyValue removed -> 2 instructions remain (Literal + Op1)
        assert_eq!(body.blocks[0].insts.len(), 2);
        assert!(matches!(body.blocks[0].insts[0].kind, InstKind::Literal { .. }));
        // The Op1 should now reference v1 (the original), not v2
        match &body.blocks[0].insts[1].kind {
            InstKind::Op1 { arg, .. } => {
                assert_eq!(*arg, ValueId::new(1));
            }
            other => panic!("expected Op1, got {:?}", other),
        }
    }

    // -- Test 4: Mixed body with multiple expansions --

    #[test]
    fn mixed_body_multiple_expansions() {
        let mut module = make_module();
        let i64_ty = module.ty_arena.i64();
        let unit = module.ty_arena.unit();
        let named_ty = module.ty_arena.named(entity(10), vec![]);

        // Body with:
        //   v1 = Literal 42          (@none i64)
        //   CopyValue v1 -> v2       (@none -> removed, v2 remaps to v1)
        //   v3 = Struct named_ty     (@owned)
        //   DestroyValue v3          (Named with shim -> Call)
        //   DestroyValue v2          (was remapped from v2->v1; @none -> removed)
        //   return v0
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
                Instruction::new(InstKind::Struct {
                    result: ValueId::new(3),
                    ty: named_ty,
                    fields: vec![],
                }),
                Instruction::new(InstKind::DestroyValue {
                    operand: ValueId::new(3),
                }),
                Instruction::new(InstKind::DestroyValue {
                    operand: ValueId::new(2),
                }),
            ],
            ValueId::new(0),
            vec![
                ValueDef::none(unit),       // v0: return value
                ValueDef::none(i64_ty),     // v1: literal
                ValueDef::none(i64_ty),     // v2: copy of v1 (@none)
                ValueDef::owned(named_ty),  // v3: struct
            ],
        );

        // Drop shim for entity(10)
        let shim_body = make_body(vec![], ValueId::new(0), vec![ValueDef::none(unit)]);
        module.add_function(make_mono_func("__drop$MyStruct", entity(20), vec![], unit, Some(shim_body)));
        module.add_function(make_mono_func("test", entity(1), vec![], unit, Some(body)));

        let generic_functions = vec![
            FunctionDef {
                entity: entity(20),
                name: "__drop$MyStruct".into(),
                kind: FunctionKind::DropShim { nominal: entity(10) },
                type_params: vec![],
                params: vec![],
                ret: unit,
                where_clause: None,
                body: None,
                extern_info: None,
            },
        ];

        expand_destroy_copy(&mut module, &generic_functions);

        let body = module.functions[1].body.as_ref().unwrap();
        let insts = &body.blocks[0].insts;

        // Expected remaining instructions:
        //   0: Literal v1
        //   1: Struct v3
        //   2: Call(drop_shim, v3)
        // CopyValue removed, DestroyValue on v2 (now @none) removed
        assert_eq!(insts.len(), 3, "got: {insts:#?}");

        assert!(matches!(insts[0].kind, InstKind::Literal { .. }));
        assert!(matches!(insts[1].kind, InstKind::Struct { .. }));
        match &insts[2].kind {
            InstKind::Call { callee, args, .. } => {
                assert!(matches!(callee, Callee::Resolved(id) if id.index() == 0));
                assert_eq!(args[0].value, ValueId::new(3));
            }
            other => panic!("expected Call, got {:?}", other),
        }
    }

    // -- Test 5: DestroyValue on Named type without shim is removed --

    #[test]
    fn destroy_value_named_without_shim_removed() {
        let mut module = make_module();
        let unit = module.ty_arena.unit();
        let named_ty = module.ty_arena.named(entity(10), vec![]);

        let body = make_body(
            vec![
                Instruction::new(InstKind::DestroyValue {
                    operand: ValueId::new(1),
                }),
            ],
            ValueId::new(0),
            vec![
                ValueDef::none(unit),
                ValueDef::owned(named_ty),
            ],
        );

        module.add_function(make_mono_func("test", entity(1), vec![], unit, Some(body)));

        // No generic functions with DropShim
        let generic_functions: Vec<FunctionDef> = vec![];
        expand_destroy_copy(&mut module, &generic_functions);

        let body = module.functions[0].body.as_ref().unwrap();
        assert_eq!(body.blocks[0].insts.len(), 0);
    }

    // -- Test 6: CopyValue on @owned is kept --

    #[test]
    fn copy_value_on_owned_kept() {
        let mut module = make_module();
        let unit = module.ty_arena.unit();
        let named_ty = module.ty_arena.named(entity(10), vec![]);

        let body = make_body(
            vec![
                Instruction::new(InstKind::CopyValue {
                    result: ValueId::new(2),
                    operand: ValueId::new(1),
                }),
            ],
            ValueId::new(0),
            vec![
                ValueDef::none(unit),
                ValueDef::owned(named_ty), // @owned — CopyValue must stay
                ValueDef::owned(named_ty),
            ],
        );

        module.add_function(make_mono_func("test", entity(1), vec![], unit, Some(body)));

        let generic_functions: Vec<FunctionDef> = vec![];
        expand_destroy_copy(&mut module, &generic_functions);

        let body = module.functions[0].body.as_ref().unwrap();
        assert_eq!(body.blocks[0].insts.len(), 1);
        assert!(matches!(
            body.blocks[0].insts[0].kind,
            InstKind::CopyValue { .. }
        ));
    }

    // -- Test 7: CopyValue remap chains resolve transitively --

    #[test]
    fn copy_value_transitive_remap() {
        let mut module = make_module();
        let i64_ty = module.ty_arena.i64();
        let unit = module.ty_arena.unit();

        // v1 = Literal, CopyValue v1->v2, CopyValue v2->v3, use v3 in Return
        // All @none, so both CopyValues are removed. v3 should resolve to v1.
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
                ValueDef::none(unit),
                ValueDef::none(i64_ty),
                ValueDef::none(i64_ty),
                ValueDef::none(i64_ty),
            ],
            blocks: vec![block],
            entry: BlockId::new(0),
            param_count: 0,
        };

        module.add_function(make_mono_func("test", entity(1), vec![], i64_ty, Some(body)));

        let generic_functions: Vec<FunctionDef> = vec![];
        expand_destroy_copy(&mut module, &generic_functions);

        let body = module.functions[0].body.as_ref().unwrap();
        // Both CopyValues removed — only Literal remains
        assert_eq!(body.blocks[0].insts.len(), 1);
        // Return should now reference v1
        match &body.blocks[0].terminator.kind {
            TerminatorKind::Return(v) => assert_eq!(*v, ValueId::new(1)),
            other => panic!("expected Return, got {:?}", other),
        }
    }

    // -- Test 8: Drop shim lookup works with type args --

    #[test]
    fn destroy_value_generic_named_with_type_args() {
        let mut module = make_module();
        let unit = module.ty_arena.unit();
        let i64_ty = module.ty_arena.i64();

        // Named type Array[Int64] — entity(10) with type_args [i64_ty]
        let named_ty = module.ty_arena.named(entity(10), vec![i64_ty]);

        let body = make_body(
            vec![
                Instruction::new(InstKind::DestroyValue {
                    operand: ValueId::new(1),
                }),
            ],
            ValueId::new(0),
            vec![
                ValueDef::none(unit),
                ValueDef::owned(named_ty),
            ],
        );

        // Drop shim for Array[Int64]: source=entity(20), type_args=[i64_ty]
        let shim_body = make_body(vec![], ValueId::new(0), vec![ValueDef::none(unit)]);
        let shim_func = make_mono_func("__drop$Array_Int64", entity(20), vec![i64_ty], unit, Some(shim_body));
        module.add_function(shim_func); // MonoFuncId(0)

        module.add_function(make_mono_func("test", entity(1), vec![], unit, Some(body)));

        let generic_functions = vec![
            FunctionDef {
                entity: entity(20),
                name: "__drop$Array".into(),
                kind: FunctionKind::DropShim { nominal: entity(10) },
                type_params: vec![TypeParamDef::new(entity(30), "T")],
                params: vec![],
                ret: unit,
                where_clause: None,
                body: None,
                extern_info: None,
            },
        ];

        expand_destroy_copy(&mut module, &generic_functions);

        let body = module.functions[1].body.as_ref().unwrap();
        assert_eq!(body.blocks[0].insts.len(), 1);
        assert!(matches!(
            &body.blocks[0].insts[0].kind,
            InstKind::Call { callee: Callee::Resolved(id), .. } if id.index() == 0
        ));
    }
}
