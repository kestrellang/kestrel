//! Common parser combinators, emitters, and data structures shared across multiple parsers
//!
//! This module contains reusable components to eliminate code duplication:
//!
//! - `parsers`: Chumsky parser combinators for common patterns
//! - `emitters`: Event emission functions for building syntax trees
//! - `data`: Shared data structures used by multiple parsers

pub mod data;
pub mod emitters;
pub mod parsers;

// Re-export internal types for use within the crate
pub(crate) use data::*;
pub(crate) use emitters::*;

// Re-export public parsers
pub use parsers::*;
