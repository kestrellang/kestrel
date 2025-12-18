//! Tests for semantic validation
//!
//! This module contains tests for:
//! - Mutability checking (let vs var)
//! - Circular reference detection
//! - Visibility consistency
//! - Duplicate symbol detection
//! - Protocol conformance validation
//! - Initializer verification (field initialization, control flow)
//! - Dead code detection
//! - Type checking

mod cycles;
mod dead_code;
mod exhaustive_return;
mod initializers;
mod misc;
mod mutability;
mod type_checking;
mod visibility;
