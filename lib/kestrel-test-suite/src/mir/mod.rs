//! MIR (Execution Graph) test expectations.
//!
//! This module provides structured assertions for testing MIR lowering.
//!
//! # Example
//!
//! ```ignore
//! use kestrel_test_suite::*;
//! use kestrel_test_suite::mir::*;
//!
//! #[test]
//! fn test_basic_function() {
//!     Test::new(r#"
//!         module Main
//!         func add(a: Int, b: Int) -> Int { a + b }
//!     "#)
//!     .expect(Compiles)
//!     .expect(Mir::compiles())
//!     .expect(Mir::mir_function("Main.add")
//!         .returns(MirTy::I64)
//!         .has_param("a", MirTy::I64)
//!         .has_param("b", MirTy::I64));
//! }
//! ```

mod types;
mod context;
mod compiles;
mod counts;
mod struct_;
mod enum_;
mod function;
mod block;
mod statement;
mod terminator;
mod witness;
mod protocol;

pub use types::MirTy;
pub use compiles::{MirCompiles, MirFails};
pub use counts::{MirEnumCount, MirFunctionCount, MirStructCount, MirWitnessCount};
pub use struct_::MirStruct;
pub use enum_::MirEnum;
pub use function::MirFunction;
pub use block::MirBlock;
pub use statement::StatementPattern;
pub use terminator::TerminatorPattern;
pub use witness::MirWitness;
pub use protocol::MirProtocol;

// Re-export useful types from kestrel-execution-graph
pub use kestrel_execution_graph::{BinOp, CastKind, PassingMode, UnOp};

/// Entry point for MIR expectations.
pub struct Mir;

impl Mir {
    /// Expect MIR lowering to succeed with no errors.
    pub fn compiles() -> MirCompiles {
        MirCompiles
    }

    /// Expect MIR lowering to fail.
    pub fn fails() -> MirFails {
        MirFails
    }

    /// Expect a struct definition exists in the MIR.
    pub fn mir_struct(name: &str) -> MirStruct {
        MirStruct::new(name)
    }

    /// Expect an enum definition exists in the MIR.
    pub fn mir_enum(name: &str) -> MirEnum {
        MirEnum::new(name)
    }

    /// Expect a function definition exists in the MIR.
    pub fn mir_function(name: &str) -> MirFunction {
        MirFunction::new(name)
    }

    /// Expect a closure function exists in the MIR.
    ///
    /// Auto-expands to the correct closure naming format.
    /// For `parent = "Module.func"` and `index = 0`, produces `Module."func.closure.0"`.
    pub fn mir_closure(parent: &str, index: usize) -> MirFunction {
        // Split parent into module and function parts
        // e.g., "Test.test" -> ("Test", "test")
        // e.g., "Main.Foo.bar" -> ("Main.Foo", "bar")
        if let Some(dot_pos) = parent.rfind('.') {
            let module = &parent[..dot_pos];
            let func = &parent[dot_pos + 1..];
            MirFunction::new(&format!("{}.\"{}\"", module, format!("{}.closure.{}", func, index)))
        } else {
            // No dot - just use parent as-is
            MirFunction::new(&format!("\"{}.closure.{}\"", parent, index))
        }
    }

    /// Expect a closure environment struct exists in the MIR.
    ///
    /// Auto-expands to the correct closure env naming format.
    /// For `parent = "Module.func"` and `index = 0`, produces `Module."func.closure.0.env"`.
    pub fn mir_closure_env(parent: &str, index: usize) -> MirStruct {
        if let Some(dot_pos) = parent.rfind('.') {
            let module = &parent[..dot_pos];
            let func = &parent[dot_pos + 1..];
            MirStruct::new(&format!("{}.\"{}\"", module, format!("{}.closure.{}.env", func, index)))
        } else {
            MirStruct::new(&format!("\"{}.closure.{}.env\"", parent, index))
        }
    }

    /// Expect a witness table exists in the MIR.
    pub fn mir_witness(impl_type: &str, protocol: &str) -> MirWitness {
        MirWitness::new(impl_type, protocol)
    }

    /// Expect a protocol definition exists in the MIR.
    pub fn mir_protocol(name: &str) -> MirProtocol {
        MirProtocol::new(name)
    }

    /// Expect exactly N structs in the MIR.
    pub fn struct_count(n: usize) -> MirStructCount {
        MirStructCount(n)
    }

    /// Expect exactly N enums in the MIR.
    pub fn enum_count(n: usize) -> MirEnumCount {
        MirEnumCount(n)
    }

    /// Expect exactly N functions in the MIR.
    pub fn function_count(n: usize) -> MirFunctionCount {
        MirFunctionCount(n)
    }

    /// Expect exactly N witnesses in the MIR.
    pub fn witness_count(n: usize) -> MirWitnessCount {
        MirWitnessCount(n)
    }
}
