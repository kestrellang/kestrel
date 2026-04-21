//! MirTy → Cranelift type translation.
//!
//! Maps Kestrel's MIR types to Cranelift's scalar types. Aggregate types
//! (struct, tuple, str, thick function) are always passed by pointer, so
//! they translate to the pointer type.

use crate::common;
use cranelift_codegen::ir;
use kestrel_codegen2::{TargetConfig, normalize_projection};
use kestrel_mir::MirTy;

/// Translate a MirTy to its Cranelift type representation.
///
/// Aggregate types translate to the pointer type (they're always
/// passed by pointer, not by value in registers). Small Named types
/// (like Bool, Int64 wrappers) are translated by their layout size.
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

        // Never (zero-size divergence) uses pointer type so it unifies in
        // phi nodes against aggregate pointers. Unit is the empty tuple and
        // is handled by the `Tuple(_)` arm below.
        MirTy::Never => ptr,

        // All pointer-like types use the pointer type
        MirTy::Pointer(_) | MirTy::Ref(_) | MirTy::RefMut(_) => ptr,

        // Thin function pointer
        MirTy::FuncThin { .. } => ptr,

        // Aggregate types: passed by pointer
        MirTy::Tuple(_) | MirTy::Str | MirTy::FuncThick { .. } => ptr,

        // Named types: large ones are aggregate (by pointer),
        // small ones are passed by value in a register
        MirTy::Named { .. } => ptr,

        // Type params should be resolved by monomorphization
        MirTy::TypeParam(_) | MirTy::SelfType => ptr,

        // Associated types should be resolved before codegen
        MirTy::AssociatedProjection { .. } => ptr,

        MirTy::Error => ir::types::I8,
    }
}

/// Layout-aware type translation for Named types. Uses the layout to determine
/// if a Named type should be passed by value (small) or by pointer (large).
pub fn translate_type_with_layout(
    ty: &MirTy,
    target: &TargetConfig,
    layouts: &mut kestrel_codegen2::LayoutCache,
) -> ir::Type {
    // Resolve any AssociatedProjection to its concrete bound type before
    // dispatching. Without this, a projection that normalizes to Int64
    // (etc.) would fall through the `_` arm and return `ptr`, disagreeing
    // with the layout-aware treatment Named types get.
    let normalized_storage;
    let ty = if matches!(ty, MirTy::AssociatedProjection { .. }) {
        normalized_storage = normalize_projection(ty, layouts.module());
        &normalized_storage
    } else {
        ty
    };

    match ty {
        MirTy::Named { .. } => {
            let layout = layouts.layout_of(ty);
            if layout.size <= 8 {
                // Small Named type — pick a register type matching the size
                match layout.size {
                    0 => common::ptr_type(target), // Unit-like
                    1 => ir::types::I8,
                    2 => ir::types::I16,
                    3..=4 => ir::types::I32,
                    5..=8 => ir::types::I64,
                    _ => common::ptr_type(target),
                }
            } else {
                common::ptr_type(target) // Large → by pointer
            }
        },
        _ => translate_type(ty, target),
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
