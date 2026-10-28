//! kestrel-ast-builder: Walks a rowan CST and creates declaration entities
//! with components in the ECS world.
//!
//! Replaces `kestrel-semantic-tree-builder` from lib1. Declarations only —
//! expressions are deferred. Components describe capabilities (what an entity
//! CAN DO) and are orthogonal and composable.

pub mod arg_binding;
pub mod ast_type;
pub mod build;
pub mod builders;
pub mod components;
pub mod lang_module;
pub mod syntax;

// Re-export the type syntax declarations are built from.
pub use kestrel_ast::{AstType, FnTypeKind, PathSegment};

pub use build::build_declarations;
pub use components::*;
pub use lang_module::seed_lang_module;
