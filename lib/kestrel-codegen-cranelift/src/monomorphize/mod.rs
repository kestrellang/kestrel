//! Monomorphization for generic code.
//!
//! This module implements monomorphization - the process of turning generic
//! code into concrete instantiations. Kestrel uses monomorphization rather
//! than type erasure, meaning each unique instantiation of a generic item
//! (e.g., `identity[Int]`, `identity[Bool]`) becomes a separate compiled entity.
//!
//! # Overview
//!
//! The monomorphization process has two phases:
//!
//! 1. **Collection**: BFS discovers all concrete instantiations needed
//! 2. **Substitution**: During codegen, type parameters are substituted
//!
//! ```text
//! MirContext (generic)
//!     │
//!     ▼ collect_all()
//! MonomorphizationSet { functions, structs, enums }
//!     │
//!     ▼ for each instantiation: compile with substitution
//! Cranelift IR (monomorphized)
//! ```
//!
//! # Example
//!
//! Given:
//! ```kestrel
//! func identity[T](x: T) -> T { x }
//! func main() { identity(42) }
//! ```
//!
//! Collection discovers `identity[Int]`. During codegen, we compile
//! `identity` with substitution `{T → Int}`, producing a concrete function.

mod collect;
mod error;
mod instantiation;
mod substitute;
mod witness;

pub use collect::collect_all;
pub use error::MonomorphizeError;
pub use instantiation::{
    EnumInstantiation, FunctionInstantiation, MonomorphizationSet, StructInstantiation,
};
pub use substitute::{build_substitution, Substitution};
pub use witness::{resolve_associated_type, resolve_witness};
