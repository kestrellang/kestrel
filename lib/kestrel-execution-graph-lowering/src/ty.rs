//! Type conversion from semantic Ty to MIR types.

use kestrel_execution_graph::{Id, MirTy, Ty as MirTyMarker};
use kestrel_semantic_tree::ty::{
    FloatBits as SemanticFloatBits, IntBits as SemanticIntBits, Ty, TyKind,
};
use semantic_tree::symbol::Symbol;

use crate::context::LoweringContext;
use crate::error::LoweringError;
use crate::name::qualified_name_for_symbol;

/// Convert a semantic type to a MIR type ID.
///
/// This handles the conversion of types from the semantic tree representation
/// to the MIR representation. Some types are not yet supported and will emit
/// a diagnostic and return a fallback type.
pub fn lower_type(ctx: &mut LoweringContext, ty: &Ty) -> Id<MirTyMarker> {
    match ty.kind() {
        // === Primitives ===
        TyKind::Unit => ctx.mir.ty_unit(),
        TyKind::Never => ctx.mir.ty_never(),
        TyKind::Bool => ctx.mir.ty_bool(),
        TyKind::String => ctx.mir.ty_str(),

        TyKind::Int(bits) => match bits {
            SemanticIntBits::I8 => ctx.mir.ty_i8(),
            SemanticIntBits::I16 => ctx.mir.ty_i16(),
            SemanticIntBits::I32 => ctx.mir.ty_i32(),
            SemanticIntBits::I64 => ctx.mir.ty_i64(),
        },

        TyKind::Float(bits) => match bits {
            SemanticFloatBits::F32 => ctx.mir.ty_f32(),
            SemanticFloatBits::F64 => ctx.mir.ty_f64(),
        },

        // === Compound Types ===
        TyKind::Tuple(elements) => {
            let mir_elements: Vec<_> = elements.iter().map(|e| lower_type(ctx, e)).collect();
            ctx.mir.ty_tuple(mir_elements)
        }

        TyKind::Array(element_ty) => {
            let element = lower_type(ctx, element_ty);
            ctx.mir.ty_array(element)
        }

        TyKind::Pointer(element_ty) => {
            let element = lower_type(ctx, element_ty);
            ctx.mir.ty_ptr(element)
        }

        // === Named Types ===
        TyKind::Struct {
            symbol,
            substitutions,
        } => {
            // Get the qualified name for the struct
            let name = qualified_name_for_symbol(ctx, &(symbol.clone() as _));

            // Convert type arguments in declaration order (not HashMap iteration order)
            let type_params = symbol.type_parameters();
            let type_args: Vec<_> = type_params
                .iter()
                .filter_map(|tp| {
                    substitutions
                        .get(tp.metadata().id())
                        .map(|sub_ty| lower_type(ctx, sub_ty))
                })
                .collect();

            ctx.mir.ty_named(name, type_args)
        }

        TyKind::Enum {
            symbol,
            substitutions,
        } => {
            // Get the qualified name for the enum
            let name = qualified_name_for_symbol(ctx, &(symbol.clone() as _));

            // Convert type arguments in declaration order (not HashMap iteration order)
            let type_params = symbol.type_parameters();
            let type_args: Vec<_> = type_params
                .iter()
                .filter_map(|tp| {
                    substitutions
                        .get(tp.metadata().id())
                        .map(|sub_ty| lower_type(ctx, sub_ty))
                })
                .collect();

            ctx.mir.ty_named(name, type_args)
        }

        TyKind::Protocol { symbol, .. } => {
            // TODO: Protocol types need witness-based handling
            ctx.emit_error(LoweringError::unsupported_type(
                format!("Protocol type '{}'", symbol.metadata().name().value),
                ty.span().clone(),
            ));
            ctx.mir.ty_error()
        }

        TyKind::TypeAlias {
            symbol,
            substitutions: _,
        } => {
            // Expand the type alias and lower the underlying type
            let expanded = ty.expand_aliases();
            if expanded.is_type_alias() {
                // Couldn't expand - emit error
                ctx.emit_error(LoweringError::unsupported_type(
                    format!("unresolved type alias '{}'", symbol.metadata().name().value),
                    ty.span().clone(),
                ));
                ctx.mir.ty_error()
            } else {
                lower_type(ctx, &expanded)
            }
        }

        // === Function Types ===
        TyKind::Function {
            params,
            return_type,
        } => {
            let mir_params: Vec<_> = params.iter().map(|p| lower_type(ctx, p)).collect();
            let mir_ret = lower_type(ctx, return_type);
            // Use thick function type for all function values.
            // This allows any function value (closure or non-closure) to be stored
            // in the same type. Thin function types are only used internally for
            // direct function calls where we know the target at compile time.
            ctx.mir.intern_type(MirTy::FuncThick {
                params: mir_params,
                ret: mir_ret,
            })
        }

        TyKind::UnresolvedFunction { return_type, .. } => {
            // This shouldn't appear after type inference, but handle gracefully
            ctx.emit_error(LoweringError::unsupported_type(
                "unresolved function type",
                ty.span().clone(),
            ));
            let mir_ret = lower_type(ctx, return_type);
            ctx.mir.intern_type(MirTy::FuncThin {
                params: vec![],
                ret: mir_ret,
            })
        }

        // === Type Parameters ===
        TyKind::TypeParameter(param_symbol) => {
            // Look up the MIR type param from our mapping
            let symbol_id = param_symbol.metadata().id();
            if let Some(mir_type_param) = ctx.get_type_param(symbol_id) {
                ctx.mir.intern_type(MirTy::TypeParam(mir_type_param))
            } else {
                // Type parameter not in scope - this can happen when lowering
                // a generic definition without entering its context first
                ctx.emit_error(LoweringError::unsupported_type(
                    format!(
                        "type parameter '{}' not in scope",
                        param_symbol.metadata().name().value
                    ),
                    ty.span().clone(),
                ));
                ctx.mir.ty_error()
            }
        }

        // === Associated Types ===
        TyKind::AssociatedType { symbol, container } => {
            // Get the associated type name
            let assoc_name = symbol.metadata().name().value.clone();

            // Get the protocol that defines this associated type (parent of the symbol)
            let protocol_name = if let Some(parent) = symbol.metadata().parent() {
                qualified_name_for_symbol(ctx, &parent)
            } else {
                // Orphan associated type - shouldn't happen
                ctx.emit_error(LoweringError::unsupported_type(
                    format!("orphan associated type '{}'", assoc_name),
                    ty.span().clone(),
                ));
                return ctx.mir.ty_error();
            };

            match container {
                Some(container_ty) => {
                    // Container is known - e.g., T.Element where T: Container
                    // Lower the base type and create a projection
                    let base = lower_type(ctx, container_ty);
                    ctx.mir.ty_assoc_projection(base, protocol_name, assoc_name)
                }
                None => {
                    // No container - bare associated type in protocol context
                    // This means we're in a protocol method signature like:
                    //   func get() -> Element
                    // The associated type should be projected from Self
                    let self_ty = ctx.mir.ty_self();
                    ctx.mir
                        .ty_assoc_projection(self_ty, protocol_name, assoc_name)
                }
            }
        }

        // === Self Type ===
        TyKind::SelfType => {
            // In protocol method signatures, Self is preserved as MirTy::SelfType.
            // During witness lookup, this gets substituted with the concrete implementing type.
            ctx.mir.ty_self()
        }

        // === Inference Placeholder ===
        TyKind::Infer => {
            // This shouldn't appear after type inference
            ctx.emit_error(LoweringError::unsupported_type(
                "unresolved inference type",
                ty.span().clone(),
            ));
            ctx.mir.ty_error()
        }

        // === Error Type ===
        TyKind::Error => {
            // Error types are poison values
            ctx.mir.ty_error()
        }
    }
}

/// Convert semantic IntBits to MIR IntBits.
#[allow(dead_code)]
pub fn convert_int_bits(bits: SemanticIntBits) -> kestrel_execution_graph::IntBits {
    match bits {
        SemanticIntBits::I8 => kestrel_execution_graph::IntBits::I8,
        SemanticIntBits::I16 => kestrel_execution_graph::IntBits::I16,
        SemanticIntBits::I32 => kestrel_execution_graph::IntBits::I32,
        SemanticIntBits::I64 => kestrel_execution_graph::IntBits::I64,
    }
}

/// Convert semantic FloatBits to MIR FloatBits.
#[allow(dead_code)]
pub fn convert_float_bits(bits: SemanticFloatBits) -> kestrel_execution_graph::FloatBits {
    match bits {
        SemanticFloatBits::F32 => kestrel_execution_graph::FloatBits::F32,
        SemanticFloatBits::F64 => kestrel_execution_graph::FloatBits::F64,
    }
}
