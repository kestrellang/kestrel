//! Tests for expression resolution
//!
//! This module contains tests for all expression forms:
//! - Literal expressions in function bodies
//! - Binary and unary operators
//! - Path expressions and variable references
//! - Function and method calls
//! - Field access on structs
//! - Control flow (if/else, while/loop, break/continue, return)

mod body_literals;
mod calls;
mod control_flow;
mod field_access;
mod loops;
mod operators;
mod paths;
mod returns;
