//! Data types for tracking monomorphized instantiations.
//!
//! Each unique combination of (function, type_args, self_type) becomes a
//! separate compiled function in the output. `IndexSet` ensures deterministic
//! iteration order for reproducible codegen.

use indexmap::IndexSet;
use kestrel_mir::{FunctionId, MirTy};

/// A specific instantiation of a (possibly generic) function.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionInstantiation {
    /// Index into `MirModule.functions`.
    pub func_id: FunctionId,
    /// Concrete type arguments (empty for non-generic functions).
    pub type_args: Vec<MirTy>,
    /// Concrete Self type for protocol extension methods.
    pub self_type: Option<MirTy>,
}

impl FunctionInstantiation {
    /// Non-generic function instantiation.
    pub fn concrete(func_id: FunctionId) -> Self {
        Self {
            func_id,
            type_args: Vec::new(),
            self_type: None,
        }
    }

    /// Generic function instantiation.
    pub fn generic(func_id: FunctionId, type_args: Vec<MirTy>) -> Self {
        Self {
            func_id,
            type_args,
            self_type: None,
        }
    }

    /// Generic function with explicit Self type.
    pub fn with_self(func_id: FunctionId, type_args: Vec<MirTy>, self_type: MirTy) -> Self {
        Self {
            func_id,
            type_args,
            self_type: Some(self_type),
        }
    }
}

/// The set of all function instantiations discovered during collection.
///
/// Uses `IndexSet` for deterministic iteration order, ensuring reproducible
/// object code output.
#[derive(Debug, Clone)]
pub struct MonomorphizationSet {
    pub functions: IndexSet<FunctionInstantiation>,
}

impl MonomorphizationSet {
    pub fn new() -> Self {
        Self {
            functions: IndexSet::new(),
        }
    }
}

impl Default for MonomorphizationSet {
    fn default() -> Self {
        Self::new()
    }
}
