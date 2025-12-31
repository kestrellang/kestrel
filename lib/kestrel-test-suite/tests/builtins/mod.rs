//! Builtin language feature tests
//!
//! Tests for the `@builtin(.Feature)` attribute system.
//!
//! Phase 3 of the memory model implementation introduces:
//! - `@builtin(.Feature)` syntax for marking language builtins
//! - `BuiltinRegistry` for tracking builtin protocols, structs, enums, functions
//! - Validation that builtins are applied to correct symbol kinds

mod protocols;
