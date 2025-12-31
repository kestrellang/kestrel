//! Memory model tests.
//!
//! Tests for Kestrel's memory model, including:
//! - Parameter access modes (borrow, mutating, consuming)
//! - Copy semantics (Copyable, not Copyable)
//! - Drop semantics (deinit) [future]
//! - Law of exclusivity [future]

mod copy_semantics;
mod negative_conformance;
mod parameter_access_modes;
