//! Static / global-variable lowering + init thunk synthesis.
//!
//! Stored fields (static var/let in types, module-level globals) become
//! StaticDefs. After all items are lowered, `synthesize_static_inits`
//! creates per-static init thunks and a master `__kestrel_init_statics`
//! function, then injects a call to it at the start of `main()`.

use std::path::PathBuf;

use kestrel_ast_builder::{Attributes, Body, FileId, FilePath, Settable};
use kestrel_hecs::Entity;
use kestrel_mir_2::body::{BasicBlock, LocalDef, MirBody};
use kestrel_mir_2::item::function::{FunctionDef, FunctionKind};
use kestrel_mir_2::item::static_def::{FileConstantData, StaticDef};
use kestrel_mir_2::statement::{Callee, Rvalue, Statement, StatementKind};
use kestrel_mir_2::terminator::TerminatorKind;
use kestrel_mir_2::{Immediate, MirTy, Operand, Place, Terminator, TyId, UseMode};

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
    let master_idx = synthesize_master_init(ctx, &thunks);
    inject_init_call_into_main(ctx, master_idx);
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

    body::lower_function_body(ctx, static_entity, func_idx);

    func_idx
}

/// Create the master `__kestrel_init_statics()` that calls each thunk
/// and assigns the result to the corresponding global.
fn synthesize_master_init(ctx: &mut LowerCtx, thunks: &[(Entity, usize, TyId)]) -> usize {
    let entity = ctx.next_synthetic_entity();
    ctx.module.register_name(entity, INIT_STATICS_NAME);

    let unit_ty = ctx.module.ty_arena.unit();
    let mut def = FunctionDef::new(entity, INIT_STATICS_NAME, unit_ty);
    def.kind = FunctionKind::Free;
    let func_id = ctx.module.add_function(def);
    let func_idx = func_id.index();

    let mut mir_body = MirBody::new();
    let entry = mir_body.add_block(BasicBlock::new());
    mir_body.entry = entry;

    for &(static_entity, thunk_idx, ty) in thunks {
        let thunk_entity = ctx.module.functions[thunk_idx].entity;

        // tmp = call __init$...()
        let tmp = mir_body.add_local(LocalDef::new(
            format!("_t{}", mir_body.locals.len()),
            ty,
        ));
        mir_body.block_mut(entry).stmts.push(Statement::new(
            StatementKind::Call {
                dest: Some(Place::local(tmp)),
                callee: Callee::direct(thunk_entity),
                args: Vec::new(),
            },
        ));

        // @global = copy tmp
        mir_body.block_mut(entry).stmts.push(Statement::new(
            StatementKind::Assign {
                dest: Place::global(static_entity),
                rvalue: Rvalue::Use(Operand::Place(Place::local(tmp)), UseMode::Copy),
            },
        ));
    }

    mir_body.block_mut(entry).terminator =
        Terminator::ret(Operand::Const(Immediate::unit()));
    ctx.module.functions[func_idx].body = Some(mir_body);

    func_idx
}

/// Prepend a call to the init function at the start of `main`'s entry block.
fn inject_init_call_into_main(ctx: &mut LowerCtx, init_func_idx: usize) {
    let main_idx = ctx
        .module
        .functions
        .iter()
        .enumerate()
        .find(|(_, f)| f.name.split('.').next_back() == Some("main"))
        .map(|(i, _)| i);
    let Some(main_idx) = main_idx else {
        return;
    };

    let init_entity = ctx.module.functions[init_func_idx].entity;

    let Some(main_body) = ctx.module.functions[main_idx].body.as_mut() else {
        return;
    };
    let entry = main_body.entry;

    let call = Statement::new(StatementKind::Call {
        dest: None,
        callee: Callee::direct(init_entity),
        args: Vec::new(),
    });
    main_body.block_mut(entry).stmts.insert(0, call);
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
