//! Conformance resolution queries.
//!
//! Collects all protocols a concrete type entity transitively conforms to,
//! walking direct conformances, extension conformances, and protocol
//! inheritance.

use std::collections::HashSet;

use kestrel_ast::AstType;
use kestrel_ast_builder::{ConformanceItem, Conformances, NodeKind};
use kestrel_hecs::{Entity, QueryContext, QueryFn};

use crate::extensions::ExtensionsFor;
use crate::resolve_type::{ResolveTypePath, TypeResolution};

// ===== ConformingProtocols =====

/// Query: collect all protocols a concrete type entity transitively conforms to.
///
/// Walks:
/// 1. Direct conformances on the entity (`Conformances` component)
/// 2. Conformances on extensions of this entity (`ExtensionsFor` query)
/// 3. Protocol inheritance — for each discovered protocol, walk its own
///    conformances and extension conformances recursively
///
/// Result is memoized per `(entity, root)`. Callers check membership
/// with `.contains(&protocol)`.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ConformingProtocols {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for ConformingProtocols {
    type Output = Vec<Entity>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Vec<Entity> {
        let mut protocols = Vec::new();
        let mut visited = HashSet::new();

        // Direct conformances on the type itself
        gather_protocol_conformances(ctx, self.entity, self.root, &mut protocols, &mut visited);

        // Conformances declared on extensions of this type
        let extensions = ctx.query(ExtensionsFor {
            target: self.entity,
            root: self.root,
        });
        for ext in &extensions {
            gather_protocol_conformances(ctx, *ext, self.root, &mut protocols, &mut visited);
        }

        // Walk extensions of discovered protocols (e.g. `extend Comparable: Less[Self]`).
        expand_protocol_closure_in_place(ctx, self.root, &mut protocols, &mut visited);

        protocols
    }

    fn describe(&self) -> String {
        format!("ConformingProtocols({:?})", self.entity)
    }
}

// ===== ConformingProtocolInstantiations =====

/// Query: like `ConformingProtocols`, but preserves each conformance's type
/// arguments so distinct protocol instantiations are tracked separately.
///
/// `Int64: Convertible[Int8], Convertible[Int16]` produces two entries, not
/// one. This is what witness generation needs — each `(protocol, type_args)`
/// pair has its own witness with its own method bindings.
///
/// Transitive inheritance (walking each discovered protocol's own
/// conformances and extension-added conformances) is handled like
/// `ConformingProtocols`; inherited protocols use the type args declared
/// on the inheritance edge (typically empty or referring to the child's
/// own type params).
///
/// Deduplication is by `(protocol, type_args)` — two conformances to the
/// same protocol with different args are both reported.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ConformingProtocolInstantiations {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for ConformingProtocolInstantiations {
    type Output = Vec<(Entity, Vec<AstType>)>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Self::Output {
        let mut instantiations: Vec<(Entity, Vec<AstType>)> = Vec::new();
        let mut visited: HashSet<(Entity, Vec<AstType>)> = HashSet::new();

        // Direct conformances on the type itself
        gather_protocol_instantiations(
            ctx,
            self.entity,
            self.root,
            &mut instantiations,
            &mut visited,
        );

        // Conformances declared on extensions of this type
        let extensions = ctx.query(ExtensionsFor {
            target: self.entity,
            root: self.root,
        });
        for ext in &extensions {
            gather_protocol_instantiations(ctx, *ext, self.root, &mut instantiations, &mut visited);
        }

        // Walk inheritance: for each discovered protocol, expand its own
        // conformances and extension-added conformances. Each protocol is
        // expanded at most once (tracked by `proto_expanded`).
        let mut proto_expanded: HashSet<Entity> = HashSet::new();
        let mut i = 0;
        while i < instantiations.len() {
            let (proto, _) = instantiations[i].clone();
            if proto_expanded.insert(proto) {
                gather_protocol_instantiations(
                    ctx,
                    proto,
                    self.root,
                    &mut instantiations,
                    &mut visited,
                );
                let proto_extensions = ctx.query(ExtensionsFor {
                    target: proto,
                    root: self.root,
                });
                for ext in &proto_extensions {
                    gather_protocol_instantiations(
                        ctx,
                        *ext,
                        self.root,
                        &mut instantiations,
                        &mut visited,
                    );
                }
            }
            i += 1;
        }

        instantiations
    }

    fn describe(&self) -> String {
        format!("ConformingProtocolInstantiations({:?})", self.entity)
    }
}

/// Gather `(protocol, type_args)` pairs from an entity's `Conformances`
/// component. Mirrors `gather_protocol_conformances` but preserves the AST
/// type args from each conformance path.
fn gather_protocol_instantiations(
    ctx: &QueryContext<'_>,
    entity: Entity,
    root: Entity,
    instantiations: &mut Vec<(Entity, Vec<AstType>)>,
    visited: &mut HashSet<(Entity, Vec<AstType>)>,
) {
    let Some(conformances) = ctx.get::<Conformances>(entity) else {
        return;
    };

    for item in &conformances.0 {
        let ConformanceItem::Positive(ast_ty, _) = item else {
            continue;
        };
        let Some(resolved) = resolve_conformance_entity(ctx, ast_ty, entity, root) else {
            continue;
        };

        // Only collect protocol entities
        if ctx.get::<NodeKind>(resolved) != Some(&NodeKind::Protocol) {
            continue;
        }

        let type_args = extract_ast_type_args(ast_ty);
        let key = (resolved, type_args.clone());
        if !visited.insert(key) {
            continue;
        }

        instantiations.push((resolved, type_args));
    }
}

/// Extract the type arguments from the final segment of a named `AstType`.
/// For `Convertible[Int16]` returns `[Int16]`; for bare `Equatable` returns `[]`.
pub fn extract_ast_type_args(ast_ty: &AstType) -> Vec<AstType> {
    match ast_ty {
        AstType::Named { segments, .. } => segments
            .last()
            .map(|seg| seg.type_args.clone())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

// ===== Shared transitive-closure walk =====

/// Expand `protocols` in place: for each protocol already in the list, walk
/// its inherited parents (via direct `Conformances`) and the conformances
/// added via `extend P: Q` (via `ExtensionsFor`). Protocols already in
/// `visited` are skipped.
///
/// Callers that seed `protocols` from where-clause bounds (as opposed to a
/// `Conformances` component) use this to complete the transitive closure.
/// Used internally by `ConformingProtocols`; exposed for use by
/// `kestrel-type-infer` on type-parameter / associated-type bounds.
pub fn expand_protocol_closure_in_place(
    ctx: &QueryContext<'_>,
    root: Entity,
    protocols: &mut Vec<Entity>,
    visited: &mut HashSet<Entity>,
) {
    let mut i = 0;
    while i < protocols.len() {
        let proto = protocols[i];
        // Inherited parents (protocol inheritance via direct Conformances)
        gather_protocol_conformances(ctx, proto, root, protocols, visited);
        // Extension-added conformances (e.g. `extend Equatable: NotEqual`)
        let proto_extensions = ctx.query(ExtensionsFor {
            target: proto,
            root,
        });
        for ext in &proto_extensions {
            gather_protocol_conformances(ctx, *ext, root, protocols, visited);
        }
        i += 1;
    }
}

/// Convenience wrapper: expand a seed set of protocol entities into their
/// full transitive closure (seeds ∪ inherited parents ∪ extension-added).
///
/// Seeds already present in the result are deduplicated.
pub fn expand_protocol_closure(
    ctx: &QueryContext<'_>,
    root: Entity,
    seeds: impl IntoIterator<Item = Entity>,
) -> Vec<Entity> {
    let mut protocols = Vec::new();
    let mut visited = HashSet::new();
    for seed in seeds {
        if visited.insert(seed) {
            protocols.push(seed);
        }
    }
    expand_protocol_closure_in_place(ctx, root, &mut protocols, &mut visited);
    protocols
}

// ===== Helpers =====

/// Gather protocols from an entity's `Conformances` component, recursively
/// walking inherited protocols (protocol parents).
fn gather_protocol_conformances(
    ctx: &QueryContext<'_>,
    entity: Entity,
    root: Entity,
    protocols: &mut Vec<Entity>,
    visited: &mut HashSet<Entity>,
) {
    let Some(conformances) = ctx.get::<Conformances>(entity) else {
        return;
    };

    for item in &conformances.0 {
        let ConformanceItem::Positive(ast_ty, _) = item else {
            continue;
        };
        let Some(resolved) = resolve_conformance_entity(ctx, ast_ty, entity, root) else {
            continue;
        };

        // Only collect protocol entities
        if ctx.get::<NodeKind>(resolved) != Some(&NodeKind::Protocol) {
            continue;
        }

        if !visited.insert(resolved) {
            continue;
        }

        protocols.push(resolved);

        // Walk inherited protocols transitively
        gather_protocol_conformances(ctx, resolved, root, protocols, visited);
    }
}

/// Resolve a conformance AstType to an entity via `ResolveTypePath`.
///
/// Uses the conformance-bearing entity as resolution context — protocol
/// names are top-level so they resolve from any scope.
fn resolve_conformance_entity(
    ctx: &QueryContext<'_>,
    ast_ty: &AstType,
    context: Entity,
    root: Entity,
) -> Option<Entity> {
    let AstType::Named { segments, .. } = ast_ty else {
        return None;
    };

    let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    match ctx.query(ResolveTypePath {
        segments: seg_names,
        context,
        root,
    }) {
        TypeResolution::Found(entity) => Some(entity),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast::PathSegment;
    use kestrel_ast_builder::{ConformanceItem, Conformances, ExtensionTarget, Name, Typed};
    use kestrel_hecs::World;
    use kestrel_span2::Span;
    use kestrel_syntax_tree2::{GreenNodeBuilder, SyntaxKind, SyntaxNode};

    fn span() -> Span {
        Span::synthetic(0)
    }

    /// Build a throwaway SyntaxNode — `ConformanceItem::Positive` stores one
    /// but the conformance walk never reads it.
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

    fn positive(name: &str) -> ConformanceItem {
        ConformanceItem::Positive(named_ast(name), fake_syntax())
    }

    fn spawn_protocol(world: &mut World, parent: Entity, name: &str) -> Entity {
        let e = world.spawn();
        world.set(e, NodeKind::Protocol);
        world.set(e, Name(name.into()));
        world.set(e, Typed);
        world.set_parent(e, parent);
        e
    }

    fn spawn_struct(world: &mut World, parent: Entity, name: &str) -> Entity {
        let e = world.spawn();
        world.set(e, NodeKind::Struct);
        world.set(e, Name(name.into()));
        world.set(e, Typed);
        world.set_parent(e, parent);
        e
    }

    fn spawn_extension(
        world: &mut World,
        parent: Entity,
        target_name: &str,
        added: Vec<&str>,
    ) -> Entity {
        let e = world.spawn();
        world.set(e, NodeKind::Extension);
        world.set(e, ExtensionTarget(named_ast(target_name)));
        world.set(e, Conformances(added.iter().map(|n| positive(n)).collect()));
        world.set_parent(e, parent);
        e
    }

    #[test]
    fn no_conformances() {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".into()));

        let bare_struct = spawn_struct(&mut world, root, "Bare");

        let ctx = world.query_context();
        let protocols = ctx.query(ConformingProtocols {
            entity: bare_struct,
            root,
        });
        assert!(protocols.is_empty());
    }

    /// `extend Equatable: NotEqual` should make any `S: Equatable` also
    /// conform to `NotEqual`.
    #[test]
    fn extension_added_conformance() {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".into()));

        let equatable = spawn_protocol(&mut world, root, "Equatable");
        let not_equal = spawn_protocol(&mut world, root, "NotEqual");
        spawn_extension(&mut world, root, "Equatable", vec!["NotEqual"]);

        let s = spawn_struct(&mut world, root, "S");
        world.set(s, Conformances(vec![positive("Equatable")]));

        let ctx = world.query_context();
        let protocols = ctx.query(ConformingProtocols { entity: s, root });
        assert!(protocols.contains(&equatable), "missing Equatable");
        assert!(
            protocols.contains(&not_equal),
            "missing NotEqual from extension"
        );
    }

    /// Chained extensions: `Ord` inherits `Eq`; `extend Ord: Greater`;
    /// `extend Greater: AtLeastAsGreat`. `S: Ord` should see all four.
    #[test]
    fn nested_extension_added_conformance() {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".into()));

        let eq = spawn_protocol(&mut world, root, "Eq");
        let ord = spawn_protocol(&mut world, root, "Ord");
        world.set(ord, Conformances(vec![positive("Eq")])); // Ord: Eq
        let greater = spawn_protocol(&mut world, root, "Greater");
        let at_least = spawn_protocol(&mut world, root, "AtLeastAsGreat");
        spawn_extension(&mut world, root, "Ord", vec!["Greater"]);
        spawn_extension(&mut world, root, "Greater", vec!["AtLeastAsGreat"]);

        let s = spawn_struct(&mut world, root, "S");
        world.set(s, Conformances(vec![positive("Ord")]));

        let ctx = world.query_context();
        let protocols = ctx.query(ConformingProtocols { entity: s, root });
        for (e, name) in [
            (ord, "Ord"),
            (eq, "Eq"),
            (greater, "Greater"),
            (at_least, "AtLeastAsGreat"),
        ] {
            assert!(protocols.contains(&e), "missing {name}");
        }
    }

    /// `expand_protocol_closure` directly: seed with `[Equatable]`, expect
    /// `NotEqual` via the `extend Equatable: NotEqual` extension.
    #[test]
    fn expand_protocol_closure_from_seeds() {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".into()));

        let equatable = spawn_protocol(&mut world, root, "Equatable");
        let not_equal = spawn_protocol(&mut world, root, "NotEqual");
        spawn_extension(&mut world, root, "Equatable", vec!["NotEqual"]);

        let ctx = world.query_context();
        let closure = expand_protocol_closure(&ctx, root, [equatable]);
        assert!(closure.contains(&equatable));
        assert!(
            closure.contains(&not_equal),
            "seed expansion missed NotEqual"
        );
    }
}
