//! Monomorphization — discovers all concrete function instantiations.
//!
//! Generic functions are compiled once per unique (func, type_args, self_type)
//! combination. This module performs BFS from non-generic entry points to
//! discover all needed instantiations.

pub mod collect;
pub mod error;
pub mod instantiation;
pub mod witness;

pub use collect::collect_all;
pub use error::MonomorphizeError;
pub use instantiation::{FunctionInstantiation, MonomorphizationSet};
