//! Type parameter extraction and entity creation.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::SyntaxNodePtr;
use kestrel_syntax_tree::ast::{AstNode, HasGenerics, HasName};
use kestrel_syntax_tree::utils::get_decl_span;

use crate::ast_type::lower_opt_type;
use crate::components::*;

/// Create an entity per type parameter of `node`'s `[T, U = D]` list and
/// set the TypeParams component on `parent`.
pub fn build_type_parameters(
    world: &mut World,
    parent: Entity,
    node: &impl HasGenerics,
    file_entity: Entity,
    file_id: usize,
) {
    let Some(list) = node.type_parameter_list() else {
        return;
    };

    let mut param_entities = Vec::new();
    for param in list.type_parameters() {
        let Some(name) = param.name_text() else {
            continue;
        };
        let entity = world.spawn();
        world.set(entity, NodeKind::TypeParameter);
        world.set(entity, Name(name));
        world.set(entity, FileId(file_entity));
        world.set(entity, DeclSpan(get_decl_span(param.syntax(), file_id)));
        world.set(entity, CstNode(SyntaxNodePtr::new(&param.syntax())));
        world.set_parent(entity, parent);

        if let Some(ty) = param
            .default_type()
            .and_then(|d| lower_opt_type(d.ty(), file_id))
        {
            world.set(entity, TypeAnnotation(ty));
        }
        param_entities.push(entity);
    }

    if !param_entities.is_empty() {
        world.set(parent, TypeParams(param_entities));
    }
}
