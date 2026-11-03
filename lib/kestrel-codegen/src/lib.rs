//! Backend-agnostic code generation infrastructure for Kestrel (lib).
//!
//! Re-exports the pipeline-wide [`TargetConfig`] (defined in `kestrel-target`)
//! for the backends. Type layout and name mangling live in the MIR pipeline
//! crates; this crate is intentionally minimal.

pub use kestrel_target::TargetConfig;
