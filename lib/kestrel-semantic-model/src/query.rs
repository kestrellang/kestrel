//! Query trait for semantic model queries
//!
//! Queries are pure functions: same model state + same inputs = same output.
//! See query-style-guide.md for conventions.

use crate::SemanticModel;

/// Trait for semantic queries.
///
/// Queries are pure functions: same model state + same inputs = same output.
/// See query-style-guide.md for conventions.
pub trait Query {
    /// The output type of this query.
    type Output;

    /// Execute this query against the semantic model.
    fn execute(self, model: &SemanticModel) -> Self::Output;
}
