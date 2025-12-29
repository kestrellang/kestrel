//! Pattern matching analysis for Kestrel.
//!
//! This crate provides algorithms for analyzing patterns in the Kestrel compiler:
//!
//! - **Irrefutability checking**: Determine if a pattern always matches any value of its type.
//!   Used for validating `let`/`var` bindings and `for` loop patterns.
//!
//! - **Exhaustiveness checking**: Determine if a set of patterns covers all possible values.
//!   Used for validating `match` expressions. Based on Maranget's algorithm.
//!
//! - **Usefulness analysis**: Determine if a pattern can match something not already matched
//!   by previous patterns. Used to detect unreachable match arms.
//!
//! # Algorithm
//!
//! This implementation uses Luc Maranget's pattern matrix algorithm from
//! "Warnings for pattern matching" (JFP 2007). The key concepts are:
//!
//! - **Pattern Matrix**: Patterns represented as a matrix where each row is a match arm
//!   and each column corresponds to a component of the scrutinee type.
//!
//! - **Usefulness**: A pattern `p` is *useful* with respect to a pattern matrix `P` if
//!   there exists a value that matches `p` but NOT any row in `P`.
//!
//! - **Exhaustiveness**: A match is exhaustive if the wildcard pattern `_` is NOT useful.
//!
//! - **Redundancy**: A pattern arm is redundant if it is NOT useful with respect to
//!   the preceding arms.
//!
//! - **Specialization**: The S(c, P) operation narrows the matrix to rows matching
//!   constructor `c`, expanding sub-patterns.
//!
//! # Example
//!
//! ```ignore
//! use kestrel_semantic_pattern_matching::{is_irrefutable, check_exhaustiveness};
//!
//! // Check if a let binding pattern is irrefutable
//! let irrefutable = is_irrefutable(&pattern);
//!
//! // Check if match arms are exhaustive
//! let result = check_exhaustiveness(&patterns, &scrutinee_type);
//! ```

mod constructor;
mod exhaustiveness;
mod irrefutability;
mod matrix;
mod usefulness;
mod witness;

pub use constructor::Constructor;
pub use exhaustiveness::{check_exhaustiveness, ExhaustivenessChecker, ExhaustivenessResult};
pub use irrefutability::is_irrefutable;
pub use matrix::{PatternMatrix, PatternRow};
pub use usefulness::{is_useful, is_useful_impl, UsefulnessResult};
pub use witness::Witness;
