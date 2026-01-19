//! Instantiation types for monomorphization.
//!
//! These types represent concrete instantiations of generic items
//! (functions, structs, enums) with specific type arguments.

use indexmap::IndexSet;
use kestrel_execution_graph::{Enum, Function, Id, Struct, Ty};

/// A concrete instantiation of a generic function.
///
/// For example, `identity[Int]` where `identity` is defined as `func identity[T](x: T) -> T`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionInstantiation {
    /// The function being instantiated.
    pub func_id: Id<Function>,
    /// The concrete type arguments.
    /// Empty for non-generic functions.
    pub type_args: Vec<Id<Ty>>,
    /// The concrete type for `Self` (used for protocol extension methods).
    /// None for functions that don't use Self type.
    pub self_type: Option<Id<Ty>>,
}

impl FunctionInstantiation {
    /// Create a new function instantiation.
    pub fn new(func_id: Id<Function>, type_args: Vec<Id<Ty>>) -> Self {
        Self { func_id, type_args, self_type: None }
    }

    /// Create a new function instantiation with a Self type.
    pub fn with_self_type(func_id: Id<Function>, type_args: Vec<Id<Ty>>, self_type: Id<Ty>) -> Self {
        Self { func_id, type_args, self_type: Some(self_type) }
    }

    /// Create an instantiation for a non-generic function.
    pub fn non_generic(func_id: Id<Function>) -> Self {
        Self {
            func_id,
            type_args: Vec::new(),
            self_type: None,
        }
    }
}

/// A concrete instantiation of a generic struct.
///
/// For example, `Box[Int]` where `Box` is defined as `struct Box[T] { value: T }`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructInstantiation {
    /// The struct being instantiated.
    pub struct_id: Id<Struct>,
    /// The concrete type arguments.
    pub type_args: Vec<Id<Ty>>,
}

impl StructInstantiation {
    /// Create a new struct instantiation.
    pub fn new(struct_id: Id<Struct>, type_args: Vec<Id<Ty>>) -> Self {
        Self {
            struct_id,
            type_args,
        }
    }
}

/// A concrete instantiation of a generic enum.
///
/// For example, `Option[Int]` where `Option` is defined as `enum Option[T] { Some(T), None }`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumInstantiation {
    /// The enum being instantiated.
    pub enum_id: Id<Enum>,
    /// The concrete type arguments.
    pub type_args: Vec<Id<Ty>>,
}

impl EnumInstantiation {
    /// Create a new enum instantiation.
    pub fn new(enum_id: Id<Enum>, type_args: Vec<Id<Ty>>) -> Self {
        Self { enum_id, type_args }
    }
}

/// The set of all concrete instantiations discovered during collection.
///
/// This is computed by the collection phase and used during codegen
/// to know which instantiations need to be compiled.
///
/// Uses `IndexSet` instead of `HashSet` to ensure deterministic iteration order,
/// which makes error messages and codegen output reproducible across runs.
#[derive(Debug, Default)]
pub struct MonomorphizationSet {
    /// All function instantiations that need to be compiled.
    pub functions: IndexSet<FunctionInstantiation>,
    /// All struct instantiations that are used.
    pub structs: IndexSet<StructInstantiation>,
    /// All enum instantiations that are used.
    pub enums: IndexSet<EnumInstantiation>,
}

impl MonomorphizationSet {
    /// Create a new empty monomorphization set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a function instantiation is in the set.
    pub fn has_function(&self, inst: &FunctionInstantiation) -> bool {
        self.functions.contains(inst)
    }

    /// Add a function instantiation to the set.
    /// Returns true if it was newly inserted.
    pub fn add_function(&mut self, inst: FunctionInstantiation) -> bool {
        self.functions.insert(inst)
    }

    /// Add a struct instantiation to the set.
    /// Returns true if it was newly inserted.
    pub fn add_struct(&mut self, inst: StructInstantiation) -> bool {
        self.structs.insert(inst)
    }

    /// Add an enum instantiation to the set.
    /// Returns true if it was newly inserted.
    pub fn add_enum(&mut self, inst: EnumInstantiation) -> bool {
        self.enums.insert(inst)
    }
}
