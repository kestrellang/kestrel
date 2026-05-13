//! Drop expansion — translate `Drop` / `DropIf` statements into the
//! actual `Call(user_deinit)` plus recursive structural field-drop
//! sequences.
//!
//! ## Pipeline position
//!
//! `drop_elab` ran first and placed `Drop` / `DropIf` statements at the
//! right program points. Those are still abstract — codegen doesn't
//! know how to lower them on its own. This pass replaces each `Drop`
//! with a concrete sequence of MIR statements that codegen *does*
//! handle: `Call(deinit)` for types with a user-defined deinit, plus
//! recursive `Drop(field)` for structural cleanup of nested non-Copy
//! fields. Trivial drops vanish.
//!
//! After this pass, the only `Drop` / `DropIf` statements that remain
//! are over *trivial* places (which collapse to nothing at codegen)
//! and `DropIf` (which is left intact for a future CFG-surgery
//! follow-up — see "Limitations" below).
//!
//! ## Algorithm
//!
//! For each function body, walk every block's statements. For every
//! `Drop(place)`:
//!
//! 1. Resolve `place`'s [`MirTy`].
//! 2. If the type is *trivial* (no user deinit, no non-Copy fields),
//!    drop the statement.
//! 3. Else if the type has a user `deinit` and we are *not* inside
//!    that very deinit's own body (the "self-recursion break"), emit
//!    `Call(deinit, [move place])`. The user deinit takes `consuming
//!    self`, so the call consumes the place; no further structural
//!    drops are appended on this branch.
//! 4. Else recursively `Drop(place.field_n)` for each non-trivial
//!    field, in declaration order. This is the *structural-only*
//!    path — what runs inside `Foo.deinit` for its own `self`.
//!
//! The self-recursion break is essential: inside `Foo.deinit`'s body,
//! drop-elab inserts `Drop(%self)` at return. If we naively expanded
//! that to `Call(Foo.deinit, [move %self])`, the deinit would call
//! itself and stack-overflow at runtime. Inside `Foo.deinit` we treat
//! `Drop(%self)` as structural-only.
//!
//! Field projections of self (`%self.f`) still expand normally: those
//! are distinct places, and their drops (if any) run after the user
//! deinit body — except in Kestrel's current model where deinit
//! consumes self, so field drops happen *inside* deinit via this
//! same pass running on the deinit body.
//!
//! ## Limitations (deferred)
//!
//! - **Enum payload drops.** A `Drop(p: Enum)` over an enum whose
//!   variant payloads contain non-Copy types needs a `Switch` on the
//!   discriminant to drop the right payload. That's CFG surgery; the
//!   current pass leaves the `Drop` statement intact for the enum
//!   case, falling back to today's codegen-side no-op. This is
//!   correct for enums whose every variant payload is trivial (e.g.
//!   `Result[Int, Int]`) but leaks for `Optional[File]`-style shapes.
//! - **`DropIf` flag-branching.** Conditional drops are left as
//!   `DropIf` statements; codegen treats them as no-ops today. The
//!   `MaybeInit` path leaks until this lands. The MIR change requires
//!   block-splitting to emit a runtime branch on the flag local.
//!
//! Both follow-ups are tracked.

use kestrel_hecs::Entity;
use kestrel_mir::passes::place_type;
use kestrel_mir::{
    BasicBlock, Callee, EnumDef, FunctionDef, FunctionKind, MirBody, MirModule, MirTy, Place,
    Statement, StatementKind, StructDef, Value,
};

/// Maximum recursion depth for nested-field expansion. In well-formed
/// MIR direct recursion is impossible (infinite size) and cycles go
/// through `Pointer` / `Ref` (Bitwise — terminal). The guard defends
/// against pathological inputs the verifier doesn't reject.
const MAX_DEPTH: usize = 64;

/// `Statement::with_span` requires `Some(span)`; this helper accepts
/// the `Option<Span>` shape we get from the original `Drop` statement
/// and produces a `Statement::new` if there's no span to carry.
fn stmt_with_optional_span(
    kind: StatementKind,
    span: Option<kestrel_span::Span>,
) -> Statement {
    match span {
        Some(s) => Statement::with_span(kind, s),
        None => Statement::new(kind),
    }
}

pub fn run(module: &mut MirModule) {
    // Snapshot the struct/enum tables — we need read-only access to
    // them while mutating each function's body, and Rust's borrow
    // checker won't permit the simultaneous &mut on module.functions
    // and & on module.structs/enums otherwise.
    let structs = module.structs.clone();
    let enums = module.enums.clone();

    // Decide per-function whether we are inside that function's own
    // deinit body — drives the self-recursion break.
    let inside_deinit_for: Vec<Option<Entity>> = module
        .functions
        .iter()
        .map(|f| match &f.kind {
            FunctionKind::Deinit { parent } => Some(*parent),
            _ => None,
        })
        .collect();

    // Need an immutable view of the module for `place_type`; clone the
    // module to keep that view stable while we mutate the real one.
    let module_snapshot = module.clone();

    for (i, func) in module.functions.iter_mut().enumerate() {
        let Some(body) = func.body.as_mut() else {
            continue;
        };
        // Use the snapshot's function for all immutable lookups; the
        // snapshot's body matches the live body's locals exactly (only
        // block.stmts change in this pass).
        let func_snapshot = &module_snapshot.functions[i];
        let ctx = ExpandCtx {
            inside_deinit_for: inside_deinit_for[i],
            structs: &structs,
            enums: &enums,
            module: &module_snapshot,
            func: func_snapshot,
        };
        expand_body(body, &ctx, func_snapshot);
    }
}

struct ExpandCtx<'a> {
    inside_deinit_for: Option<Entity>,
    structs: &'a [StructDef],
    enums: &'a [EnumDef],
    module: &'a MirModule,
    func: &'a FunctionDef,
}

fn expand_body(body: &mut MirBody, ctx: &ExpandCtx<'_>, func_snapshot: &FunctionDef) {
    for block in &mut body.blocks {
        expand_block(block, body_snapshot_locals_ref(func_snapshot), ctx);
    }
}

/// Borrow the locals slice from the snapshot's body. Helper because
/// `func_snapshot.body.as_ref().unwrap()` everywhere is ugly.
fn body_snapshot_locals_ref(func: &FunctionDef) -> &MirBody {
    func.body.as_ref().expect("snapshot must have body")
}

fn expand_block(block: &mut BasicBlock, snapshot_body: &MirBody, ctx: &ExpandCtx<'_>) {
    let mut new_stmts: Vec<Statement> = Vec::with_capacity(block.stmts.len());
    let old = std::mem::take(&mut block.stmts);
    for stmt in old {
        match &stmt.kind {
            StatementKind::Drop { place } => {
                let place = place.clone();
                let span = stmt.span.clone();
                expand_drop(&place, snapshot_body, ctx, &span, &mut new_stmts, 0);
            },
            StatementKind::DropIf { .. } => {
                // TODO: flag-branching via CFG surgery. Until then leave
                // as a no-op — codegen treats DropIf as a no-op too, so
                // this preserves today's runtime behavior.
                new_stmts.push(stmt);
            },
            _ => new_stmts.push(stmt),
        }
    }
    block.stmts = new_stmts;
}

fn expand_drop(
    place: &Place,
    snapshot_body: &MirBody,
    ctx: &ExpandCtx<'_>,
    span: &Option<kestrel_span::Span>,
    out: &mut Vec<Statement>,
    depth: usize,
) {
    if depth > MAX_DEPTH {
        return;
    }
    let Some(ty) = place_type(ctx.module, snapshot_body, ctx.func, place) else {
        // Couldn't resolve the place's type — likely a verifier issue
        // already surfaced. Skip silently.
        return;
    };
    expand_drop_for_ty(place, &ty, snapshot_body, ctx, span, out, depth);
}

fn expand_drop_for_ty(
    place: &Place,
    ty: &MirTy,
    snapshot_body: &MirBody,
    ctx: &ExpandCtx<'_>,
    span: &Option<kestrel_span::Span>,
    out: &mut Vec<Statement>,
    depth: usize,
) {
    match ty {
        MirTy::Named { entity, type_args } => {
            // Struct lookup
            if let Some(s) = ctx.structs.iter().find(|s| s.entity == *entity) {
                expand_drop_struct(place, s, type_args, snapshot_body, ctx, span, out, depth);
                return;
            }
            // Enum lookup — punted to a future CFG-surgery follow-up.
            // Re-emit the Drop statement so codegen's existing no-op
            // path preserves current behavior. Logged as a known gap
            // in the module doc comment.
            if ctx.enums.iter().any(|e| e.entity == *entity) {
                out.push(stmt_with_optional_span(
                    StatementKind::Drop {
                        place: place.clone(),
                    },
                    span.clone(),
                ));
                return;
            }
            // Unknown nominal — emit nothing. Verifier would have
            // surfaced a real diagnostic upstream.
        },
        MirTy::Tuple(elems) => {
            // Drop each non-trivial element in declaration order.
            for (i, elem_ty) in elems.iter().enumerate() {
                if elem_ty.copy_behavior(ctx.module) == kestrel_mir::CopyBehavior::None {
                    let elem_place = Place::Index {
                        parent: Box::new(place.clone()),
                        index: i,
                    };
                    expand_drop_for_ty(
                        &elem_place,
                        elem_ty,
                        snapshot_body,
                        ctx,
                        span,
                        out,
                        depth + 1,
                    );
                }
            }
        },
        // Trivial / Bitwise / Cloneable / unresolved — no drop work.
        _ => {},
    }
}

fn expand_drop_struct(
    place: &Place,
    s: &StructDef,
    type_args: &[MirTy],
    snapshot_body: &MirBody,
    ctx: &ExpandCtx<'_>,
    span: &Option<kestrel_span::Span>,
    out: &mut Vec<Statement>,
    depth: usize,
) {
    // Self-recursion break: inside `Foo.deinit` body, dropping the
    // body's own `self` parameter must NOT call `Foo.deinit` (infinite
    // recursion). Fall through to structural-only.
    let is_self_in_own_deinit = matches!(ctx.inside_deinit_for, Some(t) if t == s.entity);

    if !is_self_in_own_deinit
        && let Some(user_method) = s.deinit_behavior.user_method
    {
        // Call user deinit on a mutable borrow of the place. The user
        // deinit signature is `(self: &var Self) -> ()`; field drops
        // happen *after* the user body returns, emitted as additional
        // structural drops below. The user can read/mutate fields but
        // not move them out (move-out would be caught by the move
        // check on the borrow).
        let callee = Callee::method(
            user_method,
            type_args.to_vec(),
            MirTy::Named {
                entity: s.entity,
                type_args: type_args.to_vec(),
            },
        );
        out.push(stmt_with_optional_span(
            StatementKind::Call {
                dest: None,
                callee,
                args: vec![Value::RefMut(place.clone())],
            },
            span.clone(),
        ));
        // Fall through to structural field drops — user deinit
        // borrowed self; fields are still live and need cleanup.
    }

    // Structural-only path: drop each non-trivial field in declaration
    // order. Two cases that land here:
    //   (a) struct has no user deinit, so we just drop the fields.
    //   (b) we're inside the struct's own deinit body and the user
    //       method was already entered — we recursively clean up the
    //       fields the user's body didn't explicitly consume.
    for field in &s.fields {
        let field_ty = substitute_type_locally_to_args(&field.ty, &s.type_params, type_args);
        if field_ty.copy_behavior(ctx.module) == kestrel_mir::CopyBehavior::None {
            let field_place = Place::Field {
                parent: Box::new(place.clone()),
                name: field.name.clone(),
            };
            expand_drop_for_ty(
                &field_place,
                &field_ty,
                snapshot_body,
                ctx,
                span,
                out,
                depth + 1,
            );
        }
    }
}

/// Substitute the struct's type-params with the given args in
/// `field_ty`. Mirrors `verify.rs::substitute_struct_field_ty`; copied
/// here to keep ownership independent of verifier internals.
fn substitute_type_locally_to_args(
    field_ty: &MirTy,
    type_params: &[kestrel_mir::TypeParamDef],
    type_args: &[MirTy],
) -> MirTy {
    let subst: std::collections::HashMap<Entity, MirTy> = type_params
        .iter()
        .zip(type_args.iter())
        .map(|(tp, arg)| (tp.entity, arg.clone()))
        .collect();
    substitute(field_ty, &subst)
}

fn substitute(ty: &MirTy, subst: &std::collections::HashMap<Entity, MirTy>) -> MirTy {
    match ty {
        MirTy::TypeParam(e) => subst.get(e).cloned().unwrap_or_else(|| ty.clone()),
        MirTy::Ref(inner) => MirTy::Ref(Box::new(substitute(inner, subst))),
        MirTy::RefMut(inner) => MirTy::RefMut(Box::new(substitute(inner, subst))),
        MirTy::Pointer(inner) => MirTy::Pointer(Box::new(substitute(inner, subst))),
        MirTy::Tuple(elems) => MirTy::Tuple(elems.iter().map(|t| substitute(t, subst)).collect()),
        MirTy::Named { entity, type_args } => MirTy::Named {
            entity: *entity,
            type_args: type_args.iter().map(|t| substitute(t, subst)).collect(),
        },
        MirTy::FuncThin { params, ret } => MirTy::FuncThin {
            params: params.iter().map(|t| substitute(t, subst)).collect(),
            ret: Box::new(substitute(ret, subst)),
        },
        MirTy::FuncThick { params, ret } => MirTy::FuncThick {
            params: params.iter().map(|t| substitute(t, subst)).collect(),
            ret: Box::new(substitute(ret, subst)),
        },
        _ => ty.clone(),
    }
}
