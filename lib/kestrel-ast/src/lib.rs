//! kestrel-ast: syntax-level data shared across the front end.
//!
//! Type syntax (`AstType`, the types written in declarations, annotations and
//! expressions), the operator enums, the escape table, and the arena HIR
//! stores its nodes in. Function bodies have no AST: `kestrel-hir-lower`
//! lowers them straight from the CST. No CST or parser dependencies —
//! downstream consumers can depend on this crate without pulling in the
//! syntax tree.

pub mod arena;
pub mod ast_type;
pub mod escape;
pub mod ops;
pub mod pretty;

pub use arena::{Arena, Idx};
pub use ast_type::{AstType, FnTypeKind, ParamConvention, PathSegment};
pub use ops::{BinaryOp, CompoundAssignOp, PostfixOp, UnaryOp};
