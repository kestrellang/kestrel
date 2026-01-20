//! Tests for pattern matching.
//!
//! This module contains tests for all pattern matching features:
//! - Match expressions
//! - Pattern types (wildcard, binding, tuple, literal, enum)
//! - Let/var destructuring patterns
//! - If-let expressions
//! - Guard-let statements
//! - While-let expressions
//! - Exhaustiveness and irrefutability checking

mod exhaustiveness;
mod guard_let;
mod if_let;
mod let_destructuring;
mod match_expressions;
mod pattern_types;
mod while_let;
