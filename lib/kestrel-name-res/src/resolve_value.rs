//! Value path resolution.
//!
//! Resolves value names (variables, functions, enum cases, etc.) to
//! entities. Used by HIR lowering for expression paths.

use std::collections::HashSet;

use kestrel_ast_builder::{
    Callable, ConformanceItem, Conformances, Gettable, Name, NodeKind, Static, Typed, WhereClause,
};
use kestrel_hecs::{Entity, QueryContext, QueryFn};

use crate::helpers::find_in_extensions;
use crate::protocol_members::ProtocolMembersByName;
use crate::resolve_name::{NameResolution, ResolveName};
use crate::resolve_type::{ResolveTypePath, TypeResolution};
use crate::type_members::{TypeMemberSource, TypeMembersByName};
use crate::visibility::VisibleChildrenByName;

// ===== ValueResolution =====

/// Result of resolving a value path.
#[derive(Clone, Debug, Hash)]
pub enum ValueResolution {
    /// Single definition
    Def(Entity),
    /// Multiple overloaded functions with the same name
    Overloaded(Vec<Entity>),
    /// Multiple conflicting non-function matches
    Ambiguous(Vec<Entity>),
    /// Type parameter (e.g. `T` used as a value for static access)
    TypeParameter(Entity),
    /// Associated type (e.g. in protocol context)
    AssociatedType {
        entity: Entity,
        container: Option<Entity>,
    },
    /// Enum case used as intermediate value (e.g. `MyEnum.caseA.method()`)
    EnumCaseValue {
        entity: Entity,
        resolved_index: usize,
    },
    /// Field/getter used as intermediate value (e.g. `obj.field.method()`)
    FieldValue {
        entity: Entity,
        resolved_index: usize,
    },
    /// Static member accessed through associated type (e.g., `Item.zero` where `Item: Addable`)
    /// Preserves the associated type context for Self-substitution in type inference.
    AssociatedTypeStaticMember { entity: Entity, assoc_type: Entity },
    /// `Self` in value position — resolved to the enclosing type entity
    /// (struct/enum for direct enclosure, or extension target). Lowers the
    /// same as `Def(entity)` but carries the Self indirection so diagnostics
    /// can refer to "Self" rather than the underlying name.
    SelfValue(Entity),
    /// `Self` used in value position outside any extension/protocol/type body.
    SelfNotInScope,
    /// Not found
    NotFound(String),
}

// ===== ResolveValuePath =====

/// Query: resolve a value path to an entity.
///
/// Handles:
/// - The first segment: a scope lookup via ResolveName
/// - Later segments: member lookups (`walk_path_from`)
/// - The last segment, whichever it is: `classify_terminal`
/// - Type alias: resolves through to underlying type
/// - Extension static methods: if no direct match, search extensions
/// - Function overloads: multiple functions → Overloaded
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ResolveValuePath {
    pub segments: Vec<String>,
    pub context: Entity,
    pub root: Entity,
}

impl QueryFn for ResolveValuePath {
    type Output = ValueResolution;

    /// A bare name is a one-segment path: the first segment is a scope
    /// lookup, every later one a member lookup (`walk_path_from`), and
    /// whichever segment ends the path is classified by `classify_terminal`,
    /// so `C(...)` and `Module.C(...)` can't disagree about what `C` is.
    fn execute(&self, ctx: &QueryContext<'_>) -> ValueResolution {
        let Some((first, rest)) = self.segments.split_first() else {
            return ValueResolution::NotFound("<empty>".into());
        };
        let base = match resolve_head(ctx, first, self.context, self.root) {
            Head::SelfType(target) if rest.is_empty() => {
                return ValueResolution::SelfValue(target);
            },
            Head::SelfType(target) => target,
            Head::Names(entities) if rest.is_empty() => {
                return classify_terminal(ctx, entities, self.context, self.root);
            },
            Head::Names(entities) => match path_base(ctx, entities) {
                Ok(base) => base,
                Err(result) => return result,
            },
            Head::Ambiguous(entities) if rest.is_empty() => {
                return ValueResolution::Ambiguous(entities);
            },
            // A prefix only has to name one type (`X.y`).
            Head::Ambiguous(entities) => match path_base(ctx, entities) {
                Ok(base) => base,
                Err(result) => return result,
            },
            Head::Done(result) => return result,
        };
        walk_path_from(ctx, base, &self.segments, self.context, self.root)
    }
}

/// Resolve `Self` in value position to the enclosing type entity.
///
/// Walks parents from `context` looking for an enclosing scope that gives
/// `Self` a concrete meaning. Mirrors `resolve_self_entity_from` in
/// `kestrel-type-infer` (which already handles this for type position).
///
/// For concrete types (struct/enum, or an extension on a concrete type),
/// `Self()` is just a name for the type. For protocols, `Self()` is
/// polymorphic — callers route it through witness dispatch in type-infer,
/// the same way `T()` works when `T: SomeProtocol`.
fn try_resolve_self_value(ctx: &QueryContext<'_>, context: Entity, root: Entity) -> Option<Entity> {
    let mut current = Some(context);
    while let Some(entity) = current {
        match ctx.get::<NodeKind>(entity) {
            Some(&NodeKind::Extension) => {
                return ctx.query(crate::ExtensionTargetEntity {
                    extension: entity,
                    root,
                });
            },
            Some(&NodeKind::Struct | &NodeKind::Enum | &NodeKind::Protocol) => {
                return Some(entity);
            },
            _ => current = ctx.parent_of(entity),
        }
    }
    None
}

/// What the first segment of a value path names, from a scope lookup.
enum Head {
    /// `Self`: the enclosing type.
    SelfType(Entity),
    /// Everything the name finds in scope (one entity, or an overload set).
    Names(Vec<Entity>),
    /// Conflicting declarations visible from different scopes.
    Ambiguous(Vec<Entity>),
    /// Settled already: not found, or `Self` out of scope.
    Done(ValueResolution),
}

/// Look up the first segment in scope.
fn resolve_head(ctx: &QueryContext<'_>, name: &str, context: Entity, root: Entity) -> Head {
    if crate::helpers::is_self_name(ctx, name, context) {
        return match try_resolve_self_value(ctx, context, root) {
            Some(target) => Head::SelfType(target),
            None => Head::Done(ValueResolution::SelfNotInScope),
        };
    }
    match ctx.query(ResolveName {
        name: name.to_string(),
        context,
        root,
    }) {
        NameResolution::Found(entities) => Head::Names(entities),
        NameResolution::Ambiguous(entities) => Head::Ambiguous(entities),
        NameResolution::NotFound => Head::Done(ValueResolution::NotFound(name.to_string())),
    }
}

/// The single entity a path walks from when the first segment found several
/// (`X.y` with `X` also naming a function): a lone type wins; overloads or
/// several types can't be walked into.
fn path_base(ctx: &QueryContext<'_>, entities: Vec<Entity>) -> Result<Entity, ValueResolution> {
    if let [only] = entities.as_slice() {
        return Ok(*only);
    }
    let types: Vec<Entity> = entities
        .iter()
        .filter(|&&e| ctx.has::<Typed>(e))
        .copied()
        .collect();
    if let [only] = types.as_slice() {
        return Ok(*only);
    }
    Err(ValueResolution::Ambiguous(entities))
}

/// Classify what the last segment of a path found, from scope or as a
/// member: the one place that decides what a name means as a value.
/// - a type parameter stays a type parameter (static access through bounds);
/// - an alias inside a protocol is an associated type (a projection on Self);
/// - a free-standing alias is the type it names;
/// - otherwise one definition, an overload set, or ambiguous.
fn classify_terminal(
    ctx: &QueryContext<'_>,
    entities: Vec<Entity>,
    context: Entity,
    root: Entity,
) -> ValueResolution {
    if let [only] = entities.as_slice() {
        let e = *only;
        match ctx.get::<NodeKind>(e) {
            Some(&NodeKind::TypeParameter) => return ValueResolution::TypeParameter(e),
            Some(&NodeKind::TypeAlias) => {
                if is_associated_type(ctx, e) {
                    return ValueResolution::AssociatedType {
                        entity: e,
                        container: None,
                    };
                }
                if let Some(target) = free_alias_target(ctx, e, context, root) {
                    return ValueResolution::Def(target);
                }
            },
            _ => {},
        }
    }
    classify_value_results(ctx, entities)
}

/// An alias declared inside a protocol: an associated type.
fn is_associated_type(ctx: &QueryContext<'_>, alias: Entity) -> bool {
    ctx.parent_of(alias)
        .is_some_and(|p| ctx.get::<NodeKind>(p) == Some(&NodeKind::Protocol))
}

/// Walk `segments[1..]` as member lookups, starting from the entity the
/// first segment named (`Self`'s type for `Self.foo`).
fn walk_path_from(
    ctx: &QueryContext<'_>,
    first_entity: Entity,
    segments: &[String],
    context: Entity,
    root: Entity,
) -> ValueResolution {
    let mut current = first_entity;
    // Set once a segment walked into an associated type off a param or
    // projection (`B.Item`): from there on the base matters, and only the
    // type checker's base-aware lookup can judge a member.
    let mut via_assoc = false;
    for (i, segment) in segments[1..].iter().enumerate() {
        let is_last = i == segments.len() - 2;

        // Resolve through type aliases so the lookups below see the target.
        current = resolve_through_type_alias(ctx, current, context, root);

        // Associated types reachable from a type parameter or an abstract
        // associated type (`T.Item`, `T.Iter.Item`).
        if let Some(found) = lookup_assoc_type_segment(ctx, current, segment, context, root) {
            if is_last {
                // Associated type referenced as a value (e.g. `T.Item` as
                // the receiver of a method call). Handled downstream as a
                // type reference that dispatches on the associated type.
                return ValueResolution::AssociatedType {
                    entity: found,
                    container: Some(current),
                };
            }
            current = found;
            via_assoc = true;
            continue;
        }

        // Try direct children first
        let children = ctx.query(VisibleChildrenByName {
            parent: current,
            name: segment.clone(),
            context,
        });

        if !children.is_empty() {
            if is_last {
                return classify_terminal(ctx, children, context, root);
            }
            // For intermediate segments, prefer types
            current = children
                .iter()
                .find(|&&e| ctx.has::<Typed>(e))
                .copied()
                .unwrap_or(children[0]);
            continue;
        }

        // A protocol (`Self.seed()` in `extend Q`, or `Q.seed`) also answers
        // with its parents' requirements and every extension default along
        // the way — the protocol-side member query, not just its own children.
        if ctx.get::<NodeKind>(current) == Some(&NodeKind::Protocol) {
            let members: Vec<Entity> = ctx
                .query(ProtocolMembersByName {
                    protocol: current,
                    name: segment.clone(),
                    context,
                    root,
                })
                .into_iter()
                .map(|m| m.entity)
                .collect();
            if is_last && !members.is_empty() {
                return classify_terminal(ctx, members, context, root);
            }
        }

        // No direct children — try static members declared in extensions.
        // A static property mid-path (`Meters.zero.v`) is walked into like a
        // direct field: the next segment reports it as an intermediate value.
        if ctx.has::<Typed>(current)
            && let Some(result) =
                resolve_extension_static_member(ctx, current, segment, context, root)
        {
            if is_last {
                return result;
            }
            if let ValueResolution::Def(member) = result
                && ctx.get::<NodeKind>(member) == Some(&NodeKind::Field)
            {
                current = member;
                continue;
            }
        }

        // For associated types (abstract TypeAlias), search protocol bounds
        // for static members (e.g. Item.zero where Item: Addable).
        // Return AssociatedTypeStaticMember to preserve the associated type
        // context for Self-substitution in type inference.
        if ctx.get::<NodeKind>(current) == Some(&NodeKind::TypeAlias)
            && let Some(found) =
                resolve_assoc_type_static_member(ctx, current, segment, context, root)
        {
            if is_last {
                return ValueResolution::AssociatedTypeStaticMember {
                    entity: found,
                    assoc_type: current,
                };
            }
            current = found;
            continue;
        }

        // A static member off a projection (`B.Item.zero`) that the
        // declaration-level search above cannot see — its bound may come
        // from a where clause (`where B.Item: Zero`). Keep the projection
        // and let the type checker's base-aware lookup accept or reject it
        // (G26); falling through here turned the path into a field chain on
        // `B` ("no member 'Item' on type 'B'").
        if is_last && via_assoc && ctx.get::<NodeKind>(current) == Some(&NodeKind::TypeAlias) {
            return ValueResolution::AssociatedTypeStaticMember {
                entity: current,
                assoc_type: current,
            };
        }

        // Check for enum case or field/getter used as intermediate value;
        // `i` is the index of the last successfully resolved segment.
        if let Some(result) = resolve_intermediate_value(ctx, current, i) {
            return result;
        }

        return ValueResolution::NotFound(segment.clone());
    }

    // Reached the end with a non-terminal entity
    ValueResolution::Def(current)
}

/// The concrete type a free-standing alias names, walking chains
/// (`type A = B; type B = C`), so HIR, inference and call dispatch see the
/// type itself wherever the alias is used as a value (`A(...)`,
/// `Module.A(...)`). `None` for an associated type (an alias inside a
/// protocol) or an alias whose target can't be resolved.
fn free_alias_target(
    ctx: &QueryContext<'_>,
    alias: Entity,
    context: Entity,
    root: Entity,
) -> Option<Entity> {
    if is_associated_type(ctx, alias) {
        return None;
    }
    let mut current = alias;
    for _ in 0..8 {
        let target = resolve_type_alias_target(ctx, current, context, root)?;
        if ctx.get::<NodeKind>(target) != Some(&NodeKind::TypeAlias) {
            return Some(target);
        }
        current = target;
    }
    None
}

/// Resolve through a type alias so member lookups see the aliased type.
/// Non-aliases (and aliases whose target can't be resolved, e.g. abstract
/// associated types) pass through unchanged.
fn resolve_through_type_alias(
    ctx: &QueryContext<'_>,
    entity: Entity,
    context: Entity,
    root: Entity,
) -> Entity {
    if ctx.get::<NodeKind>(entity) == Some(&NodeKind::TypeAlias)
        && let Some(resolved) = resolve_type_alias_target(ctx, entity, context, root)
    {
        return resolved;
    }
    entity
}

/// Look up `segment` as an associated type reachable from `current`: through
/// a type parameter's protocol bounds (`T.Item`, `T.Next`) or nested through
/// an abstract associated type — a TypeAlias inside a protocol with its own
/// bounds (`T.Iter.Item`, `T.Next.Next`). The cases are disjoint on NodeKind.
fn lookup_assoc_type_segment(
    ctx: &QueryContext<'_>,
    current: Entity,
    segment: &str,
    context: Entity,
    root: Entity,
) -> Option<Entity> {
    match ctx.get::<NodeKind>(current) {
        Some(&NodeKind::TypeParameter) => {
            crate::resolve_type::resolve_type_param_assoc(ctx, current, segment, context, root)
        },
        Some(&NodeKind::TypeAlias) => {
            crate::resolve_type::resolve_assoc_type_nested(ctx, current, segment, context, root)
        },
        _ => None,
    }
}

/// Is `e` a static member — a static property, or a static callable
/// (explicitly `static`, or a receiver-less function)?
fn is_static_member(ctx: &QueryContext<'_>, e: Entity) -> bool {
    match ctx.get::<NodeKind>(e) {
        Some(NodeKind::Field) => ctx.has::<Static>(e),
        Some(NodeKind::Function) => {
            ctx.has::<Static>(e) || ctx.get::<Callable>(e).is_some_and(|c| c.receiver.is_none())
        },
        _ => false,
    }
}

/// Static members (methods and properties) on `current`, sourced from `TypeMembersByName` — the one
/// query that answers "what members does this type have?" It already merges
/// every extension of the type and every extension of every protocol it
/// transitively conforms to, so `A.staticMethod()` finds a method living in
/// `extend SomeProtocol { ... }` where `A: SomeProtocol`, mirroring how
/// instance-method dispatch walks conforming protocols
/// (`try_resolve_through_protocol` in kestrel-type-infer).
///
/// The result is the **complete** overload set: `HirExpr::OverloadSet` is
/// built straight from it and inference never re-widens a single `Def`, so
/// anything dropped here is unrecoverable — splitting one `extend` block in
/// two used to silently break a call (F10).
///
/// Precedence follows the instance-member rule already shipped in
/// `kestrel_type_infer::resolve_member`: direct and own-extension candidates
/// compete equally, and a protocol-extension default joins them only when its
/// **label signature** is not already taken. So a type's own `tag()` wins over
/// `extend SomeProtocol { static func tag() }` without making the pair
/// ambiguous, while a protocol default with different labels stays reachable
/// as an overload. Dropping it wholesale would repeat F10's mistake in the
/// other direction — name resolution is the last place the set can be widened.
fn resolve_extension_static_member(
    ctx: &QueryContext<'_>,
    current: Entity,
    segment: &str,
    context: Entity,
    root: Entity,
) -> Option<ValueResolution> {
    let members = ctx.query(TypeMembersByName {
        type_entity: current,
        name: segment.to_string(),
        context,
        root,
    });

    let (own, via_protocol): (Vec<_>, Vec<_>) = members
        .into_iter()
        .filter(|m| is_static_member(ctx, m.entity))
        .partition(|m| !matches!(m.source, TypeMemberSource::ProtocolExtension { .. }));

    let mut seen: HashSet<Vec<Option<String>>> =
        own.iter().map(|m| label_signature(ctx, m.entity)).collect();
    let mut candidates: Vec<Entity> = own.iter().map(|m| m.entity).collect();
    for m in via_protocol {
        if seen.insert(label_signature(ctx, m.entity)) {
            candidates.push(m.entity);
        }
    }

    if candidates.is_empty() {
        return None;
    }
    Some(classify_value_results(ctx, candidates))
}

/// A callable's argument labels, the key overload resolution disambiguates on.
/// Mirrors `label_signature` in kestrel-type-infer; non-callables collapse to
/// the empty signature so two of them count as the same member.
fn label_signature(ctx: &QueryContext<'_>, entity: Entity) -> Vec<Option<String>> {
    ctx.get::<Callable>(entity)
        .map(|c| c.params.iter().map(|p| p.label.clone()).collect())
        .unwrap_or_default()
}

/// Enum case or gettable field used as an intermediate value (e.g.
/// `MyEnum.caseA.method()` where `caseA` has no children to walk into):
/// stop the walk and report how far we got; the remaining segments are
/// resolved against the value's type downstream.
fn resolve_intermediate_value(
    ctx: &QueryContext<'_>,
    current: Entity,
    resolved_index: usize,
) -> Option<ValueResolution> {
    match ctx.get::<NodeKind>(current) {
        Some(&NodeKind::EnumCase) => Some(ValueResolution::EnumCaseValue {
            entity: current,
            resolved_index,
        }),
        Some(&NodeKind::Field) if ctx.has::<Gettable>(current) => {
            Some(ValueResolution::FieldValue {
                entity: current,
                resolved_index,
            })
        },
        _ => None,
    }
}

/// Classify resolved entities as Def or Overloaded.
fn classify_value_results(ctx: &QueryContext<'_>, entities: Vec<Entity>) -> ValueResolution {
    if entities.len() == 1 {
        return ValueResolution::Def(entities[0]);
    }

    // Multiple results: if all functions, it's overloading
    let all_functions = entities
        .iter()
        .all(|&e| ctx.get::<NodeKind>(e) == Some(&NodeKind::Function));

    if all_functions {
        ValueResolution::Overloaded(entities)
    } else {
        // Mixed results — ambiguous
        ValueResolution::Ambiguous(entities)
    }
}

/// Try to resolve a type alias to its underlying type entity.
fn resolve_type_alias_target(
    ctx: &QueryContext<'_>,
    alias: Entity,
    context: Entity,
    root: Entity,
) -> Option<Entity> {
    let type_ann = ctx.get::<kestrel_ast_builder::TypeAnnotation>(alias)?;
    let kestrel_ast::AstType::Named { segments, .. } = &type_ann.0 else {
        return None;
    };
    let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();

    let result = ctx.query(ResolveTypePath {
        segments: seg_names,
        context,
        root,
    });

    match result {
        TypeResolution::Found(entity) => Some(entity),
        _ => None,
    }
}

/// Search protocol bounds on an associated type for a static member.
///
/// For `Item.zero` where `Item: Addable`, finds the `zero` static member
/// inside the `Addable` protocol. Checks both direct conformances on the
/// TypeAlias and where-clause bounds in ancestor entities.
fn resolve_assoc_type_static_member(
    ctx: &QueryContext<'_>,
    assoc_type: Entity,
    member_name: &str,
    context: Entity,
    root: Entity,
) -> Option<Entity> {
    let assoc_name = ctx.get::<Name>(assoc_type)?;

    // Collect all protocol entities that bound this associated type
    let mut bound_protocols = Vec::new();

    // 1. Direct conformances on the TypeAlias (e.g. `type Item: Addable`)
    if let Some(conformances) = ctx.get::<Conformances>(assoc_type) {
        for item in &conformances.0 {
            let ConformanceItem::Positive(ast_type, ..) = item else {
                continue;
            };
            if let Some(proto) = resolve_protocol_from_ast(ctx, ast_type, context, root) {
                bound_protocols.push(proto);
            }
        }
    }

    // 2. Where-clause bounds in ancestor chain (e.g. `where Item: Addable`)
    let mut ancestor = Some(context);
    while let Some(anc) = ancestor {
        if let Some(where_clause) = ctx.get::<WhereClause>(anc) {
            for constraint in &where_clause.0 {
                let kestrel_ast_builder::WhereConstraint::Bound {
                    subject, protocols, ..
                } = constraint
                else {
                    continue;
                };
                let kestrel_ast::AstType::Named { segments, .. } = subject else {
                    continue;
                };
                // Match single-segment "Item" or two-segment "Self.Item"
                let matches = match segments.len() {
                    1 => segments[0].name == assoc_name.0,
                    2 => segments[0].name == "Self" && segments[1].name == assoc_name.0,
                    _ => false,
                };
                if matches {
                    for proto_ty in protocols {
                        if let Some(proto) = resolve_protocol_from_ast(ctx, proto_ty, anc, root) {
                            bound_protocols.push(proto);
                        }
                    }
                }
            }
        }
        ancestor = ctx.parent_of(anc);
    }

    // Search each protocol for a static member with the requested name
    for proto in &bound_protocols {
        let children = ctx.query(VisibleChildrenByName {
            parent: *proto,
            name: member_name.to_string(),
            context,
        });
        if !children.is_empty() {
            return Some(children[0]);
        }

        // Also check extensions of the protocol (any member kind)
        let ext_members = find_in_extensions(ctx, *proto, member_name, context, root, |_, _| true);
        if !ext_members.is_empty() {
            return Some(ext_members[0]);
        }
    }

    None
}

/// Resolve an AstType to a protocol entity.
fn resolve_protocol_from_ast(
    ctx: &QueryContext<'_>,
    ast_type: &kestrel_ast::AstType,
    scope: Entity,
    root: Entity,
) -> Option<Entity> {
    let kestrel_ast::AstType::Named { segments, .. } = ast_type else {
        return None;
    };
    let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    let result = ctx.query(ResolveTypePath {
        segments: seg_names,
        context: scope,
        root,
    });
    match result {
        TypeResolution::Found(entity)
            if ctx.get::<NodeKind>(entity) == Some(&NodeKind::Protocol) =>
        {
            Some(entity)
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast_builder::{Callable, ModulePath, Name, Vis};
    use kestrel_hecs::World;

    /// Build:
    ///   root > std > core > [Int64(pub, Typed)]
    ///        > MyApp > [Foo(Struct, Typed), bar(Function), baz(Function)]
    ///                > Foo > [case1(EnumCase)]  (simulated as children)
    fn setup() -> (World, Entity) {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name(Name::ROOT.into()));

        // std.core
        let std = world.spawn();
        world.set(std, NodeKind::Module);
        world.set(std, Name("std".into()));
        world.set(std, kestrel_ast_builder::StdRootModule);
        world.set_parent(std, root);

        let core = world.spawn();
        world.set(core, NodeKind::Module);
        world.set(core, Name("core".into()));
        world.set_parent(core, std);

        let int64 = world.spawn();
        world.set(int64, NodeKind::Struct);
        world.set(int64, Name("Int64".into()));
        world.set(int64, Vis::Public);
        world.set(int64, Typed);
        world.set_parent(int64, core);

        // MyApp
        let myapp = world.spawn();
        world.set(myapp, NodeKind::Module);
        world.set(myapp, Name("MyApp".into()));
        world.set_parent(myapp, root);

        // Import std.core
        let imp = world.spawn();
        world.set(imp, NodeKind::Import);
        world.set(imp, ModulePath(vec!["std".into(), "core".into()]));
        world.set_parent(imp, myapp);

        // Enum with cases
        let my_enum = world.spawn();
        world.set(my_enum, NodeKind::Enum);
        world.set(my_enum, Name("MyEnum".into()));
        world.set(my_enum, Typed);
        world.set(my_enum, Vis::Public);
        world.set_parent(my_enum, myapp);

        let case_a = world.spawn();
        world.set(case_a, NodeKind::EnumCase);
        world.set(case_a, Name("caseA".into()));
        world.set_parent(case_a, my_enum);

        let case_b = world.spawn();
        world.set(case_b, NodeKind::EnumCase);
        world.set(case_b, Name("caseB".into()));
        world.set_parent(case_b, my_enum);

        // Two overloaded functions
        let bar1 = world.spawn();
        world.set(bar1, NodeKind::Function);
        world.set(bar1, Name("bar".into()));
        world.set(
            bar1,
            Callable {
                params: vec![],
                receiver: None,
            },
        );
        world.set_parent(bar1, myapp);

        let bar2 = world.spawn();
        world.set(bar2, NodeKind::Function);
        world.set(bar2, Name("bar".into()));
        world.set(
            bar2,
            Callable {
                params: vec![kestrel_ast_builder::AstParam {
                    label: None,
                    name: "x".into(),
                    ty: None,
                    default_entity: None,
                    pattern: None,
                    is_mut: false,
                    is_consuming: false,
                    by_ref: false,
                }],
                receiver: None,
            },
        );
        world.set_parent(bar2, myapp);

        (world, root)
    }

    #[test]
    fn resolve_single_function() {
        let (world, root) = setup();
        let ctx = world.query_context();

        let myapp = ctx
            .children_of(root)
            .iter()
            .find(|&&e| ctx.get::<Name>(e).is_some_and(|n| n.0 == "MyApp"))
            .copied()
            .unwrap();

        // "bar" is overloaded
        let result = ctx.query(ResolveValuePath {
            segments: vec!["bar".into()],
            context: myapp,
            root,
        });
        assert!(matches!(result, ValueResolution::Overloaded(_)));
    }

    #[test]
    fn resolve_enum_case() {
        let (world, root) = setup();
        let ctx = world.query_context();

        let myapp = ctx
            .children_of(root)
            .iter()
            .find(|&&e| ctx.get::<Name>(e).is_some_and(|n| n.0 == "MyApp"))
            .copied()
            .unwrap();

        // MyEnum.caseA
        let result = ctx.query(ResolveValuePath {
            segments: vec!["MyEnum".into(), "caseA".into()],
            context: myapp,
            root,
        });
        match result {
            ValueResolution::Def(entity) => {
                assert_eq!(ctx.get::<Name>(entity).unwrap().0, "caseA");
                assert_eq!(ctx.get::<NodeKind>(entity), Some(&NodeKind::EnumCase));
            },
            other => panic!("expected Def, got {:?}", other),
        }
    }

    #[test]
    fn resolve_not_found() {
        let (world, root) = setup();
        let ctx = world.query_context();

        let result = ctx.query(ResolveValuePath {
            segments: vec!["nonexistent".into()],
            context: root,
            root,
        });
        assert!(matches!(result, ValueResolution::NotFound(_)));
    }

    #[test]
    fn resolve_multi_segment_not_found() {
        let (world, root) = setup();
        let ctx = world.query_context();

        let myapp = ctx
            .children_of(root)
            .iter()
            .find(|&&e| ctx.get::<Name>(e).is_some_and(|n| n.0 == "MyApp"))
            .copied()
            .unwrap();

        let result = ctx.query(ResolveValuePath {
            segments: vec!["MyEnum".into(), "nonexistent".into()],
            context: myapp,
            root,
        });
        assert!(matches!(result, ValueResolution::NotFound(_)));
    }
}
