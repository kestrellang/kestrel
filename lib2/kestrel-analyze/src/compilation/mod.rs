//! Compilation-level analyzers -- whole-compilation checks on all entities.
//!
//! Each analyzer is a stateless ZST that implements `Describe + CompilationCheck`.
//! These run once per compilation, not per entity.

pub mod conformance_completeness;
pub mod constraint_cycles;
pub mod extension_conflict;
pub mod struct_cycles;
pub mod type_alias_cycles;
