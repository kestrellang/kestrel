//! kestrel-ast-builder: Walks a rowan CST and creates declaration entities
//! with components in the ECS world.
//!
//! Replaces `kestrel-semantic-tree-builder` from lib1. Declarations only —
//! expressions are deferred. Components describe capabilities (what an entity
//! CAN DO) and are orthogonal and composable.

pub mod ast_type;
pub mod build;
pub mod builders;
pub mod components;

pub use ast_type::{AstType, PathSegment};
pub use build::build_declarations;
pub use components::*;
