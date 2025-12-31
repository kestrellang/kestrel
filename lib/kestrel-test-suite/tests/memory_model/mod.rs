//! Memory model tests.
//!
//! Tests for Kestrel's memory model, including:
//! - Parameter access modes (borrow, mutating, consuming)
//! - Copy semantics (Copyable, not Copyable) [future]
//! - Drop semantics (deinit) [future]
//! - Law of exclusivity [future]

mod parameter_access_modes;
