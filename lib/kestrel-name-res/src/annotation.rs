//! The two lookups every stage needs when it reads a type annotation without
//! lowering it to HIR: "which type does `Self` mean here?" and "which entity
//! does this annotation name?". Each used to be hand-written per crate, and
//! the copies disagreed (sugar looked up by its plain name, `Self` resolved or
//! not). Keep these the only implementations.

use kestrel_ast::AstType;
use kestrel_ast_builder::NodeKind;
use kestrel_hecs::{Entity, QueryContext, QueryFn};

use crate::extensions::{ExtensionTargetEntity, sugar_builtin};
use crate::resolve_type::{ResolveTypePath, TypeResolution};

/// The type a container declaration stands for: itself for a struct, enum or
/// protocol, the extended type for an extension, `None` for anything else.
pub fn container_type(ctx: &QueryContext<'_>, entity: Entity, root: Entity) -> Option<Entity> {
    match ctx.get::<NodeKind>(entity)? {
        NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol => Some(entity),
        NodeKind::Extension => ctx.query(ExtensionTargetEntity {
            extension: entity,
            root,
        }),
        _ => None,
    }
}

/// Query: the type `Self` names at `context` — the nearest enclosing (or
/// `context` itself) struct, enum or protocol, or an extension's target.
/// `None` outside any type, or when the extension's target doesn't resolve.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SelfTypeEntity {
    pub context: Entity,
    pub root: Entity,
}

impl QueryFn for SelfTypeEntity {
    type Output = Option<Entity>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Entity> {
        let mut current = Some(self.context);
        while let Some(entity) = current {
            // An extension answers even when its target fails to resolve:
            // `Self` is that target, never an outer type.
            if ctx.get::<NodeKind>(entity) == Some(&NodeKind::Extension) {
                return container_type(ctx, entity, self.root);
            }
            if let Some(ty) = container_type(ctx, entity, self.root) {
                return Some(ty);
            }
            current = ctx.parent_of(entity);
        }
        None
    }
}

/// Resolve a type annotation to the entity it names, looked up from
/// `context`. Paths go through `ResolveTypePath`; sugar (`[T]`, `T?`,
/// `[K: V]`, `T throws E`) goes through its lang item, as HIR lowering does,
/// so a user type named `Array` cannot shadow it. Annotations that name no
/// single entity (tuples, functions, `some P`, ...) are `NotFound`. `Self`
/// comes back as `SelfType`; [`annotation_entity`] resolves it.
pub fn resolve_annotation(
    ctx: &QueryContext<'_>,
    ast_ty: &AstType,
    context: Entity,
    root: Entity,
) -> TypeResolution {
    if let AstType::Named { segments, .. } = ast_ty {
        return ctx.query(ResolveTypePath {
            segments: segments.iter().map(|s| s.name.clone()).collect(),
            context,
            root,
        });
    }
    let Some((builtin, name)) = sugar_builtin(ast_ty) else {
        return TypeResolution::NotFound(String::new());
    };
    match crate::resolve_sugar_type(ctx, context, root, builtin, name) {
        Some(entity) => TypeResolution::Found(entity),
        None => TypeResolution::NotFound(name.to_string()),
    }
}

/// The type entity an annotation names, with `Self` resolved to the type it
/// means at `context` ([`SelfTypeEntity`]). Non-type entities are `None`.
pub fn annotation_entity(
    ctx: &QueryContext<'_>,
    ast_ty: &AstType,
    context: Entity,
    root: Entity,
) -> Option<Entity> {
    match resolve_annotation(ctx, ast_ty, context, root) {
        TypeResolution::Found(entity) => Some(entity),
        TypeResolution::SelfType => ctx.query(SelfTypeEntity { context, root }),
        TypeResolution::NotFound(_) | TypeResolution::NotAType(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::ResolvedExtensionTarget;
    use kestrel_ast_builder::Name;
    use kestrel_hecs::World;

    fn node(world: &mut World, kind: NodeKind, name: &str, parent: Entity) -> Entity {
        let e = world.spawn();
        world.set(e, kind);
        world.set(e, Name(name.into()));
        world.set_parent(e, parent);
        e
    }

    fn self_type(world: &World, context: Entity, root: Entity) -> Option<Entity> {
        world
            .query_context()
            .query(SelfTypeEntity { context, root })
    }

    #[test]
    fn self_is_the_nearest_enclosing_type() {
        let mut world = World::new();
        world.begin_revision();
        let root = world.spawn();
        world.set(root, NodeKind::Module);
        let outer = node(&mut world, NodeKind::Struct, "Outer", root);
        let inner = node(&mut world, NodeKind::Enum, "Inner", outer);
        let method = node(&mut world, NodeKind::Function, "f", inner);
        let free = node(&mut world, NodeKind::Function, "g", root);

        assert_eq!(self_type(&world, method, root), Some(inner));
        assert_eq!(self_type(&world, inner, root), Some(inner));
        assert_eq!(self_type(&world, free, root), None);
    }

    #[test]
    fn self_in_an_extension_is_its_target() {
        let mut world = World::new();
        world.begin_revision();
        let root = world.spawn();
        world.set(root, NodeKind::Module);
        let target = node(&mut world, NodeKind::Struct, "Box", root);
        let outer = node(&mut world, NodeKind::Protocol, "P", root);
        let ext = node(&mut world, NodeKind::Extension, "", outer);
        world.set(ext, ResolvedExtensionTarget(target));
        let method = node(&mut world, NodeKind::Function, "f", ext);
        // An extension whose target doesn't resolve stops the walk: `Self`
        // never falls through to an outer type.
        let broken = node(&mut world, NodeKind::Extension, "", outer);
        let broken_method = node(&mut world, NodeKind::Function, "h", broken);

        assert_eq!(self_type(&world, method, root), Some(target));
        assert_eq!(self_type(&world, broken_method, root), None);
    }
}
