//! Builtin type/protocol resolution.
//!
//! Three queries for the builtin system:
//!
//! - `EntityBuiltin`: Forward lookup — does this entity have `@builtin(.X)`?
//! - `BuiltinIndex`: Scans all entities to build a complete Builtin → Entity map.
//! - `ResolveBuiltin`: Reverse lookup — which entity is the `Addable` protocol?
//!   Uses name-based resolution first, then falls back to the attribute index.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use kestrel_ast_builder::Attributes;
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

/// Recursively scan an entity and its children for @builtin attributes.
fn scan_builtins(ctx: &QueryContext<'_>, entity: Entity, map: &mut HashMap<Builtin, Entity>) {
    if let Some(builtin) = ctx.query(EntityBuiltin { entity }) {
        map.insert(builtin, entity);
    }
    for &child in ctx.children_of(entity) {
        scan_builtins(ctx, child, map);
    }
}

// ===== ResolveBuiltin: reverse lookup (Builtin → Entity) =====

/// Query: resolve a builtin type/protocol to its entity.
///
/// Two resolution strategies:
/// 1. **Name-based**: Look up the type by its source name in the root scope.
///    Works for types that are auto-imported from std (e.g., "Addable", "Bool").
/// 2. **Attribute index**: Scan for `@builtin(.Feature)` annotations.
///    Works for features that aren't directly importable by name (e.g.,
///    OptionalEnum, protocol methods, enum cases).
///
/// Both strategies are cached by the query system.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ResolveBuiltin {
    pub builtin: Builtin,
    pub root: Entity,
}

impl QueryFn for ResolveBuiltin {
    type Output = Option<Entity>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Entity> {
        // Strategy 1: name-based lookup (fast path for auto-imported types)
        let result = ctx.query(ResolveTypePath {
            segments: vec![self.builtin.name().to_string()],
            context: self.root,
            root: self.root,
        });
        if let TypeResolution::Found(entity) = result {
            return Some(entity);
        }

        // Strategy 2: attribute index fallback
        let index = ctx.query(BuiltinIndex { root: self.root });
        index.get(&self.builtin)
    }
}
