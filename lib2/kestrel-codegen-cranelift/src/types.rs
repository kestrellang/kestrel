//! MirTy → Cranelift type translation.
//!
//! Maps Kestrel's MIR types to Cranelift's scalar types. Aggregate types
//! (struct, tuple, str, thick function) are always passed by pointer, so
//! they translate to the pointer type.

use crate::common;
use cranelift_codegen::ir;
use kestrel_codegen2::TargetConfig;
use kestrel_mir::MirTy;

/// Translate a MirTy to its Cranelift type representation.
///
/// Aggregate types translate to the pointer type (they're always
/// passed by pointer, not by value in registers).
pub fn translate_type(ty: &MirTy, target: &TargetConfig) -> ir::Type {
    let ptr = common::ptr_type(target);

    match ty {
        MirTy::I8 | MirTy::Bool => ir::types::I8,
        MirTy::I16 => ir::types::I16,
        MirTy::I32 => ir::types::I32,
        MirTy::I64 => ir::types::I64,
        MirTy::F16 => ir::types::F16,
        MirTy::F32 => ir::types::F32,
        MirTy::F64 => ir::types::F64,

        // Zero-size types still need a register representation
        MirTy::Unit | MirTy::Never => ir::types::I8,

        // All pointer-like types use the pointer type
        MirTy::Pointer(_) | MirTy::Ref(_) | MirTy::RefMut(_) => ptr,

        // Thin function pointer
        MirTy::FuncThin { .. } => ptr,

        // Aggregate types: passed by pointer
        MirTy::Named { .. } | MirTy::Tuple(_) | MirTy::Str | MirTy::FuncThick { .. } => ptr,

        // Type params should be resolved by monomorphization
        MirTy::TypeParam(_) | MirTy::SelfType => ptr,

        // Associated types should be resolved before codegen
        MirTy::AssociatedProjection { .. } => ptr,

        MirTy::Error => ir::types::I8,
    }
}

/// Convert IntBits to a Cranelift integer type.
pub fn int_bits_to_type(bits: kestrel_mir::IntBits) -> ir::Type {
    match bits {
        kestrel_mir::IntBits::I8 => ir::types::I8,
        kestrel_mir::IntBits::I16 => ir::types::I16,
        kestrel_mir::IntBits::I32 => ir::types::I32,
        kestrel_mir::IntBits::I64 => ir::types::I64,
    }
}

/// Convert FloatBits to a Cranelift float type.
pub fn float_bits_to_type(bits: kestrel_mir::FloatBits) -> ir::Type {
    match bits {
        kestrel_mir::FloatBits::F16 => ir::types::F16,
        kestrel_mir::FloatBits::F32 => ir::types::F32,
        kestrel_mir::FloatBits::F64 => ir::types::F64,
    }
}
