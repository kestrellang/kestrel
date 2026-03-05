//! TypeAlias declaration builder.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree2::{SyntaxKind, SyntaxNode};
use kestrel_syntax_tree2::utils::{extract_name, find_child, get_node_span};

use crate::ast_type::ast_type_from_cst;
use crate::components::*;
use super::helpers::*;
use super::type_param::build_type_parameters;

/// Build a type alias declaration entity from CST.
///
/// Components: NodeKind::TypeAlias, Name, FileId, Vis, Typed,
/// TypeAnnotation (target), [TypeParams], [Attributes]
pub fn build_type_alias(
    world: &mut World,
    node: &SyntaxNode,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) {
    let entity = world.spawn();

    world.set(entity, NodeKind::TypeAlias);
    world.set(entity, FileId(file_entity));
    world.set(entity, Typed);
    world.set(entity, DeclSpan(get_node_span(node, file_id)));
    world.set(entity, CstNode(node.clone()));
    world.set_parent(entity, parent);

    if let Some(name) = extract_name(node) {
        world.set(entity, Name(name));
    }

    // Target type from AliasedType child
    if let Some(aliased) = find_child(node, SyntaxKind::AliasedType) {
        if let Some(ty) = aliased
            .children()
            .find(|c| is_type_kind(c.kind()))
            .and_then(|c| ast_type_from_cst(&c, file_id))
        {
            world.set(entity, TypeAnnotation(ty));
        }
    }

    set_visibility(world, entity, node);
    set_attributes(world, entity, node);
    build_type_parameters(world, entity, node, file_entity, file_id);
}

fn is_type_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Ty
            | SyntaxKind::TyPath
            | SyntaxKind::TyTuple
            | SyntaxKind::TyFunction
            | SyntaxKind::TyArray
            | SyntaxKind::TyDictionary
            | SyntaxKind::TyOptional
            | SyntaxKind::TyResult
            | SyntaxKind::TyUnit
            | SyntaxKind::TyNever
            | SyntaxKind::TyInferred
    )
}
