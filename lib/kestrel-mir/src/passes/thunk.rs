use rustc_hash::{FxHashMap, FxHashSet};

use kestrel_hecs::Entity;

use crate::block::BlockParam;
use crate::body::OssaBody;
use crate::callee::Callee;
use crate::inst::{CallArg, InstKind, Instruction};
use crate::item::function::{FunctionDef, FunctionKind, ParamDef};
use crate::terminator::{Terminator, TerminatorKind};
use crate::ty::{MirTy, ParamConvention};
use crate::value::{Ownership, RootProvenance, ValueDef};
use crate::{Immediate, MirModule, TyId};

/// Scan for ApplyPartial references and generate thunk wrappers.
pub fn run_thunk_pass(module: &mut MirModule, next_entity: &mut u32) {
    let mut targets: Vec<Entity> = Vec::new();
    let mut seen = FxHashSet::default();
    // Targets that already have a thunk from a previous run — collected in
    // the same scan so we don't rescan all functions per target below.
    let mut has_thunk: FxHashSet<Entity> = FxHashSet::default();

    for func in module.functions.values() {
        if let FunctionKind::Thunk { original } = &func.kind {
            has_thunk.insert(*original);
        }
        let Some(body) = &func.body else { continue };
        for block in &body.blocks {
            for inst in &block.insts {
                if let InstKind::ApplyPartial {
                    callee: Callee::Direct { func: target, .. },
                    ..
                } = &inst.kind
                    && seen.insert(*target)
                {
                    targets.push(*target);
                }
            }
        }
    }

    // Map of target -> freshly created thunk, applied in ONE rewrite pass at
    // the end. Rewriting inside the per-target loop rescans every instruction
    // in the module per target — quadratic on whole-program builds.
    let mut thunk_for: FxHashMap<Entity, Entity> = FxHashMap::default();

    for target in &targets {
        if has_thunk.contains(target) {
            continue;
        }

        let Some(target_func) = module.functions.get(target) else {
            continue;
        };

        let target_name = target_func.name.clone();
        let ret_ty = target_func.ret;
        let type_params = target_func.type_params.clone();

        // Whether params[0] is a synthesized env pointer is a property of the
        // function KIND, never of the parameter's spelling — a user function
        // may legally name its first parameter `env`, `_env` or `self`, and
        // matching on the name silently forwarded the environment pointer into
        // it while dropping the last real argument (G3).
        let needs_env = target_func.kind.takes_env_param();
        // Hard `assert!`, not `debug_assert!`: nothing in CI builds a debug
        // compiler and compiles Kestrel with it, so a `debug_assert` in the
        // pipeline is effectively dead code that only bites whoever next runs
        // `cargo build` without `--release`. These are O(params) once per
        // thunk — cheap enough to always be live.
        assert!(
            !needs_env
                || target_func
                    .params
                    .first()
                    .is_some_and(|p| p.name == "env" || p.name == "_env"),
            "{}: kind {:?} promises a leading env param but params[0] is {:?} — \
             the env param must stay at index 0 in mir-lower `closure.rs` / this pass",
            target_func.name,
            target_func.kind,
            target_func.params.first().map(|p| &p.name),
        );

        // Every param after the (optional) env pointer is a real parameter and
        // is forwarded positionally. Structural — no name filtering.
        let target_params: Vec<_> = target_func
            .params
            .iter()
            .skip(usize::from(needs_env))
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

        // Env parameter — @owned (pointer)
        let env_val = body.alloc_value(ValueDef::owned(env_ty));
        body.block_mut(entry).params.push(BlockParam {
            value: env_val,
            ty: env_ty,
            ownership: Ownership::Owned,
        });
        thunk_def.params.push(ParamDef::new(
            "_env",
            env_val,
            env_ty,
            ParamConvention::Consuming,
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
            if param.convention == ParamConvention::MutBorrow {
                // By-reference (`mutating`) param: the thunk receives it as a
                // @guaranteed mutable borrow (ByRef address) and forwards it
                // unchanged, so writes in the target reach the caller's place.
                let val = body.alloc_value(ValueDef {
                    ty: param.ty,
                    ownership: Ownership::Guaranteed,
                    borrow_source: None,
                    root: RootProvenance::derived(),
                    span: None,
                });
                body.block_mut(entry).params.push(BlockParam {
                    value: val,
                    ty: param.ty,
                    ownership: Ownership::Guaranteed,
                });
                thunk_def.params.push(ParamDef::new(
                    &param.name,
                    val,
                    param.ty,
                    ParamConvention::MutBorrow,
                ));
                body.param_count += 1;
                forward_args.push(CallArg {
                    value: val,
                    convention: ParamConvention::MutBorrow,
                });
                continue;
            }

            let val = body.alloc_value(ValueDef {
                ty: param.ty,
                ownership: Ownership::Owned,
                borrow_source: None,
                root: RootProvenance::derived(),
                span: None,
            });
            body.block_mut(entry).params.push(BlockParam {
                value: val,
                ty: param.ty,
                ownership: Ownership::Owned,
            });
            thunk_def.params.push(ParamDef::new(
                &param.name,
                val,
                param.ty,
                ParamConvention::Consuming,
            ));
            body.param_count += 1;

            // The thunk receives params as Consuming (by-value for scalars).
            // Forward args must also be Consuming so compile_resolved_call
            // spills scalars to stack when the target expects ByRef (Borrow).
            forward_args.push(CallArg {
                value: val,
                convention: ParamConvention::Consuming,
            });
        }

        // Fail-loud backstop. The forwarded arg list is built positionally from
        // the target's own params, so it must match it exactly in count, and in
        // per-position type for every REAL parameter. The type check is the
        // load-bearing half: G3's miscompile had a COINCIDENTALLY CORRECT count
        // (the env pointer was pushed and the same param was filtered out), and
        // only the types disagreed.
        //
        // Index 0 under `needs_env` is the one position where the two types
        // legitimately differ, and comparing `TyId`s there was a regression that
        // panicked on every stdlib program in a debug compiler. The thunk's env
        // param is deliberately TYPE-ERASED — `Pointer[()]` — because the thunk
        // is what an indirect closure call lands on and its signature must be
        // uniform across every closure; the target closure declares the concrete
        // env it wants (`Pointer[<synthesized env struct>]`, or the box binding's
        // raw pointer for a boxed closure — see mir-lower `closure.rs` `env_ty`
        // and `closure_box.rs::unwrap_handle_to_pointer`, both of which always
        // bottom out in `MirTy::Pointer`). Two different pointees, one machine
        // word; codegen reinterprets. So the strongest check that is still
        // correct at index 0 is "the target's env param is pointer-shaped".
        //
        // That is still enough to catch G3: its repro was `combine(env: Int, ...)`
        // mistaken for a closure, and `Int` is not a pointer, so this fires. What
        // it cannot distinguish is a wrong-slot forward into a param that is
        // itself a pointer — the arity check plus the kind-derived `needs_env`
        // (rather than the old name sniff) covers that shape.
        //
        // Every position after the env is an exact `TyId` cloned off
        // `target_func.params` pre-monomorphization, so equality there is exact —
        // no substitution or ref-decay gap to reason about.
        if let Some(target_params_full) = module.functions.get(target).map(|f| &f.params) {
            assert_eq!(
                forward_args.len(),
                target_params_full.len(),
                "{thunk_name}: forwards {} args to a {}-param target",
                forward_args.len(),
                target_params_full.len(),
            );
            let env_positions = usize::from(needs_env);
            if let Some(env_param) = target_params_full.first().filter(|_| needs_env) {
                assert!(
                    matches!(module.ty_arena.get(env_param.ty), MirTy::Pointer(_)),
                    "{thunk_name}: target's kind {:?} promises a leading env pointer but \
                     params[0] `{}` is {:?} — the thunk forwards a type-erased `Pointer[()]` \
                     into that slot, so a non-pointer there is a miscompile",
                    module.functions.get(target).map(|f| &f.kind),
                    env_param.name,
                    module.ty_arena.get(env_param.ty),
                );
            }
            assert!(
                forward_args
                    .iter()
                    .zip(target_params_full.iter())
                    .skip(env_positions)
                    .all(|(a, p)| body.value(a.value).ty == p.ty),
                "{thunk_name}: forwarded arg types do not match the target's params \
                 (forwarded {:?}, expected {:?})",
                forward_args
                    .iter()
                    .skip(env_positions)
                    .map(|a| module.ty_arena.get(body.value(a.value).ty))
                    .collect::<Vec<_>>(),
                target_params_full
                    .iter()
                    .skip(env_positions)
                    .map(|p| (&p.name, module.ty_arena.get(p.ty)))
                    .collect::<Vec<_>>(),
            );
        }

        let callee = Callee::direct_with_args(*target, forward_type_args, None);
        let is_unit = module.ty_arena.get(ret_ty) == &MirTy::Tuple(vec![]);

        let mut insts = Vec::new();

        // Destroy unused env pointer (no captures)
        if !needs_env {
            insts.push(Instruction::new(InstKind::DestroyValue {
                operand: env_val,
            }));
        }

        if is_unit {
            insts.push(Instruction::new(InstKind::Call {
                result: None,
                callee,
                args: forward_args,
            }));
            let unit_val = body.alloc_value(ValueDef::owned(unit_ty));
            insts.push(Instruction::new(InstKind::Literal {
                result: unit_val,
                value: Immediate::unit(),
            }));
            body.block_mut(entry).insts = insts;
            body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Return(unit_val));
        } else {
            let result_val = body.alloc_value(ValueDef {
                ty: ret_ty,
                ownership: Ownership::Owned,
                borrow_source: None,
                root: RootProvenance::derived(),
                span: None,
            });
            insts.push(Instruction::new(InstKind::Call {
                result: Some(result_val),
                callee,
                args: forward_args,
            }));
            body.block_mut(entry).insts = insts;
            body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Return(result_val));
        }

        thunk_def.body = Some(body);
        module.add_function(thunk_def);
        thunk_for.insert(*target, thunk_entity);
    }

    if thunk_for.is_empty() {
        return;
    }

    // Rewrite all ApplyPartial references to their thunk entities in one pass.
    for func in module.functions.values_mut() {
        let Some(body) = &mut func.body else { continue };
        for block in &mut body.blocks {
            for inst in &mut block.insts {
                if let InstKind::ApplyPartial {
                    callee: Callee::Direct { func: f, .. },
                    ..
                } = &mut inst.kind
                    && let Some(thunk) = thunk_for.get(f)
                {
                    *f = *thunk;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BlockParam;
    use crate::body::OssaBody;
    use crate::value::{RootProvenance, ValueDef};

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
            let val = body.alloc_value(ValueDef {
                ty: *pty,
                ownership: Ownership::Owned,
                borrow_source: None,
                root: RootProvenance::derived(),
                span: None,
            });
            body.block_mut(entry).params.push(BlockParam {
                value: val,
                ty: *pty,
                ownership: Ownership::Owned,
            });
            func.params.push(ParamDef::new(pname, val, *pty, *conv));
            body.param_count += 1;
        }

        let unit_ty = module.ty_arena.unit();
        let unit_val = body.alloc_value(ValueDef::owned(unit_ty));
        body.block_mut(entry)
            .insts
            .push(Instruction::new(InstKind::Literal {
                result: unit_val,
                value: Immediate::unit(),
            }));
        let ret_val = if module.ty_arena.get(ret_ty) == &MirTy::Tuple(vec![]) {
            unit_val
        } else {
            // For non-unit returns, just return a literal (simplified for tests)
            let rv = body.alloc_value(ValueDef::owned(ret_ty));
            body.block_mut(entry)
                .insts
                .push(Instruction::new(InstKind::Literal {
                    result: rv,
                    value: Immediate::i64(0),
                }));
            rv
        };
        body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Return(ret_val));

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

        // ApplyPartial result — simplified as @owned i64 for test purposes
        let thick_ty = module.ty_arena.intern(MirTy::FuncThick {
            kind: crate::ty::FnKind::Normal,
            params: vec![],
            ret: i64_ty,
        });
        let result_val = body.alloc_value(ValueDef::owned(thick_ty));
        body.block_mut(entry)
            .insts
            .push(Instruction::new(InstKind::ApplyPartial {
                result: result_val,
                callee: Callee::direct(target),
                captures: vec![],
                retain: None,
                release: None,
            }));

        let unit_val = body.alloc_value(ValueDef::owned(unit_ty));
        body.block_mut(entry)
            .insts
            .push(Instruction::new(InstKind::Literal {
                result: unit_val,
                value: Immediate::unit(),
            }));
        body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Return(unit_val));

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
        add_stub_function(
            &mut module,
            add_entity,
            "add",
            i64_ty,
            vec![
                ("a".into(), i64_ty, ParamConvention::Consuming),
                ("b".into(), i64_ty, ParamConvention::Consuming),
            ],
        );

        let caller = Entity::from_raw(2);
        add_caller_with_apply(&mut module, caller, add_entity);

        let mut next_entity = 100;
        run_thunk_pass(&mut module, &mut next_entity);

        let thunk = module.functions.values().find(
            |f| matches!(&f.kind, FunctionKind::Thunk { original } if *original == add_entity),
        );
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
        add_stub_function(
            &mut module,
            target,
            "target",
            i64_ty,
            vec![("x".into(), i64_ty, ParamConvention::Consuming)],
        );

        // Two ApplyPartial references in one caller
        let caller_entity = Entity::from_raw(2);
        {
            let unit_ty = module.ty_arena.unit();
            let thick_ty = module.ty_arena.intern(MirTy::FuncThick {
                kind: crate::ty::FnKind::Normal,
                params: vec![],
                ret: i64_ty,
            });

            let mut body = OssaBody::new();
            let entry = body.alloc_block();
            body.entry = entry;

            let r1 = body.alloc_value(ValueDef::owned(thick_ty));
            body.block_mut(entry)
                .insts
                .push(Instruction::new(InstKind::ApplyPartial {
                    result: r1,
                    callee: Callee::direct(target),
                    captures: vec![],
                    retain: None,
                    release: None,
                }));
            let r2 = body.alloc_value(ValueDef::owned(thick_ty));
            body.block_mut(entry)
                .insts
                .push(Instruction::new(InstKind::ApplyPartial {
                    result: r2,
                    callee: Callee::direct(target),
                    captures: vec![],
                    retain: None,
                    release: None,
                }));

            let uv = body.alloc_value(ValueDef::owned(unit_ty));
            body.block_mut(entry)
                .insts
                .push(Instruction::new(InstKind::Literal {
                    result: uv,
                    value: Immediate::unit(),
                }));
            body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Return(uv));

            let mut func = FunctionDef::new(caller_entity, "caller", unit_ty);
            func.body = Some(body);
            module.add_function(func);
        }

        let mut next_entity = 100;
        run_thunk_pass(&mut module, &mut next_entity);

        let thunk_count = module
            .functions
            .values()
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
        add_stub_function(
            &mut module,
            target,
            "target",
            i64_ty,
            vec![("x".into(), i64_ty, ParamConvention::Consuming)],
        );

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
        add_stub_function(
            &mut module,
            target,
            "compute",
            i64_ty,
            vec![("x".into(), i64_ty, ParamConvention::Consuming)],
        );

        let caller = Entity::from_raw(2);
        add_caller_with_apply(&mut module, caller, target);

        let mut next_entity = 100;
        run_thunk_pass(&mut module, &mut next_entity);

        let thunk = module
            .functions
            .values()
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

    /// G3: whether params[0] is an env pointer is decided by `FunctionKind`,
    /// not by the parameter's spelling. A plain `FunctionKind::Free` function
    /// whose first parameter happens to be named `env` must have BOTH of its
    /// parameters forwarded, with the env pointer forwarded to neither.
    /// Under the old name-based rule this thunk forwarded (env_ptr, x) into
    /// (env: Int64, x: Int64) — a same-count, wrong-type miscompile.
    #[test]
    fn env_named_user_param_is_not_an_env_pointer() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();

        let target = Entity::from_raw(1);
        module.register_name(target, "combine");
        add_stub_function(
            &mut module,
            target,
            "combine",
            i64_ty,
            vec![
                ("env".into(), i64_ty, ParamConvention::Consuming),
                ("x".into(), i64_ty, ParamConvention::Consuming),
            ],
        );
        assert_eq!(module.functions[&target].kind, FunctionKind::Free);

        let caller = Entity::from_raw(2);
        add_caller_with_apply(&mut module, caller, target);

        let mut next_entity = 100;
        run_thunk_pass(&mut module, &mut next_entity);

        let thunk = module
            .functions
            .values()
            .find(|f| matches!(&f.kind, FunctionKind::Thunk { .. }))
            .expect("thunk should be generated");
        let body = thunk.body.as_ref().unwrap();

        let args = body.blocks[0]
            .insts
            .iter()
            .find_map(|i| match &i.kind {
                InstKind::Call { callee, args, .. }
                    if matches!(callee, Callee::Direct { func, .. } if *func == target) =>
                {
                    Some(args)
                },
                _ => None,
            })
            .expect("thunk should forward a call to the target");

        assert_eq!(
            args.len(),
            2,
            "both user params must be forwarded, not just `x`"
        );
        assert_eq!(
            body.value(args[0].value).ty,
            i64_ty,
            "first forwarded arg must be the user's `env: Int64` param, not the env pointer"
        );
        assert_eq!(body.value(args[1].value).ty, i64_ty);

        // The env pointer the thunk itself receives is destroyed, not forwarded.
        let env_param = &thunk.params[0];
        assert_eq!(env_param.name, "_env");
        assert!(
            args.iter().all(|a| a.value != env_param.value),
            "the thunk's own env pointer must not be forwarded to a Free target"
        );
    }
}
