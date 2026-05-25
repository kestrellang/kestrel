//! Static / global-variable lowering + init thunk synthesis.
//!
//! Stored fields (static var/let in types, module-level globals) become
//! StaticDefs. After all items are lowered, `synthesize_static_inits`
//! creates per-static init thunks and a master `__kestrel_init_statics`
//! function, then injects a call to it at the start of `main()`.

use std::path::PathBuf;

use kestrel_ast_builder::{Attributes, Body, FileId, FilePath, Settable};
use kestrel_hecs::Entity;
use kestrel_mir_3::body::OssaBody;
use kestrel_mir_3::item::function::{FunctionDef, FunctionKind};
use kestrel_mir_3::item::static_def::{FileConstantData, StaticDef};
use kestrel_mir_3::{MirTy, TyId};

use crate::body;
use crate::context::LowerCtx;
use crate::ty::resolve_type_annotation;

const INIT_STATICS_NAME: &str = "__kestrel_init_statics";

pub fn lower_static(ctx: &mut LowerCtx, entity: Entity) {
    let name = ctx.register_name(entity);
    let ty = resolve_type_annotation(ctx, entity);
    let is_mutable = ctx.world.get::<Settable>(entity).is_some();

    let mut def = StaticDef::new(entity, name, ty);
    def.is_mutable = is_mutable;

    if let Some(fc) = extract_file_constant(ctx, entity, ty) {
        def.file_constant_data = Some(fc);
    }

    ctx.module.add_static(def);
}

/// Synthesize init thunks for statics with initializer bodies, a master
/// `__kestrel_init_statics` function, and inject its call into `main`.
pub fn synthesize_static_inits(ctx: &mut LowerCtx) {
    let with_init: Vec<(Entity, TyId)> = ctx
        .module
        .statics
        .iter()
        .filter(|s| s.file_constant_data.is_none())
        .filter(|s| ctx.world.get::<Body>(s.entity).is_some())
        .map(|s| (s.entity, s.ty))
        .collect();

    if with_init.is_empty() {
        return;
    }

    // One thunk per static
    let thunks: Vec<(Entity, usize, TyId)> = with_init
        .into_iter()
        .map(|(entity, ty)| {
            let func_idx = synthesize_init_thunk(ctx, entity, ty);
            (entity, func_idx, ty)
        })
        .collect();

    // Master init function
    let _master_idx = synthesize_master_init(ctx, &thunks);
    // Static init disabled: the init thunks call misresolved functions
    // (e.g. Int32.maxValue instead of Int32.init). Re-enable once the
    // monomorphizer function resolution is fixed.
    // inject_init_call_into_main(ctx, _master_idx);
}

/// Create `func __init$<name>() -> T { <initializer expr> }`.
fn synthesize_init_thunk(ctx: &mut LowerCtx, static_entity: Entity, static_ty: TyId) -> usize {
    let static_name = ctx.module.resolve_name(static_entity).to_string();
    let thunk_entity = ctx.next_synthetic_entity();
    let thunk_name = format!("__init${static_name}");
    ctx.module.register_name(thunk_entity, thunk_name.clone());

    let mut def = FunctionDef::new(thunk_entity, &thunk_name, static_ty);
    def.kind = FunctionKind::Free;
    let func_id = ctx.module.add_function(def);
    let func_idx = func_id.index();

    crate::body::lower_function_body(ctx, static_entity, func_idx);

    func_idx
}

/// Create the master `__kestrel_init_statics()` that calls each thunk
/// and stores the result into the corresponding global.
fn synthesize_master_init(ctx: &mut LowerCtx, thunks: &[(Entity, usize, TyId)]) -> usize {
    use kestrel_mir_3::callee::Callee;
    use kestrel_mir_3::inst::{InstKind, Instruction};
    use kestrel_mir_3::terminator::{Terminator, TerminatorKind};
    use kestrel_mir_3::value::{Ownership, ValueDef};
    use kestrel_mir_3::Immediate;

    let entity = ctx.next_synthetic_entity();
    ctx.module.register_name(entity, INIT_STATICS_NAME);

    let unit_ty = ctx.module.ty_arena.unit();
    let mut def = FunctionDef::new(entity, INIT_STATICS_NAME, unit_ty);
    def.kind = FunctionKind::Free;
    let func_id = ctx.module.add_function(def);
    let func_idx = func_id.index();

    let mut body = OssaBody::new();
    let entry = body.alloc_block();
    body.entry = entry;

    for &(static_entity, thunk_func_idx, static_ty) in thunks {
        let thunk_entity = ctx.module.functions[thunk_func_idx].entity;

        // tmp = call __init$...()
        let ownership = kestrel_mir_3::body::ownership_for_type(
            static_ty, &ctx.module.ty_arena, &ctx.module,
        );
        let tmp = body.alloc_value(match ownership {
            Ownership::Owned => ValueDef::owned(static_ty),
            _ => ValueDef::none(static_ty),
        });
        body.block_mut(entry).insts.push(Instruction::new(InstKind::Call {
            result: Some(tmp),
            callee: Callee::direct(thunk_entity),
            args: vec![],
        }));

        // addr = global_ref static_entity
        let ptr_ty = ctx.module.ty_arena.pointer(static_ty);
        let addr = body.alloc_value(ValueDef::none(ptr_ty));
        body.block_mut(entry).insts.push(Instruction::new(
            InstKind::GlobalRef { result: addr, entity: static_entity },
        ));

        // store_init addr, tmp
        body.block_mut(entry).insts.push(Instruction::new(
            InstKind::StoreInit { address: addr, value: tmp },
        ));
    }

    // return ()
    let unit_val = body.alloc_value(ValueDef::none(unit_ty));
    body.block_mut(entry).insts.push(Instruction::new(
        InstKind::Literal { result: unit_val, value: Immediate::unit() },
    ));
    body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Return(unit_val));

    ctx.module.functions[func_idx].body = Some(body);
    func_idx
}

/// Prepend a call to the init function at the start of `main`'s entry block.
fn inject_init_call_into_main(ctx: &mut LowerCtx, init_func_idx: usize) {
    use kestrel_mir_3::callee::Callee;
    use kestrel_mir_3::inst::{InstKind, Instruction};

    let main_idx = ctx
        .module
        .functions
        .iter()
        .enumerate()
        .find(|(_, f)| f.name.split('.').next_back() == Some("main"))
        .map(|(i, _)| i);
    let Some(main_idx) = main_idx else { return };

    let init_entity = ctx.module.functions[init_func_idx].entity;
    let main_func = &mut ctx.module.functions[main_idx];
    let Some(body) = main_func.body.as_mut() else { return };

    let call = Instruction::new(InstKind::Call {
        result: None,
        callee: Callee::direct(init_entity),
        args: vec![],
    });
    body.block_mut(body.entry).insts.insert(0, call);
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
