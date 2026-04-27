//! `textDocument/codeLens` — inline actionable buttons above declarations.
//!
//! Currently surfaces a single lens: a "▶ Run" button above every
//! top-level `func main()` in the file. Clicking it triggers the
//! `kestrel.runMain` command on the client (the VS Code extension), which
//! opens a terminal in the workspace root and runs `flock run`.

use kestrel_ast_builder::{CstNode, DeclSpan, FileId, Name, NodeKind};
use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::utils::get_name_span;
use serde_json::json;
use tower_lsp::lsp_types::{CodeLens, CodeLensParams, Command};

use crate::position::LineIndex;
use crate::semantic;
use crate::server::{SharedState, url_to_path};

pub async fn handle(state: SharedState, params: CodeLensParams) -> Option<Vec<CodeLens>> {
    let uri = params.text_document.uri.clone();
    let path = url_to_path(&uri);

    let (handle, stdlib, user, line_index) = {
        let s = state.lock().await;
        let li = s.docs.get(&uri).map(|d| d.line_index.clone())?;
        let (stdlib, user) = s.partition_sources();
        (s.compiler_handle.clone(), stdlib, user, li)
    };

    let lenses = handle
        .with_compiler(stdlib, user, move |compiler, _by_path| -> Vec<CodeLens> {
            let world = compiler.world();
            let Some(file_entity) = semantic::file_entity_for_path(compiler, &path) else {
                return Vec::new();
            };
            collect_main_lenses(world, file_entity, &uri, &line_index)
        })
        .await?;

    if lenses.is_empty() {
        None
    } else {
        Some(lenses)
    }
}

/// Find every top-level `func main()` in this file and emit a Run lens
/// above its name. "Top-level" means the function's parent is the file's
/// module (not a nested type or extension).
fn collect_main_lenses(
    world: &World,
    file_entity: Entity,
    uri: &tower_lsp::lsp_types::Url,
    li: &LineIndex,
) -> Vec<CodeLens> {
    let mut lenses = Vec::new();
    for (entity, name) in world.iter_component::<Name>() {
        if name.0 != "main" {
            continue;
        }
        let Some(fid) = world.get::<FileId>(entity) else {
            continue;
        };
        if fid.0 != file_entity {
            continue;
        }
        if world.get::<NodeKind>(entity) != Some(&NodeKind::Function) {
            continue;
        }
        // Only top-level mains. Skip methods named `main` on a struct.
        let parent_kind = world
            .parent_of(entity)
            .and_then(|p| world.get::<NodeKind>(p));
        if !matches!(parent_kind, Some(NodeKind::Module)) {
            continue;
        }

        // Pin the lens range to the function's name (`main`) so the
        // button hovers right above the keyword line — VS Code anchors
        // the lens to the start of the range's line.
        let range = world
            .get::<CstNode>(entity)
            .and_then(|cst| {
                let file_id = world.get::<DeclSpan>(entity).map(|d| d.0.file_id)?;
                get_name_span(&cst.0, file_id)
            })
            .map(|s| li.range_for(s.start, s.end))
            .unwrap_or_else(|| {
                let span = world.get::<DeclSpan>(entity).map(|s| s.0.clone());
                match span {
                    Some(s) => li.range_for(s.start, s.end),
                    None => li.range_for(0, 0),
                }
            });

        lenses.push(CodeLens {
            range,
            command: Some(Command {
                title: "▶ Run".into(),
                command: "kestrel.runMain".into(),
                arguments: Some(vec![json!(uri.to_string())]),
            }),
            data: None,
        });
    }
    lenses
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_compiler::Compiler;
    use tower_lsp::lsp_types::Url;

    #[test]
    fn one_lens_per_main() {
        let src = "module Test\nfunc main() { }\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/main_lens.ks", src.into());
        c.build(f);
        let li = LineIndex::new(src.to_string());
        let uri = Url::parse("file:///tmp/main_lens.ks").unwrap();
        let lenses = collect_main_lenses(c.world(), f, &uri, &li);
        assert_eq!(lenses.len(), 1, "expected one Run lens, got {lenses:?}");
        assert_eq!(
            lenses[0].command.as_ref().unwrap().command,
            "kestrel.runMain"
        );
    }

    #[test]
    fn no_lens_when_no_main() {
        let src = "module Test\nfunc other() -> lang.i64 { 1 }\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/no_main.ks", src.into());
        c.build(f);
        let li = LineIndex::new(src.to_string());
        let uri = Url::parse("file:///tmp/no_main.ks").unwrap();
        let lenses = collect_main_lenses(c.world(), f, &uri, &li);
        assert!(lenses.is_empty(), "expected no lenses, got {lenses:?}");
    }
}
