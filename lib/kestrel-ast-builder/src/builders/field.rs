//! Field declaration builder.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::SyntaxNodePtr;
use kestrel_syntax_tree::ast::{self, AstNode, HasName, HasStatic, HasUnsafe};
use kestrel_syntax_tree::utils::get_decl_span;

use super::helpers::*;
use crate::AstType;
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
        .filter(|acc| !acc.is_observer_only());
    let observers = node.property_accessors().filter(|_| accessors.is_none());
    let owner = match world.get::<NodeKind>(parent) {
        Some(NodeKind::Struct | NodeKind::Enum) => FieldOwner::Nominal,
        Some(NodeKind::Protocol) => FieldOwner::Protocol,
        Some(NodeKind::Extension) => FieldOwner::Extension,
        _ => FieldOwner::Module,
    };
    // A computed member declared `T throws E` is a throwing place.
    if accessors.is_some() && owner != FieldOwner::Protocol {
        mark_throwing(world, entity);
    }
    let lazy_init = node.expression().filter(|_| {
        node.is_lazy()
            && is_var
            && !is_static
            && owner == FieldOwner::Nominal
            && node.property_accessors().is_none()
            && node.failable_marker().is_none()
            && world.has::<TypeAnnotation>(entity)
    });
    if node.is_lazy() {
        let cache = lazy_init
            .as_ref()
            .map(|init| build_lazy(world, entity, parent, node, init, file_entity, file_id));
        world.set(entity, Lazy { cache });
    }
    match &accessors {
        _ if lazy_init.is_some() => {},
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
    let backing = match &accessors {
        _ if lazy_init.is_some() => FieldBacking::Computed,
        Some(accessors) if accessors.has_body() => FieldBacking::Computed,
        _ => FieldBacking::Stored,
    };
    world.set(
        entity,
        FieldClass {
            backing,
            owner,
            is_static,
            lazy_cache: false,
        },
    );

    set_visibility(world, entity, node);
    set_attributes(world, entity, node, file_id);
    if node.is_unsafe() {
        world.set(entity, IsUnsafe);
    }
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
        ref_clause.map(|c| (c.syntax().clone(), c.code_block(), false, false)),
        modifying_ref_clause.map(|c| (c.syntax().clone(), c.code_block(), false, true)),
        mutating_ref_clause.map(|c| (c.syntax().clone(), c.code_block(), true, true)),
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
            body.as_ref().map(|b| b.syntax()),
            Vec::new(),
            accessor_receiver,
            mutating,
            file_entity,
            file_id,
            is_static,
        );
    }
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

/// A well-formed `lazy var x: T = init`: `x` becomes computed — a
/// synthesized `modifying get` (body built by hir-lower from `init`) and a
/// [`LazySetter`] — over a hidden stored `$x: T?` cache sibling, which is
/// returned. Reading `x` mutates `self` (the first read fills the cache), so
/// reads need a mutable base exactly like any `modifying get`.
fn build_lazy(
    world: &mut World,
    entity: Entity,
    parent: Entity,
    node: &ast::FieldDeclaration,
    init: &ast::Expression,
    file_entity: Entity,
    file_id: usize,
) -> Entity {
    let syntax = node.syntax();
    let ty = world
        .get::<TypeAnnotation>(entity)
        .map(|t| t.0.clone())
        .expect("lazy_init requires a type annotation");
    world.set(entity, Computed);
    world.set(entity, Gettable);
    world.set(entity, Settable);
    world.set(entity, MutatingRead);
    world.set(
        entity,
        DeclaredAccessors {
            modifying_get: true,
            set: true,
            ..DeclaredAccessors::default()
        },
    );
    world.set(
        entity,
        Callable {
            params: Vec::new(),
            receiver: Some(ReceiverKind::Mutating),
        },
    );
    world.set(entity, Valued(SyntaxNodePtr::new(init.syntax())));
    let setter = spawn_setter(
        world,
        entity,
        syntax,
        syntax,
        vec![AstParam::new_value(Some(ty.clone()))],
        Some(ReceiverKind::Mutating),
        file_entity,
        file_id,
        false,
    );
    world.set(setter, LazySetter);

    let span = get_decl_span(syntax, file_id);
    let name = world
        .get::<Name>(entity)
        .map(|n| n.0.clone())
        .unwrap_or_default();
    let cache = world.spawn();
    world.set(cache, NodeKind::Field);
    world.set(cache, Name(format!("${name}")));
    world.set(cache, FileId(file_entity));
    world.set(cache, DeclSpan(span.clone()));
    world.set(cache, CstNode(SyntaxNodePtr::new(syntax)));
    world.set_parent(cache, parent);
    world.set(cache, TypeAnnotation(AstType::Optional(Box::new(ty), span)));
    world.set(cache, FieldMutability::Var);
    world.set(cache, Gettable);
    world.set(cache, Settable);
    world.set(cache, Vis::Private);
    world.set(
        cache,
        FieldClass {
            backing: FieldBacking::Stored,
            owner: FieldOwner::Nominal,
            is_static: false,
            lazy_cache: true,
        },
    );
    cache
}
