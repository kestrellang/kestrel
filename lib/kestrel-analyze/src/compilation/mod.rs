//! Compilation-level analyzers -- whole-compilation checks on all entities.
//!
//! Each analyzer is a stateless ZST that implements `Describe + CompilationCheck`.
//! These run once per compilation, not per entity.

pub mod conformance_completeness;
pub mod constraint_cycles;
pub mod cycle_util;
pub mod extension_conflict;
pub mod protocol_cycles;
pub mod struct_cycles;
pub mod type_alias_cycles;
pub mod type_annotation_resolution;
pub mod unknown_attribute;
