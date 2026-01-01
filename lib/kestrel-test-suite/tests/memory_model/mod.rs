//! Memory model tests.
//!
//! Tests for Kestrel's memory model, including:
//! - Parameter access modes (borrow, mutating, consuming)
//! - Copy semantics (Copyable, not Copyable)
//! - Cloneable protocol (custom copy via clone())
//! - Drop semantics (deinit)
//! - Generic copyability (where T: not Copyable)
//! - Law of exclusivity [future]

mod cloneable;
mod copy_semantics;
mod deinit;
mod generic_copyability;
mod negative_conformance;
mod parameter_access_modes;
