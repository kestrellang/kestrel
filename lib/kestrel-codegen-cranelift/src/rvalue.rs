//! Rvalue compilation.

use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::monomorphize::{Substitution, build_substitution, resolve_witness};
use crate::place::{compile_place_read, get_enum_payload_offset};
use crate::types::{get_wrapper_primitive, translate_type, translate_type_ext};

use kestrel_codegen::{Layout, mangle_name};
use kestrel_execution_graph::{
    BinOp, CallArg, Callee, CastKind, FloatBits, Function, FunctionDef, Id, Immediate,
    ImmediateKind, IntBits, Local, MirTy, Origin, PassingMode, Place, PlaceKind, QualifiedName,
    Rvalue, Struct, Ty, UnOp, Value,
};

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::types as cl_types;
use cranelift_codegen::ir::{
    AbiParam, InstBuilder, MemFlags, Signature, StackSlotData, StackSlotKind,
    Value as CraneliftValue,
};
use cranelift_frontend::{FunctionBuilder, Variable};
use cranelift_module::Module;

use std::collections::HashMap;

use kestrel_execution_graph::MirContext;

/// Check if a type uses SelfType anywhere in its structure.
fn type_uses_self(mir: &MirContext, ty_id: Id<Ty>) -> bool {
    let ty = mir.ty(ty_id);
    match ty {
        MirTy::SelfType => true,
        MirTy::Ref(inner) | MirTy::RefMut(inner) | MirTy::Pointer(inner) => {
            type_uses_self(mir, *inner)
        },
        MirTy::Tuple(elems) => elems.iter().any(|e| type_uses_self(mir, *e)),
        MirTy::Named { type_args, .. } => type_args.iter().any(|a| type_uses_self(mir, *a)),
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            params.iter().any(|p| type_uses_self(mir, *p)) || type_uses_self(mir, *ret)
        },
        _ => false,
    }
}

/// Check if a function definition uses Self in its signature.
fn func_uses_self(mir: &MirContext, func_def: &FunctionDef) -> bool {
    func_def.params.iter().any(|&param_id| {
        let param = &mir.params[param_id];
        type_uses_self(mir, param.ty)
    }) || type_uses_self(mir, func_def.ret)
}

/// Try to infer the Self type from a method's qualified name.
///
/// For a function like `Test.Widget.create`, this returns the type `Test.Widget`
/// by looking up the parent name in structs and enums.
fn infer_self_type_from_method_name(
    ctx: &CodegenContext<'_>,
    func_name: Id<QualifiedName>,
) -> Option<Id<Ty>> {
    let name_data = ctx.mir.name(func_name);
    let parent = name_data.parent()?;

    // Try to find a struct with this name
    for (_, struct_def) in ctx.mir.structs.iter() {
        if ctx.mir.name(struct_def.name) == &parent {
            // Build the type: if the struct has type params, this won't work directly,
            // but for non-generic types it will
            if struct_def.type_params.is_empty() {
                // Look up the type - it should already be interned
                let mir_ty = MirTy::Named {
                    name: struct_def.name,
                    type_args: vec![],
                };
                return ctx.mir.lookup_type(&mir_ty);
            }
        }
    }

    // Try to find an enum with this name
    for (_, enum_def) in ctx.mir.enums.iter() {
        if ctx.mir.name(enum_def.name) == &parent && enum_def.type_params.is_empty() {
            let mir_ty = MirTy::Named {
                name: enum_def.name,
                type_args: vec![],
            };
            return ctx.mir.lookup_type(&mir_ty);
        }
    }

    None
}

fn is_main_function(ctx: &CodegenContext<'_>, func_def: &FunctionDef) -> bool {
    let name = ctx.mir.name(func_def.name);
    name.segments.last().map(|s| s.as_str()) == Some("main")
}

/// Check if a type is fully concrete (no type params, Self, or associated projections).
fn type_is_concrete(mir: &MirContext, ty_id: Id<Ty>) -> bool {
    match mir.ty(ty_id) {
        MirTy::TypeParam(_) | MirTy::SelfType | MirTy::Error => false,
        MirTy::AssociatedTypeProjection { .. } => false,
        MirTy::Pointer(inner) | MirTy::Ref(inner) | MirTy::RefMut(inner) => {
            type_is_concrete(mir, *inner)
        },
        MirTy::Tuple(elems) => elems.iter().all(|e| type_is_concrete(mir, *e)),
        MirTy::Named { type_args, .. } => type_args.iter().all(|a| type_is_concrete(mir, *a)),
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            params.iter().all(|p| type_is_concrete(mir, *p)) && type_is_concrete(mir, *ret)
        },
        _ => true,
    }
}

fn is_aggregate_value_type(mir: &MirContext, ty_id: Id<Ty>) -> bool {
    matches!(
        mir.ty(ty_id),
        MirTy::Tuple(_) | MirTy::Named { .. } | MirTy::Str | MirTy::FuncThick { .. }
    )
}

fn needs_sret_for_type(mir: &MirContext, ty_id: Id<Ty>) -> bool {
    !matches!(mir.ty(ty_id), MirTy::Unit) && is_aggregate_value_type(mir, ty_id)
}

/// Convert alignment to the shift value needed by Cranelift.
///
/// Cranelift's StackSlotData uses `align_shift` which is the log2 of the alignment.
/// For example:
/// - alignment=1 → shift=0 (2^0 = 1)
/// - alignment=4 → shift=2 (2^2 = 4)
/// - alignment=8 → shift=3 (2^3 = 8)
pub fn align_to_shift(align: usize) -> u8 {
    if align == 0 {
        return 0;
    }
    align.trailing_zeros() as u8
}

fn copy_aggregate_value(
    ctx: &mut CodegenContext<'_>,
    ty: Id<Ty>,
    dest_ptr: CraneliftValue,
    src_ptr: CraneliftValue,
    builder: &mut FunctionBuilder<'_>,
) {
    // Unit types have zero size - nothing to copy
    if matches!(ctx.mir.ty(ty), kestrel_execution_graph::MirTy::Unit) {
        return;
    }

    let layout = ctx.layouts.layout_of(ty);
    if layout.size == 0 {
        return;
    }

    // Skip copy if src_ptr is a constant 0 (null pointer from Unit value).
    // This can happen when if-else expressions have aggregate types but
    // the branch values are from discarded statement results.
    if let cranelift_codegen::ir::ValueDef::Result(inst, _) = builder.func.dfg.value_def(src_ptr)
        && let cranelift_codegen::ir::InstructionData::UnaryImm { imm, .. } =
            builder.func.dfg.insts[inst]
        && imm.bits() == 0
    {
        return;
    }

    for offset in 0..layout.size {
        let byte = builder
            .ins()
            .load(cl_types::I8, MemFlags::new(), src_ptr, offset as i32);
        builder
            .ins()
            .store(MemFlags::new(), byte, dest_ptr, offset as i32);
    }
}

/// Ensure type arguments are fully resolved before mangling.
fn ensure_concrete_type_args(
    mir: &MirContext,
    type_args: &[Id<Ty>],
    context: &str,
) -> Result<(), CodegenError> {
    for &ty in type_args {
        if !type_is_concrete(mir, ty) {
            return Err(CodegenError::Unsupported(format!(
                "unresolved type argument for {}: {:?}",
                context,
                mir.ty(ty)
            )));
        }
    }
    Ok(())
}

/// Compile an rvalue to a Cranelift value.
pub fn compile_rvalue(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    rvalue: &Rvalue,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    stack_locals: &std::collections::HashSet<Id<Local>>,
) -> Result<CraneliftValue, CodegenError> {
    match rvalue {
        Rvalue::Use(imm) => compile_immediate(ctx, subst, imm, builder),

        Rvalue::Copy(place) | Rvalue::Move(place) => {
            compile_place_read(ctx, place, builder, local_map, subst, stack_locals)
        },

        Rvalue::BinaryOp { op, lhs, rhs } => {
            let lhs_val =
                compile_value(ctx, func_def, subst, lhs, builder, local_map, stack_locals)?;
            let rhs_val =
                compile_value(ctx, func_def, subst, rhs, builder, local_map, stack_locals)?;
            let (_, lhs_ty_opt) = get_value_layout(ctx, lhs, local_map, subst)?;
            let (_, rhs_ty_opt) = get_value_layout(ctx, rhs, local_map, subst)?;

            let lhs_val = ensure_primitive_value(ctx, subst, lhs_val, lhs_ty_opt, builder)?;
            let rhs_val = ensure_primitive_value(ctx, subst, rhs_val, rhs_ty_opt, builder)?;

            compile_binop(ctx, *op, lhs_val, rhs_val, builder)
        },

        Rvalue::UnaryOp { op, operand } => {
            let operand_val = compile_value(
                ctx,
                func_def,
                subst,
                operand,
                builder,
                local_map,
                stack_locals,
            )?;
            let (_, ty_opt) = get_value_layout(ctx, operand, local_map, subst)?;
            let operand_val = ensure_primitive_value(ctx, subst, operand_val, ty_opt, builder)?;
            compile_unop(ctx, *op, operand_val, builder)
        },

        Rvalue::Call { callee, args } => compile_call(
            ctx,
            func_def,
            subst,
            callee,
            args,
            builder,
            local_map,
            stack_locals,
        ),

        Rvalue::Construct { ty, fields } => compile_construct(
            ctx,
            func_def,
            subst,
            *ty,
            fields,
            builder,
            local_map,
            stack_locals,
        ),

        Rvalue::EnumVariant {
            enum_ty,
            variant,
            payload,
        } => compile_enum_variant(
            ctx,
            func_def,
            subst,
            *enum_ty,
            variant,
            payload,
            builder,
            local_map,
            stack_locals,
        ),

        Rvalue::Ref(place) | Rvalue::RefMut(place) => {
            compile_ref(ctx, place, builder, local_map, subst, stack_locals)
        },

        // Pointer/reference conversions - these are no-ops at runtime
        Rvalue::PtrToRef(value) | Rvalue::PtrToRefMut(value) | Rvalue::RefToPtr(value) => {
            // All three are semantically different but have the same runtime representation
            compile_value(
                ctx,
                func_def,
                subst,
                value,
                builder,
                local_map,
                stack_locals,
            )
        },

        Rvalue::PtrOffset { ptr, offset } => compile_ptr_offset(
            ctx,
            func_def,
            subst,
            ptr,
            offset,
            builder,
            local_map,
            stack_locals,
        ),

        Rvalue::Cast {
            kind,
            operand,
            target,
        } => compile_cast(
            ctx,
            func_def,
            subst,
            *kind,
            operand,
            *target,
            builder,
            local_map,
            stack_locals,
        ),

        // String intrinsics
        Rvalue::StrPtr(value) => compile_str_ptr(
            ctx,
            func_def,
            subst,
            value,
            builder,
            local_map,
            stack_locals,
        ),
        Rvalue::StrLen(value) => compile_str_len(
            ctx,
            func_def,
            subst,
            value,
            builder,
            local_map,
            stack_locals,
        ),
        Rvalue::StrFromParts { ptr, len } => compile_str_from_parts(
            ctx,
            func_def,
            subst,
            ptr,
            len,
            builder,
            local_map,
            stack_locals,
        ),

        Rvalue::Tuple(values) => compile_tuple(
            ctx,
            func_def,
            subst,
            values,
            builder,
            local_map,
            stack_locals,
        ),

        Rvalue::StackAlloc { element_ty, count } => compile_stack_alloc(
            ctx,
            func_def,
            subst,
            *element_ty,
            count,
            builder,
            local_map,
            stack_locals,
        ),

        Rvalue::ApplyPartial { func, captures } => compile_apply_partial(
            ctx,
            func_def,
            subst,
            *func,
            captures,
            builder,
            local_map,
            stack_locals,
        ),

        Rvalue::FuncToEscaping(func) => compile_func_to_escaping(ctx, *func, builder),

        Rvalue::IntToString(_) => Err(CodegenError::Unsupported(
            "IntToString requires runtime support".into(),
        )),

        // Float intrinsics - to be implemented
        Rvalue::FloatConst { bits, constant } => {
            use cranelift_codegen::ir::immediates::Ieee32;
            use cranelift_codegen::ir::immediates::Ieee64;
            use kestrel_execution_graph::function::{FloatBits, FloatConstantKind};

            match (bits, constant) {
                (FloatBits::F32, FloatConstantKind::Infinity) => {
                    Ok(builder.ins().f32const(Ieee32::with_float(f32::INFINITY)))
                },
                (FloatBits::F32, FloatConstantKind::Nan) => {
                    Ok(builder.ins().f32const(Ieee32::with_float(f32::NAN)))
                },
                (FloatBits::F64, FloatConstantKind::Infinity) => {
                    Ok(builder.ins().f64const(Ieee64::with_float(f64::INFINITY)))
                },
                (FloatBits::F64, FloatConstantKind::Nan) => {
                    Ok(builder.ins().f64const(Ieee64::with_float(f64::NAN)))
                },
                (FloatBits::F16, _) => Err(CodegenError::Unsupported("f16 not supported".into())),
            }
        },

        Rvalue::FloatPred {
            bits,
            pred,
            operand,
        } => {
            use kestrel_execution_graph::function::{FloatBits, FloatPredicateKind};

            let operand_val = compile_value(
                ctx,
                func_def,
                subst,
                operand,
                builder,
                local_map,
                stack_locals,
            )?;

            match (bits, pred) {
                (FloatBits::F32, FloatPredicateKind::IsNan) => {
                    // NaN is the only value that is not equal to itself
                    let result = builder.ins().fcmp(
                        cranelift_codegen::ir::condcodes::FloatCC::Unordered,
                        operand_val,
                        operand_val,
                    );
                    Ok(result)
                },
                (FloatBits::F64, FloatPredicateKind::IsNan) => {
                    let result = builder.ins().fcmp(
                        cranelift_codegen::ir::condcodes::FloatCC::Unordered,
                        operand_val,
                        operand_val,
                    );
                    Ok(result)
                },
                (FloatBits::F32, FloatPredicateKind::IsInfinite) => {
                    // Check if absolute value equals infinity
                    let abs = builder.ins().fabs(operand_val);
                    let inf = builder.ins().f32const(
                        cranelift_codegen::ir::immediates::Ieee32::with_float(f32::INFINITY),
                    );
                    let result = builder.ins().fcmp(
                        cranelift_codegen::ir::condcodes::FloatCC::Equal,
                        abs,
                        inf,
                    );
                    Ok(result)
                },
                (FloatBits::F64, FloatPredicateKind::IsInfinite) => {
                    let abs = builder.ins().fabs(operand_val);
                    let inf = builder.ins().f64const(
                        cranelift_codegen::ir::immediates::Ieee64::with_float(f64::INFINITY),
                    );
                    let result = builder.ins().fcmp(
                        cranelift_codegen::ir::condcodes::FloatCC::Equal,
                        abs,
                        inf,
                    );
                    Ok(result)
                },
                (FloatBits::F16, _) => Err(CodegenError::Unsupported("f16 not supported".into())),
            }
        },

        Rvalue::FloatMath { bits, op, operand } => {
            use kestrel_execution_graph::function::{FloatBits, FloatMathKind};

            let operand_val = compile_value(
                ctx,
                func_def,
                subst,
                operand,
                builder,
                local_map,
                stack_locals,
            )?;

            match bits {
                FloatBits::F16 => Err(CodegenError::Unsupported("f16 not supported".into())),
                FloatBits::F32 | FloatBits::F64 => match op {
                    FloatMathKind::Sqrt => Ok(builder.ins().sqrt(operand_val)),
                    FloatMathKind::Floor => Ok(builder.ins().floor(operand_val)),
                    FloatMathKind::Ceil => Ok(builder.ins().ceil(operand_val)),
                    FloatMathKind::Trunc => Ok(builder.ins().trunc(operand_val)),
                    FloatMathKind::Round => Ok(builder.ins().nearest(operand_val)),
                },
            }
        },

        Rvalue::FloatFma { bits, a, b, c } => {
            use kestrel_execution_graph::function::FloatBits;

            let a_val = compile_value(ctx, func_def, subst, a, builder, local_map, stack_locals)?;
            let b_val = compile_value(ctx, func_def, subst, b, builder, local_map, stack_locals)?;
            let c_val = compile_value(ctx, func_def, subst, c, builder, local_map, stack_locals)?;

            match bits {
                FloatBits::F16 => Err(CodegenError::Unsupported("f16 not supported".into())),
                FloatBits::F32 | FloatBits::F64 => Ok(builder.ins().fma(a_val, b_val, c_val)),
            }
        },

        Rvalue::FloatCopysign {
            bits,
            magnitude,
            sign_source,
        } => {
            use kestrel_execution_graph::function::FloatBits;

            let mag_val = compile_value(
                ctx,
                func_def,
                subst,
                magnitude,
                builder,
                local_map,
                stack_locals,
            )?;
            let sign_val = compile_value(
                ctx,
                func_def,
                subst,
                sign_source,
                builder,
                local_map,
                stack_locals,
            )?;

            match bits {
                FloatBits::F16 => Err(CodegenError::Unsupported("f16 not supported".into())),
                FloatBits::F32 | FloatBits::F64 => Ok(builder.ins().fcopysign(mag_val, sign_val)),
            }
        },

        // === Pointer intrinsics ===
        Rvalue::PtrNull { .. } => {
            // Return a null pointer (integer 0)
            let ptr_ty = if ctx.target.is_64bit() {
                cranelift_codegen::ir::types::I64
            } else {
                cranelift_codegen::ir::types::I32
            };
            Ok(builder.ins().iconst(ptr_ty, 0))
        },

        Rvalue::PtrFromAddress { address, .. } => {
            // Address is already a pointer at the IR level
            compile_value(
                ctx,
                func_def,
                subst,
                address,
                builder,
                local_map,
                stack_locals,
            )
        },

        Rvalue::PtrToAddress { ptr } => {
            // Pointer is already an integer at the IR level
            compile_value(ctx, func_def, subst, ptr, builder, local_map, stack_locals)
        },

        Rvalue::PtrRead { ptr, ty } => {
            // Load value from pointer
            let ptr_val =
                compile_value(ctx, func_def, subst, ptr, builder, local_map, stack_locals)?;
            let pointee_ty = subst
                .apply_ty_readonly(ctx.mir, *ty)
                .expect("type substitution failed for PtrRead");
            if is_aggregate_value_type(ctx.mir, pointee_ty) {
                return Ok(ptr_val);
            }
            let cl_ty = translate_type(ctx.mir, pointee_ty, ctx.target);
            Ok(builder
                .ins()
                .load(cl_ty, cranelift_codegen::ir::MemFlags::new(), ptr_val, 0))
        },

        Rvalue::PtrWrite { ptr, value } => {
            // Store value through pointer
            let ptr_val =
                compile_value(ctx, func_def, subst, ptr, builder, local_map, stack_locals)?;
            let val = compile_value(
                ctx,
                func_def,
                subst,
                value,
                builder,
                local_map,
                stack_locals,
            )?;

            // Check if the value being written is an aggregate type
            // This is more reliable than checking the pointer's pointee type because
            // the value type can be directly substituted without needing intermediate types
            if let Ok((_, Some(val_ty))) = get_value_layout(ctx, value, local_map, subst) {
                let concrete_val_ty = subst.apply_ty_readonly(ctx.mir, val_ty).unwrap_or(val_ty);
                if is_aggregate_value_type(ctx.mir, concrete_val_ty) {
                    copy_aggregate_value(ctx, concrete_val_ty, ptr_val, val, builder);
                    // Return unit - use pointer type for phi node compatibility
                    let ptr_type = if ctx.target.is_64bit() {
                        cranelift_codegen::ir::types::I64
                    } else {
                        cranelift_codegen::ir::types::I32
                    };
                    return Ok(builder.ins().iconst(ptr_type, 0));
                }
            }

            builder
                .ins()
                .store(cranelift_codegen::ir::MemFlags::new(), val, ptr_val, 0);
            // Return unit - use pointer type for phi node compatibility
            let ptr_type = if ctx.target.is_64bit() {
                cranelift_codegen::ir::types::I64
            } else {
                cranelift_codegen::ir::types::I32
            };
            Ok(builder.ins().iconst(ptr_type, 0))
        },

        Rvalue::PtrIsNull { ptr } => {
            // Compare pointer to null (0)
            let ptr_val =
                compile_value(ctx, func_def, subst, ptr, builder, local_map, stack_locals)?;
            let ptr_ty = builder.func.dfg.value_type(ptr_val);
            let zero = builder.ins().iconst(ptr_ty, 0);
            Ok(builder.ins().icmp(
                cranelift_codegen::ir::condcodes::IntCC::Equal,
                ptr_val,
                zero,
            ))
        },

        Rvalue::PtrCast { ptr, .. } => {
            // Pointer cast is a no-op at the IR level (same representation)
            compile_value(ctx, func_def, subst, ptr, builder, local_map, stack_locals)
        },

        Rvalue::SizeOf { ty } => {
            // Return the size of the type as a constant
            let concrete_ty = subst
                .apply_ty_readonly(ctx.mir, *ty)
                .expect("type substitution failed for SizeOf");
            let layout = ctx.layouts.layout_of(concrete_ty);
            let size = layout.size as i64;
            let int_ty = if ctx.target.is_64bit() {
                cranelift_codegen::ir::types::I64
            } else {
                cranelift_codegen::ir::types::I32
            };
            Ok(builder.ins().iconst(int_ty, size))
        },

        Rvalue::AlignOf { ty } => {
            // Return the alignment of the type as a constant
            let concrete_ty = subst
                .apply_ty_readonly(ctx.mir, *ty)
                .expect("type substitution failed for AlignOf");
            let layout = ctx.layouts.layout_of(concrete_ty);
            let align = layout.align as i64;
            let int_ty = if ctx.target.is_64bit() {
                cranelift_codegen::ir::types::I64
            } else {
                cranelift_codegen::ir::types::I32
            };
            Ok(builder.ins().iconst(int_ty, align))
        },

        // Boolean (i1) intrinsics
        Rvalue::I1Eq { lhs, rhs } => {
            let lhs_val =
                compile_value(ctx, func_def, subst, lhs, builder, local_map, stack_locals)?;
            let rhs_val =
                compile_value(ctx, func_def, subst, rhs, builder, local_map, stack_locals)?;
            // Boolean equality is just integer equality
            Ok(builder.ins().icmp(IntCC::Equal, lhs_val, rhs_val))
        },
        Rvalue::I1And { lhs, rhs } => {
            let lhs_val =
                compile_value(ctx, func_def, subst, lhs, builder, local_map, stack_locals)?;
            let rhs_val =
                compile_value(ctx, func_def, subst, rhs, builder, local_map, stack_locals)?;
            // Boolean AND is just bitwise AND on i8/i1
            Ok(builder.ins().band(lhs_val, rhs_val))
        },
        Rvalue::I1Or { lhs, rhs } => {
            let lhs_val =
                compile_value(ctx, func_def, subst, lhs, builder, local_map, stack_locals)?;
            let rhs_val =
                compile_value(ctx, func_def, subst, rhs, builder, local_map, stack_locals)?;
            // Boolean OR is just bitwise OR on i8/i1
            Ok(builder.ins().bor(lhs_val, rhs_val))
        },
        Rvalue::I1Not { operand } => {
            let val = compile_value(
                ctx,
                func_def,
                subst,
                operand,
                builder,
                local_map,
                stack_locals,
            )?;
            // Boolean NOT: XOR with 1
            let one = builder.ins().iconst(cranelift_codegen::ir::types::I8, 1);
            Ok(builder.ins().bxor(val, one))
        },

        // Atomic intrinsics
        Rvalue::AtomicAdd { ptr, delta } => {
            let ptr_val =
                compile_value(ctx, func_def, subst, ptr, builder, local_map, stack_locals)?;
            let delta_val = compile_value(
                ctx,
                func_def,
                subst,
                delta,
                builder,
                local_map,
                stack_locals,
            )?;
            let delta_ty = builder.func.dfg.value_type(delta_val);
            // Cranelift atomic_rmw with Add operation
            Ok(builder.ins().atomic_rmw(
                delta_ty,
                cranelift_codegen::ir::MemFlags::new(),
                cranelift_codegen::ir::AtomicRmwOp::Add,
                ptr_val,
                delta_val,
            ))
        },
        Rvalue::AtomicSub { ptr, delta } => {
            let ptr_val =
                compile_value(ctx, func_def, subst, ptr, builder, local_map, stack_locals)?;
            let delta_val = compile_value(
                ctx,
                func_def,
                subst,
                delta,
                builder,
                local_map,
                stack_locals,
            )?;
            let delta_ty = builder.func.dfg.value_type(delta_val);
            // Cranelift atomic_rmw with Sub operation
            Ok(builder.ins().atomic_rmw(
                delta_ty,
                cranelift_codegen::ir::MemFlags::new(),
                cranelift_codegen::ir::AtomicRmwOp::Sub,
                ptr_val,
                delta_val,
            ))
        },

        Rvalue::StrConcat { .. } => {
            // TODO: String concatenation is not yet implemented
            // This should allocate a buffer and copy strings into it
            // For now, return an error value
            Ok(builder.ins().iconst(cranelift_codegen::ir::types::I64, 0))
        },
    }
}

/// Compile a struct construction.
///
/// Allocates stack space for the struct, stores each field value at its offset,
/// and returns a pointer to the stack slot.
fn compile_construct(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    ty: Id<Ty>,
    fields: &[(String, Value)],
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    stack_locals: &std::collections::HashSet<Id<Local>>,
) -> Result<CraneliftValue, CodegenError> {
    // Get the struct layout to determine size and field offsets
    let mir_ty = ctx.mir.ty(ty);

    // Find the struct ID and type_args from the type
    let (struct_id, type_args) = match mir_ty {
        MirTy::Named { name, type_args } => {
            // Look up struct by name
            let name_data = ctx.mir.name(*name);
            let mut found_struct = None;
            for (id, def) in ctx.mir.structs.iter() {
                let def_name = ctx.mir.name(def.name);
                if def_name == name_data {
                    found_struct = Some(id);
                    break;
                }
            }
            let struct_id = found_struct.ok_or_else(|| {
                CodegenError::Unsupported(format!("struct not found: {}", name_data))
            })?;
            // Apply substitution to type_args to replace any type parameters with concrete types
            let type_args: Vec<_> = type_args
                .iter()
                .map(|&ty| subst.apply_ty_readonly(ctx.mir, ty).unwrap_or(ty))
                .collect();
            (struct_id, type_args)
        },
        _ => {
            return Err(CodegenError::Unsupported(format!(
                "construct non-struct type: {:?}",
                mir_ty
            )));
        },
    };

    // Get struct layout with field offsets
    let struct_layout = ctx.layouts.struct_layout(struct_id, &type_args);
    let layout = struct_layout.layout;
    let field_offsets = struct_layout.field_offsets.clone();

    // Allocate stack slot for the struct
    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        layout.size as u32,
        align_to_shift(layout.align),
    ));

    // Get pointer type for the target
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    // Get pointer to the stack slot
    let ptr = builder.ins().stack_addr(ptr_type, slot, 0);

    // Store each field at its offset
    let struct_def = ctx.mir.struct_def(struct_id);
    let type_params: Vec<_> = struct_def.type_params.clone();

    // Build substitution from struct's type params to concrete type args
    let struct_subst = if !type_params.is_empty() && type_params.len() == type_args.len() {
        Some(build_substitution(ctx.mir, &type_params, &type_args))
    } else {
        None
    };

    for (field_name, field_value) in fields {
        let offset = field_offsets.get(field_name).ok_or_else(|| {
            let struct_name = ctx.mir.name(struct_def.name);
            let available_fields: Vec<_> = field_offsets.keys().collect();
            CodegenError::Unsupported(format!(
                "unknown field: {} in struct {} (available: {:?})",
                field_name, struct_name, available_fields
            ))
        })?;

        // Find the field type
        let mut field_ty = None;
        for field_id in &struct_def.fields {
            let field_def = &ctx.mir.fields[*field_id];
            if &field_def.name == field_name {
                field_ty = Some(field_def.ty);
                break;
            }
        }
        let field_ty = field_ty.ok_or_else(|| {
            CodegenError::Unsupported(format!("field type not found: {}", field_name))
        })?;

        // Compile the field value
        let value = compile_value(
            ctx,
            func_def,
            subst,
            field_value,
            builder,
            local_map,
            stack_locals,
        )?;

        // Apply the struct's type param -> type arg substitution to get the concrete field type
        // This handles generic struct fields like T becoming Int64 for Storage[Int64]
        let concrete_field_ty = if let Some(ref struct_subst) = struct_subst {
            struct_subst
                .apply_ty_readonly(ctx.mir, field_ty)
                .unwrap_or(field_ty)
        } else {
            subst
                .apply_ty_readonly(ctx.mir, field_ty)
                .unwrap_or(field_ty)
        };

        // Check if this is a nested aggregate - if so, copy its data
        if is_aggregate_value_type(ctx.mir, concrete_field_ty) {
            let dest_ptr = if *offset == 0 {
                ptr
            } else {
                builder.ins().iadd_imm(ptr, *offset as i64)
            };
            copy_aggregate_value(ctx, concrete_field_ty, dest_ptr, value, builder);
        } else {
            // Store primitive value directly
            builder
                .ins()
                .store(MemFlags::new(), value, ptr, *offset as i32);
        }
    }

    // Return the pointer to the struct
    Ok(ptr)
}

/// Compile a tuple construction.
///
/// Allocates stack space for the tuple, stores each element at its offset,
/// and returns a pointer to the stack slot.
fn compile_tuple(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    values: &[Value],
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    stack_locals: &std::collections::HashSet<Id<Local>>,
) -> Result<CraneliftValue, CodegenError> {
    // Calculate tuple layout by laying out elements sequentially
    let mut offsets = Vec::with_capacity(values.len());
    let mut element_layouts = Vec::with_capacity(values.len());
    let mut element_types = Vec::with_capacity(values.len());
    let mut current_offset = 0usize;
    let mut max_align = 1usize;

    // First pass: compute element layouts and offsets
    for value in values {
        let (elem_layout, elem_ty) = get_value_layout(ctx, value, local_map, subst)?;

        // Align to element's alignment
        current_offset = (current_offset + elem_layout.align - 1) & !(elem_layout.align - 1);
        offsets.push(current_offset);
        element_layouts.push(elem_layout);
        element_types.push(elem_ty);

        current_offset += elem_layout.size;
        max_align = max_align.max(elem_layout.align);
    }

    // Pad to overall alignment
    let total_size = (current_offset + max_align - 1) & !(max_align - 1);
    // Ensure minimum size of 1 byte for empty tuples
    let total_size = if total_size == 0 { 1 } else { total_size };
    let max_align = if max_align == 0 { 1 } else { max_align };

    // Allocate stack slot for the tuple
    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        total_size as u32,
        align_to_shift(max_align),
    ));

    // Get pointer type for the target
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    // Get pointer to the stack slot
    let ptr = builder.ins().stack_addr(ptr_type, slot, 0);

    // Store each element at its offset
    for (i, value) in values.iter().enumerate() {
        let offset = offsets[i];
        let elem_ty = element_types[i];

        // Compile the element value
        let val = compile_value(
            ctx,
            func_def,
            subst,
            value,
            builder,
            local_map,
            stack_locals,
        )?;

        // Check if this is a nested compound type - if so, copy the data
        let compound_ty = if let Some(ty) = elem_ty {
            // Apply substitution to get concrete type for generic tuples
            let concrete_ty = subst
                .apply_ty_readonly(ctx.mir, ty)
                .expect("type substitution failed for tuple element");
            if is_aggregate_value_type(ctx.mir, concrete_ty) {
                Some(concrete_ty)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(concrete_ty) = compound_ty {
            // Value is a pointer to the nested compound type - copy its contents
            let dest_ptr = if offset == 0 {
                ptr
            } else {
                builder.ins().iadd_imm(ptr, offset as i64)
            };
            copy_aggregate_value(ctx, concrete_ty, dest_ptr, val, builder);
        } else {
            // Store primitive value directly
            builder
                .ins()
                .store(MemFlags::new(), val, ptr, offset as i32);
        }
    }

    // Return the pointer to the tuple
    Ok(ptr)
}

/// Compile a stack allocation for array literals.
/// Allocates `count` elements of `element_ty` on the stack and returns a pointer.
fn compile_stack_alloc(
    ctx: &mut CodegenContext<'_>,
    _func_def: &FunctionDef,
    subst: &Substitution,
    element_ty: Id<Ty>,
    count: &Value,
    builder: &mut FunctionBuilder<'_>,
    _local_map: &HashMap<Id<Local>, Variable>,
    _stack_locals: &std::collections::HashSet<Id<Local>>,
) -> Result<CraneliftValue, CodegenError> {
    // Apply substitution to get concrete element type
    let concrete_element_ty = subst
        .apply_ty_readonly(ctx.mir, element_ty)
        .unwrap_or(element_ty);

    // Get element layout
    let element_layout = ctx.layouts.layout_of(concrete_element_ty);

    // For array literals, count must be a compile-time constant
    let count_value = match count {
        Value::Immediate(imm) => match &imm.kind {
            ImmediateKind::IntLiteral { value, .. } => *value as usize,
            _ => {
                return Err(CodegenError::Unsupported(
                    "stack_alloc count must be an integer literal".into(),
                ));
            },
        },
        Value::Place(_) => {
            return Err(CodegenError::Unsupported(
                "stack_alloc with dynamic count not yet supported".into(),
            ));
        },
        Value::Unreachable => {
            return Err(CodegenError::Unsupported(
                "stack_alloc with unreachable count".into(),
            ));
        },
    };

    // Calculate total size (count * element_size)
    let total_size = count_value * element_layout.size;

    // Ensure minimum size of 1 byte for empty allocations
    let total_size = if total_size == 0 { 1 } else { total_size };
    let align = if element_layout.align == 0 {
        1
    } else {
        element_layout.align
    };

    // Allocate stack slot
    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        total_size as u32,
        align_to_shift(align),
    ));

    // Get pointer type for the target
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    // Return pointer to the stack slot
    let ptr = builder.ins().stack_addr(ptr_type, slot, 0);
    Ok(ptr)
}

/// Get the layout of a value and optionally its type ID.
/// Returns (Layout, Option<type_id>).
/// The `subst` parameter is used to substitute type parameters before computing layout.
fn get_value_layout(
    ctx: &mut CodegenContext<'_>,
    value: &Value,
    local_map: &HashMap<Id<Local>, Variable>,
    subst: &Substitution,
) -> Result<(kestrel_codegen::Layout, Option<Id<Ty>>), CodegenError> {
    match value {
        Value::Place(place) => {
            let ty = get_place_type(ctx, place, local_map, subst)?;
            // Apply substitution before computing layout
            let concrete_ty = subst.apply_ty_readonly(ctx.mir, ty).unwrap_or(ty);
            let layout = ctx.layouts.layout_of(concrete_ty);
            Ok((layout, Some(ty)))
        },
        Value::Immediate(imm) => {
            let layout = get_immediate_layout_readonly(ctx, imm)?;
            let mir_ty = match &imm.kind {
                ImmediateKind::IntLiteral { bits, .. } => match bits {
                    IntBits::I8 => MirTy::I8,
                    IntBits::I16 => MirTy::I16,
                    IntBits::I32 => MirTy::I32,
                    IntBits::I64 => MirTy::I64,
                },
                ImmediateKind::FloatLiteral { bits, .. } => match bits {
                    FloatBits::F16 => MirTy::F16,
                    FloatBits::F32 => MirTy::F32,
                    FloatBits::F64 => MirTy::F64,
                },
                ImmediateKind::BoolLiteral(_) => MirTy::Bool,
                ImmediateKind::Unit => MirTy::Unit,
                ImmediateKind::StringLiteral(_) => MirTy::Str,
                ImmediateKind::StringPointer(_) => {
                    MirTy::Pointer(ctx.mir.lookup_type(&MirTy::I8).unwrap())
                },
                ImmediateKind::FunctionRef { name, type_args } => MirTy::Named {
                    name: *name,
                    type_args: type_args.clone(),
                },
                ImmediateKind::WitnessMethod { .. } => MirTy::Unit,
                ImmediateKind::NullPtr(ty) => MirTy::Pointer(*ty),
                ImmediateKind::Error => MirTy::Error,
            };
            let ty = ctx.mir.lookup_type(&mir_ty);
            Ok((layout, ty))
        },
        Value::Unreachable => Err(CodegenError::Unsupported(
            "cannot get layout of unreachable value".into(),
        )),
    }
}

fn compile_call_arg(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    arg: &CallArg,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    stack_locals: &std::collections::HashSet<Id<Local>>,
    is_extern: bool,
) -> Result<CraneliftValue, CodegenError> {
    let (layout, ty_opt) = get_value_layout(ctx, &arg.value, local_map, subst)?;
    let concrete_ty = ty_opt.map(|ty| subst.apply_ty_readonly(ctx.mir, ty).unwrap_or(ty));
    let is_aggregate = concrete_ty
        .map(|ty| is_aggregate_value_type(ctx.mir, ty))
        .unwrap_or(false);
    let is_ref_value = concrete_ty
        .map(|ty| matches!(ctx.mir.ty(ty), MirTy::Ref(_) | MirTy::RefMut(_)))
        .unwrap_or(false);
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    match arg.mode {
        PassingMode::Copy => {
            let mut val = compile_value(
                ctx,
                func_def,
                subst,
                &arg.value,
                builder,
                local_map,
                stack_locals,
            )?;
            if is_extern {
                val = ensure_primitive_value(ctx, subst, val, ty_opt, builder)?;
                return Ok(val);
            }
            if is_aggregate {
                let ty = concrete_ty.unwrap();
                let layout = ctx.layouts.layout_of(ty);
                let size = if layout.size == 0 { 1 } else { layout.size };
                let align = if layout.align == 0 { 1 } else { layout.align };
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    size as u32,
                    align_to_shift(align),
                ));
                let addr = builder.ins().stack_addr(ptr_type, slot, 0);
                copy_aggregate_value(ctx, ty, addr, val, builder);
                return Ok(addr);
            }
            Ok(val)
        },
        PassingMode::Move => {
            let mut val = compile_value(
                ctx,
                func_def,
                subst,
                &arg.value,
                builder,
                local_map,
                stack_locals,
            )?;
            if is_extern {
                val = ensure_primitive_value(ctx, subst, val, ty_opt, builder)?;
            }
            Ok(val)
        },
        PassingMode::Ref | PassingMode::MutRef => {
            if is_ref_value || is_aggregate {
                return compile_value(
                    ctx,
                    func_def,
                    subst,
                    &arg.value,
                    builder,
                    local_map,
                    stack_locals,
                );
            }

            match &arg.value {
                Value::Place(place) => {
                    compile_ref(ctx, place, builder, local_map, subst, stack_locals)
                },
                Value::Immediate(_) => {
                    let val = compile_value(
                        ctx,
                        func_def,
                        subst,
                        &arg.value,
                        builder,
                        local_map,
                        stack_locals,
                    )?;
                    let size = if layout.size == 0 { 1 } else { layout.size };
                    let align = if layout.align == 0 { 1 } else { layout.align };
                    let slot = builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        size as u32,
                        align_to_shift(align),
                    ));
                    let addr = builder.ins().stack_addr(ptr_type, slot, 0);
                    builder.ins().store(MemFlags::new(), val, addr, 0);
                    Ok(addr)
                },
                Value::Unreachable => Err(CodegenError::Unsupported(
                    "cannot take reference to unreachable value".into(),
                )),
            }
        },
    }
}

fn get_immediate_layout_readonly(
    ctx: &mut CodegenContext<'_>,
    imm: &Immediate,
) -> Result<kestrel_codegen::Layout, CodegenError> {
    use kestrel_codegen::Layout;

    match &imm.kind {
        ImmediateKind::IntLiteral { bits, .. } => {
            let layout = match bits {
                IntBits::I8 => Layout::new(1, 1),
                IntBits::I16 => Layout::new(2, 2),
                IntBits::I32 => Layout::new(4, 4),
                IntBits::I64 => Layout::new(8, 8),
            };
            Ok(layout)
        },
        ImmediateKind::FloatLiteral { bits, .. } => {
            let layout = match bits {
                FloatBits::F16 => Layout::new(2, 2),
                FloatBits::F32 => Layout::new(4, 4),
                FloatBits::F64 => Layout::new(8, 8),
            };
            Ok(layout)
        },
        ImmediateKind::BoolLiteral(_) => Ok(Layout::new(1, 1)),
        ImmediateKind::Unit => Ok(Layout::new(0, 1)),
        ImmediateKind::StringLiteral(_) => {
            let ptr_size = if ctx.target.is_64bit() { 8 } else { 4 };
            Ok(Layout::new(ptr_size * 2, ptr_size))
        },
        ImmediateKind::StringPointer(_) => {
            let ptr_size = if ctx.target.is_64bit() { 8 } else { 4 };
            Ok(Layout::new(ptr_size, ptr_size))
        },
        ImmediateKind::FunctionRef { .. } => {
            let ptr_size = if ctx.target.is_64bit() { 8 } else { 4 };
            Ok(Layout::new(ptr_size, ptr_size))
        },
        ImmediateKind::WitnessMethod { .. } => {
            let ptr_size = if ctx.target.is_64bit() { 8 } else { 4 };
            Ok(Layout::new(ptr_size, ptr_size))
        },
        ImmediateKind::NullPtr(_) => {
            let ptr_size = if ctx.target.is_64bit() { 8 } else { 4 };
            Ok(Layout::new(ptr_size, ptr_size))
        },
        ImmediateKind::Error => Ok(Layout::new(0, 1)),
    }
}

/// Get the layout of an immediate value.
#[allow(dead_code)]
fn get_immediate_layout(
    ctx: &mut CodegenContext<'_>,
    imm: &Immediate,
) -> Result<kestrel_codegen::Layout, CodegenError> {
    use kestrel_codegen::Layout;

    match &imm.kind {
        ImmediateKind::IntLiteral { bits, .. } => {
            let layout = match bits {
                IntBits::I8 => Layout::new(1, 1),
                IntBits::I16 => Layout::new(2, 2),
                IntBits::I32 => Layout::new(4, 4),
                IntBits::I64 => Layout::new(8, 8),
            };
            Ok(layout)
        },
        ImmediateKind::FloatLiteral { bits, .. } => {
            let layout = match bits {
                FloatBits::F16 => Layout::new(2, 2),
                FloatBits::F32 => Layout::new(4, 4),
                FloatBits::F64 => Layout::new(8, 8),
            };
            Ok(layout)
        },
        ImmediateKind::BoolLiteral(_) => Ok(Layout::new(1, 1)),
        ImmediateKind::Unit => Ok(Layout::new(0, 1)),
        ImmediateKind::StringLiteral(_) => {
            // String is a fat pointer: { ptr, len }
            let ptr_size = ctx.target.pointer_size();
            Ok(Layout::new(ptr_size * 2, ptr_size))
        },
        ImmediateKind::StringPointer(_) => {
            // String pointer is just a pointer
            let ptr_size = ctx.target.pointer_size();
            Ok(Layout::new(ptr_size, ptr_size))
        },
        ImmediateKind::NullPtr(ty) => {
            let layout = ctx.layouts.layout_of(*ty);
            Ok(layout)
        },
        ImmediateKind::FunctionRef { .. } => {
            // Function references are pointer-sized
            let ptr_size = ctx.target.pointer_size();
            Ok(Layout::new(ptr_size, ptr_size))
        },
        ImmediateKind::WitnessMethod { .. } => {
            Err(CodegenError::Unsupported("witness method layout".into()))
        },
        ImmediateKind::Error => Err(CodegenError::Unsupported("error immediate".into())),
    }
}

/// Compile an enum variant construction.
///
/// Allocates stack space for the enum (discriminant + max payload size),
/// stores the discriminant, then stores the payload fields.
fn compile_enum_variant(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    enum_ty: Id<Ty>,
    variant: &str,
    payload: &[Value],
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    stack_locals: &std::collections::HashSet<Id<Local>>,
) -> Result<CraneliftValue, CodegenError> {
    let mir_ty = ctx.mir.ty(enum_ty);

    // Find the enum ID and type arguments from the type
    let (enum_id, enum_type_args) = match mir_ty {
        MirTy::Named { name, type_args } => {
            let name_data = ctx.mir.name(*name);
            // Apply substitution to type_args to get concrete types
            let type_args: Vec<_> = type_args
                .iter()
                .map(|&ty| subst.apply_ty_readonly(ctx.mir, ty).unwrap_or(ty))
                .collect();
            let mut found_enum = None;
            for (id, def) in ctx.mir.enums.iter() {
                let def_name = ctx.mir.name(def.name);
                if def_name == name_data {
                    found_enum = Some(id);
                    break;
                }
            }
            let enum_id = found_enum.ok_or_else(|| {
                CodegenError::Unsupported(format!("enum not found: {}", name_data))
            })?;
            (enum_id, type_args)
        },
        _ => {
            return Err(CodegenError::Unsupported(format!(
                "enum variant on non-named type: {:?}",
                mir_ty
            )));
        },
    };

    // Apply substitution to get concrete type before computing layout
    let concrete_enum_ty = subst.apply_ty_readonly(ctx.mir, enum_ty).unwrap_or(enum_ty);

    // Get the enum layout
    let enum_layout = ctx.layouts.layout_of(concrete_enum_ty);

    // Allocate stack slot for the enum
    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        enum_layout.size as u32,
        align_to_shift(enum_layout.align),
    ));

    // Get pointer type for the target
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    // Get pointer to the stack slot
    let ptr = builder.ins().stack_addr(ptr_type, slot, 0);

    // Find the case and its discriminant
    let enum_def = ctx.mir.enum_def(enum_id);
    let case_id = enum_def
        .case_by_name(variant)
        .ok_or_else(|| CodegenError::Unsupported(format!("enum case not found: {}", variant)))?;
    let case_def = &ctx.mir.enum_cases[case_id];
    let discriminant = case_def.discriminant;

    // Store the discriminant at offset 0 (i32)
    let discr_val = builder.ins().iconst(cl_types::I32, discriminant as i64);
    builder.ins().store(MemFlags::new(), discr_val, ptr, 0);

    // If there's a payload, store the fields after the discriminant
    if !payload.is_empty() {
        // Get the payload struct layout for this specific case
        let payload_struct_id = case_def.struct_def.ok_or_else(|| {
            CodegenError::Unsupported(format!("enum case {} has no struct_def", variant))
        })?;
        // Pass the enum's type_args since payload struct uses the same type parameters
        let payload_layout = ctx
            .layouts
            .struct_layout(payload_struct_id, &enum_type_args);
        let field_offsets = payload_layout.field_offsets.clone();

        // Compute the payload offset based on the MAXIMUM payload alignment across all cases.
        // This ensures all cases have a consistent payload offset.
        let case_ids: Vec<_> = enum_def.cases.clone();
        let mut max_payload_layout = Layout::zero(1);
        for cid in &case_ids {
            let cd = &ctx.mir.enum_cases[*cid];
            if let Some(struct_id) = cd.struct_def {
                let pl = ctx.layouts.struct_layout(struct_id, &enum_type_args);
                if pl.layout.align > max_payload_layout.align
                    || (pl.layout.align == max_payload_layout.align
                        && pl.layout.size > max_payload_layout.size)
                {
                    max_payload_layout = pl.layout;
                }
            }
        }

        let discriminant_layout = Layout::new(4, 4);
        let (payload_offset, _) = discriminant_layout.append(max_payload_layout);
        let payload_base_offset = payload_offset as i32;

        // Get the struct definition to find field names in order
        let payload_struct = ctx.mir.struct_def(payload_struct_id);
        let field_ids: Vec<_> = payload_struct.fields.clone();

        // Build a substitution from enum's type parameters to its type arguments.
        // This is needed because field types in payload structs reference the enum's
        // type parameters (e.g., T in Result[T, E]), not the caller's type parameters.
        let enum_subst = build_substitution(ctx.mir, &enum_def.type_params, &enum_type_args);

        for (i, value) in payload.iter().enumerate() {
            if i >= field_ids.len() {
                break;
            }
            let field_id = field_ids[i];
            let field_def = &ctx.mir.fields[field_id];
            let field_name = &field_def.name;

            let field_offset = field_offsets.get(field_name).copied().unwrap_or(0);
            let total_offset = payload_base_offset + field_offset as i32;

            // Compile the payload value
            let val = compile_value(
                ctx,
                func_def,
                subst,
                value,
                builder,
                local_map,
                stack_locals,
            )?;

            // Check if this is a nested struct
            let field_ty = field_def.ty;
            // Apply enum's substitution to get concrete type for generic enums.
            // Use enum_subst (built from enum's type params -> type args) instead of
            // the caller's subst, since field types reference the enum's type params.
            let concrete_field_ty = enum_subst
                .apply_ty_readonly(ctx.mir, field_ty)
                .unwrap_or(field_ty);

            // Skip Unit types - they have zero size and shouldn't be stored.
            // Unit values are compiled as 64-bit 0 for phi node compatibility,
            // but storing them would write 8 bytes and corrupt adjacent memory.
            if matches!(ctx.mir.ty(concrete_field_ty), MirTy::Unit) {
                continue;
            }

            if is_aggregate_value_type(ctx.mir, concrete_field_ty) {
                // Copy nested struct data
                let dest_ptr = if total_offset == 0 {
                    ptr
                } else {
                    builder.ins().iadd_imm(ptr, total_offset as i64)
                };
                copy_aggregate_value(ctx, concrete_field_ty, dest_ptr, val, builder);
            } else {
                // Store primitive value directly
                builder.ins().store(MemFlags::new(), val, ptr, total_offset);
            }
        }
    }

    Ok(ptr)
}

/// Check if a type is a struct type.
fn is_struct_type(ctx: &CodegenContext<'_>, ty: Id<Ty>) -> bool {
    let mir_ty = ctx.mir.ty(ty);
    if let MirTy::Named { name, .. } = mir_ty {
        let name_data = ctx.mir.name(*name);
        for (_, def) in ctx.mir.structs.iter() {
            if ctx.mir.name(def.name) == name_data {
                return true;
            }
        }
    }
    false
}

/// Compile a reference operation (Ref or RefMut).
///
/// Taking a reference means getting the address of a place.
/// For primitives stored in Variables, we need to spill them to a stack slot first.
fn compile_ref(
    ctx: &mut CodegenContext<'_>,
    place: &Place,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    subst: &Substitution,
    stack_locals: &std::collections::HashSet<Id<Local>>,
) -> Result<CraneliftValue, CodegenError> {
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    match &place.kind {
        PlaceKind::Global(name_id) => {
            // Global/static variable - return its address
            let global_name = ctx.mir.name(*name_id);
            let mangled_name = format!("{}", global_name);

            // Look up the global symbol
            let global_ref = ctx
                .module
                .declare_data(
                    &mangled_name,
                    cranelift_module::Linkage::Import,
                    false,
                    false,
                )
                .map_err(|e| {
                    CodegenError::Unsupported(format!("failed to declare global: {}", e))
                })?;

            // Get the global address
            let global_addr = ctx.module.declare_data_in_func(global_ref, builder.func);

            // Return the address of the global
            Ok(builder.ins().global_value(ptr_type, global_addr))
        },

        PlaceKind::Local(local_id) => {
            // For a local variable, get its address when it's memory-backed.
            let local_def = ctx.mir.local(*local_id);
            let local_ty = local_def.ty;

            // Apply substitution to get concrete type for generic locals
            let concrete_local_ty = subst
                .apply_ty_readonly(ctx.mir, local_ty)
                .unwrap_or(local_ty);
            let var = local_map
                .get(local_id)
                .ok_or_else(|| CodegenError::Unsupported("unknown local".to_string()))?;
            if is_aggregate_value_type(ctx.mir, concrete_local_ty)
                || stack_locals.contains(local_id)
            {
                Ok(builder.use_var(*var))
            } else {
                let value = builder.use_var(*var);
                let layout = ctx.layouts.layout_of(concrete_local_ty);
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    layout.size as u32,
                    align_to_shift(layout.align),
                ));
                let addr = builder.ins().stack_addr(ptr_type, slot, 0);
                builder.ins().store(MemFlags::new(), value, addr, 0);
                Ok(addr)
            }
        },

        PlaceKind::Field { parent, name } => {
            // Taking a reference to a field means getting the field's address
            // The parent is a struct pointer, so we compute: parent_ptr + field_offset
            let struct_ptr =
                compile_place_read(ctx, parent, builder, local_map, subst, stack_locals)?;

            // Get the field offset
            let parent_ty = get_place_type(ctx, parent, local_map, subst)?;
            let (field_offset, _field_ty) = get_field_info(ctx, parent_ty, name, subst)?;

            if field_offset == 0 {
                Ok(struct_ptr)
            } else {
                Ok(builder.ins().iadd_imm(struct_ptr, field_offset as i64))
            }
        },

        PlaceKind::Index { parent, index } => {
            // Taking a reference to an indexed element
            let parent_ptr =
                compile_place_read(ctx, parent, builder, local_map, subst, stack_locals)?;

            // Get the field offset
            let parent_ty = get_place_type(ctx, parent, local_map, subst)?;
            let (field_offset, _field_ty) =
                get_field_by_index(ctx, parent, parent_ty, *index, subst)?;

            if field_offset == 0 {
                Ok(parent_ptr)
            } else {
                Ok(builder.ins().iadd_imm(parent_ptr, field_offset as i64))
            }
        },

        PlaceKind::Downcast { parent, variant } => {
            // Taking a reference to a downcast - return pointer to payload
            let enum_ptr =
                compile_place_read(ctx, parent, builder, local_map, subst, stack_locals)?;
            let enum_ty = get_place_type(ctx, parent, local_map, subst)?;
            let payload_offset = get_enum_payload_offset(ctx, enum_ty, variant, subst)?;
            Ok(builder.ins().iadd_imm(enum_ptr, payload_offset as i64))
        },

        PlaceKind::Deref(inner) => {
            // Taking a reference to a dereference: &*ptr is just ptr
            compile_place_read(ctx, inner, builder, local_map, subst, stack_locals)
        },
    }
}

/// Get the type of a place expression.
#[allow(clippy::only_used_in_recursion)]
fn get_place_type(
    ctx: &mut CodegenContext<'_>,
    place: &Place,
    local_map: &HashMap<Id<Local>, Variable>,
    subst: &Substitution,
) -> Result<Id<Ty>, CodegenError> {
    match &place.kind {
        PlaceKind::Local(local_id) => {
            let local_def = ctx.mir.local(*local_id);
            Ok(local_def.ty)
        },

        PlaceKind::Global(name_id) => {
            // Find the static definition to get its type
            let static_def = ctx
                .mir
                .statics
                .iter()
                .find(|(_, def)| def.name == *name_id)
                .map(|(_, def)| def)
                .ok_or_else(|| {
                    let global_name = ctx.mir.name(*name_id);
                    CodegenError::Unsupported(format!("static variable not found: {}", global_name))
                })?;
            Ok(static_def.ty)
        },

        PlaceKind::Field { parent, name } => {
            let parent_ty = get_place_type(ctx, parent, local_map, subst)?;
            let (_, field_ty) = get_field_info(ctx, parent_ty, name, subst)?;
            Ok(field_ty)
        },

        PlaceKind::Index { parent, index } => {
            let parent_ty = get_place_type(ctx, parent, local_map, subst)?;
            let (_, field_ty) = get_field_by_index(ctx, parent, parent_ty, *index, subst)?;
            Ok(field_ty)
        },

        PlaceKind::Downcast { parent, .. } => get_place_type(ctx, parent, local_map, subst),

        PlaceKind::Deref(inner) => {
            let inner_ty = get_place_type(ctx, inner, local_map, subst)?;
            get_pointee_type(ctx, inner_ty)
        },
    }
}

/// Get the pointee type of a pointer/reference type.
fn get_pointee_type(ctx: &CodegenContext<'_>, ptr_ty: Id<Ty>) -> Result<Id<Ty>, CodegenError> {
    match ctx.mir.ty(ptr_ty) {
        MirTy::Pointer(inner) | MirTy::Ref(inner) | MirTy::RefMut(inner) => Ok(*inner),
        _ => Err(CodegenError::Unsupported(format!(
            "not a pointer/reference type: {:?}",
            ctx.mir.ty(ptr_ty)
        ))),
    }
}

/// Get field offset and type for a named field in a struct type.
fn get_field_info(
    ctx: &mut CodegenContext<'_>,
    parent_ty: Id<Ty>,
    field_name: &str,
    subst: &Substitution,
) -> Result<(usize, Id<Ty>), CodegenError> {
    let mir_ty = ctx.mir.ty(parent_ty).clone();

    let (struct_id, type_args) = match mir_ty {
        MirTy::Named { name, type_args } => {
            let name_data = ctx.mir.name(name);
            // Apply substitution to type_args to replace any type parameters
            let type_args: Vec<_> = type_args
                .iter()
                .map(|&ty| subst.apply_ty_readonly(ctx.mir, ty).unwrap_or(ty))
                .collect();
            let mut found = None;
            for (id, def) in ctx.mir.structs.iter() {
                if ctx.mir.name(def.name) == name_data {
                    found = Some(id);
                    break;
                }
            }
            let struct_id = found.ok_or_else(|| {
                CodegenError::Unsupported(format!("struct not found: {}", name_data))
            })?;
            (struct_id, type_args)
        },
        _ => {
            return Err(CodegenError::Unsupported(format!(
                "field access on non-struct type: {:?}",
                mir_ty
            )));
        },
    };

    // Get field offset from layout (pass substituted type_args for generic structs)
    let struct_def = ctx.mir.struct_def(struct_id);
    let struct_layout = ctx.layouts.struct_layout(struct_id, &type_args);
    let offset = *struct_layout.field_offsets.get(field_name).ok_or_else(|| {
        let struct_name = ctx.mir.name(struct_def.name);
        let available_fields: Vec<_> = struct_layout.field_offsets.keys().collect();
        CodegenError::Unsupported(format!(
            "unknown field: {} in struct {} (available: {:?})",
            field_name, struct_name, available_fields
        ))
    })?;

    // Get field type and apply substitution for generic structs
    let type_params = struct_def.type_params.clone();
    let mut field_ty = None;
    for field_id in &struct_def.fields {
        let field_def = &ctx.mir.fields[*field_id];
        if field_def.name == field_name {
            field_ty = Some(field_def.ty);
            break;
        }
    }

    let mut field_ty = field_ty.ok_or_else(|| {
        CodegenError::Unsupported(format!("field type not found: {}", field_name))
    })?;

    // Apply substitution from struct's type params to parent's type args
    if !type_params.is_empty() && !type_args.is_empty() {
        use crate::monomorphize::build_substitution;
        let field_subst = build_substitution(ctx.mir, &type_params, &type_args);
        // Use apply_ty_readonly since we're in codegen (MIR is immutable)
        // If the type isn't interned, we fall back to the original
        if let Ok(substituted_ty) = field_subst.apply_ty_readonly(ctx.mir, field_ty) {
            field_ty = substituted_ty;
        }
    }

    Ok((offset, field_ty))
}

/// Get field offset and type by index for a struct or tuple.
fn get_field_by_index(
    ctx: &mut CodegenContext<'_>,
    parent_place: &Place,
    parent_ty: Id<Ty>,
    index: usize,
    subst: &Substitution,
) -> Result<(usize, Id<Ty>), CodegenError> {
    // Check if the parent is a downcast - in that case, we need to find the variant struct
    if let PlaceKind::Downcast {
        parent: grandparent,
        variant,
    } = &parent_place.kind
    {
        // Get the enum type from the grandparent
        let enum_ty = get_place_type(ctx, grandparent, &HashMap::new(), subst)?;
        let mir_ty = ctx.mir.ty(enum_ty);

        if let MirTy::Named { name, type_args } = mir_ty {
            let name_data = ctx.mir.name(*name);
            let type_args = type_args.clone();

            // Find the enum
            for (_enum_id, enum_def) in ctx.mir.enums.iter() {
                let def_name = ctx.mir.name(enum_def.name);
                if def_name == name_data {
                    // Find the case
                    let case_id = enum_def.case_by_name(variant).ok_or_else(|| {
                        CodegenError::Unsupported(format!("enum case not found: {}", variant))
                    })?;
                    let case_def = &ctx.mir.enum_cases[case_id];

                    // Get the payload struct
                    let struct_id = case_def.struct_def.ok_or_else(|| {
                        CodegenError::Unsupported(format!(
                            "enum case {} has no struct_def",
                            variant
                        ))
                    })?;

                    // Pass the enum's type_args since payload struct uses the same type parameters
                    return get_struct_field_by_index(ctx, struct_id, &type_args, index, subst);
                }
            }

            return Err(CodegenError::Unsupported(format!(
                "enum not found for downcast: {}",
                name_data
            )));
        }
    }

    // Otherwise, it's a regular struct or tuple - look up by index
    let mir_ty = ctx.mir.ty(parent_ty);

    match mir_ty {
        MirTy::Named { name, type_args } => {
            let name_data = ctx.mir.name(*name);
            let type_args = type_args.clone();

            // Try to find as struct
            for (struct_id, def) in ctx.mir.structs.iter() {
                if ctx.mir.name(def.name) == name_data {
                    return get_struct_field_by_index(ctx, struct_id, &type_args, index, subst);
                }
            }

            Err(CodegenError::Unsupported(format!(
                "struct not found for index access: {}",
                name_data
            )))
        },
        MirTy::Tuple(elements) => {
            // For tuples, calculate offset sequentially
            let elements = elements.clone();
            if index >= elements.len() {
                return Err(CodegenError::Unsupported(format!(
                    "tuple index {} out of bounds (len {})",
                    index,
                    elements.len()
                )));
            }

            // Calculate offset by summing sizes of previous elements
            // Apply substitution to element types to get correct layouts
            let mut offset = 0usize;
            for (i, elem_ty) in elements.iter().enumerate() {
                let concrete_ty = subst
                    .apply_ty_readonly(ctx.mir, *elem_ty)
                    .unwrap_or(*elem_ty);
                let elem_layout = ctx.layouts.layout_of(concrete_ty);
                // Align to this element's alignment
                offset = (offset + elem_layout.align - 1) & !(elem_layout.align - 1);
                if i == index {
                    return Ok((offset, *elem_ty));
                }
                offset += elem_layout.size;
            }

            unreachable!()
        },
        _ => Err(CodegenError::Unsupported(format!(
            "index access on unsupported type: {:?}",
            mir_ty
        ))),
    }
}

/// Get a struct field by index.
fn get_struct_field_by_index(
    ctx: &mut CodegenContext<'_>,
    struct_id: kestrel_execution_graph::Id<kestrel_execution_graph::Struct>,
    type_args: &[Id<Ty>],
    index: usize,
    subst: &Substitution,
) -> Result<(usize, Id<Ty>), CodegenError> {
    let struct_def = ctx.mir.struct_def(struct_id);
    let fields: Vec<_> = struct_def.fields.clone();

    if index >= fields.len() {
        return Err(CodegenError::Unsupported(format!(
            "field index {} out of bounds (struct has {} fields)",
            index,
            fields.len()
        )));
    }

    let field_id = fields[index];
    let field_def = &ctx.mir.fields[field_id];
    let field_name = &field_def.name;
    let field_ty = field_def.ty;

    // Apply substitution to type_args to get concrete types
    let concrete_type_args: Vec<_> = type_args
        .iter()
        .map(|&ty| subst.apply_ty_readonly(ctx.mir, ty).unwrap_or(ty))
        .collect();

    // Get field offset from layout (pass substituted type_args for generic structs)
    let struct_layout = ctx.layouts.struct_layout(struct_id, &concrete_type_args);
    let offset = *struct_layout.field_offsets.get(field_name).ok_or_else(|| {
        CodegenError::Unsupported(format!("field offset not found: {}", field_name))
    })?;

    Ok((offset, field_ty))
}

fn ensure_primitive_value(
    ctx: &mut CodegenContext<'_>,
    subst: &Substitution,
    val: CraneliftValue,
    ty_opt: Option<Id<Ty>>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<CraneliftValue, CodegenError> {
    let Some(ty) = ty_opt else {
        return Ok(val);
    };
    let concrete_ty = subst.apply_ty_readonly(ctx.mir, ty).unwrap_or(ty);
    if is_aggregate_value_type(ctx.mir, concrete_ty) {
        // It's an aggregate wrapper, load the primitive value
        if let Ok((field_offset, field_ty)) = get_field_info(ctx, concrete_ty, "value", subst) {
            let cl_field_ty = translate_type(ctx.mir, field_ty, ctx.target);
            Ok(builder
                .ins()
                .load(cl_field_ty, MemFlags::new(), val, field_offset as i32))
        } else if let MirTy::Named { name, .. } = ctx.mir.ty(concrete_ty) {
            if let Some((_, struct_def)) = ctx.mir.structs.iter().find(|(_, s)| s.name == *name)
                && struct_def.fields.len() == 1
            {
                let field_id = struct_def.fields[0];
                let field_def = &ctx.mir.fields[field_id];
                if let Ok((field_offset, field_ty)) =
                    get_field_info(ctx, concrete_ty, &field_def.name, subst)
                {
                    let cl_field_ty = translate_type(ctx.mir, field_ty, ctx.target);
                    return Ok(builder.ins().load(
                        cl_field_ty,
                        MemFlags::new(),
                        val,
                        field_offset as i32,
                    ));
                }
            }
            Ok(val)
        } else {
            Ok(val)
        }
    } else {
        Ok(val)
    }
}

/// Wrap a primitive return value from an extern function into its wrapper struct type.
///
/// This is the inverse of `ensure_primitive_value`. When an extern function returns
/// a wrapper type like Int32 (which wraps lang.i32), the C ABI returns the raw primitive.
/// This function allocates a stack slot for the wrapper struct and stores the primitive
/// value into the struct's single field, returning a pointer to the struct.
///
/// Returns the original value unchanged if the type is not a wrapper type.
fn wrap_extern_return_value(
    ctx: &mut CodegenContext<'_>,
    subst: &Substitution,
    val: CraneliftValue,
    mir_ret_ty: Id<Ty>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<CraneliftValue, CodegenError> {
    let concrete_ret_ty = subst
        .apply_ty_readonly(ctx.mir, mir_ret_ty)
        .unwrap_or(mir_ret_ty);

    // Check if wrapper type (single-field struct wrapping primitive)
    let Some(inner_ty) = get_wrapper_primitive(ctx.mir, concrete_ret_ty) else {
        return Ok(val); // Not a wrapper, return as-is
    };

    // Verify inner is primitive
    if !matches!(
        ctx.mir.ty(inner_ty),
        MirTy::I8
            | MirTy::I16
            | MirTy::I32
            | MirTy::I64
            | MirTy::F16
            | MirTy::F32
            | MirTy::F64
            | MirTy::Bool
    ) {
        return Ok(val);
    }

    // Allocate stack slot for wrapper struct
    let layout = ctx.layouts.layout_of(concrete_ret_ty);
    let size = if layout.size == 0 { 1 } else { layout.size };
    let align = if layout.align == 0 { 1 } else { layout.align };
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        size as u32,
        align_to_shift(align),
    ));
    let struct_ptr = builder.ins().stack_addr(ptr_type, slot, 0);

    // Find field offset and store primitive
    if let MirTy::Named { name, type_args } = ctx.mir.ty(concrete_ret_ty) {
        let type_args = type_args.clone();
        if let Some((struct_id, struct_def)) = ctx.mir.structs.iter().find(|(_, s)| s.name == *name)
            && struct_def.fields.len() == 1
        {
            let field_def = &ctx.mir.fields[struct_def.fields[0]];
            let struct_layout = ctx.layouts.struct_layout(struct_id, &type_args);
            let field_offset = struct_layout
                .field_offsets
                .get(&field_def.name)
                .copied()
                .unwrap_or(0);
            builder
                .ins()
                .store(MemFlags::new(), val, struct_ptr, field_offset as i32);
            return Ok(struct_ptr);
        }
    }

    // Fallback: store at offset 0
    builder.ins().store(MemFlags::new(), val, struct_ptr, 0);
    Ok(struct_ptr)
}

/// Compile a pointer offset operation.
fn compile_ptr_offset(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    ptr: &Value,
    offset: &Value,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    stack_locals: &std::collections::HashSet<Id<Local>>,
) -> Result<CraneliftValue, CodegenError> {
    let ptr_val = compile_value(ctx, func_def, subst, ptr, builder, local_map, stack_locals)?;
    let offset_val = compile_value(
        ctx,
        func_def,
        subst,
        offset,
        builder,
        local_map,
        stack_locals,
    )?;
    let (_, offset_ty_opt) = get_value_layout(ctx, offset, local_map, subst)?;

    let mut offset_val = ensure_primitive_value(ctx, subst, offset_val, offset_ty_opt, builder)?;

    let ptr_ty = builder.func.dfg.value_type(ptr_val);
    let offset_cl_ty = builder.func.dfg.value_type(offset_val);

    if offset_cl_ty != ptr_ty {
        offset_val = builder.ins().sextend(ptr_ty, offset_val);
    }

    Ok(builder.ins().iadd(ptr_val, offset_val))
}

/// Compile a value (place or immediate).
pub fn compile_value(
    ctx: &mut CodegenContext<'_>,
    _func_def: &FunctionDef,
    subst: &Substitution,
    value: &Value,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    stack_locals: &std::collections::HashSet<Id<Local>>,
) -> Result<CraneliftValue, CodegenError> {
    match value {
        Value::Place(place) => {
            compile_place_read(ctx, place, builder, local_map, subst, stack_locals)
        },
        Value::Immediate(imm) => compile_immediate(ctx, subst, imm, builder),
        Value::Unreachable => Err(CodegenError::Unsupported(
            "cannot compile unreachable value - this indicates a MIR lowering bug".into(),
        )),
    }
}

/// Compile an immediate value.
fn compile_immediate(
    ctx: &mut CodegenContext<'_>,
    subst: &Substitution,
    imm: &Immediate,
    builder: &mut FunctionBuilder<'_>,
) -> Result<CraneliftValue, CodegenError> {
    match &imm.kind {
        ImmediateKind::IntLiteral { bits, value } => {
            let cl_type = match bits {
                IntBits::I8 => cl_types::I8,
                IntBits::I16 => cl_types::I16,
                IntBits::I32 => cl_types::I32,
                IntBits::I64 => cl_types::I64,
            };
            Ok(builder.ins().iconst(cl_type, *value as i64))
        },

        ImmediateKind::FloatLiteral { bits, value } => {
            match bits {
                FloatBits::F32 => Ok(builder.ins().f32const(*value as f32)),
                FloatBits::F64 => Ok(builder.ins().f64const(*value)),
                FloatBits::F16 => {
                    // F16 needs special handling
                    Err(CodegenError::Unsupported("f16 literals".to_string()))
                },
            }
        },

        ImmediateKind::BoolLiteral(b) => {
            Ok(builder.ins().iconst(cl_types::I8, if *b { 1 } else { 0 }))
        },

        ImmediateKind::Unit => {
            // Unit is zero-sized. Use pointer type to avoid type mismatches in phi nodes
            // when Unit values merge with aggregate pointers in control flow.
            let ptr_type = if ctx.target.is_64bit() {
                cl_types::I64
            } else {
                cl_types::I32
            };
            Ok(builder.ins().iconst(ptr_type, 0))
        },

        ImmediateKind::StringLiteral(s) => compile_string_literal(ctx, s, builder),

        ImmediateKind::StringPointer(s) => {
            // Just return the pointer to string data (no fat pointer struct)
            let ptr_type = if ctx.target.is_64bit() {
                cl_types::I64
            } else {
                cl_types::I32
            };
            let data_id = ctx.add_string_data(s)?;
            let data_ref = ctx.module.declare_data_in_func(data_id, builder.func);
            Ok(builder.ins().global_value(ptr_type, data_ref))
        },

        ImmediateKind::FunctionRef { name, type_args } => {
            // Get the function address as a pointer value
            let ptr_type = if ctx.target.is_64bit() {
                cl_types::I64
            } else {
                cl_types::I32
            };
            let ptr_size = if ctx.target.is_64bit() { 8 } else { 4 };

            // Apply substitution to type args
            let concrete_args: Vec<_> = type_args
                .iter()
                .map(|ty| {
                    subst
                        .apply_ty_readonly(ctx.mir, *ty)
                        .expect("type substitution failed for intrinsic call")
                })
                .collect();
            ensure_concrete_type_args(
                ctx.mir,
                &concrete_args,
                &format!("function reference {}", ctx.mir.name(*name)),
            )?;

            // Look up the function by name to get func_id for mangling
            let func_lookup = ctx.mir.functions.iter().find(|(_, def)| def.name == *name);

            let self_type = match func_lookup {
                Some((_, def)) if func_uses_self(ctx.mir, def) => {
                    // First try to get self_type from substitution
                    let st = match subst.get_self_type() {
                        Some(st) => st,
                        None => {
                            // Try to infer self_type from the method's containing type
                            infer_self_type_from_method_name(ctx, *name).ok_or_else(|| {
                                CodegenError::Unsupported(format!(
                                    "function reference requires Self type: {}",
                                    ctx.mir.name(*name)
                                ))
                            })?
                        },
                    };
                    if !type_is_concrete(ctx.mir, st) {
                        return Err(CodegenError::Unsupported(format!(
                            "unresolved Self type for function reference: {}",
                            ctx.mir.name(*name)
                        )));
                    }
                    Some(st)
                },
                _ => None,
            };

            // Look up the function by its mangled name (with param types for overloads)
            let mangled_name = ctx.resolve_symbol_name(*name, &concrete_args, self_type);
            let cl_func_id = ctx.func_ids_by_name.get(&mangled_name).ok_or_else(|| {
                CodegenError::Unsupported(format!(
                    "function not found for reference: {} (mangled: {})",
                    ctx.mir.name(*name),
                    mangled_name
                ))
            })?;

            // Get the function reference for use in this function
            let func_ref = ctx.module.declare_func_in_func(*cl_func_id, builder.func);
            // Get the address of the function
            let func_ptr = builder.ins().func_addr(ptr_type, func_ref);

            // Create a thick function struct: { func_ptr, env_ptr }
            // For a plain function reference, env_ptr is null.
            // This ensures compatibility with FuncThick types which all function types
            // are lowered to in the MIR.
            let thick_slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                (ptr_size * 2) as u32,
                align_to_shift(ptr_size),
            ));
            let thick_ptr = builder.ins().stack_addr(ptr_type, thick_slot, 0);

            // Store func_ptr at offset 0
            builder.ins().store(MemFlags::new(), func_ptr, thick_ptr, 0);
            // Store null env_ptr at offset ptr_size
            let null_env = builder.ins().iconst(ptr_type, 0);
            builder
                .ins()
                .store(MemFlags::new(), null_env, thick_ptr, ptr_size as i32);

            Ok(thick_ptr)
        },

        ImmediateKind::WitnessMethod {
            protocol,
            method,
            for_type,
        } => {
            // Apply substitution to for_type
            let concrete_for_type = subst
                .apply_ty_readonly(ctx.mir, *for_type)
                .unwrap_or(*for_type);
            if !type_is_concrete(ctx.mir, concrete_for_type) {
                return Err(CodegenError::Unsupported(
                    "unresolved Self type for witness method reference".to_string(),
                ));
            }

            // Resolve the witness to get the concrete function
            let (impl_name, impl_type_args) =
                resolve_witness(ctx.mir, *protocol, method, concrete_for_type)?;
            ensure_concrete_type_args(
                ctx.mir,
                &impl_type_args,
                &format!("witness method reference {}", ctx.mir.name(impl_name)),
            )?;

            // Get the function address
            let ptr_type = if ctx.target.is_64bit() {
                cl_types::I64
            } else {
                cl_types::I32
            };
            let ptr_size = if ctx.target.is_64bit() { 8 } else { 4 };

            // For witness method references, ALWAYS use concrete_for_type as self_type.
            // This matches what the monomorphization collection phase does - it uses
            // FunctionInstantiation::with_self_type for ALL witness calls, not just
            // those with self receivers.
            let self_type = Some(concrete_for_type);
            let mangled_name = ctx.resolve_symbol_name(impl_name, &impl_type_args, self_type);
            let cl_func_id = ctx.func_ids_by_name.get(&mangled_name).ok_or_else(|| {
                CodegenError::Unsupported(format!(
                    "witness method function not found: {} (mangled: {})",
                    ctx.mir.name(impl_name),
                    mangled_name
                ))
            })?;

            let func_ref = ctx.module.declare_func_in_func(*cl_func_id, builder.func);
            let func_ptr = builder.ins().func_addr(ptr_type, func_ref);

            // Create a thick function struct: { func_ptr, env_ptr }
            // For a witness method reference, env_ptr is null.
            // This ensures compatibility with FuncThick types which all function types
            // are lowered to in the MIR.
            let thick_slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                (ptr_size * 2) as u32,
                align_to_shift(ptr_size),
            ));
            let thick_ptr = builder.ins().stack_addr(ptr_type, thick_slot, 0);

            // Store func_ptr at offset 0
            builder.ins().store(MemFlags::new(), func_ptr, thick_ptr, 0);
            // Store null env_ptr at offset ptr_size
            let null_env = builder.ins().iconst(ptr_type, 0);
            builder
                .ins()
                .store(MemFlags::new(), null_env, thick_ptr, ptr_size as i32);

            Ok(thick_ptr)
        },

        ImmediateKind::NullPtr(_) => Ok(builder.ins().iconst(cl_types::I64, 0)),

        ImmediateKind::Error => Err(CodegenError::Unsupported("error immediate".to_string())),
    }
}

/// Compile a string literal.
///
/// String literals are compiled as fat pointers: { ptr_to_data, length }.
/// The string content is stored in the binary's data section.
fn compile_string_literal(
    ctx: &mut CodegenContext<'_>,
    s: &str,
    builder: &mut FunctionBuilder<'_>,
) -> Result<CraneliftValue, CodegenError> {
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    // Add string to data section
    let data_id = ctx.add_string_data(s)?;

    // Get reference to the string data in this function
    let data_ref = ctx.module.declare_data_in_func(data_id, builder.func);
    let str_ptr = builder.ins().global_value(ptr_type, data_ref);

    // Create length constant
    let str_len = builder.ins().iconst(ptr_type, s.len() as i64);

    // Allocate stack slot for the fat pointer struct (ptr + len = 16 bytes on 64-bit)
    let ptr_size = if ctx.target.is_64bit() { 8 } else { 4 };
    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        (ptr_size * 2) as u32,
        align_to_shift(ptr_size),
    ));
    let struct_ptr = builder.ins().stack_addr(ptr_type, slot, 0);

    // Store ptr at offset 0
    builder.ins().store(MemFlags::new(), str_ptr, struct_ptr, 0);
    // Store len at offset ptr_size
    builder
        .ins()
        .store(MemFlags::new(), str_len, struct_ptr, ptr_size as i32);

    Ok(struct_ptr)
}

/// Compile a binary operation.
fn compile_binop(
    ctx: &mut CodegenContext<'_>,
    op: BinOp,
    lhs: CraneliftValue,
    rhs: CraneliftValue,
    builder: &mut FunctionBuilder<'_>,
) -> Result<CraneliftValue, CodegenError> {
    let result = match op {
        // Signed integer arithmetic
        BinOp::AddSigned => builder.ins().iadd(lhs, rhs),
        BinOp::SubSigned => builder.ins().isub(lhs, rhs),
        BinOp::MulSigned => builder.ins().imul(lhs, rhs),
        BinOp::DivSigned => builder.ins().sdiv(lhs, rhs),
        BinOp::RemSigned => builder.ins().srem(lhs, rhs),

        // Unsigned integer arithmetic
        BinOp::AddUnsigned => builder.ins().iadd(lhs, rhs),
        BinOp::SubUnsigned => builder.ins().isub(lhs, rhs),
        BinOp::MulUnsigned => builder.ins().imul(lhs, rhs),
        BinOp::DivUnsigned => builder.ins().udiv(lhs, rhs),
        BinOp::RemUnsigned => builder.ins().urem(lhs, rhs),

        // Float arithmetic
        BinOp::FAdd => builder.ins().fadd(lhs, rhs),
        BinOp::FSub => builder.ins().fsub(lhs, rhs),
        BinOp::FMul => builder.ins().fmul(lhs, rhs),
        BinOp::FDiv => builder.ins().fdiv(lhs, rhs),

        // Bitwise operations
        BinOp::And => builder.ins().band(lhs, rhs),
        BinOp::Or => builder.ins().bor(lhs, rhs),
        BinOp::Xor => builder.ins().bxor(lhs, rhs),
        BinOp::Shl => builder.ins().ishl(lhs, rhs),
        BinOp::ShrSigned => builder.ins().sshr(lhs, rhs),
        BinOp::ShrUnsigned => builder.ins().ushr(lhs, rhs),

        // Integer comparisons
        // Note: icmp returns I8 on most platforms, no need to extend
        BinOp::Eq => builder
            .ins()
            .icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, lhs, rhs),
        BinOp::Ne => {
            builder
                .ins()
                .icmp(cranelift_codegen::ir::condcodes::IntCC::NotEqual, lhs, rhs)
        },
        BinOp::LtSigned => builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedLessThan,
            lhs,
            rhs,
        ),
        BinOp::LeSigned => builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedLessThanOrEqual,
            lhs,
            rhs,
        ),
        BinOp::GtSigned => builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThan,
            lhs,
            rhs,
        ),
        BinOp::GeSigned => builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThanOrEqual,
            lhs,
            rhs,
        ),
        BinOp::LtUnsigned => builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::UnsignedLessThan,
            lhs,
            rhs,
        ),
        BinOp::LeUnsigned => builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::UnsignedLessThanOrEqual,
            lhs,
            rhs,
        ),
        BinOp::GtUnsigned => builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::UnsignedGreaterThan,
            lhs,
            rhs,
        ),
        BinOp::GeUnsigned => builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::UnsignedGreaterThanOrEqual,
            lhs,
            rhs,
        ),

        // Float comparisons
        // Note: fcmp returns I8 on most platforms, no need to extend
        BinOp::FEq => {
            builder
                .ins()
                .fcmp(cranelift_codegen::ir::condcodes::FloatCC::Equal, lhs, rhs)
        },
        BinOp::FNe => builder.ins().fcmp(
            cranelift_codegen::ir::condcodes::FloatCC::NotEqual,
            lhs,
            rhs,
        ),
        BinOp::FLt => builder.ins().fcmp(
            cranelift_codegen::ir::condcodes::FloatCC::LessThan,
            lhs,
            rhs,
        ),
        BinOp::FLe => builder.ins().fcmp(
            cranelift_codegen::ir::condcodes::FloatCC::LessThanOrEqual,
            lhs,
            rhs,
        ),
        BinOp::FGt => builder.ins().fcmp(
            cranelift_codegen::ir::condcodes::FloatCC::GreaterThan,
            lhs,
            rhs,
        ),
        BinOp::FGe => builder.ins().fcmp(
            cranelift_codegen::ir::condcodes::FloatCC::GreaterThanOrEqual,
            lhs,
            rhs,
        ),

        // Boolean operations
        BinOp::BoolAnd => builder.ins().band(lhs, rhs),
        BinOp::BoolOr => builder.ins().bor(lhs, rhs),

        // String comparison - lhs and rhs are pointers to str fat pointers (ptr, len)
        BinOp::StrEq => {
            // String structs are at lhs and rhs addresses
            // Each has: ptr at offset 0, len at offset 8 (on 64-bit)
            let ptr_type = if ctx.target.is_64bit() {
                cranelift_codegen::ir::types::I64
            } else {
                cranelift_codegen::ir::types::I32
            };
            let ptr_size = if ctx.target.is_64bit() { 8i32 } else { 4i32 };

            // Load lengths from both strings
            let lhs_len = builder.ins().load(ptr_type, MemFlags::new(), lhs, ptr_size);
            let rhs_len = builder.ins().load(ptr_type, MemFlags::new(), rhs, ptr_size);

            // Compare lengths first
            let len_eq = builder.ins().icmp(
                cranelift_codegen::ir::condcodes::IntCC::Equal,
                lhs_len,
                rhs_len,
            );

            // Create blocks for length-equal path and join
            let len_eq_block = builder.create_block();
            let join_block = builder.create_block();
            builder.append_block_param(join_block, cranelift_codegen::ir::types::I8);

            // If lengths differ, result is false (0)
            let false_val = builder.ins().iconst(cranelift_codegen::ir::types::I8, 0);
            builder
                .ins()
                .brif(len_eq, len_eq_block, &[], join_block, &[false_val]);

            // Length-equal block: compare contents
            builder.switch_to_block(len_eq_block);
            builder.seal_block(len_eq_block);

            // Load pointers
            let lhs_ptr = builder.ins().load(ptr_type, MemFlags::new(), lhs, 0);
            let rhs_ptr = builder.ins().load(ptr_type, MemFlags::new(), rhs, 0);

            // Get the memcmp function that was declared at module level
            let memcmp_id = ctx.func_ids_by_name.get("memcmp").ok_or_else(|| {
                CodegenError::Unsupported("memcmp function not declared".to_string())
            })?;
            let memcmp_ref = ctx.module.declare_func_in_func(*memcmp_id, builder.func);

            // Call memcmp(lhs_ptr, rhs_ptr, len)
            let memcmp_result = builder.ins().call(memcmp_ref, &[lhs_ptr, rhs_ptr, lhs_len]);
            let cmp_result = builder.inst_results(memcmp_result)[0];

            // Result is true (1) if memcmp returns 0
            let zero = builder.ins().iconst(cranelift_codegen::ir::types::I32, 0);
            let content_eq = builder.ins().icmp(
                cranelift_codegen::ir::condcodes::IntCC::Equal,
                cmp_result,
                zero,
            );
            builder.ins().jump(join_block, &[content_eq]);

            // Join block
            builder.switch_to_block(join_block);
            builder.seal_block(join_block);
            builder.block_params(join_block)[0]
        },
    };

    Ok(result)
}

/// Compile a unary operation.
fn compile_unop(
    _ctx: &CodegenContext<'_>,
    op: UnOp,
    operand: CraneliftValue,
    builder: &mut FunctionBuilder<'_>,
) -> Result<CraneliftValue, CodegenError> {
    let result = match op {
        UnOp::Neg => builder.ins().ineg(operand),
        UnOp::FNeg => builder.ins().fneg(operand),
        UnOp::Not => builder.ins().bnot(operand),
        UnOp::BoolNot => {
            // Boolean not: xor with 1
            let one = builder.ins().iconst(cl_types::I8, 1);
            builder.ins().bxor(operand, one)
        },
        UnOp::Popcount => builder.ins().popcnt(operand),
        UnOp::Clz => builder.ins().clz(operand),
        UnOp::Ctz => builder.ins().ctz(operand),
        UnOp::Bswap => builder.ins().bswap(operand),
    };

    Ok(result)
}

/// Compile a function call.
///
/// Returns the return value of the call. For unit-returning functions,
/// returns a dummy I8 value.
pub fn compile_call(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    callee: &Callee,
    args: &[CallArg],
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    stack_locals: &std::collections::HashSet<Id<Local>>,
) -> Result<CraneliftValue, CodegenError> {
    match callee {
        Callee::Direct { name, type_args } => {
            // Apply substitution to type args
            let concrete_args: Vec<_> = type_args
                .iter()
                .map(|ty| {
                    subst
                        .apply_ty_readonly(ctx.mir, *ty)
                        .expect("type substitution failed for direct call")
                })
                .collect();
            ensure_concrete_type_args(
                ctx.mir,
                &concrete_args,
                &format!("direct call {}", ctx.mir.name(*name)),
            )?;

            // Look up the Cranelift FuncId for this function.
            // For extern functions, use the symbol name from extern_info.
            // Otherwise, use the mangled name (with param types for overloads).
            let callee_lookup = ctx.mir.functions.iter().find(|(_, def)| def.name == *name);

            let self_type = match callee_lookup {
                Some((_, def)) if func_uses_self(ctx.mir, def) => {
                    // First try to get self_type from substitution
                    let st = match subst.get_self_type() {
                        Some(st) => st,
                        None => {
                            // Try to infer self_type from the method's containing type
                            // e.g., Test.Widget.create -> Self = Test.Widget
                            infer_self_type_from_method_name(ctx, *name).ok_or_else(|| {
                                CodegenError::Unsupported(format!(
                                    "direct call requires Self type: {}",
                                    ctx.mir.name(*name)
                                ))
                            })?
                        },
                    };
                    if !type_is_concrete(ctx.mir, st) {
                        return Err(CodegenError::Unsupported(format!(
                            "unresolved Self type for direct call: {}",
                            ctx.mir.name(*name)
                        )));
                    }
                    Some(st)
                },
                _ => None,
            };
            let lookup_name = ctx.resolve_symbol_name(*name, &concrete_args, self_type);

            let cl_func_id = ctx.func_ids_by_name.get(&lookup_name).ok_or_else(|| {
                CodegenError::Unsupported(format!(
                    "function not found: {} (lookup: {})",
                    ctx.mir.name(*name),
                    lookup_name
                ))
            })?;

            let callee_def = callee_lookup.map(|(_, def)| def);
            let is_extern = callee_def.map(|def| def.is_extern()).unwrap_or(false);
            let mut needs_sret = false;
            let mut ret_ptr = None;
            if let Some(def) = callee_def {
                let mut callee_subst =
                    build_substitution(ctx.mir, &def.type_params, &concrete_args);
                if let Some(st) = self_type {
                    callee_subst.set_self_type(st);
                }
                let concrete_ret = callee_subst
                    .apply_ty_readonly(ctx.mir, def.ret)
                    .unwrap_or(def.ret);
                needs_sret = !def.is_extern()
                    && !is_main_function(ctx, def)
                    && needs_sret_for_type(ctx.mir, concrete_ret);
                if needs_sret {
                    let layout = ctx.layouts.layout_of(concrete_ret);
                    let size = if layout.size == 0 { 1 } else { layout.size };
                    let align = if layout.align == 0 { 1 } else { layout.align };
                    let ptr_type = if ctx.target.is_64bit() {
                        cl_types::I64
                    } else {
                        cl_types::I32
                    };
                    let slot = builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        size as u32,
                        align_to_shift(align),
                    ));
                    ret_ptr = Some(builder.ins().stack_addr(ptr_type, slot, 0));
                }
            }

            // Get the function reference for use in this function
            let func_ref = ctx.module.declare_func_in_func(*cl_func_id, builder.func);

            // Compile arguments with proper PassingMode handling
            let mut arg_values = Vec::with_capacity(args.len() + if needs_sret { 1 } else { 0 });
            if let Some(ptr) = ret_ptr {
                arg_values.push(ptr);
            }
            for arg in args {
                let val = compile_call_arg(
                    ctx,
                    func_def,
                    subst,
                    arg,
                    builder,
                    local_map,
                    stack_locals,
                    is_extern,
                )?;
                arg_values.push(val);
            }

            // Emit the call instruction
            let call_inst = builder.ins().call(func_ref, &arg_values);

            // Get the return value (if any)
            if let Some(ptr) = ret_ptr {
                return Ok(ptr);
            }

            let results = builder.inst_results(call_inst);
            if results.is_empty() {
                // Unit return - use pointer type to avoid type mismatches in phi nodes
                let ptr_type = if ctx.target.is_64bit() {
                    cl_types::I64
                } else {
                    cl_types::I32
                };
                Ok(builder.ins().iconst(ptr_type, 0))
            } else {
                let raw_result = results[0];
                // For extern functions, wrap primitive returns back into wrapper structs
                if is_extern {
                    if let Some(def) = callee_def {
                        let mut callee_subst =
                            build_substitution(ctx.mir, &def.type_params, &concrete_args);
                        if let Some(st) = self_type {
                            callee_subst.set_self_type(st);
                        }
                        wrap_extern_return_value(ctx, &callee_subst, raw_result, def.ret, builder)
                    } else {
                        Ok(raw_result)
                    }
                } else {
                    Ok(raw_result)
                }
            }
        },

        Callee::Thin(place) => compile_thin_call(
            ctx,
            func_def,
            subst,
            place,
            args,
            builder,
            local_map,
            stack_locals,
        ),

        Callee::Thick(place) => compile_thick_call(
            ctx,
            func_def,
            subst,
            place,
            args,
            builder,
            local_map,
            stack_locals,
        ),

        Callee::Witness {
            protocol,
            method,
            for_type,
            method_type_args,
        } => {
            // First apply substitution to for_type
            let substituted_for_type = subst
                .apply_ty_readonly(ctx.mir, *for_type)
                .unwrap_or(*for_type);

            // If substituted type uses SelfType, we need to resolve it further
            let concrete_for_type = if type_uses_self(ctx.mir, substituted_for_type) {
                // Get the self type from the substitution
                let self_ty = subst.get_self_type().ok_or_else(|| {
                    CodegenError::Unsupported(format!(
                        "witness call requires Self type for: {}",
                        ctx.mir.name(*protocol)
                    ))
                })?;
                if !type_is_concrete(ctx.mir, self_ty) {
                    return Err(CodegenError::Unsupported(format!(
                        "unresolved Self type for witness call: {:?}",
                        ctx.mir.ty(self_ty)
                    )));
                }
                self_ty
            } else {
                // Substitution already applied, check if concrete
                if !type_is_concrete(ctx.mir, substituted_for_type) {
                    return Err(CodegenError::Unsupported(format!(
                        "unresolved type for witness call: {:?}",
                        ctx.mir.ty(substituted_for_type)
                    )));
                }
                substituted_for_type
            };

            // Apply substitution to method_type_args (the method's own type parameters)
            let concrete_method_type_args: Vec<_> = method_type_args
                .iter()
                .filter_map(|ty| subst.apply_ty_readonly(ctx.mir, *ty).ok())
                .collect();

            // Resolve the witness to get the concrete implementation
            let (impl_name, mut impl_type_args) =
                resolve_witness(ctx.mir, *protocol, method, concrete_for_type)?;

            // Append the method's own type arguments (e.g., H in hash[H])
            impl_type_args.extend(concrete_method_type_args);
            ensure_concrete_type_args(
                ctx.mir,
                &impl_type_args,
                &format!("witness call {}", ctx.mir.name(impl_name)),
            )?;

            // Look up the function by name to get func_id for mangling
            let func_lookup = ctx
                .mir
                .functions
                .iter()
                .find(|(_, def)| def.name == impl_name);
            let callee_def = func_lookup.map(|(_, def)| def);
            // For witness calls, ALWAYS use concrete_for_type as self_type.
            // This matches what the monomorphization collection phase does - it uses
            // FunctionInstantiation::with_self_type for ALL witness calls, not just
            // those with self receivers. This is necessary for extension methods like
            // `fromResidual` which are static but still need the self_type to distinguish
            // between different instantiations.
            let self_type = Some(concrete_for_type);
            let mangled_name = ctx.resolve_symbol_name(impl_name, &impl_type_args, self_type);
            let cl_func_id = ctx.func_ids_by_name.get(&mangled_name).ok_or_else(|| {
                CodegenError::Unsupported(format!(
                    "witness method function not found: {} (mangled: {})",
                    ctx.mir.name(impl_name),
                    mangled_name
                ))
            })?;

            let func_ref = ctx.module.declare_func_in_func(*cl_func_id, builder.func);
            let is_extern = callee_def.map(|def| def.is_extern()).unwrap_or(false);
            let mut needs_sret = false;
            let mut ret_ptr = None;
            if let Some(def) = callee_def {
                let mut callee_subst =
                    build_substitution(ctx.mir, &def.type_params, &impl_type_args);
                if let Some(st) = self_type {
                    callee_subst.set_self_type(st);
                }
                let concrete_ret = callee_subst
                    .apply_ty_readonly(ctx.mir, def.ret)
                    .unwrap_or(def.ret);
                needs_sret = !def.is_extern()
                    && !is_main_function(ctx, def)
                    && needs_sret_for_type(ctx.mir, concrete_ret);
                if needs_sret {
                    let layout = ctx.layouts.layout_of(concrete_ret);
                    let size = if layout.size == 0 { 1 } else { layout.size };
                    let align = if layout.align == 0 { 1 } else { layout.align };
                    let ptr_type = if ctx.target.is_64bit() {
                        cl_types::I64
                    } else {
                        cl_types::I32
                    };
                    let slot = builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        size as u32,
                        align_to_shift(align),
                    ));
                    ret_ptr = Some(builder.ins().stack_addr(ptr_type, slot, 0));
                }
            }

            // Compile arguments with proper PassingMode handling
            let mut arg_values = Vec::with_capacity(args.len() + if needs_sret { 1 } else { 0 });
            if let Some(ptr) = ret_ptr {
                arg_values.push(ptr);
            }
            for arg in args {
                let val = compile_call_arg(
                    ctx,
                    func_def,
                    subst,
                    arg,
                    builder,
                    local_map,
                    stack_locals,
                    is_extern,
                )?;
                arg_values.push(val);
            }

            // Emit the call instruction
            let call_inst = builder.ins().call(func_ref, &arg_values);

            // Get the return value (if any)
            if let Some(ptr) = ret_ptr {
                return Ok(ptr);
            }

            let results = builder.inst_results(call_inst);
            if results.is_empty() {
                // Unit return - use pointer type to avoid type mismatches in phi nodes
                let ptr_type = if ctx.target.is_64bit() {
                    cl_types::I64
                } else {
                    cl_types::I32
                };
                Ok(builder.ins().iconst(ptr_type, 0))
            } else {
                Ok(results[0])
            }
        },
    }
}

/// Compile a type cast operation.
fn compile_cast(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    kind: CastKind,
    operand: &Value,
    target: Id<Ty>,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    stack_locals: &std::collections::HashSet<Id<Local>>,
) -> Result<CraneliftValue, CodegenError> {
    let val = compile_value(
        ctx,
        func_def,
        subst,
        operand,
        builder,
        local_map,
        stack_locals,
    )?;
    let target_ty = translate_type(ctx.mir, target, ctx.target);

    match kind {
        CastKind::IntWiden => {
            // Integer widening - sign-extend to larger integer type
            // Kestrel integers are signed, so use sextend
            let src_ty = builder.func.dfg.value_type(val);
            if target_ty.bits() > src_ty.bits() {
                Ok(builder.ins().sextend(target_ty, val))
            } else {
                // Same size or smaller - this shouldn't happen for IntWiden
                // but handle gracefully
                Ok(val)
            }
        },

        CastKind::IntTruncate => {
            // Integer narrowing - truncate to smaller integer type
            let src_ty = builder.func.dfg.value_type(val);
            if target_ty.bits() < src_ty.bits() {
                Ok(builder.ins().ireduce(target_ty, val))
            } else {
                // Same size or larger - this shouldn't happen for IntTruncate
                Ok(val)
            }
        },

        CastKind::IntToFloat => {
            // Convert signed integer to float
            Ok(builder.ins().fcvt_from_sint(target_ty, val))
        },

        CastKind::FloatToInt => {
            // Convert float to signed integer
            // Use fcvt_to_sint_sat for saturating conversion (safer, no undefined behavior)
            Ok(builder.ins().fcvt_to_sint_sat(target_ty, val))
        },

        CastKind::FloatWiden => {
            // f32 -> f64 promotion
            Ok(builder.ins().fpromote(target_ty, val))
        },

        CastKind::FloatTruncate => {
            // f64 -> f32 demotion
            Ok(builder.ins().fdemote(target_ty, val))
        },

        CastKind::PtrBitcast => {
            // Pointer bitcast - same representation, just reinterpret the type
            // At the IR level, all pointers have the same representation
            Ok(val)
        },

        CastKind::RefToImmut => {
            // &var T -> &T conversion - same representation, just type change
            Ok(val)
        },
    }
}

/// Compile str.ptr operation - extract the pointer from a string fat pointer.
fn compile_str_ptr(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    value: &Value,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    stack_locals: &std::collections::HashSet<Id<Local>>,
) -> Result<CraneliftValue, CodegenError> {
    let str_ptr = compile_value(
        ctx,
        func_def,
        subst,
        value,
        builder,
        local_map,
        stack_locals,
    )?;

    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    // String is a fat pointer: { ptr: p[i8], len: i64 }
    // Load the ptr field at offset 0
    Ok(builder.ins().load(ptr_type, MemFlags::new(), str_ptr, 0))
}

/// Compile str.len operation - extract the length from a string fat pointer.
fn compile_str_len(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    value: &Value,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    stack_locals: &std::collections::HashSet<Id<Local>>,
) -> Result<CraneliftValue, CodegenError> {
    let str_ptr = compile_value(
        ctx,
        func_def,
        subst,
        value,
        builder,
        local_map,
        stack_locals,
    )?;

    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };
    let ptr_size = if ctx.target.is_64bit() { 8 } else { 4 };

    // String is a fat pointer: { ptr: p[i8], len: i64 }
    // Load the len field at offset ptr_size
    Ok(builder
        .ins()
        .load(ptr_type, MemFlags::new(), str_ptr, ptr_size))
}

/// Compile str.from_parts operation - create a string fat pointer from ptr and len.
fn compile_str_from_parts(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    ptr: &Value,
    len: &Value,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    stack_locals: &std::collections::HashSet<Id<Local>>,
) -> Result<CraneliftValue, CodegenError> {
    let ptr_val = compile_value(ctx, func_def, subst, ptr, builder, local_map, stack_locals)?;
    let len_val = compile_value(ctx, func_def, subst, len, builder, local_map, stack_locals)?;

    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };
    let ptr_size: i32 = if ctx.target.is_64bit() { 8 } else { 4 };

    // Allocate stack slot for the fat pointer struct (ptr + len)
    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        (ptr_size * 2) as u32,
        align_to_shift(ptr_size as usize),
    ));
    let struct_ptr = builder.ins().stack_addr(ptr_type, slot, 0);

    // Store ptr at offset 0
    builder.ins().store(MemFlags::new(), ptr_val, struct_ptr, 0);
    // Store len at offset ptr_size
    builder
        .ins()
        .store(MemFlags::new(), len_val, struct_ptr, ptr_size);

    Ok(struct_ptr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_execution_graph::{MirTy, TypeParamDef, TypeParamOwner};

    #[test]
    fn concrete_type_args_reject_type_params() {
        let mut mir = MirContext::new();
        let int_ty = mir.ty_i64();
        assert!(type_is_concrete(&mir, int_ty));

        let tp = mir.type_params.alloc(TypeParamDef {
            meta: Default::default(),
            priors: vec![],
            name: "T".to_string(),
            owner: TypeParamOwner::Function(Id::from_raw(0)),
        });
        let tp_ty = mir.intern_type(MirTy::TypeParam(tp));
        assert!(!type_is_concrete(&mir, tp_ty));

        assert!(ensure_concrete_type_args(&mir, &[int_ty], "test").is_ok());
        assert!(ensure_concrete_type_args(&mir, &[tp_ty], "test").is_err());
    }

    #[test]
    fn concrete_type_args_reject_self_type() {
        let mut mir = MirContext::new();
        let self_ty = mir.ty_self();
        assert!(!type_is_concrete(&mir, self_ty));
        assert!(ensure_concrete_type_args(&mir, &[self_ty], "test").is_err());
    }
}

/// Get the type of a place expression (for determining function signature in indirect calls).
#[allow(clippy::only_used_in_recursion)]
fn get_place_type_for_call(
    ctx: &CodegenContext<'_>,
    place: &Place,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<Id<Ty>, CodegenError> {
    match &place.kind {
        PlaceKind::Local(local_id) => {
            let local_def = ctx.mir.local(*local_id);
            Ok(local_def.ty)
        },

        PlaceKind::Global(name_id) => {
            // Find the static definition to get its type
            let static_def = ctx
                .mir
                .statics
                .iter()
                .find(|(_, def)| def.name == *name_id)
                .map(|(_, def)| def)
                .ok_or_else(|| {
                    let global_name = ctx.mir.name(*name_id);
                    CodegenError::Unsupported(format!("static variable not found: {}", global_name))
                })?;
            Ok(static_def.ty)
        },

        PlaceKind::Field { parent, name } => {
            let parent_ty = get_place_type_for_call(ctx, parent, local_map)?;
            get_field_type_for_call(ctx, parent_ty, name)
        },

        PlaceKind::Index { parent, index } => {
            let parent_ty = get_place_type_for_call(ctx, parent, local_map)?;
            get_field_type_by_index_for_call(ctx, parent_ty, *index)
        },

        PlaceKind::Downcast { parent, .. } => get_place_type_for_call(ctx, parent, local_map),

        PlaceKind::Deref(inner) => {
            let inner_ty = get_place_type_for_call(ctx, inner, local_map)?;
            match ctx.mir.ty(inner_ty) {
                MirTy::Pointer(pointee) | MirTy::Ref(pointee) | MirTy::RefMut(pointee) => {
                    Ok(*pointee)
                },
                _ => Err(CodegenError::Unsupported(format!(
                    "deref of non-pointer type: {:?}",
                    ctx.mir.ty(inner_ty)
                ))),
            }
        },
    }
}

/// Get the type of a field by name.
fn get_field_type_for_call(
    ctx: &CodegenContext<'_>,
    parent_ty: Id<Ty>,
    field_name: &str,
) -> Result<Id<Ty>, CodegenError> {
    let mir_ty = ctx.mir.ty(parent_ty);

    if let MirTy::Named { name, type_args } = mir_ty {
        let name_data = ctx.mir.name(*name);
        for (_struct_id, def) in ctx.mir.structs.iter() {
            if ctx.mir.name(def.name) == name_data {
                for field_id in &def.fields {
                    let field_def = &ctx.mir.fields[*field_id];
                    if field_def.name == field_name {
                        let mut field_ty = field_def.ty;

                        // Apply substitution from struct's type params to concrete type args
                        let type_params = &def.type_params;
                        if !type_params.is_empty() && type_params.len() == type_args.len() {
                            let subst = build_substitution(ctx.mir, type_params, type_args);
                            if let Ok(substituted_ty) = subst.apply_ty_readonly(ctx.mir, field_ty) {
                                field_ty = substituted_ty;
                            }
                        }

                        return Ok(field_ty);
                    }
                }
            }
        }
    }

    Err(CodegenError::Unsupported(format!(
        "field {} not found in type {:?}",
        field_name, mir_ty
    )))
}

/// Get the type of a field by index.
fn get_field_type_by_index_for_call(
    ctx: &CodegenContext<'_>,
    parent_ty: Id<Ty>,
    index: usize,
) -> Result<Id<Ty>, CodegenError> {
    let mir_ty = ctx.mir.ty(parent_ty);

    match mir_ty {
        MirTy::Tuple(elements) => {
            if index < elements.len() {
                Ok(elements[index])
            } else {
                Err(CodegenError::Unsupported(format!(
                    "tuple index {} out of bounds",
                    index
                )))
            }
        },
        MirTy::Named { name, type_args } => {
            let name_data = ctx.mir.name(*name);
            for (_struct_id, def) in ctx.mir.structs.iter() {
                if ctx.mir.name(def.name) == name_data && index < def.fields.len() {
                    let field_id = def.fields[index];
                    let field_def = &ctx.mir.fields[field_id];
                    let mut field_ty = field_def.ty;

                    // Apply substitution from struct's type params to concrete type args
                    let type_params = &def.type_params;
                    if !type_params.is_empty() && type_params.len() == type_args.len() {
                        let subst = build_substitution(ctx.mir, type_params, type_args);
                        if let Ok(substituted_ty) = subst.apply_ty_readonly(ctx.mir, field_ty) {
                            field_ty = substituted_ty;
                        }
                    }

                    return Ok(field_ty);
                }
            }
            Err(CodegenError::Unsupported(format!(
                "field index {} out of bounds in {:?}",
                index, name_data
            )))
        },
        _ => Err(CodegenError::Unsupported(format!(
            "index access on unsupported type: {:?}",
            mir_ty
        ))),
    }
}

/// Resolve through references to find the underlying function type.
fn resolve_func_type(ctx: &CodegenContext<'_>, ty: Id<Ty>) -> Id<Ty> {
    let mir_ty = ctx.mir.ty(ty);
    match mir_ty {
        MirTy::Ref(inner) | MirTy::RefMut(inner) | MirTy::Pointer(inner) => {
            resolve_func_type(ctx, *inner)
        },
        _ => ty,
    }
}

/// Build a Cranelift signature from a function type.
fn build_signature_from_func_type(
    ctx: &CodegenContext<'_>,
    func_ty: Id<Ty>,
    builder: &FunctionBuilder<'_>,
) -> Result<Signature, CodegenError> {
    let mir_ty = ctx.mir.ty(func_ty);

    let (params, ret) = match mir_ty {
        MirTy::FuncThin { params, ret } => (params.clone(), *ret),
        MirTy::FuncThick { params, ret } => (params.clone(), *ret),
        _ => {
            return Err(CodegenError::Unsupported(format!(
                "not a function type: {:?}",
                mir_ty
            )));
        },
    };

    let call_conv = builder.func.signature.call_conv;
    let mut sig = Signature::new(call_conv);
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };
    let needs_sret = needs_sret_for_type(ctx.mir, ret);

    if needs_sret {
        sig.params.push(AbiParam::new(ptr_type));
    }

    // Add parameters
    for param_ty in &params {
        let cl_type = translate_type_ext(ctx.mir, *param_ty, ctx.target, false); // TODO: check if extern
        sig.params.push(AbiParam::new(cl_type));
    }

    // Add return type if not unit
    let ret_mir_ty = ctx.mir.ty(ret);
    if !matches!(ret_mir_ty, MirTy::Unit) && !needs_sret {
        let cl_type = translate_type_ext(ctx.mir, ret, ctx.target, false); // TODO: check if extern
        sig.returns.push(AbiParam::new(cl_type));
    }

    Ok(sig)
}

/// Compile an ApplyPartial rvalue.
///
/// ApplyPartial creates a thick callable (closure) from a function reference and
/// captured values. The result is a struct: { func_ptr: *const (), env_ptr: *const () }
///
/// For non-capturing closures (empty captures), we still allocate an environment struct
/// (possibly zero-sized) to keep the code path uniform.
///
/// For capturing closures, we:
/// 1. Get the function pointer for the closure's call function
/// 2. Allocate stack space for the environment struct
/// 3. Store each captured value into the appropriate field
/// 4. Create the thick callable struct with (func_ptr, env_ptr)
fn compile_apply_partial(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    func: Id<QualifiedName>,
    captures: &[Value],
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    stack_locals: &std::collections::HashSet<Id<Local>>,
) -> Result<CraneliftValue, CodegenError> {
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };
    let ptr_size = if ctx.target.is_64bit() { 8 } else { 4 };

    // 1. Find the closure function and get its environment struct
    let (closure_func_id, env_struct_id) = find_closure_function_and_env(ctx, func)?;
    let closure_def = &ctx.mir.functions[closure_func_id];

    // Closures inherit type parameters from their parent function.
    // We need to apply the current substitution to get the concrete type args
    // for this closure instantiation.
    let mut closure_type_args = Vec::with_capacity(closure_def.type_params.len());
    for &tp in &closure_def.type_params {
        // Look up the type param in the substitution
        if let Some(concrete_ty) = subst.get(tp) {
            closure_type_args.push(concrete_ty);
        } else {
            // Type param not in substitution - this shouldn't happen for properly
            // instantiated closures, but return an error if it does
            return Err(CodegenError::Unsupported(format!(
                "closure type param {:?} not in substitution for apply partial: {}",
                tp,
                ctx.mir.name(func)
            )));
        }
    }

    // Check for any remaining type parameters or Self types after substitution
    // (this would indicate incomplete monomorphization)
    for &type_arg in &closure_type_args {
        let ty = ctx.mir.ty(type_arg);
        if matches!(ty, MirTy::TypeParam(_)) {
            return Err(CodegenError::Unsupported(format!(
                "closure has unsubstituted type parameter in apply partial: {}",
                ctx.mir.name(func)
            )));
        }
        if matches!(ty, MirTy::SelfType) {
            return Err(CodegenError::Unsupported(format!(
                "closure has unsubstituted Self type in apply partial: {}",
                ctx.mir.name(func)
            )));
        }
    }

    // 2. Get the function pointer for the closure function
    // Use the instantiated type args derived from the parent function's substitution
    let mangled_name =
        ctx.symbol_name_for_function(closure_func_id, closure_def, &closure_type_args, None);
    let cl_func_id = ctx.func_ids_by_name.get(&mangled_name).ok_or_else(|| {
        CodegenError::Unsupported(format!(
            "closure function not found: {} (mangled: {})",
            ctx.mir.name(func),
            mangled_name
        ))
    })?;

    let func_ref = ctx.module.declare_func_in_func(*cl_func_id, builder.func);
    let func_ptr = builder.ins().func_addr(ptr_type, func_ref);

    // 3. Allocate and populate the environment struct
    let env_ptr = if let Some(env_struct_id) = env_struct_id {
        // Get the environment struct layout.
        // Env structs inherit type params from their parent closure function,
        // so we pass the same type args used for the closure instantiation.
        let env_layout = ctx.layouts.struct_layout(env_struct_id, &closure_type_args);
        let layout = env_layout.layout;
        let field_offsets = env_layout.field_offsets.clone();

        // Allocate stack space for the environment
        // Use at least 1 byte to avoid zero-sized allocations
        let alloc_size = if layout.size == 0 { 1 } else { layout.size };
        let alloc_align = if layout.align == 0 { 1 } else { layout.align };

        let slot = builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            alloc_size as u32,
            align_to_shift(alloc_align),
        ));
        let env_ptr = builder.ins().stack_addr(ptr_type, slot, 0);

        // Store each capture into the environment struct
        let env_struct_def = ctx.mir.struct_def(env_struct_id);
        let field_ids: Vec<_> = env_struct_def.fields.clone();

        for (i, capture_value) in captures.iter().enumerate() {
            if i >= field_ids.len() {
                break;
            }
            let field_id = field_ids[i];
            let field_def = &ctx.mir.fields[field_id];
            let field_name = &field_def.name;
            let field_ty = field_def.ty;

            let offset = field_offsets.get(field_name).copied().unwrap_or(0);

            // Compile the capture value
            let val = compile_value(
                ctx,
                func_def,
                subst,
                capture_value,
                builder,
                local_map,
                stack_locals,
            )?;

            // Check if this is an aggregate type that needs copying (structs, thick callables, etc.)
            let concrete_field_ty = subst
                .apply_ty_readonly(ctx.mir, field_ty)
                .unwrap_or(field_ty);
            let field_mir_ty = ctx.mir.ty(concrete_field_ty);
            let is_aggregate = match field_mir_ty {
                MirTy::Named { .. } => is_struct_type(ctx, concrete_field_ty),
                MirTy::FuncThick { .. } => true, // Thick callables are 16-byte structs
                MirTy::Tuple(_) => true,
                MirTy::Str => true,
                _ => false,
            };

            if is_aggregate {
                // Copy nested struct data
                let nested_layout = ctx.layouts.layout_of(concrete_field_ty);
                let dest_ptr = if offset == 0 {
                    env_ptr
                } else {
                    builder.ins().iadd_imm(env_ptr, offset as i64)
                };
                let words = nested_layout.size.div_ceil(8);
                for w in 0..words {
                    let word_offset = (w * 8) as i32;
                    let word = builder
                        .ins()
                        .load(cl_types::I64, MemFlags::new(), val, word_offset);
                    builder
                        .ins()
                        .store(MemFlags::new(), word, dest_ptr, word_offset);
                }
            } else {
                // Store primitive value directly
                builder
                    .ins()
                    .store(MemFlags::new(), val, env_ptr, offset as i32);
            }
        }

        env_ptr
    } else {
        // No environment struct found - use null pointer
        // This shouldn't happen for properly lowered closures, but handle gracefully
        builder.ins().iconst(ptr_type, 0)
    };

    // 4. Create the thick callable struct: { func_ptr, env_ptr }
    let thick_slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        (ptr_size * 2) as u32,
        align_to_shift(ptr_size),
    ));
    let thick_ptr = builder.ins().stack_addr(ptr_type, thick_slot, 0);

    // Store func_ptr at offset 0
    builder.ins().store(MemFlags::new(), func_ptr, thick_ptr, 0);
    // Store env_ptr at offset ptr_size
    builder
        .ins()
        .store(MemFlags::new(), env_ptr, thick_ptr, ptr_size as i32);

    Ok(thick_ptr)
}

/// Find a closure function by its qualified name and return its ID along with
/// the environment struct ID (if any).
fn find_closure_function_and_env(
    ctx: &CodegenContext<'_>,
    func_name: Id<QualifiedName>,
) -> Result<(Id<Function>, Option<Id<Struct>>), CodegenError> {
    // Find the function by name
    for (func_id, func_def) in ctx.mir.functions.iter() {
        if func_def.name == func_name {
            // Check if it has ClosureCall origin with an env struct
            let env_struct_id = match &func_def.meta.origin {
                Some(Origin::ClosureCall { env_struct, .. }) => Some(*env_struct),
                _ => None,
            };
            return Ok((func_id, env_struct_id));
        }
    }

    Err(CodegenError::Unsupported(format!(
        "closure function not found: {}",
        ctx.mir.name(func_name)
    )))
}

/// Compile a function-to-escaping conversion.
///
/// This converts a regular function to an escaping (thick) function pointer.
/// The thick callable has the layout: { func_ptr: *const (), env_ptr: *const () }
/// Since there are no captures, env_ptr is null.
///
/// Note: The function being converted must have a compatible signature. When called
/// through the thick pointer, it will receive a null env pointer as the first argument,
/// which it should ignore. This means we need to create a wrapper function that:
/// 1. Accepts (env_ptr, args...)
/// 2. Ignores env_ptr and calls the original function with just args...
///
/// For now, we assume the function already has the correct signature (env_ptr as first param).
fn compile_func_to_escaping(
    ctx: &mut CodegenContext<'_>,
    func: Id<QualifiedName>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<CraneliftValue, CodegenError> {
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };
    let ptr_size = if ctx.target.is_64bit() { 8 } else { 4 };

    // Look up the function by name to get func_id for mangling
    let func_lookup = ctx.mir.functions.iter().find(|(_, def)| def.name == func);

    // Get the function pointer
    let mangled_name = match func_lookup {
        Some((_, def)) => {
            if !def.type_params.is_empty() {
                return Err(CodegenError::Unsupported(format!(
                    "generic function requires type arguments for escaping conversion: {}",
                    ctx.mir.name(func)
                )));
            }
            if func_uses_self(ctx.mir, def) {
                return Err(CodegenError::Unsupported(format!(
                    "function requires Self type for escaping conversion: {}",
                    ctx.mir.name(func)
                )));
            }
            ctx.resolve_symbol_name(func, &[], None)
        },
        None => mangle_name(ctx.mir, func, &[]), // Fallback
    };
    let cl_func_id = ctx.func_ids_by_name.get(&mangled_name).ok_or_else(|| {
        CodegenError::Unsupported(format!(
            "function not found for escaping conversion: {} (mangled: {})",
            ctx.mir.name(func),
            mangled_name
        ))
    })?;

    let func_ref = ctx.module.declare_func_in_func(*cl_func_id, builder.func);
    let func_ptr = builder.ins().func_addr(ptr_type, func_ref);

    // Create a null environment pointer (no captures)
    let env_ptr = builder.ins().iconst(ptr_type, 0);

    // Create the thick callable struct: { func_ptr, env_ptr }
    let thick_slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        (ptr_size * 2) as u32,
        align_to_shift(ptr_size),
    ));
    let thick_ptr = builder.ins().stack_addr(ptr_type, thick_slot, 0);

    // Store func_ptr at offset 0
    builder.ins().store(MemFlags::new(), func_ptr, thick_ptr, 0);
    // Store env_ptr at offset ptr_size
    builder
        .ins()
        .store(MemFlags::new(), env_ptr, thick_ptr, ptr_size as i32);

    Ok(thick_ptr)
}

/// Compile a thin function pointer call.
///
/// A thin function pointer is just an address - we load it and call indirectly.
fn compile_thin_call(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    place: &Place,
    args: &[CallArg],
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    stack_locals: &std::collections::HashSet<Id<Local>>,
) -> Result<CraneliftValue, CodegenError> {
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    // Get the function pointer value
    let place_value = compile_place_read(ctx, place, builder, local_map, subst, stack_locals)?;

    let func_ptr = if let PlaceKind::Local(local_id) = place.kind {
        if stack_locals.contains(&local_id) {
            builder
                .ins()
                .load(ptr_type, MemFlags::new(), place_value, 0)
        } else {
            place_value
        }
    } else {
        place_value
    };

    // Get the type of the place to determine the function signature
    let func_ty = get_place_type_for_call(ctx, place, local_map)?;
    let resolved_ty = resolve_func_type(ctx, func_ty);
    let ret_ty = match ctx.mir.ty(resolved_ty) {
        MirTy::FuncThin { ret, .. } | MirTy::FuncThick { ret, .. } => *ret,
        _ => {
            return Err(CodegenError::Unsupported(format!(
                "not a function type: {:?}",
                ctx.mir.ty(resolved_ty)
            )));
        },
    };
    let needs_sret = needs_sret_for_type(ctx.mir, ret_ty);
    let mut ret_ptr = None;
    if needs_sret {
        let layout = ctx.layouts.layout_of(ret_ty);
        let size = if layout.size == 0 { 1 } else { layout.size };
        let align = if layout.align == 0 { 1 } else { layout.align };
        let slot = builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            size as u32,
            align_to_shift(align),
        ));
        ret_ptr = Some(builder.ins().stack_addr(ptr_type, slot, 0));
    }

    // Build the signature
    let sig = build_signature_from_func_type(ctx, func_ty, builder)?;
    let sig_ref = builder.import_signature(sig);

    // Compile arguments with proper PassingMode handling
    let mut arg_values = Vec::with_capacity(args.len() + if needs_sret { 1 } else { 0 });
    if let Some(ptr) = ret_ptr {
        arg_values.push(ptr);
    }
    for arg in args {
        let val = compile_call_arg(
            ctx,
            func_def,
            subst,
            arg,
            builder,
            local_map,
            stack_locals,
            false,
        )?;
        arg_values.push(val);
    }

    // Make the indirect call
    let call_inst = builder.ins().call_indirect(sig_ref, func_ptr, &arg_values);

    // Get the return value (if any)
    if let Some(ptr) = ret_ptr {
        return Ok(ptr);
    }

    let results = builder.inst_results(call_inst);
    if results.is_empty() {
        // Unit return - use pointer type to avoid type mismatches in phi nodes
        let ptr_type = if ctx.target.is_64bit() {
            cl_types::I64
        } else {
            cl_types::I32
        };
        Ok(builder.ins().iconst(ptr_type, 0))
    } else {
        // Check if the return type is a string - if so, copy the fat pointer
        if matches!(ctx.mir.ty(ret_ty), kestrel_execution_graph::MirTy::Str) {
            return Ok(copy_string_return_value(ctx, results[0], builder));
        }
        Ok(results[0])
    }
}

/// Compile a thick function pointer (closure) call.
///
/// A thick callable has the layout: { func_ptr: *const (), env_ptr: *const () }
/// The function pointer expects the environment pointer as the first argument.
///
/// Note: The MIR lowering may use Callee::Thick for all function calls for
/// simplicity. We check the actual type and handle FuncThin types appropriately.
fn compile_thick_call(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    place: &Place,
    args: &[CallArg],
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    stack_locals: &std::collections::HashSet<Id<Local>>,
) -> Result<CraneliftValue, CodegenError> {
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };
    let ptr_size = if ctx.target.is_64bit() { 8 } else { 4 };

    // Get the type of the place to determine how to handle the call
    let func_ty = get_place_type_for_call(ctx, place, local_map)?;
    let resolved_ty = resolve_func_type(ctx, func_ty);
    let mir_ty = ctx.mir.ty(resolved_ty);

    // Check if this is actually a thin function type
    // The MIR lowering may use Callee::Thick for all function calls
    match mir_ty {
        MirTy::FuncThin { params, ret } => {
            // For thin function types, the value is the function pointer directly
            // (not a struct with func_ptr + env_ptr)
            //
            // Note: If this is a parameter local, the value is a POINTER to the
            // function pointer (because Kestrel passes all parameters by pointer).
            // In that case, we need to load from it.
            let place_value =
                compile_place_read(ctx, place, builder, local_map, subst, stack_locals)?;

            let func_ptr = if let PlaceKind::Local(local_id) = place.kind {
                if stack_locals.contains(&local_id) {
                    builder
                        .ins()
                        .load(ptr_type, MemFlags::new(), place_value, 0)
                } else {
                    place_value
                }
            } else {
                place_value
            };

            let call_conv = builder.func.signature.call_conv;
            let mut sig = Signature::new(call_conv);
            let needs_sret = needs_sret_for_type(ctx.mir, *ret);
            let mut ret_ptr = None;
            if needs_sret {
                sig.params.push(AbiParam::new(ptr_type));
                let layout = ctx.layouts.layout_of(*ret);
                let size = if layout.size == 0 { 1 } else { layout.size };
                let align = if layout.align == 0 { 1 } else { layout.align };
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    size as u32,
                    align_to_shift(align),
                ));
                ret_ptr = Some(builder.ins().stack_addr(ptr_type, slot, 0));
            }

            for param_ty in params {
                let cl_type = translate_type(ctx.mir, *param_ty, ctx.target);
                sig.params.push(AbiParam::new(cl_type));
            }

            let ret_mir_ty = ctx.mir.ty(*ret);
            if !matches!(ret_mir_ty, MirTy::Unit) && !needs_sret {
                let cl_type = translate_type(ctx.mir, *ret, ctx.target);
                sig.returns.push(AbiParam::new(cl_type));
            }

            let sig_ref = builder.import_signature(sig);

            let mut arg_values = Vec::with_capacity(args.len() + if needs_sret { 1 } else { 0 });
            if let Some(ptr) = ret_ptr {
                arg_values.push(ptr);
            }
            for arg in args {
                let val = compile_call_arg(
                    ctx,
                    func_def,
                    subst,
                    arg,
                    builder,
                    local_map,
                    stack_locals,
                    false,
                )?;
                arg_values.push(val);
            }

            let call_inst = builder.ins().call_indirect(sig_ref, func_ptr, &arg_values);
            if let Some(ptr) = ret_ptr {
                return Ok(ptr);
            }
            let results = builder.inst_results(call_inst);
            if results.is_empty() {
                // Unit return - use pointer type to avoid type mismatches in phi nodes
                let ptr_type = if ctx.target.is_64bit() {
                    cl_types::I64
                } else {
                    cl_types::I32
                };
                Ok(builder.ins().iconst(ptr_type, 0))
            } else {
                // Check if return type is string - if so, copy the fat pointer
                if matches!(ctx.mir.ty(*ret), MirTy::Str) {
                    return Ok(copy_string_return_value(ctx, results[0], builder));
                }
                Ok(results[0])
            }
        },
        MirTy::FuncThick { params, ret } => {
            // Apply substitution to params and ret to handle generic closures
            let params: Vec<_> = params
                .iter()
                .map(|&ty| subst.apply_ty_readonly(ctx.mir, ty).unwrap_or(ty))
                .collect();
            let ret = subst.apply_ty_readonly(ctx.mir, *ret).unwrap_or(*ret);

            // For thick function types, the value is a struct with func_ptr and env_ptr
            let place_value =
                compile_place_read(ctx, place, builder, local_map, subst, stack_locals)?;

            // Determine the actual thick_ptr based on the place and type
            // If this is a parameter local with a reference type to a thick function,
            // we need to dereference appropriately.
            let thick_ptr = if let PlaceKind::Local(local_id) = place.kind {
                if stack_locals.contains(&local_id) {
                    builder
                        .ins()
                        .load(ptr_type, MemFlags::new(), place_value, 0)
                } else {
                    place_value
                }
            } else {
                place_value
            };

            // Load the function pointer from offset 0
            let func_ptr = builder.ins().load(ptr_type, MemFlags::new(), thick_ptr, 0);

            // Load the environment pointer from offset ptr_size
            let env_ptr = builder
                .ins()
                .load(ptr_type, MemFlags::new(), thick_ptr, ptr_size);

            let call_conv = builder.func.signature.call_conv;
            let mut sig = Signature::new(call_conv);
            let needs_sret = needs_sret_for_type(ctx.mir, ret);
            let mut ret_ptr = None;
            if needs_sret {
                sig.params.push(AbiParam::new(ptr_type));
                let layout = ctx.layouts.layout_of(ret);
                let size = if layout.size == 0 { 1 } else { layout.size };
                let align = if layout.align == 0 { 1 } else { layout.align };
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    size as u32,
                    align_to_shift(align),
                ));
                ret_ptr = Some(builder.ins().stack_addr(ptr_type, slot, 0));
            }

            // First parameter is the environment pointer
            sig.params.push(AbiParam::new(ptr_type));

            // Then add the regular parameters
            for param_ty in &params {
                let cl_type = translate_type(ctx.mir, *param_ty, ctx.target);
                sig.params.push(AbiParam::new(cl_type));
            }

            // Add return type if not unit
            let ret_mir_ty = ctx.mir.ty(ret);
            if !matches!(ret_mir_ty, MirTy::Unit) && !needs_sret {
                let cl_type = translate_type(ctx.mir, ret, ctx.target);
                sig.returns.push(AbiParam::new(cl_type));
            }

            let sig_ref = builder.import_signature(sig);

            // Compile arguments with proper PassingMode handling
            // env_ptr is the first argument
            let mut arg_values =
                Vec::with_capacity(args.len() + 1 + if needs_sret { 1 } else { 0 });
            if let Some(ptr) = ret_ptr {
                arg_values.push(ptr);
            }
            arg_values.push(env_ptr);
            for arg in args {
                let val = compile_call_arg(
                    ctx,
                    func_def,
                    subst,
                    arg,
                    builder,
                    local_map,
                    stack_locals,
                    false,
                )?;
                arg_values.push(val);
            }

            // Make the indirect call
            let call_inst = builder.ins().call_indirect(sig_ref, func_ptr, &arg_values);

            if let Some(ptr) = ret_ptr {
                return Ok(ptr);
            }

            let results = builder.inst_results(call_inst);
            if results.is_empty() {
                // Unit return - use pointer type to avoid type mismatches in phi nodes
                let ptr_type = if ctx.target.is_64bit() {
                    cl_types::I64
                } else {
                    cl_types::I32
                };
                Ok(builder.ins().iconst(ptr_type, 0))
            } else {
                // Check if return type is string - if so, copy the fat pointer
                if matches!(ctx.mir.ty(ret), MirTy::Str) {
                    return Ok(copy_string_return_value(ctx, results[0], builder));
                }
                Ok(results[0])
            }
        },
        _ => Err(CodegenError::Unsupported(format!(
            "not a function type: {:?}",
            mir_ty
        ))),
    }
}

/// Copy a string return value from callee's stack to caller's stack.
/// This is necessary because the callee's stack is deallocated after return.
fn copy_string_return_value(
    ctx: &CodegenContext<'_>,
    src_ptr: CraneliftValue,
    builder: &mut FunctionBuilder<'_>,
) -> CraneliftValue {
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };
    let ptr_size = if ctx.target.is_64bit() { 8 } else { 4 };

    // Allocate space in our stack frame for the fat pointer (ptr, len)
    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        (ptr_size * 2) as u32,
        align_to_shift(ptr_size),
    ));
    let dest_ptr = builder.ins().stack_addr(ptr_type, slot, 0);

    // Copy the ptr field (offset 0)
    let str_ptr = builder.ins().load(ptr_type, MemFlags::new(), src_ptr, 0);
    builder.ins().store(MemFlags::new(), str_ptr, dest_ptr, 0);

    // Copy the len field (offset ptr_size)
    let str_len = builder
        .ins()
        .load(ptr_type, MemFlags::new(), src_ptr, ptr_size as i32);
    builder
        .ins()
        .store(MemFlags::new(), str_len, dest_ptr, ptr_size as i32);

    dest_ptr
}
