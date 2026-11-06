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
use kestrel_ast_builder::TypeAnnotation;
use kestrel_ast_builder::attrs::attrs_of;
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir::Builtin;

use crate::resolve_type::{ResolveTypePath, TypeResolution};

// ===== EntityBuiltin: forward lookup (entity → Builtin) =====

/// Query: the entity's well-formed `@builtin(.Feature)`, from `AttrsOf`.
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
        attrs_of(ctx, self.entity).builtin()
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
        scan_builtins(ctx, self.root, self.root, &mut map);
        Arc::new(BuiltinMap(map))
    }
}

/// Scan an entity and its children for @builtin attributes, depth-first in
/// declaration order. The FIRST annotation of a builtin wins; any later one
/// is a duplicate, reported as E400 by kestrel-analyze (`duplicate_builtin`).
/// Annotations [`builtin_annotation_allowed`] rejects never enter the index.
fn scan_builtins(
    ctx: &QueryContext<'_>,
    root: Entity,
    entity: Entity,
    map: &mut HashMap<Builtin, Entity>,
) {
    if let Some(builtin) = ctx.query(EntityBuiltin { entity })
        && builtin_annotation_allowed(ctx, root, entity)
    {
        map.entry(builtin).or_insert(entity);
    }
    for &child in ctx.children_of(entity) {
        scan_builtins(ctx, root, child, map);
    }
}

/// Whether `entity`'s `@builtin` annotation may define a lang item. With a
/// stdlib loaded (a top-level `std` module exists) only declarations inside
/// it may — user code cannot claim a lang item (E401, kestrel-analyze
/// `duplicate_builtin`). Without a stdlib (`--no-std` programs, test
/// preludes) the program supplies its own lang items, so any module may.
///
/// Single source of truth for both the index and the diagnostic. Stdlib
/// membership is by module path, so a user file declaring `module std.x`
/// still passes; telling stdlib files apart by origin needs a file-level
/// marker the loader does not set today.
pub fn builtin_annotation_allowed(ctx: &QueryContext<'_>, root: Entity, entity: Entity) -> bool {
    let Some(std_module) = crate::resolve_module::find_child_module(ctx, root, "std") else {
        return true;
    };
    let mut current = entity;
    while let Some(parent) = ctx.parent_of(current) {
        if current == std_module {
            return true;
        }
        current = parent;
    }
    false
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

// ===== Type sugar (`[T]`, `T?`, `[K: V]`) =====

/// Resolve the type that a piece of type sugar (`[T]`, `T?`, `[K: V]`,
/// `T throws E`) denotes.
///
/// Goes through the `@builtin(.XTypeOperator)` lang item, which is what the
/// stdlib annotation is *for*. Before this, the four annotations in `lang/std`
/// had zero readers and the sugar resolved the hardcoded name `"Array"` in the
/// **user's** scope — so `struct Array[T] {}` in a user file silently captured
/// `[Int]`, and every array literal in that file failed to conform to
/// `_ExpressibleByArrayLiteral`.
///
/// Falls back to the name for worlds built without the stdlib (unit tests,
/// `--no-std`), where there is no lang item to find.
pub fn resolve_sugar_type(
    ctx: &QueryContext<'_>,
    owner: Entity,
    root: Entity,
    builtin: Builtin,
    name: &str,
) -> Option<Entity> {
    if let Some(alias) = ctx.query(ResolveBuiltin { builtin, root })
        && let Some(target) = sugar_alias_target(ctx, root, alias)
    {
        return Some(target);
    }
    resolve_std_type(ctx, owner, root, name)
}

/// Follow `@builtin(.ArrayTypeOperator) type ArrayTypeOperator[T] = Array[T]`
/// to `Array`.
///
/// The target is resolved with the **alias** as context, so it lands in the
/// stdlib's own scope no matter what the user file declares or imports.
fn sugar_alias_target(ctx: &QueryContext<'_>, root: Entity, alias: Entity) -> Option<Entity> {
    let annotation = ctx.get::<TypeAnnotation>(alias)?;
    let AstType::Named { segments, .. } = &annotation.0 else {
        return None;
    };
    let segments: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    match ctx.query(ResolveTypePath {
        segments,
        context: alias,
        root,
    }) {
        TypeResolution::Found(entity) => Some(entity),
        _ => None,
    }
}

/// Resolve a well-known standard library type name (e.g. "Array", "Optional").
fn resolve_std_type(
    ctx: &QueryContext<'_>,
    owner: Entity,
    root: Entity,
    name: &str,
) -> Option<Entity> {
    let result = ctx.query(ResolveTypePath {
        segments: vec![name.to_string()],
        context: owner,
        root,
    });
    match result {
        TypeResolution::Found(entity) => Some(entity),
        _ => None,
    }
}
