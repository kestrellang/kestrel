//! Static / global-variable lowering.
//!
//! Stored fields (static var/let in types, module-level globals) become
//! StaticDefs. Computed properties are lowered as functions, not here.
//!
//! Init thunk synthesis (`synthesize_static_inits`) is deferred to Phase 10.

use std::path::PathBuf;

use kestrel_ast_builder::{Attributes, FileId, FilePath, Settable};
use kestrel_hecs::Entity;
use kestrel_mir_2::item::static_def::{FileConstantData, StaticDef};
use kestrel_mir_2::{MirTy, TyId};

use crate::context::LowerCtx;
use crate::ty::resolve_type_annotation;

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

fn extract_file_constant(ctx: &LowerCtx, entity: Entity, ty: TyId) -> Option<FileConstantData> {
    let attrs = ctx.world.get::<Attributes>(entity)?;
    let attr = attrs.0.iter().find(|a| a.name == "fileconstant")?;
    let raw = &attr.args.first()?.value;
    let relative_path = raw.strip_prefix('"').and_then(|s| s.strip_suffix('"'))?;

    // Element type: LiteralSlice[T] → T
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
