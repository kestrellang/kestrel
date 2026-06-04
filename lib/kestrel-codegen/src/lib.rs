//! Backend-agnostic code generation infrastructure for Kestrel (lib).
//!
//! Shared target configuration — host detection and target triple handling —
//! used by the Cranelift backend. Type layout and name mangling now live in
//! the MIR pipeline crates; this crate is intentionally minimal.

pub mod target;

pub use target::TargetConfig;
