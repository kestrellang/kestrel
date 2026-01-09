//! Tests for symbol declarations
//!
//! This module contains tests for all top-level declaration forms:
//! - Structs and their fields
//! - Functions and methods
//! - Protocols and inheritance
//! - Type aliases
//! - Associated types
//! - Import statements
//! - Extensions with conformances
//! - Enums and cases

mod associated_types;
mod enums;
mod extensions;
mod extern_functions;
mod functions;
mod imports;
mod protocol_method_linking;
mod protocols;
mod structs;
mod type_aliases;
mod wacky_inference;