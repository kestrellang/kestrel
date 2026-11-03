//! Post-lowering ICE backstops, each emitting one ICE diagnostic per
//! offending location via `QueryContext::accumulate`:
//! - any `MirTy::Error` that escapes into the MIR (an internal inconsistency);
//! - any write that reaches an observed property's storage without running
//!   its observers ([`validate_observed_writes`]).

use kestrel_ast_builder::{DeclSpan, Name, NodeKind, ObserverSetter};
use kestrel_debug::ktrace;
use kestrel_hecs::Entity;
use kestrel_mir::inst::InstKind;
use kestrel_mir::{FieldIdx, MirModule, MirTy, OssaBody, TyId, ValueId};
use kestrel_name_res::ExtensionTargetEntity;
use kestrel_reporting::{Diagnostic, Label};
use kestrel_span::Span;
use kestrel_type_infer::captures::is_observed_field;
use std::collections::HashMap;

use crate::context::LowerCtx;

pub fn validate_no_error_types(ctx: &LowerCtx, module: &MirModule) -> usize {
    let mut n = 0usize;

    for s in module.structs.values() {
        for field in &s.fields {
            if is_error(module, field.ty) {
                emit(
                    ctx,
                    &mut n,
                    s.entity,
                    format!("field '{}.{}'", s.name, field.name),
                );
            }
        }
    }

    for e in module.enums.values() {
        for case in &e.cases {
            for field in &case.payload_fields {
                if is_error(module, field.ty) {
                    emit(
                        ctx,
                        &mut n,
                        e.entity,
                        format!("payload field '{}.{}.{}'", e.name, case.name, field.name),
                    );
                }
            }
        }
    }

    for st in module.statics.values() {
        if is_error(module, st.ty) {
            emit(ctx, &mut n, st.entity, format!("static '{}'", st.name));
        }
    }

    for f in module.functions.values() {
        if is_error(module, f.ret) {
            emit(
                ctx,
                &mut n,
                f.entity,
                format!("return type of '{}'", f.name),
            );
        }
        for p in &f.params {
            if is_error(module, p.ty) {
                emit(
                    ctx,
                    &mut n,
                    f.entity,
                    format!("parameter '{}' of '{}'", p.name, f.name),
                );
            }
        }
        // TODO: walk OssaBody instructions for Error types
    }

    n
}

fn is_error(module: &MirModule, ty: TyId) -> bool {
    matches!(module.ty_arena.get(ty), MirTy::Error)
}

fn emit(ctx: &LowerCtx, n: &mut usize, entity: Entity, location: String) {
    let span = ctx
        .world
        .get::<DeclSpan>(entity)
        .map(|s| s.0.clone())
        .unwrap_or_else(|| Span::synthetic(0));
    let diag = Diagnostic::error()
        .with_message(format!(
            "internal compiler error: unresolved MIR type at {location}"
        ))
        .with_labels(vec![
            Label::primary(span.file_id, span.range())
                .with_message("an earlier phase should have diagnosed this"),
        ])
        .with_notes(vec![
            "this indicates a missing `UnresolvedTypeParam` diagnostic in type inference; \
             please file a bug with the program that triggered it"
                .into(),
        ]);
    ctx.query.accumulate(diag);
    *n += 1;
}

/// Post-lowering backstop for property observers: no instruction may write
/// an observed stored field's storage directly (store, `&mutating` borrow or
/// move-out), or through a field address under it, except in the bodies that
/// own that storage: the field's own observer setter and the initializers of
/// its type. Every other write must go through the observer setter (see
/// `body::place::Access`). One ICE per offending instruction, so a write
/// path that skips observers fails loudly instead of silently.
///
/// Initializers are exempt as a whole: an init's writes to ANOTHER instance
/// of its own type aren't checked here (lowering still routes them through
/// the setter).
pub fn validate_observed_writes(ctx: &LowerCtx, module: &MirModule) -> usize {
    let mut n = 0usize;
    for f in module.functions.values() {
        let Some(body) = &f.body else { continue };
        let exempt = ObserverExemption::of(ctx, f.entity);
        let observed = observed_field_addrs(ctx, module, body, &exempt);
        if observed.is_empty() {
            continue;
        }
        for block in &body.blocks {
            for inst in &block.insts {
                let address = match &inst.kind {
                    InstKind::StoreInit { address, .. }
                    | InstKind::StoreAssign { address, .. }
                    | InstKind::BeginMutBorrowAddr { address, .. }
                    | InstKind::Take { address, .. } => *address,
                    _ => continue,
                };
                let Some(&field) = observed.get(&address) else {
                    continue;
                };
                let span = inst
                    .span
                    .clone()
                    .or_else(|| body.value(address).span.clone());
                emit_observer_skip(ctx, &mut n, f.entity, field, span);
            }
        }
    }
    n
}

/// Which observed storage a body may write directly.
enum ObserverExemption {
    None,
    /// The observer setter of this field.
    Field(Entity),
    /// An initializer of this type: all of its observed fields.
    Owner(Entity),
}

impl ObserverExemption {
    fn of(ctx: &LowerCtx, func: Entity) -> Self {
        let world = ctx.world;
        if world.get::<ObserverSetter>(func).is_some() {
            return world.parent_of(func).map_or(Self::None, Self::Field);
        }
        if world.get::<NodeKind>(func) != Some(&NodeKind::Initializer) {
            return Self::None;
        }
        let Some(owner) = world.parent_of(func) else {
            return Self::None;
        };
        if world.get::<NodeKind>(owner) != Some(&NodeKind::Extension) {
            return Self::Owner(owner);
        }
        ctx.query
            .query(ExtensionTargetEntity {
                extension: owner,
                root: ctx.root,
            })
            .map_or(Self::None, Self::Owner)
    }

    fn allows(&self, ctx: &LowerCtx, field: Entity) -> bool {
        match *self {
            Self::None => false,
            Self::Field(f) => f == field,
            Self::Owner(owner) => ctx.world.parent_of(field) == Some(owner),
        }
    }
}

/// Every `FieldAddr` in `body` that addresses an observed stored field this
/// body may not write directly, or a field under one, mapped to that
/// observed field. Iterated to a fixpoint because blocks needn't be listed
/// in dominance order.
fn observed_field_addrs(
    ctx: &LowerCtx,
    module: &MirModule,
    body: &OssaBody,
    exempt: &ObserverExemption,
) -> HashMap<ValueId, Entity> {
    let mut observed: HashMap<ValueId, Entity> = HashMap::new();
    loop {
        let before = observed.len();
        for inst in body.blocks.iter().flat_map(|b| &b.insts) {
            let InstKind::FieldAddr {
                result,
                base,
                ty,
                field,
            } = &inst.kind
            else {
                continue;
            };
            if observed.contains_key(result) {
                continue;
            }
            if let Some(&outer) = observed.get(base) {
                observed.insert(*result, outer);
                continue;
            }
            if let Some(f) = observed_field(ctx, module, *ty, *field)
                && !exempt.allows(ctx, f)
            {
                observed.insert(*result, f);
            }
        }
        if observed.len() == before {
            return observed;
        }
    }
}

/// The field entity at `field` of struct type `ty`, when it is observed.
fn observed_field(ctx: &LowerCtx, module: &MirModule, ty: TyId, field: FieldIdx) -> Option<Entity> {
    let MirTy::Named { entity, .. } = module.ty_arena.get(ty) else {
        return None;
    };
    let def = module.structs.get(entity)?;
    let name = &def.fields.get(field.index())?.name;
    let f = ctx.field_named(*entity, name)?;
    is_observed_field(&ctx.query, f).then_some(f)
}

fn emit_observer_skip(
    ctx: &LowerCtx,
    n: &mut usize,
    func: Entity,
    field: Entity,
    span: Option<Span>,
) {
    let span = span
        .or_else(|| ctx.world.get::<DeclSpan>(func).map(|s| s.0.clone()))
        .unwrap_or_else(|| Span::synthetic(0));
    let name = ctx.world.get::<Name>(field).map_or("?", |n| n.0.as_str());
    ktrace!(
        "observer-audit",
        "raw write to observed '{name}' in {func:?}"
    );
    let diag = Diagnostic::error()
        .with_message(format!(
            "internal compiler error: write to observed property '{name}' skips its observers"
        ))
        .with_labels(vec![
            Label::primary(span.file_id, span.range())
                .with_message("this write reaches the property's storage directly"),
        ])
        .with_notes(vec![
            "a write path resolved its place with `Access::Read` or a raw field address; \
             writes must use `Access::Write` / `write_place_addr` so the observer setter runs"
                .into(),
        ]);
    ctx.query.accumulate(diag);
    *n += 1;
}
