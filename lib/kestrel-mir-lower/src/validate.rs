//! Post-lowering ICE backstop: any `MirTy::Error` that escapes into the MIR
//! signals an internal compiler inconsistency. Walks the module and emits
//! one ICE diagnostic per location carrying Error via `QueryContext::accumulate`.

use kestrel_ast_builder::DeclSpan;
use kestrel_hecs::Entity;
use kestrel_mir::{MirModule, MirTy, TyId};
use kestrel_reporting::{Diagnostic, Label};
use kestrel_span::Span;

use crate::context::LowerCtx;

pub fn validate_no_error_types(ctx: &LowerCtx, module: &MirModule) -> usize {
    let mut n = 0usize;

    for s in module.structs.values() {
        for field in &s.fields {
            if is_error(&module, field.ty) {
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

    for st in module.statics.values() {
        if is_error(&module, st.ty) {
            emit(ctx, &mut n, st.entity, format!("static '{}'", st.name));
        }
    }

    for f in module.functions.values() {
        if is_error(&module, f.ret) {
            emit(
                ctx,
                &mut n,
                f.entity,
                format!("return type of '{}'", f.name),
            );
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
