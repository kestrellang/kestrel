//! Field declaration builder.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::SyntaxNodePtr;
use kestrel_syntax_tree::ast::{self, AstNode, HasName, HasStatic};
use kestrel_syntax_tree::utils::get_decl_span;

use super::helpers::*;
use crate::ast_type::lower_opt_type;
use crate::components::*;
use crate::lower;

/// Build a field declaration entity from CST.
///
/// Components: NodeKind::Field, Name, FileId, Vis, TypeAnnotation,
/// Gettable, [Settable], [Valued (init expr)], [Static],
/// [Attributes], [Documentation]
///
/// Fields declared with `var` are Settable. Fields with `let` are read-only.
/// Computed properties (with get/set accessors) set Gettable/Settable
/// based on which accessors are present.
pub fn build_field(
    world: &mut World,
    node: &ast::FieldDeclaration,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) {
    let syntax = node.syntax();
    let entity = world.spawn();

    world.set(entity, NodeKind::Field);
    world.set(entity, FileId(file_entity));
    world.set(entity, DeclSpan(get_decl_span(syntax, file_id)));
    world.set(entity, CstNode(SyntaxNodePtr::new(&syntax)));
    world.set_parent(entity, parent);

    if let Some(name) = node.name_text() {
        world.set(entity, Name(name));
    }
    if let Some(ty) = lower_opt_type(node.ty(), file_id) {
        world.set(entity, TypeAnnotation(ty));
    }

    // Mutability as a component, so downstream analyzers don't re-scan tokens.
    let is_var = node.var_token().is_some();
    world.set(
        entity,
        if is_var {
            FieldMutability::Var
        } else {
            FieldMutability::Let
        },
    );

    let is_static = node.is_static();
    let accessors = node.property_accessors();
    match &accessors {
        Some(accessors) => {
            world.set(entity, Computed);
            build_accessors(
                world,
                entity,
                parent,
                accessors,
                file_entity,
                file_id,
                is_static,
            );
        },
        None => {
            // Stored property: always Gettable
            world.set(entity, Gettable);
            if is_var {
                world.set(entity, Settable);
            }
            // `= expr` initializer
            if let Some(init) = node.expression() {
                world.set(
                    entity,
                    Body(lower::lower_default_value_expr(init.syntax(), file_id)),
                );
                world.set(entity, Valued(SyntaxNodePtr::new(&init.syntax())));
            }
        },
    }

    if is_static {
        world.set(entity, Static);
    }

    // Storage classification. Computed here, where the accessor CST is already
    // in hand, and stored as a component so no downstream site reconstructs it
    // from the absence of Computed/Callable/Static — see `FieldClass`.
    let owner = match world.get::<NodeKind>(parent) {
        Some(NodeKind::Struct | NodeKind::Enum) => FieldOwner::Nominal,
        Some(NodeKind::Protocol) => FieldOwner::Protocol,
        Some(NodeKind::Extension) => FieldOwner::Extension,
        _ => FieldOwner::Module,
    };
    let backing = match &accessors {
        Some(accessors) if accessors.has_body() => FieldBacking::Computed,
        _ => FieldBacking::Stored,
    };
    world.set(
        entity,
        FieldClass {
            backing,
            owner,
            is_static,
        },
    );

    set_visibility(world, entity, node);
    set_attributes(world, entity, node, file_id);
    set_documentation(world, entity, syntax);
}

/// A computed property's accessors: Gettable/Settable, the getter body,
/// and a child entity per setter / place accessor.
fn build_accessors(
    world: &mut World,
    entity: Entity,
    parent: Entity,
    accessors: &ast::PropertyAccessors,
    file_entity: Entity,
    file_id: usize,
    is_static: bool,
) {
    // Place accessors (stage 1.5): `ref` is a read provider (Gettable),
    // `mutating ref` a write provider (Settable).
    let ref_clause = accessors.ref_clause();
    let mutating_ref_clause = accessors.mutating_ref_clause();
    let has_setter = accessors.declares_set();
    if accessors.declares_get() || ref_clause.is_some() {
        world.set(entity, Gettable);
    }
    if has_setter || mutating_ref_clause.is_some() {
        world.set(entity, Settable);
    }

    // Instance computed properties access `self` via a borrowing receiver;
    // `static` fields and module-level computed globals have no receiver
    // (the latter have no parent type to bind `self` to).
    let has_receiver = !is_static
        && world
            .get::<NodeKind>(parent)
            .is_some_and(NodeKind::is_type_scope);
    let receiver = has_receiver.then_some(ReceiverKind::Borrowing);
    let getter_callable = || Callable {
        params: Vec::new(),
        receiver: receiver.clone(),
    };

    if let Some(getter) = accessors.getter() {
        if let Some(body) = getter.code_block() {
            world.set(entity, Body(lower::lower_body(body.syntax(), file_id)));
            world.set(entity, Valued(SyntaxNodePtr::new(&body.syntax())));
            world.set(entity, getter_callable());
        }
    } else if let Some(body) = accessors.code_block() {
        // Shorthand computed property `var foo: Type { expr }`: an implicit getter.
        world.set(entity, Gettable);
        world.set(entity, Body(lower::lower_body(body.syntax(), file_id)));
        world.set(entity, Valued(SyntaxNodePtr::new(&body.syntax())));
        world.set(entity, getter_callable());
    } else if ref_clause.is_some() || mutating_ref_clause.is_some() {
        // Pure-ref member (`{ ref {…} }`, no getter): the parent stays
        // bodyless — reads route to the RefAccessor child — but still
        // needs a Callable so member resolution sees the signature.
        world.set(entity, getter_callable());
    }

    // Setter: a child entity with its own Callable + Body. `newValue` is an
    // implicit parameter typed as the field's type. Instance setters are
    // Mutating (they write self's backing storage); static/global setters
    // have no receiver.
    if has_setter
        && let Some(clause) = accessors.setter()
        && let Some(body) = clause.code_block()
    {
        let params = vec![AstParam {
            label: None,
            name: "newValue".into(),
            ty: world.get::<TypeAnnotation>(entity).map(|t| t.0.clone()),
            default_entity: None,
            pattern: None,
            is_mut: false,
            is_consuming: false,
        }];
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

    // Place accessors: a RefAccessor child per clause. Field accessors take
    // no params (no index, no newValue).
    let ref_clauses = [
        ref_clause.and_then(|c| Some((c.syntax().clone(), c.code_block()?, false))),
        mutating_ref_clause.and_then(|c| Some((c.syntax().clone(), c.code_block()?, true))),
    ];
    for (clause, body, mutating) in ref_clauses.into_iter().flatten() {
        let accessor_receiver = has_receiver.then_some(if mutating {
            ReceiverKind::Mutating
        } else {
            ReceiverKind::Borrowing
        });
        spawn_ref_accessor(
            world,
            entity,
            &clause,
            body.syntax(),
            Vec::new(),
            accessor_receiver,
            mutating,
            file_entity,
            file_id,
            is_static,
        );
    }
}
