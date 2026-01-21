//! Builtin language feature tests
//!
//! Tests for the `@builtin(.Feature)` attribute system.
//!
//! Phase 3 of the memory model implementation introduces:
//! - `@builtin(.Feature)` syntax for marking language builtins
//! - `BuiltinRegistry` for tracking builtin protocols, structs, enums, functions
//! - Validation that builtins are applied to correct symbol kinds
//!
//! Also includes tests for:
//! - Literal protocols (ExpressibleByIntegerLiteral, etc.)
//! - Matchable protocol for custom pattern matching
//! - Formattable protocol for string formatting
//! - BooleanConditional protocol for custom conditionals

mod boolean_conditional;
mod intrinsics;
mod literal_protocols;
mod matchable;
mod protocols;
