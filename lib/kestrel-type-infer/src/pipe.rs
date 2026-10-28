//! Fix-its for a failed `x |> rhs` (pipe-operator design §3, "Error cases").
//!
//! A pipe lowers to an ordinary call, so when it fails the errors talk about
//! that call ("no matching overload", "no member 'coalesce'"). After solving,
//! this pass replaces the errors at the pipe (or at its right side) with one
//! that names the likely mistake, using what HIR lowering recorded about the
//! right side (`HirBody::pipes`). Errors inside the right side's own
//! arguments have other spans and are left alone.

use kestrel_ast_builder::Callable;
use kestrel_hir::body::{HirBody, HirExpr, PipeShape, PipeSite};
use kestrel_span::Span;

use crate::ctx::InferCtx;
use crate::error::InferError;

/// Reword each failed pipe's errors into one fix-it.
pub(crate) fn reword_pipe_errors(ctx: &mut InferCtx<'_>, hir: &HirBody) {
    for site in &hir.pipes {
        let hits: Vec<usize> = (0..ctx.errors.len())
            .filter(|&i| at_pipe(ctx.errors[i].span(), site))
            .collect();
        let Some(&first) = hits.first() else {
            continue;
        };
        let Some((message, note)) = fix_it(ctx, hir, site) else {
            continue;
        };
        let reworded = InferError::PipeFixIt {
            code: ctx.errors[first].code(),
            message: message.clone(),
            note,
            span: site.span.clone(),
        };
        ctx.errors[first] = reworded;
        ctx.error_details[first] = message;
        ctx.error_is_cascade[first] = false;
        // Drop the rest, last first so the indices stay valid.
        for &i in hits[1..].iter().rev() {
            ctx.errors.remove(i);
            ctx.error_details.remove(i);
            ctx.error_is_cascade.remove(i);
        }
    }
}

/// Is an error reported at the pipe itself or at its right side?
fn at_pipe(span: &Span, site: &PipeSite) -> bool {
    let same = |s: &Span| s.file_id == span.file_id && s.range() == span.range();
    same(&site.span) || same(&site.rhs_span)
}

/// The message and note for this pipe's shape, if it has a fix-it.
fn fix_it(ctx: &InferCtx<'_>, hir: &HirBody, site: &PipeSite) -> Option<(String, String)> {
    let rhs = &site.rhs_text;
    match &site.shape {
        PipeShape::CallValue { suggestion } => Some((
            format!(
                "this calls the result of `{rhs}` with the piped value: \
                 did you mean `{suggestion}`?"
            ),
            "a call on the right of `|>` with no `_` must return a function; \
             `_` marks where the piped value goes"
                .into(),
        )),
        PipeShape::Value => Some((
            format!("`{rhs}` is not callable, so it can't take the piped value"),
            "the right side of `|>` must be a function, or a call with `_` in it".into(),
        )),
        PipeShape::Path { call } => {
            let label = first_param_label(ctx, hir, *call)?;
            Some((
                format!("`{rhs}` labels its first parameter: write `{rhs}({label}: _)`"),
                "`x |> f` passes `x` unlabeled; a labeled parameter needs a `_` slot".into(),
            ))
        },
        PipeShape::Coalesce => Some((
            format!(
                "`|>` binds looser than `??`, so this pipes into `{rhs}`: write `(x |> f) ?? d`"
            ),
            "parenthesize the pipe to apply `??` to its result".into(),
        )),
        PipeShape::Slotted | PipeShape::Closure => None,
    }
}

/// The label of the first parameter of the function a path pipe calls.
fn first_param_label(
    ctx: &InferCtx<'_>,
    hir: &HirBody,
    call: kestrel_hir::body::HirExprId,
) -> Option<String> {
    let HirExpr::Call { callee, .. } = &hir.exprs[call] else {
        return None;
    };
    let entity = match &hir.exprs[*callee] {
        HirExpr::Def(entity, _, _) => *entity,
        _ => *ctx.resolutions.get(callee)?,
    };
    let callable = ctx.query_ctx.get::<Callable>(entity)?;
    callable.params.first()?.label.clone()
}
