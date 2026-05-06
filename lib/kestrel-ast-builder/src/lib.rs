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
pub mod lang_module;
pub mod lower;
pub mod string_token;

// Re-export kestrel-ast types for backward compatibility
pub use kestrel_ast::arena;
pub use kestrel_ast::ast_body;
pub use kestrel_ast::ast_body::*;
pub use kestrel_ast::{Arena, AstType, Idx, PathSegment};

pub use build::build_declarations;
pub use components::*;
pub use lang_module::seed_lang_module;
