//! Protocol member discovery queries.
//!
//! Single source of truth for "what methods / associated types belong to
//! this protocol." Unifies the traversal of direct children, extension
//! defaults, and parent-protocol inheritance so consumers (witness table
//! generation, name resolution) don't reassemble the walk themselves.
//!
//! Members are returned in a stable order: direct children of the
//! starting protocol first, then its extension defaults, then parent
//! protocols (in `ConformingProtocols` order) applying the same rule.
//! Callers that use insert-overwrite semantics (e.g. `IndexMap::insert`)
//! get the expected "extensions override, but direct declarations come
//! first" behavior — the direct declaration is inserted first and then
//! overwritten by a later entry only if one exists.

use kestrel_ast_builder::{Callable, Gettable, NodeKind, QualifiedTarget};
use kestrel_hecs::{Entity, QueryContext, QueryFn};

use crate::conformances::ConformingProtocols;
use crate::extensions::ExtensionsFor;
use crate::helpers::filter_members_by_name;

/// A member discovered via protocol traversal, with provenance info.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProtocolMember {
    /// The member entity (function, init, subscript, or field).
    pub entity: Entity,
    /// The protocol this member satisfies — equal to the queried
    /// protocol for direct/extension members, a parent protocol for
    /// inherited ones.
    pub declaring_protocol: Entity,
    /// The extension providing this member as a default implementation,
    /// if any. `None` for members declared directly on the protocol.
    pub extension: Option<Entity>,
}

// ===== ProtocolMembers =====

/// Query: all callable / gettable members reachable from a protocol.
///
/// Collects functions, inits, subscripts, and property requirements
/// (fields) from:
/// 1. Direct children of the protocol
/// 2. Children of extensions targeting the protocol
/// 3. Each parent protocol (via `ConformingProtocols`), applying the
///    same rules
///
/// Not visibility-filtered — witnesses dispatch private methods too.
/// Use `ProtocolMembersByName` for the filtered variant.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ProtocolMembers {
    pub protocol: Entity,
    pub root: Entity,
}

impl QueryFn for ProtocolMembers {
    type Output = Vec<ProtocolMember>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Vec<ProtocolMember> {
        collect_with_filter(ctx, self.protocol, self.root, is_method)
    }
}

// ===== ProtocolAssociatedTypes =====

/// Query: all associated types reachable from a protocol.
///
/// Same traversal as `ProtocolMembers` but filters to `NodeKind::TypeAlias`
/// children. Used by witness table binding and type-inference associated
/// type resolution.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ProtocolAssociatedTypes {
    pub protocol: Entity,
    pub root: Entity,
}

impl QueryFn for ProtocolAssociatedTypes {
    type Output = Vec<ProtocolMember>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Vec<ProtocolMember> {
        collect_with_filter(ctx, self.protocol, self.root, is_associated_type)
    }
}

// ===== ProtocolMembersByName =====

/// Query: members of a protocol with a given name, visible from `context`.
///
/// Composes `ProtocolMembers` with a name filter and
/// `IsVisibleFrom` check. The typical replacement for the
/// `ExtensionsFor + VisibleChildrenByName` pattern in name resolution.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ProtocolMembersByName {
    pub protocol: Entity,
    pub name: String,
    pub context: Entity,
    pub root: Entity,
}

impl QueryFn for ProtocolMembersByName {
    type Output = Vec<ProtocolMember>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Vec<ProtocolMember> {
        let all = ctx.query(ProtocolMembers {
            protocol: self.protocol,
            root: self.root,
        });
        filter_members_by_name(ctx, all, &self.name, self.context, |m| m.entity)
    }
}

// ===== Internal =====

/// Shared traversal: walk the queried protocol and its transitive
/// ancestors (via `ConformingProtocols`), emitting children that pass
/// `filter` from both the protocol itself and its extensions.
fn collect_with_filter(
    ctx: &QueryContext<'_>,
    protocol: Entity,
    root: Entity,
    filter: fn(&QueryContext<'_>, Entity) -> bool,
) -> Vec<ProtocolMember> {
    let mut out = Vec::new();

    // Start with the protocol itself, then its ancestors. ConformingProtocols
    // already deduplicates and expands through protocol-inheritance plus
    // extension-added conformances transitively.
    let mut protocols = vec![protocol];
    protocols.extend(ctx.query(ConformingProtocols {
        entity: protocol,
        root,
    }));

    for proto in protocols {
        // Direct children first — keeps "protocol declaration wins" ordering
        // when a consumer uses insert-overwrite semantics on the same name.
        for &child in ctx.children_of(proto) {
            if filter(ctx, child) {
                out.push(ProtocolMember {
                    entity: child,
                    declaring_protocol: proto,
                    extension: None,
                });
            }
        }

        // Extension defaults on this protocol.
        let extensions = ctx.query(ExtensionsFor {
            target: proto,
            root,
        });
        for ext in extensions {
            for &child in ctx.children_of(ext) {
                if filter(ctx, child) {
                    out.push(ProtocolMember {
                        entity: child,
                        declaring_protocol: proto,
                        extension: Some(ext),
                    });
                }
            }
        }
    }

    out
}

/// Include entities that can be invoked or read: functions, inits,
/// subscripts, and property requirements.
fn is_method(ctx: &QueryContext<'_>, entity: Entity) -> bool {
    ctx.get::<Callable>(entity).is_some() || ctx.get::<Gettable>(entity).is_some()
}

/// Include entities that declare an unqualified associated type.
///
/// Excludes qualified forms like `type Equal.Output = Bool`, which bind the
/// Output of a *specific* protocol rather than acting as a generic associated
/// type. Treating those as regular associated types leaks the concrete
/// binding into unrelated `T.Output` lookups.
fn is_associated_type(ctx: &QueryContext<'_>, entity: Entity) -> bool {
    ctx.get::<NodeKind>(entity) == Some(&NodeKind::TypeAlias)
        && ctx.get::<QualifiedTarget>(entity).is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast::{AstType, PathSegment};
    use kestrel_ast_builder::{
        AstParam, Callable, ConformanceItem, Conformances, ExtensionTarget, Gettable, Name,
        NodeKind, TypeAnnotation, Typed, Vis,
    };
    use kestrel_hecs::World;
    use kestrel_span::Span;
    use kestrel_syntax_tree::{GreenNodeBuilder, SyntaxKind, SyntaxNode};

    fn span() -> Span {
        Span::synthetic(0)
    }

    fn fake_syntax() -> SyntaxNode {
        let mut builder = GreenNodeBuilder::new();
        builder.start_node(SyntaxKind::Root.into());
        builder.finish_node();
        SyntaxNode::new_root(builder.finish())
    }

    fn named_ast(name: &str) -> AstType {
        AstType::Named {
            segments: vec![PathSegment {
                name: name.into(),
                type_args: vec![],
                span: span(),
            }],
            span: span(),
        }
    }

    fn spawn_module(world: &mut World, parent: Option<Entity>, name: &str) -> Entity {
        let e = world.spawn();
        world.set(e, NodeKind::Module);
        world.set(e, Name(name.into()));
        if let Some(p) = parent {
            world.set_parent(e, p);
        }
        e
    }

    fn spawn_protocol(world: &mut World, parent: Entity, name: &str) -> Entity {
        let e = world.spawn();
        world.set(e, NodeKind::Protocol);
        world.set(e, Name(name.into()));
        world.set(e, Typed);
        world.set_parent(e, parent);
        e
    }

    fn spawn_method(world: &mut World, parent: Entity, name: &str) -> Entity {
        let e = world.spawn();
        world.set(e, NodeKind::Function);
        world.set(e, Name(name.into()));
        world.set(e, Vis::Public);
        world.set(
            e,
            Callable {
                params: Vec::<AstParam>::new(),
                receiver: None,
            },
        );
        world.set_parent(e, parent);
        e
    }

    fn spawn_assoc_type(world: &mut World, parent: Entity, name: &str) -> Entity {
        let e = world.spawn();
        world.set(e, NodeKind::TypeAlias);
        world.set(e, Name(name.into()));
        world.set(e, Typed);
        world.set_parent(e, parent);
        e
    }

    fn spawn_property(world: &mut World, parent: Entity, name: &str) -> Entity {
        let e = world.spawn();
        world.set(e, NodeKind::Field);
        world.set(e, Name(name.into()));
        world.set(e, Gettable);
        world.set(e, TypeAnnotation(named_ast("Int64")));
        world.set_parent(e, parent);
        e
    }

    fn spawn_extension_of_protocol(world: &mut World, parent: Entity, target_name: &str) -> Entity {
        let e = world.spawn();
        world.set(e, NodeKind::Extension);
        world.set(e, ExtensionTarget(named_ast(target_name)));
        world.set_parent(e, parent);
        e
    }

    #[test]
    fn direct_methods_only() {
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, "<root>");

        let proto = spawn_protocol(&mut world, root, "Proto");
        let greet = spawn_method(&mut world, proto, "greet");

        let ctx = world.query_context();
        let members = ctx.query(ProtocolMembers {
            protocol: proto,
            root,
        });
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].entity, greet);
        assert_eq!(members[0].declaring_protocol, proto);
        assert_eq!(members[0].extension, None);
    }

    #[test]
    fn extension_default_method_is_discovered() {
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, "<root>");

        let proto = spawn_protocol(&mut world, root, "Proto");
        let greet = spawn_method(&mut world, proto, "greet");

        let ext = spawn_extension_of_protocol(&mut world, root, "Proto");
        let shout = spawn_method(&mut world, ext, "shout");

        let ctx = world.query_context();
        let members = ctx.query(ProtocolMembers {
            protocol: proto,
            root,
        });
        assert_eq!(members.len(), 2, "expected direct + extension method");

        // Direct first, extension second (ordering guarantee)
        assert_eq!(members[0].entity, greet);
        assert_eq!(members[0].extension, None);
        assert_eq!(members[1].entity, shout);
        assert_eq!(members[1].extension, Some(ext));
        assert_eq!(members[1].declaring_protocol, proto);
    }

    #[test]
    fn inherited_protocol_methods_are_discovered() {
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, "<root>");

        let base = spawn_protocol(&mut world, root, "Base");
        let base_m = spawn_method(&mut world, base, "baseMethod");

        let derived = spawn_protocol(&mut world, root, "Derived");
        world.set(
            derived,
            Conformances(vec![ConformanceItem::Positive(
                named_ast("Base"),
                fake_syntax(),
            )]),
        );
        let derived_m = spawn_method(&mut world, derived, "derivedMethod");

        let ctx = world.query_context();
        let members = ctx.query(ProtocolMembers {
            protocol: derived,
            root,
        });
        // Derived's method first, then inherited Base's method.
        let entities: Vec<Entity> = members.iter().map(|m| m.entity).collect();
        assert!(entities.contains(&derived_m));
        assert!(entities.contains(&base_m));

        let base_entry = members.iter().find(|m| m.entity == base_m).unwrap();
        assert_eq!(base_entry.declaring_protocol, base);
        assert_eq!(base_entry.extension, None);
    }

    #[test]
    fn extension_on_parent_protocol_is_discovered() {
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, "<root>");

        let base = spawn_protocol(&mut world, root, "Base");
        let base_ext = spawn_extension_of_protocol(&mut world, root, "Base");
        let default_m = spawn_method(&mut world, base_ext, "defaultOnBase");

        let derived = spawn_protocol(&mut world, root, "Derived");
        world.set(
            derived,
            Conformances(vec![ConformanceItem::Positive(
                named_ast("Base"),
                fake_syntax(),
            )]),
        );

        let ctx = world.query_context();
        let members = ctx.query(ProtocolMembers {
            protocol: derived,
            root,
        });
        let hit = members
            .iter()
            .find(|m| m.entity == default_m)
            .expect("extension default on parent not found");
        assert_eq!(hit.declaring_protocol, base);
        assert_eq!(hit.extension, Some(base_ext));
    }

    #[test]
    fn qualified_associated_types_are_excluded() {
        // `type Equal.Output = Bool` (qualified to Equal protocol) must not
        // match a generic `Output` lookup on Hash or Addable — qualified
        // bindings are specific to the named protocol.
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, "<root>");

        let proto = spawn_protocol(&mut world, root, "Proto");
        // Unqualified: `type Output`
        let plain_output = spawn_assoc_type(&mut world, proto, "Output");
        // Qualified: `type Equal.Output = Bool`
        let qualified = spawn_assoc_type(&mut world, proto, "Output");
        world.set(
            qualified,
            kestrel_ast_builder::QualifiedTarget(named_ast("Equal")),
        );

        let ctx = world.query_context();
        let assoc = ctx.query(ProtocolAssociatedTypes {
            protocol: proto,
            root,
        });
        assert_eq!(assoc.len(), 1, "qualified alias should be excluded");
        assert_eq!(assoc[0].entity, plain_output);
    }

    #[test]
    fn associated_type_query_filters_by_kind() {
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, "<root>");

        let proto = spawn_protocol(&mut world, root, "Iterator");
        let _method = spawn_method(&mut world, proto, "next");
        let item = spawn_assoc_type(&mut world, proto, "Item");

        let ctx = world.query_context();
        let assoc = ctx.query(ProtocolAssociatedTypes {
            protocol: proto,
            root,
        });
        assert_eq!(assoc.len(), 1);
        assert_eq!(assoc[0].entity, item);

        let members = ctx.query(ProtocolMembers {
            protocol: proto,
            root,
        });
        // Methods query excludes the associated type.
        assert!(members.iter().all(|m| m.entity != item));
    }

    #[test]
    fn property_requirement_is_a_member() {
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, "<root>");

        let proto = spawn_protocol(&mut world, root, "HasCount");
        let count = spawn_property(&mut world, proto, "count");

        let ctx = world.query_context();
        let members = ctx.query(ProtocolMembers {
            protocol: proto,
            root,
        });
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].entity, count);
    }

    fn spawn_initializer(world: &mut World, parent: Entity) -> Entity {
        let e = world.spawn();
        world.set(e, NodeKind::Initializer);
        world.set(e, Vis::Public);
        world.set(
            e,
            Callable {
                params: Vec::<AstParam>::new(),
                receiver: None,
            },
        );
        world.set_parent(e, parent);
        e
    }

    fn spawn_subscript(world: &mut World, parent: Entity) -> Entity {
        let e = world.spawn();
        world.set(e, NodeKind::Subscript);
        world.set(e, Vis::Public);
        world.set(
            e,
            Callable {
                params: Vec::<AstParam>::new(),
                receiver: None,
            },
        );
        world.set(e, Gettable);
        world.set(e, kestrel_ast_builder::Subscript);
        world.set_parent(e, parent);
        e
    }

    #[test]
    fn sentinel_init_matches_nameless_initializer() {
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, "<root>");

        let proto = spawn_protocol(&mut world, root, "Proto");
        let init = spawn_initializer(&mut world, proto);
        let _greet = spawn_method(&mut world, proto, "greet");

        let ctx = world.query_context();
        let hits = ctx.query(ProtocolMembersByName {
            protocol: proto,
            name: "init".into(),
            context: root,
            root,
        });
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entity, init);
    }

    #[test]
    fn sentinel_subscript_matches_nameless_subscript() {
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, "<root>");

        let proto = spawn_protocol(&mut world, root, "Proto");
        let sub = spawn_subscript(&mut world, proto);

        let ctx = world.query_context();
        let hits = ctx.query(ProtocolMembersByName {
            protocol: proto,
            name: "subscript".into(),
            context: root,
            root,
        });
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entity, sub);
    }

    #[test]
    fn members_by_name_filters_and_includes_extensions() {
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, "<root>");

        let proto = spawn_protocol(&mut world, root, "Proto");
        let _other = spawn_method(&mut world, proto, "other");

        let ext = spawn_extension_of_protocol(&mut world, root, "Proto");
        let shout = spawn_method(&mut world, ext, "shout");

        let ctx = world.query_context();
        let hits = ctx.query(ProtocolMembersByName {
            protocol: proto,
            name: "shout".into(),
            context: root,
            root,
        });
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entity, shout);
        assert_eq!(hits[0].extension, Some(ext));
    }
}
