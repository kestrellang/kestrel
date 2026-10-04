//! TypeAlias declaration builder.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::ast::{self, AstNode};
use kestrel_syntax_tree::utils::get_decl_span;

use super::helpers::*;
use super::type_param::build_type_parameters;
use crate::ast_type::lower_opt_type;
use crate::components::*;

/// Build a type alias declaration entity from CST.
///
/// Components: NodeKind::TypeAlias, Name, FileId, Vis, Typed,
/// TypeAnnotation (target), [QualifiedTarget], [TypeParams],
/// [Conformances], [WhereClause], [Attributes], [Documentation]
pub fn build_type_alias(
    world: &mut World,
    node: &ast::TypeAliasDeclaration,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) {
    let syntax = node.syntax();
    let entity = world.spawn();

    world.set(entity, NodeKind::TypeAlias);
    world.set(entity, FileId(file_entity));
    world.set(entity, Typed);
    world.set(entity, DeclSpan(get_decl_span(syntax, file_id)));
    world.set(entity, CstNode(syntax.clone()));
    world.set_parent(entity, parent);

    // `type Iterator.Item = Int` names `Item`, not the qualifying protocol.
    if let Some(name) = node.alias_name().and_then(|n| n.text()) {
        world.set(entity, Name(name));
    }

    // The qualifying protocol (`Protocol` in `type Protocol.Assoc = …`), so
    // analyzers can resolve it via ResolveTypePath.
    if let Some(proto_ty) = node
        .associated_type_target()
        .and_then(|t| lower_opt_type(t.ty(), file_id))
    {
        world.set(entity, QualifiedTarget(proto_ty));
    }

    if let Some(ty) = node
        .aliased_type()
        .and_then(|a| lower_opt_type(a.ty(), file_id))
    {
        world.set(entity, TypeAnnotation(ty));
    }

    set_visibility(world, entity, node);
    set_attributes(world, entity, node, file_id);
    set_documentation(world, entity, syntax);
    // Associated types in protocols can have bounds: `type Iter: Iterator`
    set_conformances(world, entity, node, file_id);
    // And where clauses: `type Iter: Iterator where Iter.Item = Item`
    set_where_clause(world, entity, node, file_id);
    build_type_parameters(world, entity, node, file_entity, file_id);
}
