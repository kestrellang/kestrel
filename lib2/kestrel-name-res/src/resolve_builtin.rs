//! Builtin type/protocol resolution.
//!
//! Resolves well-known types and protocols (operator protocols, literal protocols,
//! etc.) to their entity IDs. Uses the root module as context so auto-imports
//! from std are always available, regardless of caller scope.

use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir::Builtin;

use crate::resolve_type::{ResolveTypePath, TypeResolution};

/// Query: resolve a builtin type/protocol to its entity.
///
/// Always resolves from the root module's context, ensuring all std types
/// are visible via auto-imports. Query caching means each builtin is
/// resolved at most once per revision.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ResolveBuiltin {
    pub builtin: Builtin,
    pub root: Entity,
}

impl QueryFn for ResolveBuiltin {
    type Output = Option<Entity>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Entity> {
        let result = ctx.query(ResolveTypePath {
            segments: vec![self.builtin.name().to_string()],
            context: self.root,
            root: self.root,
        });
        match result {
            TypeResolution::Found(entity) => Some(entity),
            _ => None,
        }
    }
}
