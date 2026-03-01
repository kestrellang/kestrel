//! Query trait for semantic model queries
//!
//! Queries are pure functions: same model state + same inputs = same output.
//! See query-style-guide.md for conventions.

use std::hash::Hash;

use crate::SemanticModel;

/// Trait for semantic queries.
///
/// Queries are pure functions: same model state + same inputs = same output.
/// Results are memoized — repeated calls with the same inputs return cached results.
/// See query-style-guide.md for conventions.
pub trait Query: Hash + Eq + Clone + 'static {
    /// The output type of this query.
    type Output: Clone;

    /// Execute this query against the semantic model.
    fn execute(self, model: &SemanticModel) -> Self::Output;
}
