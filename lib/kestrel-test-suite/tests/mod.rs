//! Kestrel Test Suite
//!
//! This test suite is organized by semantic domain:
//!
//! - `attributes/` - Attribute system (@attribute syntax and semantics)
//! - `declarations/` - Symbol declarations (structs, functions, protocols, type aliases, imports)
//! - `types/` - Type system (generics, literals)
//! - `expressions/` - Expression resolution (literals, operators, paths, calls, field access)
//! - `statements/` - Statement resolution (variables, assignments)
//! - `patterns/` - Pattern matching (match, if-let, guard-let, while-let, destructuring)
//! - `validation/` - Semantic validation (mutability, cycles, visibility, duplicates, conformance)
//! - `instantiation/` - Creating instances of types
//! - `inference/` - Type inference
//! - `framework/` - Test framework features

mod attributes;
mod declarations;
mod execution_graph;
mod expressions;
mod framework;
mod inference;
mod instantiation;
mod memory_model;
mod patterns;
mod statements;
mod types;
mod validation;
