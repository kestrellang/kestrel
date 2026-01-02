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

pub mod target;
pub mod mangle;
pub mod layout;

pub use target::TargetConfig;
pub use mangle::{mangle_name, Mangler};
pub use layout::{Layout, LayoutCache};
