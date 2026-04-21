//! Backend-agnostic code generation infrastructure for Kestrel (lib2).
//!
//! Shared utilities used by all code generation backends:
//! - **Target configuration** — host detection and target triple handling
//! - **Type layout** — size, alignment, and field offset computation
//! - **Name mangling** — unique linker-safe symbol names
//!
//! Consumes `MirModule` from `kestrel-mir`. Individual backends (like
//! a future Cranelift backend) use these utilities to generate target-specific code.

pub mod layout;
pub mod mangle;
pub mod target;

pub use layout::{
    DetailedStructLayout, Layout, LayoutCache, NamedKind, substitute_type, substitute_type_with_self,
};
pub use mangle::{Mangler, mangle_function, mangle_function_with_self, mangle_name};
pub use target::TargetConfig;
