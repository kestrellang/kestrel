//! Static / global-variable lowering + init thunk synthesis.
//!
//! Stored fields (static var/let in types, module-level globals) become
//! StaticDefs. After all items are lowered, `synthesize_static_inits`
//! creates per-static init thunks and a master `__kestrel_init_statics`
//! function, then injects a call to it at the start of `main()`.

use std::path::PathBuf;

use kestrel_ast::ast_body::{AstExpr, AstLiteral};
use kestrel_ast_builder::{Attributes, Body, FileId, FilePath, Settable};
use kestrel_hecs::Entity;
use kestrel_mir::body::OssaBody;
use kestrel_mir::item::function::{FunctionDef, FunctionKind};
use kestrel_mir::item::static_def::{FileConstantData, StaticDef};
use kestrel_mir::op::IntBits;
use kestrel_mir::{FieldIdx, Immediate, MirTy, TyId, WitnessMethodKey};

use crate::context::LowerCtx;
use crate::ty::resolve_type_annotation;

const INIT_STATICS_NAME: &str = "__kestrel_init_statics";
/// Internal name of the synthesized C entry point. Codegen exports the
/// `is_main` function as the symbol `main` regardless of this name.
const MAIN_WRAPPER_NAME: &str = "__kestrel_main";

pub fn lower_static(ctx: &mut LowerCtx, entity: Entity) {
    let name = ctx.register_name(entity);
    let ty = resolve_type_annotation(ctx, entity);
    let is_mutable = ctx.world.get::<Settable>(entity).is_some();

    let mut def = StaticDef::new(entity, name, ty);
    def.is_mutable = is_mutable;

    if let Some(fc) = extract_file_constant(ctx, entity, ty) {
        def.file_constant_data = Some(fc);
    } else if let Some(imm) = extract_literal_initializer(ctx, entity, ty) {
        def.initializer = Some(imm);
    }

    ctx.module.add_static(def);
}

/// Synthesize init thunks for statics with initializer bodies, a master
/// `__kestrel_init_statics` function, and the C `main` wrapper that runs static
/// init then the user `@main`.
pub fn synthesize_static_inits(ctx: &mut LowerCtx) {
    let with_init: Vec<(Entity, TyId)> = ctx
        .module
        .statics
        .values()
        .filter(|s| s.file_constant_data.is_none())
        .filter(|s| ctx.world.get::<Body>(s.entity).is_some())
        .map(|s| (s.entity, s.ty))
        .collect();

    // Per-static thunks + master init (only when there are statics to initialize).
    let master = if with_init.is_empty() {
        None
    } else {
        let thunks: Vec<(Entity, Entity, TyId)> = with_init
            .into_iter()
            .map(|(entity, ty)| {
                let thunk_entity = synthesize_init_thunk(ctx, entity, ty);
                (entity, thunk_entity, ty)
            })
            .collect();
        Some(synthesize_master_init(ctx, &thunks))
    };

    // The entry point itself — synthesized whenever a `@main` exists, even with
    // no statics to initialize.
    synthesize_main_wrapper(ctx, master);
}

/// Synthesize the real C `main`: a wrapper `__kestrel_main() -> i64` that runs
/// static init (if any), calls the user `@main` (now an ordinary function), and
/// turns its result into the process exit code. The user `@main` is demoted to
/// `is_main = false`; the wrapper carries `is_main = true` so codegen exports it
/// as the `main` symbol. No `@main` (a library) → nothing to synthesize.
///
/// The wrapper branches on the user return type:
/// - `()` / `!`              → run, return `0` (`!` diverges → unreachable)
/// - raw `lang.iN`           → run, sign-extend the result to `i64` (back-compat)
/// - any `Exitable` conformer → `run().report().rawValue`, sign-extended to `i64`
fn synthesize_main_wrapper(ctx: &mut LowerCtx, master_init: Option<Entity>) {
    use kestrel_hir::builtin::Builtin;
    use kestrel_mir::ParamConvention;
    use kestrel_mir::callee::Callee;
    use kestrel_mir::inst::{CallArg, InstKind, Instruction};
    use kestrel_mir::terminator::{Terminator, TerminatorKind};
    use kestrel_mir::value::ValueDef;
    use kestrel_name_res::ResolveBuiltin;

    // Find the user `@main` and demote it; no entry point ⇒ a library.
    let Some(run_entity) = ctx
        .module
        .functions
        .iter()
        .find(|(_, f)| f.body.is_some() && f.is_main)
        .map(|(entity, _)| *entity)
    else {
        return;
    };
    let run_ret = ctx.module.functions.get(&run_entity).unwrap().ret;
    ctx.module.functions.get_mut(&run_entity).unwrap().is_main = false;

    // Wrapper: `__kestrel_main() -> i64`, marked as the entry point.
    let i64_ty = ctx.module.ty_arena.i64();
    let wrapper = ctx.next_synthetic_entity();
    ctx.module.register_name(wrapper, MAIN_WRAPPER_NAME);
    let mut def = FunctionDef::new(wrapper, MAIN_WRAPPER_NAME, i64_ty);
    def.kind = FunctionKind::Free;
    def.is_main = true;
    ctx.module.add_function(def);

    let mut body = OssaBody::new();
    let entry = body.alloc_block();
    body.entry = entry;

    // 1. Static initialization (moved out of the user `@main`).
    if let Some(master) = master_init {
        body.block_mut(entry)
            .insts
            .push(Instruction::new(InstKind::Call {
                result: None,
                callee: Callee::direct(master),
                args: vec![],
            }));
    }

    let run_mir = ctx.module.ty_arena.get(run_ret).clone();

    if matches!(&run_mir, MirTy::Tuple(elems) if elems.is_empty()) {
        // `()` — run for effect, exit 0.
        push_call(&mut body, entry, None, Callee::direct(run_entity), vec![]);
        let zero = body.alloc_value(ValueDef::owned(i64_ty));
        body.block_mut(entry)
            .insts
            .push(Instruction::new(InstKind::Literal {
                result: zero,
                value: Immediate::i64(0),
            }));
        body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Return(zero));
    } else if matches!(&run_mir, MirTy::Never) {
        // `!` — the call diverges; the wrapper never returns.
        push_call(&mut body, entry, None, Callee::direct(run_entity), vec![]);
        body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Unreachable);
    } else if let Some(src_bits) = int_bits(&run_mir) {
        // Raw `lang.iN` — sign-extend the result into the C `int` (back-compat).
        let run_val = body.alloc_value(ValueDef::owned(run_ret));
        push_call(
            &mut body,
            entry,
            Some(run_val),
            Callee::direct(run_entity),
            vec![],
        );
        let ret_val = widen_to_i64(&mut body, entry, run_val, src_bits, i64_ty);
        body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Return(ret_val));
    } else {
        // Any `Exitable` conformer — `run().report().rawValue`.
        let exitable = ctx.query.query(ResolveBuiltin {
            builtin: Builtin::Exitable,
            root: ctx.root,
        });
        let exitcode_ent = ctx.query.query(ResolveBuiltin {
            builtin: Builtin::ExitCode,
            root: ctx.root,
        });

        let run_val = body.alloc_value(ValueDef::owned(run_ret));
        push_call(
            &mut body,
            entry,
            Some(run_val),
            Callee::direct(run_entity),
            vec![],
        );

        let ret_val = match (exitable, exitcode_ent) {
            (Some(exitable), Some(exitcode_ent)) => {
                let exitcode_ty = ctx.module.ty_arena.named(exitcode_ent, vec![]);
                let i8_ty = ctx.module.ty_arena.i8();
                let field = ctx
                    .resolve_field_idx(exitcode_ent, "rawValue")
                    .unwrap_or(FieldIdx::new(0));

                // run_val.report() -> ExitCode  (consumes run_val)
                let code = body.alloc_value(ValueDef::owned(exitcode_ty));
                push_call(
                    &mut body,
                    entry,
                    Some(code),
                    Callee::Witness {
                        protocol: exitable,
                        method: WitnessMethodKey::simple("report"),
                        self_type: run_ret,
                        method_type_args: vec![],
                    },
                    vec![CallArg {
                        value: run_val,
                        convention: ParamConvention::Consuming,
                    }],
                );

                // code.rawValue : lang.i8
                let raw = body.alloc_value(ValueDef::owned(i8_ty));
                body.block_mut(entry)
                    .insts
                    .push(Instruction::new(InstKind::StructExtract {
                        result: raw,
                        operand: code,
                        field,
                    }));
                widen_to_i64(&mut body, entry, raw, IntBits::I8, i64_ty)
            },
            // Exitable/ExitCode unavailable (e.g. `--no-std`) — best effort: exit 0.
            _ => {
                let zero = body.alloc_value(ValueDef::owned(i64_ty));
                body.block_mut(entry)
                    .insts
                    .push(Instruction::new(InstKind::Literal {
                        result: zero,
                        value: Immediate::i64(0),
                    }));
                zero
            },
        };
        body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Return(ret_val));
    }

    ctx.module.functions.get_mut(&wrapper).unwrap().body = Some(body);
}

/// Push a `Call` instruction onto `entry`.
fn push_call(
    body: &mut OssaBody,
    entry: kestrel_mir::BlockId,
    result: Option<kestrel_mir::ValueId>,
    callee: kestrel_mir::callee::Callee,
    args: Vec<kestrel_mir::inst::CallArg>,
) {
    use kestrel_mir::inst::{InstKind, Instruction};
    body.block_mut(entry)
        .insts
        .push(Instruction::new(InstKind::Call {
            result,
            callee,
            args,
        }));
}

/// Sign-extend `value` (of width `src_bits`) to `i64`, returning the widened
/// value (or `value` itself when already 64-bit).
fn widen_to_i64(
    body: &mut OssaBody,
    entry: kestrel_mir::BlockId,
    value: kestrel_mir::ValueId,
    src_bits: IntBits,
    i64_ty: TyId,
) -> kestrel_mir::ValueId {
    use kestrel_mir::inst::{InstKind, Instruction};
    use kestrel_mir::op::Op;
    use kestrel_mir::value::ValueDef;
    if src_bits == IntBits::I64 {
        return value;
    }
    let widened = body.alloc_value(ValueDef::owned(i64_ty));
    body.block_mut(entry)
        .insts
        .push(Instruction::new(InstKind::Op1 {
            result: widened,
            op: Op::IntWiden(src_bits, IntBits::I64),
            arg: value,
        }));
    // `Op1` is a non-consuming read, so the @owned source scalar must still be
    // consumed. DestroyValue on a trivial scalar is a no-op the expand pass drops.
    body.block_mut(entry)
        .insts
        .push(Instruction::new(InstKind::DestroyValue { operand: value }));
    widened
}

/// The `IntBits` for a raw `lang.iN` primitive type, if `ty` is one.
fn int_bits(ty: &MirTy) -> Option<IntBits> {
    match ty {
        MirTy::I8 => Some(IntBits::I8),
        MirTy::I16 => Some(IntBits::I16),
        MirTy::I32 => Some(IntBits::I32),
        MirTy::I64 => Some(IntBits::I64),
        _ => None,
    }
}

/// Create `func __init$<name>() -> T { <initializer expr> }`.
fn synthesize_init_thunk(ctx: &mut LowerCtx, static_entity: Entity, static_ty: TyId) -> Entity {
    let static_name = ctx.module.resolve_name(static_entity).to_string();
    let thunk_entity = ctx.next_synthetic_entity();
    let thunk_name = format!("__init${static_name}");
    ctx.module.register_name(thunk_entity, thunk_name.clone());

    let mut def = FunctionDef::new(thunk_entity, &thunk_name, static_ty);
    def.kind = FunctionKind::Free;
    ctx.module.add_function(def);

    // Lower body using the static's entity (source of the init expression)
    // but the thunk's entity is the function key in the module
    crate::body::lower_function_body(ctx, static_entity, thunk_entity);

    thunk_entity
}

/// Create the master `__kestrel_init_statics()` that calls each thunk
/// and stores the result into the corresponding global.
fn synthesize_master_init(ctx: &mut LowerCtx, thunks: &[(Entity, Entity, TyId)]) -> Entity {
    use kestrel_mir::Immediate;
    use kestrel_mir::callee::Callee;
    use kestrel_mir::inst::{InstKind, Instruction};
    use kestrel_mir::terminator::{Terminator, TerminatorKind};
    use kestrel_mir::value::ValueDef;

    let entity = ctx.next_synthetic_entity();
    ctx.module.register_name(entity, INIT_STATICS_NAME);

    let unit_ty = ctx.module.ty_arena.unit();
    let mut def = FunctionDef::new(entity, INIT_STATICS_NAME, unit_ty);
    def.kind = FunctionKind::Free;
    ctx.module.add_function(def);

    let mut body = OssaBody::new();
    let entry = body.alloc_block();
    body.entry = entry;

    for &(static_entity, thunk_entity, static_ty) in thunks {
        // tmp = call __init$...()
        let tmp = body.alloc_value(ValueDef::owned(static_ty));
        body.block_mut(entry)
            .insts
            .push(Instruction::new(InstKind::Call {
                result: Some(tmp),
                callee: Callee::direct(thunk_entity),
                args: vec![],
            }));

        // addr = global_ref static_entity
        let ptr_ty = ctx.module.ty_arena.pointer(static_ty);
        let addr = body.alloc_value(ValueDef::owned(ptr_ty));
        body.block_mut(entry)
            .insts
            .push(Instruction::new(InstKind::GlobalRef {
                result: addr,
                entity: static_entity,
            }));

        // store_init addr, tmp
        body.block_mut(entry)
            .insts
            .push(Instruction::new(InstKind::StoreInit {
                address: addr,
                value: tmp,
            }));
        // Consume the address pointer (trivial — expand pass removes this)
        body.block_mut(entry)
            .insts
            .push(Instruction::new(InstKind::DestroyValue { operand: addr }));
    }

    // return ()
    let unit_val = body.alloc_value(ValueDef::owned(unit_ty));
    body.block_mut(entry)
        .insts
        .push(Instruction::new(InstKind::Literal {
            result: unit_val,
            value: Immediate::unit(),
        }));
    body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Return(unit_val));

    ctx.module.functions.get_mut(&entity).unwrap().body = Some(body);
    entity
}

/// If the initializer body is a single literal expression (no statements),
/// extract it as an Immediate so it can be baked into the static data section
/// without needing an init thunk.
fn extract_literal_initializer(ctx: &LowerCtx, entity: Entity, ty: TyId) -> Option<Immediate> {
    let body = &ctx.world.get::<Body>(entity)?.0;
    if !body.statements.is_empty() {
        return None;
    }
    let tail = body.tail_expr?;
    match &body.exprs[tail] {
        AstExpr::Literal { kind, .. } => match kind {
            AstLiteral::Integer(s) => {
                let v: i128 = s.parse().ok()?;
                Some(match ctx.module.ty_arena.get(ty) {
                    MirTy::I8 => Immediate::i8(v),
                    MirTy::I16 => Immediate::i16(v),
                    MirTy::I32 => Immediate::i32(v),
                    _ => Immediate::i64(v),
                })
            },
            AstLiteral::Float(s) => {
                let v: f64 = s.parse().ok()?;
                Some(match ctx.module.ty_arena.get(ty) {
                    MirTy::F32 => Immediate::f32(v),
                    _ => Immediate::f64(v),
                })
            },
            AstLiteral::Bool(v) => Some(Immediate::bool(*v)),
            _ => None,
        },
        _ => None,
    }
}

fn extract_file_constant(ctx: &LowerCtx, entity: Entity, ty: TyId) -> Option<FileConstantData> {
    let attrs = ctx.world.get::<Attributes>(entity)?;
    let attr = attrs.0.iter().find(|a| a.name == "fileconstant")?;
    let raw = &attr.args.first()?.value;
    let relative_path = raw.strip_prefix('"').and_then(|s| s.strip_suffix('"'))?;

    let element_ty = match ctx.module.ty_arena.get(ty) {
        MirTy::Named { type_args, .. } if type_args.len() == 1 => type_args[0],
        _ => return None,
    };

    let file_entity = ctx.world.get::<FileId>(entity)?.0;
    let file_path = ctx.world.get::<FilePath>(file_entity)?;
    let base_path = PathBuf::from(&file_path.0).parent().map(PathBuf::from);

    Some(FileConstantData {
        relative_path: relative_path.to_string(),
        element_ty,
        base_path,
    })
}
