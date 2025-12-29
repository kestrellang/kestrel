//! Execution Graph (MIR) tests.
//!
//! This module contains tests for the MIR lowering phase of the Kestrel compiler.
//! Tests are organized by feature:
//!
//! - `basic.rs` - Basic functions, arithmetic, simple control flow (12 tests)
//! - `closures.rs` - Closure generation and captures (21 tests)
//! - `control_flow.rs` - Loops, break/continue, early returns, nested control flow (15 tests)
//! - `enums.rs` - Enum definitions, payloads, recursive enums, match expressions (18 tests)
//! - `generics.rs` - Generic types, functions, methods, and enums (16 tests)
//! - `match_.rs` - Pattern matching: if-let, guard-let, while-let, tuple patterns (15 tests)
//! - `protocols.rs` - Protocols, witnesses, conformances, witness method calls (36 tests)
//! - `structs.rs` - Struct definitions, methods, initializers, field access (26 tests)
//!
//! Total: 159 tests

mod basic;
mod closures;
mod control_flow;
mod enums;
mod generics;
mod match_;
mod protocols;
mod structs;
