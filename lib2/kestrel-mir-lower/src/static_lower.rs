//! Static / global-variable lowering.
//!
//! Handles two shapes of "stored field":
//!   * `static var/let` fields inside structs and enums
//!   * Module-level `var/let` declarations (top-level globals)
//!
//! Both become a `StaticDef` in `MirModule.statics`. Field entities with a
//! `Callable` component (computed properties / getters) are *not* handled here
//! — those are lowered as functions by `function_lower`.
//!
//! After all items are lowered, `synthesize_static_inits` creates a synthetic
//! `__kestrel_init_statics` function that evaluates each static's initializer
//! expression and assigns it to the corresponding global. A call to that
//! function is prepended to `main`'s entry block so statics are initialized
//! before user code runs.

use kestrel_ast_builder::{Body, Settable};
use kestrel_hecs::Entity;
use kestrel_mir::{
    BasicBlock, Callee, FunctionDef, FunctionId, FunctionKind, Immediate, LocalDef, MirBody,
    MirTy, Place, Rvalue, Statement, StatementKind, StaticDef, StaticId, Terminator, Value,
};

use crate::body_lower::lower_function_body;
use crate::context::LowerCtx;
use crate::ty::resolve_type_annotation;

/// Synthetic name of the module-init function.
const INIT_STATICS_NAME: &str = "__kestrel_init_statics";

/// Lower a stored field (static or module-level global) into a MIR `StaticDef`.
///
/// Callers must have verified that the entity is a `NodeKind::Field` with no
/// `Callable` component (i.e. it's stored, not computed).
pub fn lower_static(ctx: &mut LowerCtx, entity: Entity) -> StaticId {
    let name = ctx.register_name(entity);
    let ty = resolve_type_annotation(ctx, entity);
    let is_mutable = ctx.world.get::<Settable>(entity).is_some();

    let mut def = StaticDef::new(entity, name, ty);
    if is_mutable {
        def = def.mutable();
    }
    ctx.module.add_static(def)
}

/// After all items are lowered, synthesize the `__kestrel_init_statics`
/// function that evaluates every static's initializer and assigns it to the
/// corresponding global, then inject a call to it at the start of `main()`.
pub fn synthesize_static_inits(ctx: &mut LowerCtx) {
    // Snapshot the statics that have an initializer body attached. Skip file
    // constants — they're materialized directly by codegen from embedded bytes.
    let with_init: Vec<(Entity, MirTy)> = ctx
        .module
        .statics
        .iter()
        .filter(|s| s.file_constant_data.is_none())
        .filter(|s| ctx.world.get::<Body>(s.entity).is_some())
        .map(|s| (s.entity, s.ty.clone()))
        .collect();

    if with_init.is_empty() {
        return;
    }

    // One init-expression thunk per static.
    let thunks: Vec<(Entity, FunctionId, MirTy)> = with_init
        .into_iter()
        .map(|(entity, ty)| {
            let func_id = synthesize_init_thunk(ctx, entity, ty.clone());
            (entity, func_id, ty)
        })
        .collect();

    // Single `__kestrel_init_statics` that calls each thunk and assigns.
    let master = synthesize_master_init(ctx, &thunks);
    ctx.module.module_init = Some(master);

    inject_init_call_into_main(ctx, master);
}

/// Synthesize `func __init$<qualified-name>() -> T { <initializer expr> }`.
///
/// The body is built by reusing `lower_function_body` on the static entity —
/// the static's `Body` component is the initializer, and it has no `Callable`
/// so the HIR body has no params and a tail expression that becomes a return.
fn synthesize_init_thunk(ctx: &mut LowerCtx, static_entity: Entity, static_ty: MirTy) -> FunctionId {
    let static_name = ctx.module.resolve_name(static_entity).to_string();
    let thunk_entity = ctx.next_synthetic_entity();
    let thunk_name = format!("__init${static_name}");
    ctx.module.register_name(thunk_entity, thunk_name.clone());

    let mut def = FunctionDef::new(thunk_entity, thunk_name, static_ty);
    def.kind = FunctionKind::Free;
    let func_id = ctx.module.add_function(def);

    // Pass the static entity for HIR/infer lookup, func_id for body attachment.
    lower_function_body(ctx, static_entity, func_id);

    func_id
}

/// Synthesize the master `__kestrel_init_statics()` function.
fn synthesize_master_init(
    ctx: &mut LowerCtx,
    thunks: &[(Entity, FunctionId, MirTy)],
) -> FunctionId {
    let entity = ctx.next_synthetic_entity();
    ctx.module.register_name(entity, INIT_STATICS_NAME.to_string());

    let mut def = FunctionDef::new(entity, INIT_STATICS_NAME, MirTy::Unit);
    def.kind = FunctionKind::Free;
    let func_id = ctx.module.add_function(def);

    let mut body = MirBody::new();
    let entry = body.add_block(BasicBlock::new());
    body.entry = entry;

    for (static_entity, thunk_id, ty) in thunks {
        let thunk_entity = ctx.module.functions[thunk_id.index()].entity;

        // tmp = call __init$...()
        let tmp = body.add_local(LocalDef::new(format!("_t{}", body.locals.len()), ty.clone()));
        body.block_mut(entry).stmts.push(Statement::new(StatementKind::Call {
            dest: Some(Place::Local(tmp)),
            callee: Callee::direct(thunk_entity),
            args: Vec::new(),
        }));

        // @global = copy tmp
        body.block_mut(entry).stmts.push(Statement::new(StatementKind::Assign {
            dest: Place::Global(*static_entity),
            rvalue: Rvalue::Copy(Place::Local(tmp)),
        }));
    }

    body.block_mut(entry).terminator = Terminator::ret(Value::Immediate(Immediate::unit()));
    ctx.module.functions[func_id.index()].body = Some(body);

    func_id
}

/// Prepend a call to the init function at the start of `main`'s entry block.
fn inject_init_call_into_main(ctx: &mut LowerCtx, init_func_id: FunctionId) {
    let main_id = ctx
        .module
        .functions
        .iter()
        .enumerate()
        .find(|(_, f)| f.name.split('.').next_back() == Some("main"))
        .map(|(i, _)| i);
    let Some(main_idx) = main_id else {
        return;
    };

    let init_entity = ctx.module.functions[init_func_id.index()].entity;

    let main_func = &mut ctx.module.functions[main_idx];
    let Some(body) = main_func.body.as_mut() else {
        return;
    };
    let entry = body.entry;

    let call = Statement::new(StatementKind::Call {
        dest: None,
        callee: Callee::direct(init_entity),
        args: Vec::new(),
    });
    body.block_mut(entry).stmts.insert(0, call);
}
