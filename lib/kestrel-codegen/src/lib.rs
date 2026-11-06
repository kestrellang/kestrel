//! Backend-agnostic code generation logic shared by the Cranelift and LLVM
//! backends.
//!
//! Anything that decides *what* a value looks like at the machine level — how a
//! MIR type is represented, how it is passed and returned, where a field lives
//! — is written once here, generic over the backend's scalar type. Each backend
//! only maps the result onto its own IR. Two copies of these rules would let
//! `-O0` (Cranelift) and `-O1`/`-O2` (LLVM) disagree about the same type.

pub mod abi;
pub mod layout;
pub mod repr;

pub use abi::{PassMode, ReturnMode, param_pass_mode, return_mode};
pub use kestrel_target::TargetConfig;
pub use repr::{ScalarModel, TypeCache, TypeRepr, ref_slot_loads_pointee};
