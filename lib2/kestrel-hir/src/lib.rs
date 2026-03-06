//! HIR (High-level Intermediate Representation) data types.
//!
//! Desugared, partially-resolved representation where:
//! - All syntactic sugar is expanded (operators → protocol calls, for → loop, etc.)
//! - Scope-resolvable names are resolved to entities/locals
//! - Type-dependent names (methods, fields) remain as strings for type inference
//! - Types are resolved to entities

pub mod body;
pub mod builtin;
pub mod res;
pub mod ty;

pub use body::*;
pub use builtin::Builtin;
pub use res::{Local, LocalId, Res};
pub use ty::HirTy;
