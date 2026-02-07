//! Kestrel Language Server Protocol implementation.
//!
//! This crate provides an LSP server for the Kestrel programming language,
//! enabling IDE features like diagnostics in editors that support LSP.

pub mod backend;
pub mod diagnostics;
pub mod position;

pub use backend::Backend;
