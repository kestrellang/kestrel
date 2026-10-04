//! Subscript declaration builder.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::ast::{self, AstNode, HasStatic};
use kestrel_syntax_tree::utils::get_decl_span;

use super::helpers::*;
use super::params::extract_params;
use super::type_param::build_type_parameters;
use crate::ast_type::lower_opt_type;
use crate::components::*;
use crate::lower;

/// Build a subscript declaration entity from CST.
///
/// Components: NodeKind::Subscript, FileId, Vis, Callable, TypeAnnotation,
/// Subscript, Gettable, [Settable], [Static], [TypeParams],
/// [WhereClause], [Attributes], [Documentation]
pub fn build_subscript(
    world: &mut World,
    node: &ast::SubscriptDeclaration,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) {
    let syntax = node.syntax();
    let entity = world.spawn();

    world.set(entity, NodeKind::Subscript);
    world.set(entity, FileId(file_entity));
    world.set(entity, DeclSpan(get_decl_span(syntax, file_id)));
    world.set(entity, CstNode(syntax.clone()));
    world.set(entity, Subscript);
    world.set(entity, Gettable);
    world.set_parent(entity, parent);

    // Parameters — subscripts inside types have a borrowing receiver
    let params = extract_params(world, node.parameter_list(), entity, file_entity, file_id);
    let is_static = node.is_static();
    let has_receiver = !is_static
        && world
            .get::<NodeKind>(parent)
            .is_some_and(NodeKind::is_type_scope);
    world.set(
        entity,
        Callable {
            params,
            receiver: has_receiver.then_some(ReceiverKind::Borrowing),
        },
    );

    if let Some(ty) = node
        .return_type()
        .and_then(|r| lower_opt_type(r.ty(), file_id))
    {
        world.set(entity, TypeAnnotation(ty));
    }

    if let Some(body) = node.subscript_body() {
        if let Some(acc) = body.property_accessors() {
            build_accessors(
                world,
                entity,
                &acc,
                has_receiver,
                file_entity,
                file_id,
                is_static,
            );
        } else if let Some(block) = body.code_block() {
            // Shorthand getter-only form: subscript(...) -> T { expr }
            world.set(entity, Body(lower::lower_body(block.syntax(), file_id)));
            world.set(entity, Valued(block.syntax().clone()));
        }
    }

    if is_static {
        world.set(entity, Static);
    }

    set_visibility(world, entity, node);
    set_attributes(world, entity, node, file_id);
    set_documentation(world, entity, syntax);
    set_where_clause(world, entity, node, file_id);
    build_type_parameters(world, entity, node, file_entity, file_id);
}

/// `{ get … set … ref … }`: Settable, the getter body, and a child entity
/// per setter / place accessor, each taking the index parameters.
fn build_accessors(
    world: &mut World,
    entity: Entity,
    acc: &ast::PropertyAccessors,
    has_receiver: bool,
    file_entity: Entity,
    file_id: usize,
    is_static: bool,
) {
    let has_setter = acc.declares_set();
    if has_setter {
        world.set(entity, Settable);
    }
    if let Some(block) = acc.getter().and_then(|g| g.code_block()) {
        world.set(entity, Body(lower::lower_body(block.syntax(), file_id)));
        world.set(entity, Valued(block.syntax().clone()));
    }
    let index_params = |world: &World| {
        world
            .get::<Callable>(entity)
            .map(|c| c.params.clone())
            .unwrap_or_default()
    };

    // Setter: `[index_params..., newValue]`, receiver upgraded to Mutating
    // (setters mutate self's backing storage); None for static.
    if has_setter
        && let Some(clause) = acc.setter()
        && let Some(body) = clause.code_block()
    {
        let mut params = index_params(world);
        params.push(AstParam {
            label: None,
            name: "newValue".into(),
            ty: world.get::<TypeAnnotation>(entity).map(|t| t.0.clone()),
            default_entity: None,
            pattern: None,
            is_mut: false,
            is_consuming: false,
        });
        spawn_setter(
            world,
            entity,
            clause.syntax(),
            body.syntax(),
            params,
            has_receiver.then_some(ReceiverKind::Mutating),
            file_entity,
            file_id,
            is_static,
        );
    }

    // Place accessors (stage 1.5): `ref { … }` is the read provider,
    // `mutating ref { … }` the write provider (→ Settable, so assignment
    // checks accept it). Each spawns a RefAccessor child whose params are
    // the index params — no `newValue`.
    let ref_clauses = [
        acc.ref_clause()
            .and_then(|c| Some((c.syntax().clone(), c.code_block()?, false))),
        acc.mutating_ref_clause()
            .and_then(|c| Some((c.syntax().clone(), c.code_block()?, true))),
    ];
    for (clause, body, mutating) in ref_clauses.into_iter().flatten() {
        if mutating {
            world.set(entity, Settable);
        }
        let accessor_receiver = has_receiver.then_some(if mutating {
            ReceiverKind::Mutating
        } else {
            ReceiverKind::Borrowing
        });
        let params = index_params(world);
        spawn_ref_accessor(
            world,
            entity,
            &clause,
            body.syntax(),
            params,
            accessor_receiver,
            mutating,
            file_entity,
            file_id,
            is_static,
        );
    }
}
