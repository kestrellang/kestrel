//! Post-lowering ICE backstop: any `MirTy::Error` that escapes into the MIR
//! signals an internal compiler inconsistency — inference's phase-4
//! `UnresolvedTypeParam` diagnostic is supposed to catch these at the source.
//! If one reaches here, inference failed to ground a type but didn't report
//! it, and Cranelift would later panic with a misleading "declared type of
//! variable varN doesn't match value vM" message.
//!
//! This walks the built MIR module and accumulates one ICE-flavored
//! diagnostic per location carrying `MirTy::Error`. Diagnostics are attached
//! to the owning entity's `DeclSpan`. The user-facing message asks for a
//! bug report — if you see these firing, the real fix is upstream in
//! inference, not here.

use kestrel_ast_builder::DeclSpan;
use kestrel_hecs::Entity;
use kestrel_mir::{Callee, MirModule, MirTy, Rvalue, StatementKind};
use kestrel_reporting::{Diagnostic, Label};
use kestrel_span::Span;

use crate::context::LowerCtx;

/// Walk the MIR module and accumulate one ICE diagnostic per location
/// containing `MirTy::Error`. Returns the number of diagnostics accumulated.
/// Call after `lower_module` finishes building items. A nonzero return
/// means inference failed to diagnose an unresolved type at the source —
/// that's the bug to fix; this pass only ensures we don't silently
/// miscompile.
pub fn validate_no_error_types(ctx: &LowerCtx, module: &MirModule) -> usize {
    let mut n = 0usize;

    for s in &module.structs {
        for field in &s.fields {
            if field.ty.contains_error() {
                emit(
                    ctx,
                    &mut n,
                    s.entity,
                    format!("field '{}.{}'", s.name, field.name),
                );
            }
        }
    }
    // Enum payloads are stored in synthesized payload structs, so they're
    // already covered by the struct pass above.

    for st in &module.statics {
        if st.ty.contains_error() {
            emit(ctx, &mut n, st.entity, format!("static '{}'", st.name));
        }
    }

    for f in &module.functions {
        if f.ret.contains_error() {
            emit(
                ctx,
                &mut n,
                f.entity,
                format!("return type of '{}'", f.name),
            );
        }
        for p in &f.params {
            if p.ty.contains_error() {
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
                if local.ty.contains_error() {
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
                    scan_stmt(ctx, &mut n, f.entity, &f.name, bi, si, &stmt.kind);
                }
            }
        }
    }

    n
}

fn scan_stmt(
    ctx: &LowerCtx,
    n: &mut usize,
    owner: Entity,
    fn_name: &str,
    bi: usize,
    si: usize,
    stmt: &StatementKind,
) {
    match stmt {
        StatementKind::Assign { rvalue, .. } => match rvalue {
            Rvalue::Construct { ty, .. } => {
                if ty.contains_error() {
                    emit(
                        ctx,
                        n,
                        owner,
                        format!("Construct rvalue (bb{bi}[{si}] of '{fn_name}')"),
                    );
                }
            },
            Rvalue::EnumVariant { enum_ty, .. } => {
                if enum_ty.contains_error() {
                    emit(
                        ctx,
                        n,
                        owner,
                        format!("EnumVariant rvalue (bb{bi}[{si}] of '{fn_name}')"),
                    );
                }
            },
            Rvalue::ArrayLiteral { element_ty, .. } => {
                if element_ty.contains_error() {
                    emit(
                        ctx,
                        n,
                        owner,
                        format!("ArrayLiteral element type (bb{bi}[{si}] of '{fn_name}')"),
                    );
                }
            },
            _ => {},
        },
        StatementKind::Call { callee, .. } => match callee {
            Callee::Direct {
                type_args,
                self_type,
                ..
            } => {
                if type_args.iter().any(MirTy::contains_error) {
                    emit(
                        ctx,
                        n,
                        owner,
                        format!("direct-call type_args (bb{bi}[{si}] of '{fn_name}')"),
                    );
                }
                if let Some(t) = self_type
                    && t.contains_error()
                {
                    emit(
                        ctx,
                        n,
                        owner,
                        format!("direct-call self_type (bb{bi}[{si}] of '{fn_name}')"),
                    );
                }
            },
            Callee::Witness {
                self_type,
                method_type_args,
                ..
            } => {
                if self_type.contains_error() {
                    emit(
                        ctx,
                        n,
                        owner,
                        format!("witness-call self_type (bb{bi}[{si}] of '{fn_name}')"),
                    );
                }
                if method_type_args.iter().any(MirTy::contains_error) {
                    emit(
                        ctx,
                        n,
                        owner,
                        format!("witness-call type_args (bb{bi}[{si}] of '{fn_name}')"),
                    );
                }
            },
            _ => {},
        },
        _ => {},
    }
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
