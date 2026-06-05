//! kestrel-ast: Pure AST data types for the Kestrel compiler.
//!
//! Contains arena infrastructure, expression/pattern/statement enums,
//! and type representations. No CST or parser dependencies — downstream
//! consumers can depend on this crate without pulling in the syntax tree.

pub mod arena;
pub mod ast_body;
pub mod ast_type;
pub mod pretty;

pub use arena::{Arena, Idx};
pub use ast_body::*;
pub use ast_type::{AstType, ParamConvention, PathSegment};
