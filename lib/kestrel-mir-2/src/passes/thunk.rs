use std::collections::HashSet;

use kestrel_hecs::Entity;

use crate::body::{BasicBlock, LocalDef, MirBody};
use crate::immediate::Immediate;
use crate::item::function::{FunctionDef, FunctionKind, ParamDef};
use crate::operand::{ArgMode, Operand};
use crate::place::Place;
use crate::statement::{Callee, Rvalue, Statement, StatementKind};
use crate::terminator::Terminator;
use crate::ty::{MirTy, ParamConvention};
use crate::{MirModule, TyId};

/// Scan for ApplyPartial references and generate thunk wrappers.
pub fn run_thunk_pass(module: &mut MirModule, next_entity: &mut u32) {
    let mut targets: Vec<Entity> = Vec::new();
    let mut seen = HashSet::new();

    for func in &module.functions {
        let Some(body) = &func.body else { continue };
        for block in &body.blocks {
            for stmt in &block.stmts {
                if let StatementKind::Assign {
                    rvalue: Rvalue::ApplyPartial { func: target, .. },
                    ..
                } = &stmt.kind
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

        // Check if target expects an env parameter (closures)
        let needs_env = target_func
            .params
            .first()
            .is_some_and(|p| p.name == "env" || p.name == "_env");

        // Collect non-self, non-env params
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

        // Build type args for forwarding: TypeParam(tp.entity) for each type param
        let forward_type_args: Vec<TyId> = type_params
            .iter()
            .map(|tp| module.ty_arena.intern(MirTy::TypeParam(tp.entity)))
            .collect();

        let mut thunk_def = FunctionDef::new(thunk_entity, &thunk_name, ret_ty);
        thunk_def.type_params = type_params;
        thunk_def.kind = FunctionKind::Thunk { original: *target };

        let mut body = MirBody::new();

        // Env parameter (ignored by the thunk, passed through if target needs it)
        let env_local = body.add_local(LocalDef::new("_env", env_ty));
        thunk_def.params.push(ParamDef::new(
            "_env",
            env_local,
            env_ty,
            ParamConvention::Consuming,
        ));
        body.param_count += 1;

        // Build forward args
        let mut forward_args: Vec<(Operand, ArgMode)> = Vec::new();
        if needs_env {
            forward_args.push((Operand::Place(Place::local(env_local)), ArgMode::Copy));
        }

        for param in &target_params {
            let local = body.add_local(LocalDef::new(&param.name, param.ty));
            // Thunk params use Consuming convention to match the FuncThick
            // type (which always uses Consuming). The thunk forwards to the
            // original function using the original's expected ArgMode.
            thunk_def
                .params
                .push(ParamDef::new(&param.name, local, param.ty, ParamConvention::Consuming));
            body.param_count += 1;
            let mode = match param.convention {
                ParamConvention::Borrow => ArgMode::Ref,
                ParamConvention::MutBorrow => ArgMode::RefMut,
                ParamConvention::Consuming => ArgMode::Move,
            };
            forward_args.push((Operand::Place(Place::local(local)), mode));
        }

        // Build callee with forwarded type args
        let callee = Callee::direct_with_args(*target, forward_type_args, None);

        // Entry block: call target, return result
        let mut entry = BasicBlock::new();
        let is_unit = module.ty_arena.get(ret_ty) == &MirTy::Tuple(vec![]);

        if is_unit {
            entry.stmts.push(Statement::new(StatementKind::Call {
                dest: None,
                callee,
                args: forward_args,
            }));
            entry.terminator = Terminator::ret(Operand::Const(Immediate::unit()));
        } else {
            let result = body.add_local(LocalDef::new("_result", ret_ty));
            entry.stmts.push(Statement::new(StatementKind::Call {
                dest: Some(Place::local(result)),
                callee,
                args: forward_args,
            }));
            entry.terminator = Terminator::ret(Operand::Place(Place::local(result)));
        }

        let entry_id = body.add_block(entry);
        body.entry = entry_id;
        thunk_def.body = Some(body);
        module.register_name(thunk_entity, &thunk_name);
        module.add_function(thunk_def);

        // Rewrite ApplyPartial references to use the thunk entity so the
        // thick closure calls through the thunk (which matches FuncThick
        // calling conventions) rather than the original function directly.
        for func in &mut module.functions {
            let Some(body) = &mut func.body else { continue };
            for block in &mut body.blocks {
                for stmt in &mut block.stmts {
                    if let StatementKind::Assign {
                        rvalue: Rvalue::ApplyPartial { func: f, .. },
                        ..
                    } = &mut stmt.kind
                    {
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
    use crate::builder::ModuleBuilder;
    use crate::operand::UseMode;

    #[test]
    fn generates_thunk_for_apply_partial() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let unit_ty = m.unit();

        // Target function: add(a: i64, b: i64) -> i64
        let add_entity = m.fresh_entity();
        m.register_name(add_entity, "add");
        {
            let mut f = m.function_with_entity(add_entity, "add", i64_ty);
            let a = f.param("a", i64_ty, ParamConvention::Consuming);
            let b = f.param("b", i64_ty, ParamConvention::Consuming);
            let bb = f.block_id();
            f.block_at(bb).ret(Operand::Place(Place::local(a)));
        }

        // Caller uses ApplyPartial on add
        {
            let mut f = m.function("caller", unit_ty);
            let closure = f.local("closure", i64_ty); // simplified
            let bb = f.block_id();
            let mut b = f.block_at(bb);
            b.assign(
                Place::local(closure),
                Rvalue::ApplyPartial {
                    func: add_entity,
                    captures: vec![(Operand::Const(Immediate::i64(1)), UseMode::Copy)],
                },
            );
            b.ret_unit();
        }

        let mut module = m.finish();
        let mut next_entity = 100;
        run_thunk_pass(&mut module, &mut next_entity);

        // Should have generated a thunk
        let thunk = module
            .functions
            .iter()
            .find(|f| matches!(&f.kind, FunctionKind::Thunk { original } if *original == add_entity));
        assert!(thunk.is_some(), "thunk should be generated");

        let thunk = thunk.unwrap();
        assert!(thunk.name.contains("thunk"));
        assert!(thunk.body.is_some());

        // Thunk should have env + forwarded params
        let body = thunk.body.as_ref().unwrap();
        assert!(body.param_count >= 1); // at least _env
        assert_eq!(body.locals[0].name, "_env");
    }

    #[test]
    fn no_duplicate_thunks() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let unit_ty = m.unit();

        let target = m.fresh_entity();
        m.register_name(target, "target");
        {
            let mut f = m.function_with_entity(target, "target", i64_ty);
            let x = f.param("x", i64_ty, ParamConvention::Consuming);
            let bb = f.block_id();
            f.block_at(bb).ret(Operand::Place(Place::local(x)));
        }

        // Two ApplyPartial references to the same target
        {
            let mut f = m.function("caller", unit_ty);
            let a = f.local("a", i64_ty);
            let b = f.local("b", i64_ty);
            let bb = f.block_id();
            let mut bl = f.block_at(bb);
            bl.assign(
                Place::local(a),
                Rvalue::ApplyPartial {
                    func: target,
                    captures: vec![],
                },
            );
            bl.assign(
                Place::local(b),
                Rvalue::ApplyPartial {
                    func: target,
                    captures: vec![],
                },
            );
            bl.ret_unit();
        }

        let mut module = m.finish();
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
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let unit_ty = m.unit();

        let target = m.fresh_entity();
        m.register_name(target, "target");
        {
            let mut f = m.function_with_entity(target, "target", i64_ty);
            let x = f.param("x", i64_ty, ParamConvention::Consuming);
            let bb = f.block_id();
            f.block_at(bb).ret(Operand::Place(Place::local(x)));
        }

        // Pre-existing thunk
        let thunk_entity = m.fresh_entity();
        {
            let mut f = m.function_with_entity(thunk_entity, "target.thunk", i64_ty);
            f.set_kind(FunctionKind::Thunk { original: target });
            let bb = f.block_id();
            f.block_at(bb).ret(Operand::Const(Immediate::i64(0)));
        }

        // ApplyPartial reference
        {
            let mut f = m.function("caller", unit_ty);
            let a = f.local("a", i64_ty);
            let bb = f.block_id();
            let mut bl = f.block_at(bb);
            bl.assign(
                Place::local(a),
                Rvalue::ApplyPartial {
                    func: target,
                    captures: vec![],
                },
            );
            bl.ret_unit();
        }

        let mut module = m.finish();
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
        let mut m = ModuleBuilder::new("test");
        let unit_ty = m.unit();
        {
            let mut f = m.function("main", unit_ty);
            let bb = f.block_id();
            f.block_at(bb).ret_unit();
        }
        let mut module = m.finish();
        let func_count_before = module.functions.len();
        let mut next_entity = 100;
        run_thunk_pass(&mut module, &mut next_entity);
        assert_eq!(module.functions.len(), func_count_before);
    }

    #[test]
    fn thunk_forwards_call() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let unit_ty = m.unit();

        let target = m.fresh_entity();
        m.register_name(target, "compute");
        {
            let mut f = m.function_with_entity(target, "compute", i64_ty);
            let x = f.param("x", i64_ty, ParamConvention::Consuming);
            let bb = f.block_id();
            f.block_at(bb).ret(Operand::Place(Place::local(x)));
        }

        {
            let mut f = m.function("caller", unit_ty);
            let a = f.local("a", i64_ty);
            let bb = f.block_id();
            let mut bl = f.block_at(bb);
            bl.assign(
                Place::local(a),
                Rvalue::ApplyPartial {
                    func: target,
                    captures: vec![],
                },
            );
            bl.ret_unit();
        }

        let mut module = m.finish();
        let mut next_entity = 100;
        run_thunk_pass(&mut module, &mut next_entity);

        let thunk = module
            .functions
            .iter()
            .find(|f| matches!(&f.kind, FunctionKind::Thunk { .. }))
            .unwrap();
        let body = thunk.body.as_ref().unwrap();

        // Entry block should have a Call statement
        let has_call = body.blocks[0].stmts.iter().any(|s| {
            matches!(
                &s.kind,
                StatementKind::Call {
                    callee: Callee::Direct { func, .. },
                    ..
                } if *func == target
            )
        });
        assert!(has_call, "thunk should forward call to target");
    }
}
