//! Type member discovery queries.
//!
//! Single source of truth for "what members does this type have?" Walks
//! the type's direct children, extensions targeting the type, and
//! extensions targeting protocols the type conforms to (transitively).
//!
//! Where-clause entailment for protocol-extension members is the
//! caller's job — this query returns every candidate so it can be
//! shared across consumers with different scoping needs (call site,
//! conformance check, witness binding). For entailment, see
//! `kestrel_type_infer::entailment::constraint_entailed_by`.
//!
//! Members are returned in order: direct children first, then
//! type extensions, then conformed-protocol extensions. Callers using
//! insert-overwrite semantics get "type declaration wins."

use std::sync::Arc;

use kestrel_ast_builder::{Callable, Gettable, NodeKind};
use kestrel_hecs::{Entity, QueryContext, QueryFn};

use crate::helpers::filter_members_by_name;
use crate::traversal::{MemberMap, collect_members_transitive};

/// A member discovered via type traversal, with provenance info.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeMember {
    /// The member entity (function, init, subscript, field, or type alias).
    pub entity: Entity,
    /// Where this member was discovered.
    pub source: TypeMemberSource,
}

/// Provenance of a discovered member.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeMemberSource {
    /// Direct child of the queried type.
    Direct,
    /// Child of an extension targeting the queried type.
    Extension(Entity),
    /// Child of an extension targeting a protocol the type conforms to.
    /// Carries both the protocol and the extension entity so callers can
    /// check entailment of the extension's where clauses.
    ProtocolExtension { protocol: Entity, extension: Entity },
}

// ===== TypeMembers =====

/// Name-indexed map of every member discoverable on a type. See
/// `MemberMap` for the order/precedence guarantees.
pub type TypeMemberMap = MemberMap<TypeMember>;

/// Query: every member discoverable on a type.
///
/// Walks (in order):
/// 1. Direct children of `type_entity`
/// 2. Children of every extension targeting `type_entity`
/// 3. For every protocol `type_entity` transitively conforms to, children
///    of every extension targeting that protocol
///
/// Includes functions, inits, subscripts, fields/properties, and
/// associated types. Not name-filtered, not visibility-filtered, no
/// where-clause entailment — it's the union of every candidate.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TypeMembers {
    pub type_entity: Entity,
    pub root: Entity,
}

impl QueryFn for TypeMembers {
    type Output = Arc<TypeMemberMap>;

    fn describe(&self) -> String {
        format!("TypeMembers({:?})", self.type_entity)
    }

    fn execute(&self, ctx: &QueryContext<'_>) -> Arc<TypeMemberMap> {
        // Types never inherit a protocol's *direct* children — only
        // protocol-extension members surface on conforming types.
        let members = collect_members_transitive(
            ctx,
            self.type_entity,
            self.root,
            false,
            is_member,
            |entity, via_protocol, extension| TypeMember {
                entity,
                source: match (via_protocol, extension) {
                    (None, None) => TypeMemberSource::Direct,
                    (None, Some(ext)) => TypeMemberSource::Extension(ext),
                    (Some(protocol), Some(extension)) => TypeMemberSource::ProtocolExtension {
                        protocol,
                        extension,
                    },
                    (Some(_), None) => {
                        unreachable!("parent direct children disabled for TypeMembers")
                    },
                },
            },
        );
        Arc::new(TypeMemberMap::build(ctx, members, |m| m.entity))
    }
}

// ===== TypeMembersByName =====

/// Query: members of a type with a given name, visible from `context`.
///
/// Composes `TypeMembers` with a name filter and `IsVisibleFrom` check.
/// Recognizes the keyword sentinels `"init"` (→ Initializer NodeKind)
/// and `"subscript"` (→ Subscript marker) for nameless callables.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TypeMembersByName {
    pub type_entity: Entity,
    pub name: String,
    pub context: Entity,
    pub root: Entity,
}

impl QueryFn for TypeMembersByName {
    type Output = Vec<TypeMember>;

    fn describe(&self) -> String {
        format!("TypeMembersByName({:?}, {:?})", self.type_entity, self.name)
    }

    fn execute(&self, ctx: &QueryContext<'_>) -> Vec<TypeMember> {
        let map = ctx.query(TypeMembers {
            type_entity: self.type_entity,
            root: self.root,
        });
        // Bucket lookup replaces the full scan; the visibility check stays
        // here because it's per-`context` and can't be baked into the map.
        let bucket: Vec<TypeMember> = map.named(&self.name).cloned().collect();
        filter_members_by_name(ctx, bucket, &self.name, self.context, |m| m.entity)
    }
}

// ===== Internal =====

/// A "member" is anything a type can be queried for: methods (Callable),
/// fields/properties (Gettable), type aliases (including qualified forms
/// like `type Equal.Output = Bool`, which bind an associated type for a
/// specific protocol the type conforms to), or enum cases (`.None`-style
/// implicit member resolution depends on these surfacing).
fn is_member(ctx: &QueryContext<'_>, entity: Entity) -> bool {
    if ctx.get::<Callable>(entity).is_some() || ctx.get::<Gettable>(entity).is_some() {
        return true;
    }
    matches!(
        ctx.get::<NodeKind>(entity),
        Some(NodeKind::TypeAlias | NodeKind::EnumCase)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast::{AstType, PathSegment};
    use kestrel_ast_builder::{
        AstParam, ConformanceItem, Conformances, ExtensionTarget, Name, NodeKind, TypeAnnotation,
        Typed, Vis,
    };
    use kestrel_hecs::World;
    use kestrel_span::Span;

    fn span() -> Span {
        Span::synthetic(0)
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

    fn spawn_struct(world: &mut World, parent: Entity, name: &str) -> Entity {
        let e = world.spawn();
        world.set(e, NodeKind::Struct);
        world.set(e, Name(name.into()));
        world.set(e, Vis::Public);
        world.set(e, Typed);
        world.set_parent(e, parent);
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

    fn spawn_field(world: &mut World, parent: Entity, name: &str) -> Entity {
        let e = world.spawn();
        world.set(e, NodeKind::Field);
        world.set(e, Name(name.into()));
        world.set(e, Vis::Public);
        world.set(e, Gettable);
        world.set(e, TypeAnnotation(named_ast("Int64")));
        world.set_parent(e, parent);
        e
    }

    fn spawn_extension(world: &mut World, parent: Entity, target_name: &str) -> Entity {
        let e = world.spawn();
        world.set(e, NodeKind::Extension);
        world.set(e, ExtensionTarget(named_ast(target_name)));
        world.set_parent(e, parent);
        e
    }

    #[test]
    fn direct_members_only() {
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, "<root>");
        let s = spawn_struct(&mut world, root, "S");
        let m = spawn_method(&mut world, s, "m");
        let f = spawn_field(&mut world, s, "f");

        let ctx = world.query_context();
        let members = ctx.query(TypeMembers {
            type_entity: s,
            root,
        });
        let entities: Vec<Entity> = members.iter().map(|tm| tm.entity).collect();
        assert!(entities.contains(&m));
        assert!(entities.contains(&f));
        assert!(
            members
                .iter()
                .all(|tm| tm.source == TypeMemberSource::Direct)
        );
    }

    #[test]
    fn extension_members_are_discovered() {
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, "<root>");
        let s = spawn_struct(&mut world, root, "S");
        spawn_method(&mut world, s, "direct");
        let ext = spawn_extension(&mut world, root, "S");
        let ext_method = spawn_method(&mut world, ext, "extended");

        let ctx = world.query_context();
        let members = ctx.query(TypeMembers {
            type_entity: s,
            root,
        });
        let hit = members
            .iter()
            .find(|tm| tm.entity == ext_method)
            .expect("extension method not found");
        assert_eq!(hit.source, TypeMemberSource::Extension(ext));
    }

    #[test]
    fn conformed_protocol_extension_members_are_discovered() {
        // S: P, and `extend P { method }` should surface `method` on S.
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, "<root>");
        let proto = spawn_protocol(&mut world, root, "P");
        let s = spawn_struct(&mut world, root, "S");
        world.set(
            s,
            Conformances(vec![ConformanceItem::Positive(
                named_ast("P"),
                kestrel_syntax_tree::SyntaxNode::new_root({
                    let mut b = kestrel_syntax_tree::GreenNodeBuilder::new();
                    b.start_node(kestrel_syntax_tree::SyntaxKind::Root.into());
                    b.finish_node();
                    b.finish()
                }),
            )]),
        );

        let proto_ext = spawn_extension(&mut world, root, "P");
        let proto_method = spawn_method(&mut world, proto_ext, "fromExt");

        let ctx = world.query_context();
        let members = ctx.query(TypeMembers {
            type_entity: s,
            root,
        });
        let hit = members
            .iter()
            .find(|tm| tm.entity == proto_method)
            .expect("conformed-protocol extension method not surfaced on S");
        assert_eq!(
            hit.source,
            TypeMemberSource::ProtocolExtension {
                protocol: proto,
                extension: proto_ext,
            }
        );
    }

    #[test]
    fn members_by_name_filters_and_includes_visibility() {
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, "<root>");
        let s = spawn_struct(&mut world, root, "S");
        let m1 = spawn_method(&mut world, s, "wanted");
        spawn_method(&mut world, s, "other");

        let ctx = world.query_context();
        let hits = ctx.query(TypeMembersByName {
            type_entity: s,
            name: "wanted".into(),
            context: root,
            root,
        });
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entity, m1);
    }
}
