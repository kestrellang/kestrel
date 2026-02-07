//! MIR to Cranelift type translation.

use crate::monomorphize::{Substitution, build_substitution, resolve_associated_type};
use kestrel_codegen::TargetConfig;
use kestrel_execution_graph::{Id, MirContext, MirTy, Ty};

use cranelift_codegen::ir::Type as CraneliftType;
use cranelift_codegen::ir::types as cl_types;

/// Resolve associated type projections in a type.
///
/// If the type is an `AssociatedTypeProjection`, resolve it via witness lookup.
/// Otherwise, return the type unchanged.
///
/// This function is recursive - if resolving a projection produces another projection,
/// we continue resolving until we get a concrete type.
///
/// IMPORTANT: If a substitution is provided, it will be applied to the base type
/// before attempting to resolve the projection. This is critical for resolving
/// projections like `I.Item` where `I` is a type parameter.
fn resolve_projection(
    ctx: &MirContext,
    ty: Id<Ty>,
    subst: Option<&Substitution>,
) -> Result<Id<Ty>, String> {
    if let MirTy::AssociatedTypeProjection {
        base,
        protocol,
        associated,
    } = ctx.ty(ty)
    {
        // Apply substitution to the base type first, if provided
        let substituted_base = if let Some(s) = subst {
            s.apply_ty_readonly(ctx, *base)
                .map_err(|e| format!("failed to apply substitution to base type: {:?}", e))?
        } else {
            *base
        };

        let resolved = resolve_associated_type(ctx, substituted_base, *protocol, associated)
            .map_err(|e| format!("failed to resolve associated type projection: {:?}", e))?;

        // Recursively resolve if the result is still a projection
        // This handles cases like MapIterator[T].Item -> ArrayIterator[T].Item -> T
        if matches!(ctx.ty(resolved), MirTy::AssociatedTypeProjection { .. }) {
            resolve_projection(ctx, resolved, subst)
        } else {
            Ok(resolved)
        }
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
pub fn translate_type(ctx: &MirContext, ty: Id<Ty>, target: &TargetConfig) -> CraneliftType {
    translate_type_ext_with_subst(ctx, ty, target, false, None)
}

pub fn translate_type_ext(
    ctx: &MirContext,
    ty: Id<Ty>,
    target: &TargetConfig,
    is_extern: bool,
) -> CraneliftType {
    translate_type_ext_with_subst(ctx, ty, target, is_extern, None)
}

fn translate_type_ext_with_subst(
    ctx: &MirContext,
    ty: Id<Ty>,
    target: &TargetConfig,
    is_extern: bool,
    subst: Option<&Substitution>,
) -> CraneliftType {
    let ptr_type = if target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    // Try to resolve any associated type projections before translation
    // Pass the substitution context if available
    let ty = resolve_projection(ctx, ty, subst).unwrap_or_else(|e| {
        eprintln!("\n=== DEBUG: resolve_projection failed in translate_type_ext ===");
        eprintln!("Type ID: {:?}", ty);
        eprintln!("Type: {:?}", ctx.ty(ty));
        eprintln!("Error: {:?}", e);

        // If it's an associated type projection, print more details about the base type
        if let kestrel_execution_graph::MirTy::AssociatedTypeProjection {
            base,
            protocol: _,
            associated: _,
        } = ctx.ty(ty)
        {
            eprintln!("\nBase type ID: {:?}", base);
            eprintln!("Base type MirTy: {:?}", ctx.ty(*base));

            // If base is a type param, print its details
            if let kestrel_execution_graph::MirTy::TypeParam(tp) = ctx.ty(*base) {
                use kestrel_execution_graph::TypeParamOwner;
                let tp_def = &ctx.type_params[*tp];
                eprintln!("TypeParam name: {}", tp_def.name);
                eprintln!("TypeParam owner: {:?}", tp_def.owner);

                // If owned by a struct, print the struct name
                if let TypeParamOwner::Struct(struct_id) = tp_def.owner {
                    let struct_def = &ctx.structs[struct_id];
                    eprintln!("Owner struct name: {}", ctx.name(struct_def.name));
                }
            }
        }

        // Print backtrace to see where this is coming from
        eprintln!("\nBacktrace:");
        let bt = std::backtrace::Backtrace::force_capture();
        eprintln!("{}", bt);

        panic!("failed to resolve projection in translate_type: {:?}", e)
    });

    if is_extern && let Some(inner) = get_wrapper_primitive(ctx, ty) {
        return translate_type_ext_with_subst(ctx, inner, target, is_extern, subst);
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
        },

        // Error - use pointer as fallback
        MirTy::Error => ptr_type,
    }
}

/// Check if a type should be passed by value (fits in a register).
#[allow(dead_code)]
pub fn is_pass_by_value(ctx: &MirContext, ty: Id<Ty>) -> bool {
    is_pass_by_value_ext(ctx, ty, false)
}

#[allow(dead_code)]
pub fn is_pass_by_value_ext(ctx: &MirContext, ty: Id<Ty>, is_extern: bool) -> bool {
    // Resolve any associated type projections first
    // Note: we don't have substitution context here, so we pass None
    let ty = resolve_projection(ctx, ty, None)
        .expect("failed to resolve projection in is_pass_by_value");

    if is_extern && let Some(inner) = get_wrapper_primitive(ctx, ty) {
        return is_pass_by_value_ext(ctx, inner, is_extern);
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
    if let MirTy::Named { name, type_args } = ctx.ty(ty)
        && let Some((_, struct_def)) = ctx.structs.iter().find(|(_, s)| s.name == *name)
        && struct_def.fields.len() == 1
    {
        let field_id = struct_def.fields[0];
        let field_def = &ctx.fields[field_id];
        let mut field_ty = field_def.ty;

        // Apply substitution from struct's type params to concrete type args
        let type_params = &struct_def.type_params;
        if !type_params.is_empty() && type_params.len() == type_args.len() {
            let subst = build_substitution(ctx, type_params, type_args);
            if let Ok(substituted_ty) = subst.apply_ty_readonly(ctx, field_ty) {
                field_ty = substituted_ty;
            }
        }

        return Some(field_ty);
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
    // Apply substitution first - this will resolve projections as part of substitution
    let concrete_ty = subst
        .apply_ty_readonly(ctx, ty)
        .expect("type substitution failed for translate_type");

    // Then translate the resolved type, passing substitution for any additional resolution needed
    translate_type_ext_with_subst(ctx, concrete_ty, target, false, Some(subst))
}

/// Check if a type should be passed by value, applying substitution first.
#[allow(dead_code)]
pub fn is_pass_by_value_with_subst(ctx: &MirContext, ty: Id<Ty>, subst: &Substitution) -> bool {
    let concrete_ty = subst
        .apply_ty_readonly(ctx, ty)
        .expect("type substitution failed for is_pass_by_value");

    // is_pass_by_value will handle any remaining projection resolution
    is_pass_by_value(ctx, concrete_ty)
}
