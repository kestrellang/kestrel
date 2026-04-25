//! `textDocument/hover` — show the inferred type of the expression at the
//! cursor.
//!
//! `Compiler`/`World` aren't `Send` (they hold non-`Send` `dyn` trait
//! objects), so they can't live in our async state. Instead we rebuild a
//! fresh compiler inside `spawn_blocking` per request, mirroring the
//! diagnostics handler. Compiler reuse is an optimisation tracked for M5.

use kestrel_hir::body::HirBody;
use kestrel_hir_lower::LowerBody;
use kestrel_type_infer::InferBody;
use tower_lsp::lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind, Range};

use crate::semantic;
use crate::server::{rebuild_compiler, url_to_path, SharedState};
use crate::ty_format::format_ty;

pub async fn handle(state: SharedState, params: HoverParams) -> Option<Hover> {
    let uri = params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let path = url_to_path(&uri);

    let (sources, line_index) = {
        let s = state.lock().await;
        let line_index = s.docs.get(&uri).map(|d| d.line_index.clone())?;
        (s.sources.clone(), line_index)
    };
    let offset = line_index.position_to_offset(pos);

    let result = tokio::task::spawn_blocking(move || -> Option<(String, Range)> {
        let (compiler, _) = rebuild_compiler(&sources);
        let file_entity = semantic::file_entity_for_path(&compiler, &path)?;
        let body_entity = semantic::body_entity_at(compiler.world(), file_entity, offset)?;

        let world = compiler.world();
        let ctx = world.query_context();
        let hir: HirBody =
            ctx.query(LowerBody { entity: body_entity, root: compiler.root() })?;
        let typed = ctx.query(InferBody { entity: body_entity, root: compiler.root() })?;

        let expr_id = semantic::hir_expr_at(&hir, offset)?;
        let ty = typed.expr_types.get(&expr_id)?;
        let rendered = format_ty(world, ty);

        let span = semantic::hir_expr_span(&hir.exprs[expr_id]);
        let range = line_index.range_for(span.start, span.end);
        Some((rendered, range))
    })
    .await
    .ok()??;

    let (rendered, range) = result;
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("```kestrel\n{}\n```", rendered),
        }),
        range: Some(range),
    })
}
