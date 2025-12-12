//! Syntax tree lowering (build phase) for Kestrel.
//!
//! Phase 1: This crate is a thin façade that produces a `SemanticModel` from syntax trees
//! using the existing declaration-building logic.

pub mod builder;
pub mod builders;
mod lowerer;

pub use lowerer::{BuildFile, build};
