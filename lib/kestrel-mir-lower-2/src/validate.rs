//! Post-lowering ICE backstop: any `MirTy::Error` that escapes into the MIR
//! signals an internal compiler inconsistency. Walks the module and emits
//! one ICE diagnostic per location carrying Error via `QueryContext::accumulate`.

use kestrel_ast_builder::DeclSpan;
use kestrel_hecs::Entity;
use kestrel_mir_2::statement::{Callee, Rvalue, StatementKind};
use kestrel_mir_2::{MirModule, MirTy, TyId};
use kestrel_reporting::{Diagnostic, Label};
use kestrel_span::Span;

use crate::context::LowerCtx;

pub fn validate_no_error_types(ctx: &LowerCtx, module: &MirModule) -> usize {
    let mut n = 0usize;

    for s in &module.structs {
        for field in &s.fields {
            if is_error(&module, field.ty) {
                emit(ctx, &mut n, s.entity, format!("field '{}.{}'", s.name, field.name));
            }
        }
    }

    for e in &module.enums {
        for case in &e.cases {
            for field in &case.payload_fields {
                if is_error(&module, field.ty) {
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

    for st in &module.statics {
        if is_error(&module, st.ty) {
            emit(ctx, &mut n, st.entity, format!("static '{}'", st.name));
        }
    }

    for f in &module.functions {
        if is_error(&module, f.ret) {
            emit(ctx, &mut n, f.entity, format!("return type of '{}'", f.name));
        }
        for p in &f.params {
            if is_error(&module, p.ty) {
                emit(
                    ctx,
                    &mut n,
                    f.entity,
                    format!("parameter '{}' of '{}'", p.name, f.name),
                );
            }
        }
        if let Some(body) = &f.body {
            for (i, local) in body.locals.iter().enumerate() {
                if is_error(&module, local.ty) {
                    emit(
                        ctx,
                        &mut n,
                        f.entity,
                        format!("local '{}' (var{}) in '{}'", local.name, i, f.name),
                    );
                }
            }
            for (bi, block) in body.blocks.iter().enumerate() {
                for (si, stmt) in block.stmts.iter().enumerate() {
                    scan_stmt(ctx, module, &mut n, f.entity, &f.name, bi, si, &stmt.kind);
                }
            }
        }
    }

    n
}

fn scan_stmt(
    ctx: &LowerCtx,
    module: &MirModule,
    n: &mut usize,
    owner: Entity,
    fn_name: &str,
    bi: usize,
    si: usize,
    stmt: &StatementKind,
) {
    match stmt {
        StatementKind::Assign { rvalue, .. } => match rvalue {
            Rvalue::Construct { ty, .. } if is_error(module, *ty) => {
                emit(ctx, n, owner, format!("Construct rvalue (bb{bi}[{si}] of '{fn_name}')"));
            }
            Rvalue::EnumVariant { enum_ty, .. } if is_error(module, *enum_ty) => {
                emit(ctx, n, owner, format!("EnumVariant rvalue (bb{bi}[{si}] of '{fn_name}')"));
            }
            Rvalue::ArrayLiteral { element_ty, .. } if is_error(module, *element_ty) => {
                emit(ctx, n, owner, format!("ArrayLiteral element type (bb{bi}[{si}] of '{fn_name}')"));
            }
            _ => {}
        },
        StatementKind::Call { callee, .. } => match callee {
            Callee::Direct {
                type_args,
                self_type,
                ..
            } => {
                if type_args.iter().any(|&t| is_error(module, t)) {
                    emit(ctx, n, owner, format!("direct-call type_args (bb{bi}[{si}] of '{fn_name}')"));
                }
                if let Some(t) = self_type {
                    if is_error(module, *t) {
                        emit(ctx, n, owner, format!("direct-call self_type (bb{bi}[{si}] of '{fn_name}')"));
                    }
                }
            }
            Callee::Witness {
                self_type,
                method_type_args,
                ..
            } => {
                if is_error(module, *self_type) {
                    emit(ctx, n, owner, format!("witness-call self_type (bb{bi}[{si}] of '{fn_name}')"));
                }
                if method_type_args.iter().any(|&t| is_error(module, t)) {
                    emit(ctx, n, owner, format!("witness-call type_args (bb{bi}[{si}] of '{fn_name}')"));
                }
            }
            _ => {}
        },
        _ => {}
    }
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
        .with_labels(vec![Label::primary(span.file_id, span.range())
            .with_message("an earlier phase should have diagnosed this")])
        .with_notes(vec![
            "this indicates a missing `UnresolvedTypeParam` diagnostic in type inference; \
             please file a bug with the program that triggered it"
                .into(),
        ]);
    ctx.query.accumulate(diag);
    *n += 1;
}
