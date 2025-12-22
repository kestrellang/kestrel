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
//! # Example
//!
//! ```ignore
//! use kestrel_semantic_pattern_matching::{is_irrefutable, check_exhaustiveness};
//!
//! // Check if a let binding pattern is irrefutable
//! let irrefutable = is_irrefutable(&pattern);
//!
//! // Check if match arms are exhaustive
//! let result = check_exhaustiveness(&patterns, &scrutinee_type, &type_context);
//! ```

mod exhaustiveness;
mod irrefutability;
mod usefulness;
mod witness;

pub use exhaustiveness::{check_exhaustiveness, ExhaustivenessChecker, ExhaustivenessResult};
pub use irrefutability::is_irrefutable;
pub use usefulness::is_useful;
pub use witness::Witness;
