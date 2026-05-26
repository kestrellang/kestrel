use std::collections::HashSet;

use kestrel_hecs::Entity;

use crate::block::BlockParam;
use crate::body::{OssaBody, ownership_for_type};
use crate::callee::Callee;
use crate::inst::{CallArg, InstKind, Instruction};
use crate::item::function::{FunctionDef, FunctionKind, ParamDef};
use crate::terminator::{Terminator, TerminatorKind};
use crate::ty::{MirTy, ParamConvention};
use crate::value::{Ownership, ValueDef};
use crate::{Immediate, MirModule, TyId};

/// Scan for ApplyPartial references and generate thunk wrappers.
pub fn run_thunk_pass(module: &mut MirModule, next_entity: &mut u32) {
    let mut targets: Vec<Entity> = Vec::new();
    let mut seen = HashSet::new();

    for func in &module.functions {
        let Some(body) = &func.body else { continue };
        for block in &body.blocks {
            for inst in &block.insts {
                if let InstKind::ApplyPartial { func: target, .. } = &inst.kind
                    && seen.insert(*target)
                {
                    targets.push(*target);
                }
            }
        }
    }

    for target in &targets {
        let already_has_thunk = module
            .functions
            .iter()
            .any(|f| matches!(&f.kind, FunctionKind::Thunk { original } if *original == *target));
        if already_has_thunk {
            continue;
        }

        let Some(target_func) = module.functions.iter().find(|f| f.entity == *target) else {
            continue;
        };

        let target_name = target_func.name.clone();
        let ret_ty = target_func.ret;
        let type_params = target_func.type_params.clone();

        let needs_env = target_func
            .params
            .first()
            .is_some_and(|p| p.name == "env" || p.name == "_env");

        // Non-self, non-env params from the target
        let target_params: Vec<_> = target_func
            .params
            .iter()
            .filter(|p| p.name != "self" && p.name != "env" && p.name != "_env")
            .cloned()
            .collect();

        let thunk_entity = Entity::from_raw(*next_entity);
        *next_entity += 1;
        let thunk_name = format!("{target_name}.thunk");
        module.register_name(thunk_entity, &thunk_name);

        let unit_ty = module.ty_arena.unit();
        let env_ty = module.ty_arena.pointer(unit_ty);

        // Type args for forwarding to the original
        let forward_type_args: Vec<TyId> = type_params
            .iter()
            .map(|tp| module.ty_arena.intern(MirTy::TypeParam(tp.entity)))
            .collect();

        let mut thunk_def = FunctionDef::new(thunk_entity, &thunk_name, ret_ty);
        thunk_def.type_params = type_params;
        thunk_def.kind = FunctionKind::Thunk { original: *target };

        let mut body = OssaBody::new();
        let entry = body.alloc_block();
        body.entry = entry;

        // Env parameter — @none (pointer)
        let env_val = body.alloc_value(ValueDef::none(env_ty));
        body.block_mut(entry).params.push(BlockParam {
            value: env_val,
            ty: env_ty,
            ownership: Ownership::None,
        });
        thunk_def.params.push(ParamDef::new(
            "_env", env_val, env_ty, ParamConvention::Consuming,
        ));
        body.param_count += 1;

        // Build forward args
        let mut forward_args: Vec<CallArg> = Vec::new();
        if needs_env {
            forward_args.push(CallArg {
                value: env_val,
                convention: ParamConvention::Consuming,
            });
        }

        for param in &target_params {
            let ownership = ownership_for_type(param.ty, &module.ty_arena, module);
            let val = body.alloc_value(ValueDef {
                ty: param.ty,
                ownership,
                borrow_source: None,
            });
            body.block_mut(entry).params.push(BlockParam {
                value: val,
                ty: param.ty,
                ownership,
            });
            thunk_def.params.push(ParamDef::new(
                &param.name, val, param.ty, ParamConvention::Consuming,
            ));
            body.param_count += 1;

            // The thunk receives params as Consuming (by-value for scalars).
            // Forward args must also be Consuming so compile_resolved_call
            // spills scalars to stack when the target expects ByRef (Borrow).
            forward_args.push(CallArg { value: val, convention: ParamConvention::Consuming });
        }

        let callee = Callee::direct_with_args(*target, forward_type_args, None);
        let is_unit = module.ty_arena.get(ret_ty) == &MirTy::Tuple(vec![]);

        let mut insts = Vec::new();

        if is_unit {
            insts.push(Instruction::new(InstKind::Call {
                result: None,
                callee,
                args: forward_args,
            }));
            let unit_val = body.alloc_value(ValueDef::none(unit_ty));
            insts.push(Instruction::new(InstKind::Literal {
                result: unit_val,
                value: Immediate::unit(),
            }));
            body.block_mut(entry).insts = insts;
            body.block_mut(entry).terminator =
                Terminator::new(TerminatorKind::Return(unit_val));
        } else {
            let ret_ownership = ownership_for_type(ret_ty, &module.ty_arena, module);
            let result_val = body.alloc_value(ValueDef {
                ty: ret_ty,
                ownership: ret_ownership,
                borrow_source: None,
            });
            insts.push(Instruction::new(InstKind::Call {
                result: Some(result_val),
                callee,
                args: forward_args,
            }));
            body.block_mut(entry).insts = insts;
            body.block_mut(entry).terminator =
                Terminator::new(TerminatorKind::Return(result_val));
        }

        thunk_def.body = Some(body);
        module.add_function(thunk_def);

        // Rewrite ApplyPartial references to use the thunk entity
        for func in &mut module.functions {
            let Some(body) = &mut func.body else { continue };
            for block in &mut body.blocks {
                for inst in &mut block.insts {
                    if let InstKind::ApplyPartial { func: f, .. } = &mut inst.kind {
                        if *f == *target {
                            *f = thunk_entity;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::OssaBody;
    use crate::block::BlockParam;
    use crate::value::ValueDef;

    /// Build a minimal function with an OSSA body (just returns unit).
    fn add_stub_function(
        module: &mut MirModule,
        entity: Entity,
        name: &str,
        ret_ty: TyId,
        params: Vec<(String, TyId, ParamConvention)>,
    ) {
        let mut body = OssaBody::new();
        let entry = body.alloc_block();
        body.entry = entry;

        let mut func = FunctionDef::new(entity, name, ret_ty);

        for (pname, pty, conv) in &params {
            let ownership = ownership_for_type(*pty, &module.ty_arena, module);
            let val = body.alloc_value(ValueDef {
                ty: *pty,
                ownership,
                borrow_source: None,
            });
            body.block_mut(entry).params.push(BlockParam {
                value: val,
                ty: *pty,
                ownership,
            });
            func.params.push(ParamDef::new(pname, val, *pty, *conv));
            body.param_count += 1;
        }

        let unit_ty = module.ty_arena.unit();
        let unit_val = body.alloc_value(ValueDef::none(unit_ty));
        body.block_mut(entry).insts.push(Instruction::new(InstKind::Literal {
            result: unit_val,
            value: Immediate::unit(),
        }));
        let ret_val = if module.ty_arena.get(ret_ty) == &MirTy::Tuple(vec![]) {
            unit_val
        } else {
            // For non-unit returns, just return a literal (simplified for tests)
            let rv = body.alloc_value(ValueDef::none(ret_ty));
            body.block_mut(entry).insts.push(Instruction::new(InstKind::Literal {
                result: rv,
                value: Immediate::i64(0),
            }));
            rv
        };
        body.block_mut(entry).terminator =
            Terminator::new(TerminatorKind::Return(ret_val));

        func.body = Some(body);
        module.add_function(func);
    }

    /// Build a caller function that has an ApplyPartial instruction.
    fn add_caller_with_apply(module: &mut MirModule, caller_entity: Entity, target: Entity) {
        let unit_ty = module.ty_arena.unit();
        let i64_ty = module.ty_arena.i64();

        let mut body = OssaBody::new();
        let entry = body.alloc_block();
        body.entry = entry;

        // ApplyPartial result — simplified as @none i64 for test purposes
        let thick_ty = module.ty_arena.intern(MirTy::FuncThick {
            params: vec![],
            ret: i64_ty,
        });
        let result_val = body.alloc_value(ValueDef::none(thick_ty));
        body.block_mut(entry).insts.push(Instruction::new(InstKind::ApplyPartial {
            result: result_val,
            func: target,
            captures: vec![],
        }));

        let unit_val = body.alloc_value(ValueDef::none(unit_ty));
        body.block_mut(entry).insts.push(Instruction::new(InstKind::Literal {
            result: unit_val,
            value: Immediate::unit(),
        }));
        body.block_mut(entry).terminator =
            Terminator::new(TerminatorKind::Return(unit_val));

        let mut func = FunctionDef::new(caller_entity, "caller", unit_ty);
        func.body = Some(body);
        module.add_function(func);
    }

    #[test]
    fn generates_thunk_for_apply_partial() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();

        let add_entity = Entity::from_raw(1);
        module.register_name(add_entity, "add");
        add_stub_function(&mut module, add_entity, "add", i64_ty, vec![
            ("a".into(), i64_ty, ParamConvention::Consuming),
            ("b".into(), i64_ty, ParamConvention::Consuming),
        ]);

        let caller = Entity::from_raw(2);
        add_caller_with_apply(&mut module, caller, add_entity);

        let mut next_entity = 100;
        run_thunk_pass(&mut module, &mut next_entity);

        let thunk = module
            .functions
            .iter()
            .find(|f| matches!(&f.kind, FunctionKind::Thunk { original } if *original == add_entity));
        assert!(thunk.is_some(), "thunk should be generated");

        let thunk = thunk.unwrap();
        assert!(thunk.name.contains("thunk"));
        assert!(thunk.body.is_some());

        let body = thunk.body.as_ref().unwrap();
        // At least env param
        assert!(body.param_count >= 1);
    }

    #[test]
    fn no_duplicate_thunks() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();

        let target = Entity::from_raw(1);
        module.register_name(target, "target");
        add_stub_function(&mut module, target, "target", i64_ty, vec![
            ("x".into(), i64_ty, ParamConvention::Consuming),
        ]);

        // Two ApplyPartial references in one caller
        let caller_entity = Entity::from_raw(2);
        {
            let unit_ty = module.ty_arena.unit();
            let thick_ty = module.ty_arena.intern(MirTy::FuncThick {
                params: vec![],
                ret: i64_ty,
            });

            let mut body = OssaBody::new();
            let entry = body.alloc_block();
            body.entry = entry;

            let r1 = body.alloc_value(ValueDef::none(thick_ty));
            body.block_mut(entry).insts.push(Instruction::new(InstKind::ApplyPartial {
                result: r1,
                func: target,
                captures: vec![],
            }));
            let r2 = body.alloc_value(ValueDef::none(thick_ty));
            body.block_mut(entry).insts.push(Instruction::new(InstKind::ApplyPartial {
                result: r2,
                func: target,
                captures: vec![],
            }));

            let uv = body.alloc_value(ValueDef::none(unit_ty));
            body.block_mut(entry).insts.push(Instruction::new(InstKind::Literal {
                result: uv,
                value: Immediate::unit(),
            }));
            body.block_mut(entry).terminator =
                Terminator::new(TerminatorKind::Return(uv));

            let mut func = FunctionDef::new(caller_entity, "caller", unit_ty);
            func.body = Some(body);
            module.add_function(func);
        }

        let mut next_entity = 100;
        run_thunk_pass(&mut module, &mut next_entity);

        let thunk_count = module
            .functions
            .iter()
            .filter(|f| matches!(&f.kind, FunctionKind::Thunk { .. }))
            .count();
        assert_eq!(thunk_count, 1, "should deduplicate thunks");
    }

    #[test]
    fn skips_existing_thunk() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();

        let target = Entity::from_raw(1);
        module.register_name(target, "target");
        add_stub_function(&mut module, target, "target", i64_ty, vec![
            ("x".into(), i64_ty, ParamConvention::Consuming),
        ]);

        // Pre-existing thunk
        let thunk_entity = Entity::from_raw(2);
        {
            let mut thunk_func = FunctionDef::new(thunk_entity, "target.thunk", i64_ty);
            thunk_func.kind = FunctionKind::Thunk { original: target };
            module.add_function(thunk_func);
        }

        let caller = Entity::from_raw(3);
        add_caller_with_apply(&mut module, caller, target);

        let func_count_before = module.functions.len();
        let mut next_entity = 100;
        run_thunk_pass(&mut module, &mut next_entity);

        assert_eq!(
            module.functions.len(),
            func_count_before,
            "should not generate another thunk"
        );
    }

    #[test]
    fn no_apply_partial_no_thunks() {
        let mut module = MirModule::new("test");
        let unit_ty = module.ty_arena.unit();

        let main_entity = Entity::from_raw(1);
        add_stub_function(&mut module, main_entity, "main", unit_ty, vec![]);

        let func_count_before = module.functions.len();
        let mut next_entity = 100;
        run_thunk_pass(&mut module, &mut next_entity);
        assert_eq!(module.functions.len(), func_count_before);
    }

    #[test]
    fn thunk_forwards_call() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();

        let target = Entity::from_raw(1);
        module.register_name(target, "compute");
        add_stub_function(&mut module, target, "compute", i64_ty, vec![
            ("x".into(), i64_ty, ParamConvention::Consuming),
        ]);

        let caller = Entity::from_raw(2);
        add_caller_with_apply(&mut module, caller, target);

        let mut next_entity = 100;
        run_thunk_pass(&mut module, &mut next_entity);

        let thunk = module
            .functions
            .iter()
            .find(|f| matches!(&f.kind, FunctionKind::Thunk { .. }))
            .unwrap();
        let body = thunk.body.as_ref().unwrap();

        // Entry block should have a Call instruction forwarding to target
        let has_call = body.blocks[0].insts.iter().any(|i| {
            matches!(
                &i.kind,
                InstKind::Call {
                    callee: Callee::Direct { func, .. },
                    ..
                } if *func == target
            )
        });
        assert!(has_call, "thunk should forward call to target");
    }
}
