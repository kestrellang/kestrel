//! Hindley-Milner style constraint-based type inference for Kestrel.
//!
//! This crate provides a type inference system that:
//! - Collects type constraints during expression resolution
//! - Solves constraints using unification and fixpoint iteration
//! - Resolves type-directed member accesses and associated types
//!
//! # Architecture
//!
//! The inference system is decoupled from the semantic model via the [`TypeOracle`] trait.
//! This allows the solver to query type information without direct dependencies on
//! the semantic model's implementation details.
//!
//! # Usage
//!
//! ```ignore
//! use kestrel_semantic_type_inference::{InferenceContext, TypeOracle};
//!
//! // Create a context with an oracle implementation
//! let mut ctx = InferenceContext::new(&oracle);
//!
//! // Add constraints during expression resolution
//! ctx.equate(ty1.id(), ty2.id());
//! ctx.conforms(ty.id(), protocol_ref);
//! ctx.member_access(receiver.id(), "field", false, result.id(), expr.id());
//!
//! // Solve all constraints
//! let solution = ctx.solve()?;
//!
//! // Use solution to get resolved types
//! let resolved_ty = solution.types.get(&ty.id());
//! ```

mod apply;
mod constraint;
mod constraint_generator;
mod context;
mod error;
mod oracle;
mod solution;
mod solver;

pub use apply::{apply_solution, apply_solution_to_locals};
pub use constraint::{Constraint, ProtocolRef};
pub use constraint_generator::generate_constraints;
pub use context::InferenceContext;
pub use error::InferenceError;
pub use oracle::{MemberError, MemberResolution, TypeOracle};
pub use solution::{Solution, ValueResolution};
