//! Diagnostic error types for the semantic tree builder.
//!
//! This module provides structured error types that implement `IntoDiagnostic`,
//! organized by category:
//!
//! - `access_mode` - Parameter access mode validation errors (mutating/consuming)
//! - `module` - Module declaration errors
//! - `type_resolution` - Type lookup and generic instantiation errors
//! - `protocol` - Protocol binding and associated type errors
//! - `declaration` - Declaration binding errors
//! - `member_access` - Member access errors
//! - `call` - Function and method call errors
//! - `operators` - Operator resolution errors
//! - `struct_init` - Struct initialization errors
//! - `control_flow` - Break/continue/label errors
//! - `pattern` - Pattern matching errors

mod access_mode;
mod attributes;
mod builtins;
mod call;
mod control_flow;
mod declaration;
mod member_access;
mod module;
mod operators;
mod pattern;
mod protocol;
mod struct_init;
mod type_resolution;

pub use access_mode::*;
pub use attributes::*;
pub use builtins::*;
pub use call::*;
pub use control_flow::*;
pub use declaration::*;
pub use member_access::*;
pub use module::*;
pub use operators::*;
pub use pattern::*;
pub use protocol::*;
pub use struct_init::*;
pub use type_resolution::*;
