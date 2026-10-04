//! Builtin type/protocol resolution.
//!
//! Three queries for the builtin system:
//!
//! - `EntityBuiltin`: Forward lookup — does this entity have `@builtin(.X)`?
//! - `BuiltinIndex`: Scans all entities to build a complete Builtin → Entity map.
//! - `ResolveBuiltin`: Reverse lookup — which entity is the `Addable` protocol?
//!   Answered by the attribute index ONLY. A lang item is whatever carries the
//!   `@builtin(.X)` annotation; its source name means nothing (a user
//!   `module Int64` must not become the integer type — audit H2).

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use kestrel_ast::AstType;
use kestrel_ast_builder::{Attributes, TypeAnnotation};
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir::Builtin;

use crate::resolve_type::{ResolveTypePath, TypeResolution};

// ===== EntityBuiltin: forward lookup (entity → Builtin) =====

/// Query: extract `@builtin(.Feature)` from an entity's Attributes component.
///
/// Returns `Some(Builtin)` if the entity has a valid `@builtin` attribute,
/// `None` otherwise.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct EntityBuiltin {
    pub entity: Entity,
}

impl QueryFn for EntityBuiltin {
    type Output = Option<Builtin>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Builtin> {
        let attrs = ctx.get::<Attributes>(self.entity)?;

        // Find the @builtin attribute
        let builtin_attr = attrs.0.iter().find(|a| a.name == "builtin")?;

        // Must have exactly one unlabeled arg with implicit member syntax (.Name)
        let arg = builtin_attr.args.first()?;
        if arg.label.is_some() {
            return None;
        }

        // Value is ".FeatureName" from the implicit member syntax
        let feature_name = arg.value.strip_prefix('.')?;
        Builtin::from_attribute_name(feature_name)
    }
}

// ===== BuiltinMap: hashable wrapper for HashMap =====

/// Hashable map from Builtin → Entity. Needed because `HashMap` doesn't
/// implement `Hash`, but `QueryFn::Output` requires it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuiltinMap(pub HashMap<Builtin, Entity>);

impl Hash for BuiltinMap {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Sort entries for deterministic hashing (Builtin is Copy + Eq)
        let mut entries: Vec<_> = self.0.iter().collect();
        entries.sort_by(|(a, _), (b, _)| {
            // Use debug representation for stable ordering since Builtin
            // doesn't implement Ord. Could derive Ord instead, but this
            // is only called once per revision for fingerprinting.
            format!("{a:?}").cmp(&format!("{b:?}"))
        });
        entries.len().hash(state);
        for (k, v) in entries {
            k.hash(state);
            v.hash(state);
        }
    }
}

impl BuiltinMap {
    /// Look up the entity for a builtin.
    pub fn get(&self, builtin: &Builtin) -> Option<Entity> {
        self.0.get(builtin).copied()
    }
}

// ===== BuiltinIndex: scan all entities for @builtin attributes =====

/// Query: build a complete index of all `@builtin`-annotated entities.
///
/// Walks the entire entity hierarchy under root, checking each entity's
/// Attributes component. Cached per revision — the scan runs at most once.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BuiltinIndex {
    pub root: Entity,
}

impl QueryFn for BuiltinIndex {
    type Output = Arc<BuiltinMap>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Arc<BuiltinMap> {
        let mut map = HashMap::new();
        scan_builtins(ctx, self.root, &mut map);
        Arc::new(BuiltinMap(map))
    }
}

/// Scan an entity and its children for @builtin attributes, depth-first in
/// declaration order. The FIRST annotation of a builtin wins; any later one
/// is a duplicate, reported as E400 by kestrel-analyze (`duplicate_builtin`).
fn scan_builtins(ctx: &QueryContext<'_>, entity: Entity, map: &mut HashMap<Builtin, Entity>) {
    if let Some(builtin) = ctx.query(EntityBuiltin { entity }) {
        map.entry(builtin).or_insert(entity);
    }
    for &child in ctx.children_of(entity) {
        scan_builtins(ctx, child, map);
    }
}

// ===== ResolveBuiltin: reverse lookup (Builtin → Entity) =====

/// Query: resolve a builtin type/protocol to its entity — the declaration
/// annotated `@builtin(.Feature)`, from [`BuiltinIndex`]. There is no
/// name-based lookup: a lang item is identified by its annotation only, so a
/// user declaration that merely shares a builtin's name (`module Int64`,
/// `struct Bool`) is never taken for it.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ResolveBuiltin {
    pub builtin: Builtin,
    pub root: Entity,
}

impl QueryFn for ResolveBuiltin {
    type Output = Option<Entity>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Entity> {
        let annotated = ctx
            .query(BuiltinIndex { root: self.root })
            .get(&self.builtin)?;
        if !self.builtin.denotes_alias_target() {
            return Some(annotated);
        }
        alias_target(ctx, annotated, self.root)
    }
}

/// The nominal a type alias names (`Int64` for `type IntegerLiteralType =
/// Int64`, `Array` for `type ArrayLiteralType[T] = std.collections.Array[T]`),
/// resolved in the alias's own scope.
fn alias_target(ctx: &QueryContext<'_>, alias: Entity, root: Entity) -> Option<Entity> {
    let TypeAnnotation(AstType::Named { segments, .. }) = ctx.get::<TypeAnnotation>(alias)? else {
        return None;
    };
    let resolution = ctx.query(ResolveTypePath {
        segments: segments.iter().map(|s| s.name.clone()).collect(),
        context: ctx.parent_of(alias).unwrap_or(root),
        root,
    });
    match resolution {
        TypeResolution::Found(entity) => Some(entity),
        _ => None,
    }
}
