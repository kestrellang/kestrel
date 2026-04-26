//! `textDocument/inlayHint` — type hints for `let` / `var` bindings without
//! an explicit annotation.
//!
//! Walks every `Body` entity in the file, runs `LowerBody` + `InferBody`,
//! and emits a `: T` hint for each `HirStmt::Let { ty: None, .. }` whose
//! local was bound from a real source pattern (skipping `$let_tmp`,
//! `$iter`, and other desugaring synthetics whose names start with `$`).
//!
//! The hint position is the end of the `BindingPattern` CST node — i.e.
//! immediately after the binding identifier and before any `=`. We rely
//! on that node existing in the CST; complex patterns (which the parser
//! lowers via a `$let_tmp` indirection) are excluded by the `$`-name
//! filter so we don't have to handle them here.

use kestrel_ast_builder::{Body, FileId, Valued};
use kestrel_hir::body::HirStmt;
use kestrel_hir_lower::LowerBody;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use kestrel_type_infer::result::ResolvedTy;
use kestrel_type_infer::InferBody;
use rowan::TextSize;
use tower_lsp::lsp_types::{
    InlayHint, InlayHintKind, InlayHintLabel, InlayHintParams,
};

use crate::semantic;
use crate::server::{url_to_path, SharedState};
use crate::ty_format::format_ty;

pub async fn handle(state: SharedState, params: InlayHintParams) -> Option<Vec<InlayHint>> {
    let uri = params.text_document.uri;
    let path = url_to_path(&uri);

    let (handle, stdlib, user, line_index) = {
        let s = state.lock().await;
        let line_index = s.docs.get(&uri).map(|d| d.line_index.clone())?;
        let (stdlib, user) = s.partition_sources();
        (s.compiler_handle.clone(), stdlib, user, line_index)
    };

    // Visible range — clip hints to what the editor will actually paint.
    let range_start = line_index.position_to_offset(params.range.start);
    let range_end = line_index.position_to_offset(params.range.end);

    let li = line_index.clone();
    handle.with_compiler(stdlib, user, move |compiler, _by_path| -> Vec<InlayHint> {
        let Some(file_entity) = semantic::file_entity_for_path(compiler, &path) else {
            return vec![];
        };
        let world = compiler.world();
        let root = compiler.root();
        let ctx = world.query_context();

        let mut hints: Vec<InlayHint> = Vec::new();

        let body_entities: Vec<_> = world
            .iter_component::<Body>()
            .filter_map(|(e, _)| {
                let fid = world.get::<FileId>(e)?;
                (fid.0 == file_entity).then_some(e)
            })
            .collect();

        for body_entity in body_entities {
            let Some(hir) = ctx.query(LowerBody { entity: body_entity, root }) else { continue };
            let Some(typed) = ctx.query(InferBody { entity: body_entity, root }) else { continue };
            let Some(cst) = world.get::<Valued>(body_entity).map(|v| v.0.clone()) else { continue };

            for (_id, stmt) in hir.stmts.iter() {
                let HirStmt::Let { local, ty: None, span, .. } = stmt else { continue };
                if span.end < range_start || span.start > range_end { continue }

                let local_data = &hir.locals[*local];
                // Synthetic locals from desugaring (`$let_tmp`, `$iter`, …)
                // share the HirStmt::Let shape but have no source binding to
                // hint against.
                if local_data.name.starts_with('$') { continue }

                let Some(ty) = typed.local_types.get(local) else { continue };
                if matches!(ty, ResolvedTy::Error) { continue }

                let Some(end_offset) = binding_pattern_end(&cst, span) else { continue };

                let label = format!(": {}", format_ty(world, ty));
                hints.push(InlayHint {
                    position: li.offset_to_position(end_offset),
                    label: InlayHintLabel::String(label),
                    kind: Some(InlayHintKind::TYPE),
                    text_edits: None,
                    tooltip: None,
                    padding_left: Some(false),
                    padding_right: Some(false),
                    data: None,
                });
            }
        }

        hints
    })
    .await
    .unwrap_or_default()
    .into()
}

/// Find the end offset of the `BindingPattern` inside the `VariableDeclaration`
/// whose CST range covers `let_span`. Returns `None` for synthesized lets that
/// don't correspond to a real source statement.
fn binding_pattern_end(cst: &SyntaxNode, let_span: &Span) -> Option<usize> {
    let start = TextSize::from(let_span.start as u32);
    let end = TextSize::from(let_span.end as u32);
    for n in cst.descendants() {
        if n.kind() != SyntaxKind::VariableDeclaration { continue }
        let r = n.text_range();
        if r.start() > start || r.end() < end { continue }
        for desc in n.descendants() {
            if desc.kind() == SyntaxKind::BindingPattern {
                return Some(desc.text_range().end().into());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_compiler::Compiler;

    fn collect_hints(src: &str) -> Vec<(usize, String)> {
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/inlay.ks", src.into());
        c.build(f);
        let driver = kestrel_compiler_driver::CompilerDriver::new(&c);
        let _ = driver.infer_all();
        let world = c.world();
        let root = c.root();
        let ctx = world.query_context();
        let body_entities: Vec<_> = world
            .iter_component::<Body>()
            .filter_map(|(e, _)| {
                let fid = world.get::<FileId>(e)?;
                (fid.0 == f).then_some(e)
            })
            .collect();
        let mut out: Vec<(usize, String)> = Vec::new();
        for body_entity in body_entities {
            let Some(hir) = ctx.query(LowerBody { entity: body_entity, root }) else { continue };
            let Some(typed) = ctx.query(InferBody { entity: body_entity, root }) else { continue };
            let Some(cst) = world.get::<Valued>(body_entity).map(|v| v.0.clone()) else { continue };
            for (_id, stmt) in hir.stmts.iter() {
                let HirStmt::Let { local, ty: None, span, .. } = stmt else { continue };
                let local_data = &hir.locals[*local];
                if local_data.name.starts_with('$') { continue }
                let Some(ty) = typed.local_types.get(local) else { continue };
                if matches!(ty, ResolvedTy::Error) { continue }
                let Some(end_offset) = binding_pattern_end(&cst, span) else { continue };
                out.push((end_offset, format!(": {}", format_ty(world, ty))));
            }
        }
        out
    }

    #[test]
    fn hint_for_simple_let() {
        // Without stdlib loaded, integer literal defaulting falls back to
        // Error — drive the type via a known-typed parameter instead.
        let src = "module T\nstruct P { var x: lang.i64 }\nfunc f(p: P) { let x = p; }\n";
        let hints = collect_hints(src);
        assert_eq!(hints.len(), 1, "{:?}", hints);
        let name_end = src.find("let x =").unwrap() + "let x".len();
        assert_eq!(hints[0].0, name_end);
        assert!(hints[0].1.contains("P"), "label = {:?}", hints[0].1);
    }

    #[test]
    fn hint_for_var_binding() {
        let src = "module T\nstruct P { var x: lang.i64 }\nfunc f(p: P) { var x = p; }\n";
        let hints = collect_hints(src);
        assert_eq!(hints.len(), 1, "{:?}", hints);
        assert!(hints[0].1.contains("P"), "{:?}", hints[0].1);
    }

    #[test]
    fn no_hint_when_type_annotated() {
        let src = "module T\nstruct P { var x: lang.i64 }\nfunc f(p: P) { let x: P = p; }\n";
        let hints = collect_hints(src);
        assert!(hints.is_empty(), "{:?}", hints);
    }

    #[test]
    fn no_hint_for_destructured_let() {
        // `let (a, b) = …` lowers via a `$let_tmp` plus a desugared match
        // — both are filtered out by the `$` name check, so no hint surfaces
        // until destructured-binding hints are wired up explicitly.
        let src = "module T\nstruct P { var x: lang.i64 }\nfunc f(p: P, q: P) { let (a, b) = (p, q); }\n";
        let hints = collect_hints(src);
        assert!(hints.is_empty(), "{:?}", hints);
    }
}
