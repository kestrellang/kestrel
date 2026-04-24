//! Import declaration builder.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::SyntaxNode;
use kestrel_syntax_tree::imports::extract_import_declaration;
use kestrel_syntax_tree::utils::get_decl_span;

use crate::components::*;

/// Build an import declaration entity from CST.
///
/// Components: NodeKind::Import, FileId, ModulePath,
/// [ImportAlias], [ImportItems]
pub fn build_import(
    world: &mut World,
    node: &SyntaxNode,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) {
    let decl = match extract_import_declaration(node, file_id) {
        Some(d) => d,
        None => return,
    };

    let entity = world.spawn();

    world.set(entity, NodeKind::Import);
    world.set(entity, FileId(file_entity));
    world.set(entity, DeclSpan(get_decl_span(node, file_id)));
    world.set(entity, CstNode(node.clone()));
    world.set_parent(entity, parent);

    // Module path as list of segment strings
    let path: Vec<String> = decl.module_path.iter().map(|(s, _)| s.clone()).collect();
    world.set(entity, ModulePath(path));

    // Import alias (`import Foo as Bar`)
    if let Some(alias) = decl.alias {
        world.set(entity, ImportAlias(alias));
    }

    // Specific import items (`import Foo (Bar, Baz)`)
    if !decl.items.is_empty() {
        let items = decl
            .items
            .into_iter()
            .map(|item| ImportItem {
                name: item.name,
                alias: item.alias,
            })
            .collect();
        world.set(entity, ImportItems(items));
    }
}
