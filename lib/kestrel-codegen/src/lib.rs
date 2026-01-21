//! Backend-agnostic code generation infrastructure for Kestrel.
//!
//! This crate provides shared utilities used by all code generation backends:
//! - Target configuration and host detection
//! - Name mangling for linker symbols
//! - Type layout calculation (size and alignment)
//! - Generic instantiation collection for monomorphization
//!
//! Individual backends (like `kestrel-codegen-cranelift`) use these utilities
//! to generate target-specific code.

pub mod layout;
pub mod mangle;
pub mod target;

pub use layout::{Layout, LayoutCache};
pub use mangle::{Mangler, mangle_function, mangle_function_with_self, mangle_name};
pub use target::TargetConfig;
