//! Function body resolution.
//!
//! This module handles resolving function bodies from syntax to semantic
//! representations (Expression, Statement, CodeBlock). It runs during the
//! bind phase after all symbols have been created.
//!
//! # Module Structure
//!
//! - [`context`]: Resolution context and entry points
//! - [`statements`]: Statement resolution (let/var bindings, expression statements)
//! - [`expressions`]: Core expression resolution dispatcher
//! - [`calls`]: Function calls, method calls, and struct instantiation
//! - [`members`]: Member access and method resolution on types
//! - [`operators`]: Unary/binary operators and Pratt parsing
//! - [`paths`]: Path expression resolution (variables, functions, qualified names)
//! - [`utils`]: Shared utilities (type formatting, signature matching)

mod calls;
mod context;
mod expressions;
mod members;
mod operators;
mod paths;
mod statements;
mod utils;

// Re-export main public interface
pub use context::{BodyResolutionContext, resolve_and_attach_body, resolve_function_body};
