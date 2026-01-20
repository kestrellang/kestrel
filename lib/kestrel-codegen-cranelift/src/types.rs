//! MIR to Cranelift type translation.

use crate::monomorphize::{Substitution, resolve_associated_type};
use kestrel_codegen::TargetConfig;
use kestrel_execution_graph::{Id, MirContext, MirTy, Ty};

use cranelift_codegen::ir::Type as CraneliftType;
use cranelift_codegen::ir::types as cl_types;

/// Resolve associated type projections in a type.
///
/// If the type is an `AssociatedTypeProjection`, resolve it via witness lookup.
/// Otherwise, return the type unchanged.
fn resolve_projection(ctx: &MirContext, ty: Id<Ty>) -> Result<Id<Ty>, String> {
    if let MirTy::AssociatedTypeProjection { base, protocol, associated } = ctx.ty(ty) {
        resolve_associated_type(ctx, *base, *protocol, associated)
            .map_err(|e| format!("failed to resolve associated type projection: {:?}", e))
    } else {
        Ok(ty)
    }
}

/// Translate a MIR type to a Cranelift type.
///
/// Note: Compound types (structs, tuples) are passed by pointer,
/// so they translate to pointer type.
///
/// IMPORTANT: If you call this with a type that might be an `AssociatedTypeProjection`,
/// you should call `resolve_projection` first, or use `translate_type_with_subst` instead.
pub fn translate_type(
    ctx: &MirContext,
    ty: Id<Ty>,
    target: &TargetConfig,
) -> CraneliftType {
    translate_type_ext(ctx, ty, target, false)
}

pub fn translate_type_ext(
    ctx: &MirContext,
    ty: Id<Ty>,
    target: &TargetConfig,
    is_extern: bool,
) -> CraneliftType {
    let ptr_type = if target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    // Try to resolve any associated type projections before translation
    let ty = resolve_projection(ctx, ty).expect("failed to resolve projection in translate_type");

    if is_extern {
        if let Some(inner) = get_wrapper_primitive(ctx, ty) {
            return translate_type_ext(ctx, inner, target, is_extern);
        }
    }

    match ctx.ty(ty) {
        // ...
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

        // Compound types are passed by pointer
        MirTy::Tuple(_) | MirTy::Named { .. } => ptr_type,

        // Type parameters - resolved at monomorphization, use pointer for now
        MirTy::TypeParam(_) => ptr_type,

        // Function pointers
        MirTy::FuncThin { .. } => ptr_type,
        MirTy::FuncThick { .. } => ptr_type, // Actually two words, but passed by ptr

        // Protocol types
        MirTy::SelfType => ptr_type,
        MirTy::AssociatedTypeProjection { .. } => {
            // Should have been resolved above
            panic!("AssociatedTypeProjection should have been resolved")
        }

        // Error - use pointer as fallback
        MirTy::Error => ptr_type,
    }
}

/// Check if a type should be passed by value (fits in a register).
pub fn is_pass_by_value(ctx: &MirContext, ty: Id<Ty>) -> bool {
    is_pass_by_value_ext(ctx, ty, false)
}

pub fn is_pass_by_value_ext(ctx: &MirContext, ty: Id<Ty>, is_extern: bool) -> bool {
    // Resolve any associated type projections first
    let ty = resolve_projection(ctx, ty).expect("failed to resolve projection in is_pass_by_value");

    if is_extern {
        if let Some(inner) = get_wrapper_primitive(ctx, ty) {
            return is_pass_by_value_ext(ctx, inner, is_extern);
        }
    }

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

pub fn get_wrapper_primitive(ctx: &MirContext, ty: Id<Ty>) -> Option<Id<Ty>> {
    if let MirTy::Named { name, .. } = ctx.ty(ty) {
        if let Some((_, struct_def)) = ctx.structs.iter().find(|(_, s)| s.name == *name) {
            if struct_def.fields.len() == 1 {
                let field_id = struct_def.fields[0];
                let field_def = &ctx.fields[field_id];
                return Some(field_def.ty);
            }
        }
    }
    None
}

/// Translate a MIR type to a Cranelift type, applying substitution for type params.
pub fn translate_type_with_subst(
    ctx: &MirContext,
    ty: Id<Ty>,
    target: &TargetConfig,
    subst: &Substitution,
) -> CraneliftType {
    // Apply substitution first
    let concrete_ty = subst.apply_ty_readonly(ctx, ty).expect("type substitution failed for translate_type");

    // translate_type will handle any remaining projection resolution
    translate_type(ctx, concrete_ty, target)
}

/// Check if a type should be passed by value, applying substitution first.
pub fn is_pass_by_value_with_subst(ctx: &MirContext, ty: Id<Ty>, subst: &Substitution) -> bool {
    let concrete_ty = subst.apply_ty_readonly(ctx, ty).expect("type substitution failed for is_pass_by_value");

    // is_pass_by_value will handle any remaining projection resolution
    is_pass_by_value(ctx, concrete_ty)
}
