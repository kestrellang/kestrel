//! Thunk pass — generate and deduplicate thunk wrappers for ApplyPartial.
//!
//! When a function is used as a thick callable (via ApplyPartial), codegen
//! needs a thunk that bridges the calling convention: it accepts an env_ptr
//! parameter (which it ignores) and forwards the remaining args to the
//! original function.
//!
//! This pass:
//! 1. Scans all function bodies for `Rvalue::ApplyPartial` references
//! 2. For each unique function reference, generates a thunk if one doesn't exist
//! 3. Deduplicates — the same function referenced multiple times gets one thunk

use std::collections::HashMap;

use kestrel_hecs::Entity;

use crate::MirModule;
use crate::body::{BasicBlock, LocalDef, MirBody};
use crate::id::FunctionId;
use crate::immediate::Immediate;
use crate::item::{FunctionDef, FunctionKind, ParamDef};
use crate::statement::{CallArg, Callee, Rvalue, Statement, StatementKind};
use crate::terminator::Terminator;
use crate::ty::MirTy;
use crate::value::Value;

/// Scan for ApplyPartial references and generate thunk wrappers.
///
/// Each unique function entity referenced by ApplyPartial gets a thunk
/// function that wraps it with a thick-callable ABI (env_ptr + params).
pub fn run_thunk_pass(module: &mut MirModule) {
    // Collect all function entities referenced by ApplyPartial
    let mut thunk_targets: Vec<Entity> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for func in &module.functions {
        let Some(body) = &func.body else { continue };
        for block in &body.blocks {
            for stmt in &block.stmts {
                if let StatementKind::Assign {
                    rvalue: Rvalue::ApplyPartial { func: target, .. },
                    ..
                } = &stmt.kind
                {
                    if seen.insert(*target) {
                        thunk_targets.push(*target);
                    }
                }
            }
        }
    }

    // Generate a thunk for each target that doesn't already have one
    let mut thunk_map: HashMap<Entity, FunctionId> = HashMap::new();

    for target in &thunk_targets {
        // Check if the target function already has a thunk
        let already_has_thunk = module
            .functions
            .iter()
            .any(|f| matches!(&f.kind, FunctionKind::Thunk { original } if *original == *target));
        if already_has_thunk {
            continue;
        }

        // Find the target function and extract its signature data
        let target_info = module
            .functions
            .iter()
            .find(|f| f.entity == *target)
            .map(|f| {
                let name = f.name.clone();
                let ret = f.ret.clone();
                let type_params = f.type_params.clone();
                let kind = f.kind.clone();
                // Check if target expects an env parameter (closures do)
                let needs_env = f
                    .params
                    .first()
                    .map_or(false, |p| p.name == "env" || p.name == "_env");
                let params: Vec<_> = f
                    .params
                    .iter()
                    .filter(|p| p.name != "self" && p.name != "env" && p.name != "_env")
                    .cloned()
                    .collect();
                (name, ret, params, type_params, kind, needs_env)
            });
        let Some((
            target_name,
            ret_ty,
            target_params,
            target_type_params,
            target_kind,
            target_needs_env,
        )) = target_info
        else {
            continue;
        };

        // Build thunk
        let thunk_name = format!("{}.thunk", target_name);
        let thunk_entity = Entity::from_raw(u32::MAX / 2 + module.functions.len() as u32);
        module.register_name(thunk_entity, &thunk_name);

        let mut thunk_def = FunctionDef::new(thunk_entity, &thunk_name, ret_ty.clone());
        thunk_def.type_params = target_type_params;
        thunk_def.kind = FunctionKind::Thunk { original: *target };
        let forward_type_args: Vec<MirTy> = thunk_def
            .type_params
            .iter()
            .map(|tp| MirTy::TypeParam(tp.entity))
            .collect();

        // Create body
        let mut body = MirBody::new();

        // Env parameter (ignored)
        let env_local =
            body.add_local(LocalDef::new("_env", MirTy::Pointer(Box::new(MirTy::unit()))));
        thunk_def.params.push(ParamDef::new(
            "_env",
            env_local,
            MirTy::Pointer(Box::new(MirTy::unit())),
        ));
        body.param_count += 1;

        // If the target closure expects an env pointer, forward it.
        // Use Copy mode (not Borrow): env_local is itself a pointer; we want to
        // pass the pointer VALUE so the callee receives env_ptr directly.
        // Borrow mode would pass &env_ptr_storage and the callee would need a
        // double dereference it doesn't perform — captured values become garbage.
        let mut forward_args = Vec::new();
        if target_needs_env {
            forward_args.push(CallArg::copy(Value::Place(crate::place::Place::local(
                env_local,
            ))));
        }
        for param in &target_params {
            let local = body.add_local(LocalDef::new(&param.name, param.ty.clone()));
            thunk_def
                .params
                .push(ParamDef::new(&param.name, local, param.ty.clone()));
            body.param_count += 1;
            forward_args.push(CallArg::borrow(Value::Place(crate::place::Place::local(
                local,
            ))));
        }

        // Create entry block: call target and return result
        let mut entry = BasicBlock::new();
        let callee = match target_kind {
            FunctionKind::Method { parent, .. }
            | FunctionKind::Initializer { parent }
            | FunctionKind::Deinit { parent } => Callee::method(
                *target,
                forward_type_args.clone(),
                MirTy::Named {
                    entity: parent,
                    type_args: forward_type_args.clone(),
                },
            ),
            _ => Callee::direct_generic(*target, forward_type_args.clone()),
        };

        if ret_ty.is_unit() || ret_ty == MirTy::Never {
            // Void call + return unit
            entry.stmts.push(Statement::new(StatementKind::Call {
                dest: None,
                callee: callee.clone(),
                args: forward_args,
            }));
            entry.terminator = Terminator::ret(Immediate::unit());
        } else {
            // Call with return value
            let result = body.add_local(LocalDef::new("_result", ret_ty));
            entry.stmts.push(Statement::new(StatementKind::Call {
                dest: Some(crate::place::Place::local(result)),
                callee,
                args: forward_args,
            }));
            entry.terminator = Terminator::ret(Value::Place(crate::place::Place::local(result)));
        }

        let entry_id = body.add_block(entry);
        body.entry = entry_id;

        thunk_def.body = Some(body);
        let thunk_id = module.add_function(thunk_def);
        thunk_map.insert(*target, thunk_id);
    }
}
