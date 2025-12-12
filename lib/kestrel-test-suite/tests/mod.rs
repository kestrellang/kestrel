//! Kestrel Test Suite
//!
//! This test suite is organized by semantic domain:
//!
//! - `declarations/` - Symbol declarations (structs, functions, protocols, type aliases, imports)
//! - `types/` - Type system (generics, literals)
//! - `expressions/` - Expression resolution (literals, operators, paths, calls, field access)
//! - `statements/` - Statement resolution (variables, assignments)
//! - `validation/` - Semantic validation (mutability, cycles, visibility, duplicates, conformance)
//! - `instantiation/` - Creating instances of types
//! - `framework/` - Test framework features

mod declarations;
mod expressions;
mod framework;
mod instantiation;
mod statements;
mod types;
mod validation;
