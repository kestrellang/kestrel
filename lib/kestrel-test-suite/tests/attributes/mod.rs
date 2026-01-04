//! Attribute system tests
//!
//! Tests for the `@attribute` syntax and semantic processing.
//!
//! Phase 2 of the memory model implementation introduces:
//! - `@attribute` syntax for annotating declarations
//! - `@attribute(args)` syntax with labeled and unlabeled arguments
//! - `AttributesBehavior` to store resolved attributes on symbols
//! - Warning for unknown attributes

mod declarations;
mod parsing;
mod semantic;
