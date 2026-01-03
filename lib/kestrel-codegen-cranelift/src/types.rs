//! MIR to Cranelift type translation.

use crate::monomorphize::Substitution;
use kestrel_codegen::TargetConfig;
use kestrel_execution_graph::{Id, MirContext, MirTy, Ty};

use cranelift_codegen::ir::types as cl_types;
use cranelift_codegen::ir::Type as CraneliftType;

/// Translate a MIR type to a Cranelift type.
///
/// Note: Compound types (structs, tuples) are passed by pointer,
/// so they translate to pointer type.
pub fn translate_type(ctx: &MirContext, ty: Id<Ty>, target: &TargetConfig) -> CraneliftType {
    let ptr_type = if target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    match ctx.ty(ty) {
        // Primitives
        MirTy::I8 => cl_types::I8,
        MirTy::I16 => cl_types::I16,
        MirTy::I32 => cl_types::I32,
        MirTy::I64 => cl_types::I64,
        MirTy::F16 => cl_types::F16,
        MirTy::F32 => cl_types::F32,
        MirTy::F64 => cl_types::F64,
        MirTy::Bool => cl_types::I8,  // Bools are i8 in Cranelift
        MirTy::Unit => cl_types::I8,  // Unit is zero-sized, but we need something
        MirTy::Never => cl_types::I8, // Never is also placeholder

        // Pointers and references are pointer-sized
        MirTy::Pointer(_) | MirTy::Ref(_) | MirTy::RefMut(_) => ptr_type,

        // String is fat pointer - but when passed, it's by pointer to the struct
        MirTy::Str => ptr_type,

        // Arrays are thin pointers (for now)
        MirTy::Array(_) => ptr_type,

        // Compound types are passed by pointer
        MirTy::Tuple(_) | MirTy::Named { .. } => ptr_type,

        // Type parameters - resolved at monomorphization, use pointer for now
        MirTy::TypeParam(_) => ptr_type,

        // Function pointers
        MirTy::FuncThin { .. } => ptr_type,
        MirTy::FuncThick { .. } => ptr_type, // Actually two words, but passed by ptr

        // Protocol types
        MirTy::SelfType => ptr_type,
        MirTy::AssociatedTypeProjection { .. } => ptr_type,

        // Error - use pointer as fallback
        MirTy::Error => ptr_type,
    }
}

/// Check if a type should be passed by value (fits in a register).
pub fn is_pass_by_value(ctx: &MirContext, ty: Id<Ty>) -> bool {
    matches!(
        ctx.ty(ty),
        MirTy::I8
            | MirTy::I16
            | MirTy::I32
            | MirTy::I64
            | MirTy::F16
            | MirTy::F32
            | MirTy::F64
            | MirTy::Bool
            | MirTy::Unit
            | MirTy::Pointer(_)
            | MirTy::Ref(_)
            | MirTy::RefMut(_)
            | MirTy::FuncThin { .. }
    )
}

/// Translate a MIR type to a Cranelift type, applying substitution for type params.
pub fn translate_type_with_subst(
    ctx: &MirContext,
    ty: Id<Ty>,
    target: &TargetConfig,
    subst: &Substitution,
) -> CraneliftType {
    // Apply substitution first
    let concrete_ty = subst.apply_ty_readonly(ctx, ty).unwrap_or(ty);
    translate_type(ctx, concrete_ty, target)
}

/// Check if a type should be passed by value, applying substitution first.
pub fn is_pass_by_value_with_subst(ctx: &MirContext, ty: Id<Ty>, subst: &Substitution) -> bool {
    let concrete_ty = subst.apply_ty_readonly(ctx, ty).unwrap_or(ty);
    is_pass_by_value(ctx, concrete_ty)
}
