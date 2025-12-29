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

mod match_expressions;
mod pattern_types;
mod let_destructuring;
mod if_let;
mod guard_let;
mod while_let;
mod exhaustiveness;
