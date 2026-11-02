//! Field declaration builder.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::SyntaxNodePtr;
use kestrel_syntax_tree::ast::{self, AstNode, HasName, HasStatic};
use kestrel_syntax_tree::utils::get_decl_span;

use super::helpers::*;
use crate::ast_type::lower_opt_type;
use crate::components::*;

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
    world.set(entity, CstNode(SyntaxNodePtr::new(syntax)));
    world.set_parent(entity, parent);

    if let Some(name) = node.name_text() {
        world.set(entity, Name(name));
    }
    if let Some(ty) = lower_opt_type(node.ty(), file_id) {
        world.set(entity, TypeAnnotation(ty));
    }
    if let Some(marker) = node.failable_marker() {
        mark_failable(world, entity, get_decl_span(marker.syntax(), file_id));
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
    // An accessor block of only `willSet` / `didSet` observes a STORED
    // property: no Computed marker, the storage and initializer stay.
    let accessors = node
        .property_accessors()
        .filter(|acc| !is_observer_only(&declared_accessors(acc), acc));
    let observers = node.property_accessors().filter(|_| accessors.is_none());
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
                world.set(entity, Valued(SyntaxNodePtr::new(init.syntax())));
            }
            if let Some(observers) = &observers {
                world.set(entity, declared_accessors(observers));
                // A `let` is never written after initialization: nothing to
                // observe (E638 reports it), so no setter.
                if is_var {
                    build_observers(
                        world,
                        entity,
                        parent,
                        observers,
                        file_entity,
                        file_id,
                        is_static,
                    );
                }
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
    // Place accessors (stage 1.5): `ref` / `modifying ref` are read
    // providers (Gettable), `mutating ref` a write provider (Settable).
    let declared = declared_accessors(accessors);
    world.set(entity, declared);
    let ref_clause = accessors.ref_clause();
    let modifying_ref_clause = accessors.modifying_ref_clause();
    let mutating_ref_clause = accessors.mutating_ref_clause();
    let has_setter = accessors.declares_set();
    if declared.read_providers() > 0 {
        world.set(entity, Gettable);
    }
    if has_setter || mutating_ref_clause.is_some() {
        world.set(entity, Settable);
    }

    // Instance computed properties access `self` via a borrowing receiver
    // (a `modifying` read provider via a mutable one); `static` fields and
    // module-level computed globals have no receiver (the latter have no
    // parent type to bind `self` to).
    let has_receiver = !is_static
        && world
            .get::<NodeKind>(parent)
            .is_some_and(NodeKind::is_type_scope);
    let reads_mutate = has_receiver && declared.any_modifying() && !declared.get && !declared.ref_;
    if reads_mutate {
        world.set(entity, MutatingRead);
    }
    let getter_receiver = has_receiver.then_some(if reads_mutate && declared.modifying_get {
        ReceiverKind::Mutating
    } else {
        ReceiverKind::Borrowing
    });
    let getter_callable = || Callable {
        params: Vec::new(),
        receiver: getter_receiver.clone(),
    };

    let getter_body = accessors
        .getter()
        .and_then(|g| g.code_block())
        .or_else(|| accessors.modifying_getter().and_then(|g| g.code_block()));
    if let Some(body) = getter_body {
        world.set(entity, Valued(SyntaxNodePtr::new(body.syntax())));
        world.set(entity, getter_callable());
    } else if accessors.getter().is_some() || accessors.modifying_getter().is_some() {
        // A bodyless clause keeps the signature visible to member resolution.
        world.set(entity, getter_callable());
    } else if let Some(body) = accessors.code_block() {
        // Shorthand computed property `var foo: Type { expr }`: an implicit getter.
        world.set(entity, Valued(SyntaxNodePtr::new(body.syntax())));
        world.set(entity, getter_callable());
    } else if ref_clause.is_some()
        || modifying_ref_clause.is_some()
        || mutating_ref_clause.is_some()
    {
        // Pure-ref member (`{ ref {…} }`, no getter): the parent stays
        // bodyless — reads route to the RefAccessor child — but still
        // needs a Callable so member resolution sees the signature.
        world.set(entity, getter_callable());
    }

    // Setter: a child entity with its own Callable + body. `newValue` is an
    // implicit parameter typed as the field's type. Instance setters are
    // Mutating (they write self's backing storage); static/global setters
    // have no receiver.
    if has_setter
        && let Some(clause) = accessors.setter()
        && let Some(body) = clause.code_block()
    {
        let params = vec![AstParam::new_value(
            world.get::<TypeAnnotation>(entity).map(|t| t.0.clone()),
        )];
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
    // no params (no index, no newValue). `(yields_mutating, receiver_mutating)`.
    let ref_clauses = [
        ref_clause.and_then(|c| Some((c.syntax().clone(), c.code_block()?, false, false))),
        modifying_ref_clause.and_then(|c| Some((c.syntax().clone(), c.code_block()?, false, true))),
        mutating_ref_clause.and_then(|c| Some((c.syntax().clone(), c.code_block()?, true, true))),
    ];
    for (clause, body, mutating, receiver_mutating) in ref_clauses.into_iter().flatten() {
        let accessor_receiver = has_receiver.then_some(if receiver_mutating {
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

/// An accessor block spelling only `willSet` / `didSet` (no read or write
/// provider, no `{ expr }` shorthand body).
fn is_observer_only(declared: &DeclaredAccessors, acc: &ast::PropertyAccessors) -> bool {
    declared.any_observer()
        && declared.read_providers() == 0
        && !declared.set
        && !declared.mutating_ref
        && acc.code_block().is_none()
}

/// An observed stored `var`: spawn the synthesized [`ObserverSetter`] (`newValue` param, mutating receiver) whose
/// HIR body hir-lower builds from the observer clauses. Observers on statics
/// and globals are rejected by the analyzer (E638), so only instance members
/// get a receiver.
fn build_observers(
    world: &mut World,
    entity: Entity,
    parent: Entity,
    observers: &ast::PropertyAccessors,
    file_entity: Entity,
    file_id: usize,
    is_static: bool,
) {
    let has_receiver = !is_static
        && world
            .get::<NodeKind>(parent)
            .is_some_and(NodeKind::is_type_scope);
    let params = vec![AstParam::new_value(
        world.get::<TypeAnnotation>(entity).map(|t| t.0.clone()),
    )];
    let setter = spawn_setter(
        world,
        entity,
        observers.syntax(),
        observers.syntax(),
        params,
        has_receiver.then_some(ReceiverKind::Mutating),
        file_entity,
        file_id,
        is_static,
    );
    world.set(setter, ObserverSetter);
}
