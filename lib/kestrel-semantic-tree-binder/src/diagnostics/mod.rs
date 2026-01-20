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
//! - `deinit` - Deinit declaration errors and warnings
//! - `member_access` - Member access errors
//! - `call` - Function and method call errors
//! - `operators` - Operator resolution errors
//! - `struct_init` - Struct initialization errors
//! - `control_flow` - Break/continue/label errors
//! - `pattern` - Pattern matching errors
//! - `move_tracking` - Use-after-move errors for non-copyable types

mod access_mode;
mod attributes;
mod builtins;
mod call;
mod control_flow;
mod copy_semantics;
mod declaration;
mod deinit;
mod extern_fn;
mod member_access;
mod module;
mod move_tracking;
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
pub use copy_semantics::*;
pub use declaration::*;
pub use deinit::*;
pub use extern_fn::*;
pub use member_access::*;
pub use module::*;
pub use move_tracking::*;
pub use operators::*;
pub use pattern::*;
pub use protocol::*;
pub use struct_init::*;
pub use type_resolution::*;
