//! `textDocument/documentSymbol` — outline panel + symbol picker.
//!
//! Walks every entity tagged with the file's `FileId`, builds a flat list of
//! `(entity, parent_in_file)` pairs, then folds it into a `DocumentSymbol`
//! tree. Each node uses `DeclSpan` for the `range` (the whole declaration,
//! used for hover-highlighting) and `get_name_span` from the CST for
//! `selection_range` (just the identifier — what the editor focuses when
//! you click the outline entry).

use kestrel_ast_builder::{CstNode, DeclSpan, FileId, Name, NodeKind};
use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::utils::get_name_span;
use tower_lsp::lsp_types::{
    DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse, SymbolKind,
};

use crate::position::LineIndex;
use crate::semantic;
use crate::server::{url_to_path, SharedState};

pub async fn handle(
    state: SharedState,
    params: DocumentSymbolParams,
) -> Option<DocumentSymbolResponse> {
    let uri = params.text_document.uri;
    let path = url_to_path(&uri);

    let (handle, stdlib, user, line_index) = {
        let s = state.lock().await;
        let li = s.docs.get(&uri).map(|d| d.line_index.clone())?;
        let (stdlib, user) = s.partition_sources();
        (s.compiler_handle.clone(), stdlib, user, li)
    };

    handle.with_compiler(stdlib, user, move |compiler, _by_path| -> Option<DocumentSymbolResponse> {
        let file_entity = semantic::file_entity_for_path(compiler, &path)?;
        let symbols = build_outline(compiler.world(), file_entity, &line_index);
        Some(DocumentSymbolResponse::Nested(symbols))
    })
    .await
    .flatten()
}

/// Build the outline for a single file by walking the decl tree.
fn build_outline(world: &World, file_entity: Entity, li: &LineIndex) -> Vec<DocumentSymbol> {
    // Collect every entity whose `FileId` matches our file.
    let in_file: Vec<Entity> = world
        .iter_component::<FileId>()
        .filter(|(_, fid)| fid.0 == file_entity)
        .map(|(e, _)| e)
        .collect();

    use std::collections::HashSet;
    let in_file_set: HashSet<Entity> = in_file.iter().copied().collect();

    // Top-level = entities whose parent is NOT in the file (they hang off
    // the file's enclosing module or root).
    let roots: Vec<Entity> = in_file
        .iter()
        .copied()
        .filter(|&e| {
            world
                .parent_of(e)
                .map(|p| !in_file_set.contains(&p))
                .unwrap_or(true)
        })
        .collect();

    let mut out = Vec::new();
    for root_entity in roots {
        if let Some(sym) = build_symbol(world, root_entity, &in_file_set, li) {
            out.push(sym);
        }
    }
    out
}

fn build_symbol(
    world: &World,
    entity: Entity,
    in_file: &std::collections::HashSet<Entity>,
    li: &LineIndex,
) -> Option<DocumentSymbol> {
    let kind = world.get::<NodeKind>(entity)?;
    let symbol_kind = node_kind_to_symbol_kind(kind)?;
    let name = world
        .get::<Name>(entity)
        .map(|n| n.0.clone())
        .filter(|s| !s.is_empty())?;

    let decl_span = world.get::<DeclSpan>(entity)?.0.clone();
    let range = li.range_for(decl_span.start, decl_span.end);

    let selection_range = world
        .get::<CstNode>(entity)
        .and_then(|cst| get_name_span(&cst.0, decl_span.file_id))
        .map(|s| li.range_for(s.start, s.end))
        .unwrap_or(range);

    let mut children: Vec<DocumentSymbol> = Vec::new();
    for &child in world.children_of(entity) {
        if !in_file.contains(&child) {
            continue;
        }
        if let Some(sym) = build_symbol(world, child, in_file, li) {
            children.push(sym);
        }
    }

    let detail = world.get::<NodeKind>(entity).map(|k| format!("{k:?}"));

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name,
        detail,
        kind: symbol_kind,
        tags: None,
        deprecated: None,
        range,
        selection_range,
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    })
}

/// Map a Kestrel `NodeKind` to an LSP `SymbolKind`. Returns `None` for kinds
/// we deliberately exclude from the outline (imports, param defaults, type
/// parameters, anonymous accessors).
fn node_kind_to_symbol_kind(kind: &NodeKind) -> Option<SymbolKind> {
    Some(match kind {
        NodeKind::Module => SymbolKind::MODULE,
        NodeKind::Struct => SymbolKind::STRUCT,
        NodeKind::Enum => SymbolKind::ENUM,
        NodeKind::EnumCase => SymbolKind::ENUM_MEMBER,
        NodeKind::Protocol => SymbolKind::INTERFACE,
        NodeKind::Extension => SymbolKind::NAMESPACE,
        NodeKind::Function => SymbolKind::FUNCTION,
        NodeKind::Initializer => SymbolKind::CONSTRUCTOR,
        NodeKind::Field => SymbolKind::FIELD,
        NodeKind::TypeAlias => SymbolKind::CLASS,
        NodeKind::Subscript => SymbolKind::OPERATOR,
        // Hidden from outline: imports clutter the panel; setters / deinits /
        // param-defaults / type-parameters are sub-decls without their own
        // identifier worth highlighting.
        NodeKind::Import
        | NodeKind::Setter
        | NodeKind::Deinit
        | NodeKind::ParamDefault
        | NodeKind::TypeParameter => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_compiler::Compiler;

    fn outline(src: &str) -> Vec<DocumentSymbol> {
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/outline.ks", src.into());
        c.build(f);
        let li = LineIndex::new(src.to_string());
        build_outline(c.world(), f, &li)
    }

    #[test]
    fn outline_lists_top_level_decls() {
        let src = "module Test\n\
                   struct Point { var x: lang.i64; var y: lang.i64; }\n\
                   func origin() -> Point { Point(x: 0, y: 0) }\n";
        let syms = outline(src);
        let names: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Point"), "expected Point in {names:?}");
        assert!(names.contains(&"origin"), "expected origin in {names:?}");
    }

    #[test]
    fn outline_nests_struct_fields() {
        let src = "module Test\nstruct Point { var x: lang.i64; var y: lang.i64; }\n";
        let syms = outline(src);
        let point = syms
            .iter()
            .find(|s| s.name == "Point")
            .expect("Point present");
        let children = point.children.as_ref().expect("Point has fields");
        let names: Vec<&str> = children.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"x"));
        assert!(names.contains(&"y"));
    }

    #[test]
    fn outline_excludes_imports() {
        let src = "module Test\nimport lang;\nfunc foo() -> lang.i64 { 1 }\n";
        let syms = outline(src);
        for s in &syms {
            assert_ne!(s.kind, SymbolKind::FILE, "no file kind expected");
            assert!(
                s.name != "lang",
                "import shouldn't show up in outline ({:?})",
                s.name
            );
        }
        assert!(syms.iter().any(|s| s.name == "foo"));
    }

    #[test]
    fn outline_selection_range_is_just_identifier() {
        let src = "module Test\nfunc foo() -> lang.i64 { 1 }\n";
        let syms = outline(src);
        let foo = syms.iter().find(|s| s.name == "foo").expect("foo present");
        // selection_range should cover only `foo`, which is one line and 3 chars
        // wide. range covers the whole `func foo() -> ...` declaration so it's
        // strictly larger.
        let sel = &foo.selection_range;
        let r = &foo.range;
        let same = sel.start == r.start && sel.end == r.end;
        assert!(
            !same,
            "selection_range should be narrower than range; both = {sel:?}"
        );
    }
}
