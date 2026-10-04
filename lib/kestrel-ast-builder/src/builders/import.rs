//! Import declaration builder.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::ast::{self, AstNode};
use kestrel_syntax_tree::utils::get_decl_span;

use crate::components::*;

/// Build an import declaration entity from CST.
///
/// Components: NodeKind::Import, FileId, ModulePath,
/// [ImportAlias], [ImportItems]
pub fn build_import(
    world: &mut World,
    node: &ast::ImportDeclaration,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) {
    let Some(path) = node.module_path() else {
        return;
    };
    let syntax = node.syntax();
    let entity = world.spawn();

    world.set(entity, NodeKind::Import);
    world.set(entity, FileId(file_entity));
    world.set(entity, DeclSpan(get_decl_span(syntax, file_id)));
    world.set(entity, CstNode(syntax.clone()));
    world.set_parent(entity, parent);

    let segments = path
        .segment_tokens()
        .map(|t| t.text().to_string())
        .collect();
    world.set(entity, ModulePath(segments));

    // `import Foo as Bar`
    if !node.has_item_list()
        && let Some(alias) = node.alias()
    {
        world.set(entity, ImportAlias(alias.text().to_string()));
    }

    // `import Foo.(Bar, Baz as Q)`
    let items: Vec<ImportItem> = node
        .import_items()
        .filter_map(|item| {
            Some(ImportItem {
                name: item.identifier_token()?.text().to_string(),
                alias: item.alias().map(|t| t.text().to_string()),
            })
        })
        .collect();
    if !items.is_empty() {
        world.set(entity, ImportItems(items));
    }
}
