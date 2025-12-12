//! Diagnostic error types for the semantic tree builder.
//!
//! This module provides structured error types that implement `IntoDiagnostic`,
//! organized by category:
//!
//! - `module` - Module declaration errors
//! - `type_resolution` - Type lookup and generic instantiation errors
//! - `protocol` - Protocol conformance and inheritance errors
//! - `visibility` - Visibility consistency errors
//! - `declaration` - Duplicate symbols and missing body errors
//! - `member_access` - Member access errors
//! - `call` - Function and method call errors
//! - `assignment` - Assignment validation errors
//! - `operators` - Operator resolution errors
//! - `cycles` - Cycle detection errors (struct containment, generic constraints)
//! - `type_check` - Type checking errors (type mismatches)

mod assignment;
mod call;
mod control_flow;
mod cycles;
mod declaration;
mod member_access;
mod module;
mod operators;
mod protocol;
mod struct_init;
mod type_check;
mod type_resolution;
mod visibility;

pub use assignment::*;
pub use call::*;
pub use control_flow::*;
pub use cycles::*;
pub use declaration::*;
pub use member_access::*;
pub use module::*;
pub use operators::*;
pub use protocol::*;
pub use struct_init::*;
pub use type_check::*;
pub use type_resolution::*;
pub use visibility::*;
