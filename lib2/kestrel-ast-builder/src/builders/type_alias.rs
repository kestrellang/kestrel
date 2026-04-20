//! TypeAlias declaration builder.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree2::{SyntaxKind, SyntaxNode};
use kestrel_syntax_tree2::utils::{extract_name, find_child, get_decl_span};

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
    world.set(entity, DeclSpan(get_decl_span(node, file_id)));
    world.set(entity, CstNode(node.clone()));
    world.set_parent(entity, parent);

    // For qualified bindings like `type Iterator.Item = Int`, the Name child
    // is inside an AssociatedTypeTarget node. extract_name would return "Iterator"
    // (the protocol name) instead of "Item" (the actual alias name).
    // Check for AssociatedTypeTarget first and extract the last Name from it.
    let assoc_target = find_child(node, SyntaxKind::AssociatedTypeTarget);
    let name = if let Some(target) = &assoc_target {
        // Qualified: last Name > Identifier in the target is the alias name
        target.children()
            .filter(|c| c.kind() == SyntaxKind::Name)
            .last()
            .and_then(|n| {
                n.children_with_tokens()
                    .filter_map(|e| e.into_token())
                    .find(|t| t.kind() == SyntaxKind::Identifier)
                    .map(|t| t.text().to_string())
            })
    } else {
        extract_name(node)
    };
    if let Some(name) = name {
        world.set(entity, Name(name));
    }

    // Capture the qualifying protocol path (`Protocol` in `type Protocol.Assoc = …`)
    // as an AstType so analyzers can resolve it via ResolveTypePath.
    if let Some(target) = &assoc_target {
        if let Some(proto_ty) = target
            .children()
            .find(|c| is_type_kind(c.kind()))
            .and_then(|c| ast_type_from_cst(&c, file_id))
        {
            world.set(entity, QualifiedTarget(proto_ty));
        }
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
    // Associated types in protocols can have bounds: `type Iter: Iterator`
    set_conformances(world, entity, node, file_id);
    // And where clauses: `type Iter: Iterator where Iter.Item = Item`
    set_where_clause(world, entity, node, file_id);
    build_type_parameters(world, entity, node, file_entity, file_id);
}
