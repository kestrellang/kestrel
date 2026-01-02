//! Rvalue compilation.

use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::place::compile_place_read;

use kestrel_codegen::mangle_name;
use kestrel_execution_graph::{
    BinOp, CallArg, Callee, FloatBits, FunctionDef, Id, Immediate, ImmediateKind, IntBits, Local,
    Rvalue, UnOp, Value,
};

use cranelift_codegen::ir::types as cl_types;
use cranelift_codegen::ir::{InstBuilder, Value as CraneliftValue};
use cranelift_frontend::{FunctionBuilder, Variable};
use cranelift_module::Module;

use std::collections::HashMap;

/// Compile an rvalue to a Cranelift value.
pub fn compile_rvalue(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    rvalue: &Rvalue,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<CraneliftValue, CodegenError> {
    match rvalue {
        Rvalue::Use(imm) => compile_immediate(ctx, imm, builder),

        Rvalue::Copy(place) | Rvalue::Move(place) => {
            compile_place_read(ctx, place, builder, local_map)
        }

        Rvalue::BinaryOp { op, lhs, rhs } => {
            let lhs_val = compile_value(ctx, func_def, lhs, builder, local_map)?;
            let rhs_val = compile_value(ctx, func_def, rhs, builder, local_map)?;
            compile_binop(ctx, *op, lhs_val, rhs_val, builder)
        }

        Rvalue::UnaryOp { op, operand } => {
            let operand_val = compile_value(ctx, func_def, operand, builder, local_map)?;
            compile_unop(ctx, *op, operand_val, builder)
        }

        Rvalue::Call { callee, args } => {
            compile_call(ctx, func_def, callee, args, builder, local_map)
        }

        // TODO: Implement remaining rvalues
        _ => Err(CodegenError::Unsupported(format!("rvalue: {:?}", rvalue))),
    }
}

/// Compile a value (place or immediate).
pub fn compile_value(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    value: &Value,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<CraneliftValue, CodegenError> {
    match value {
        Value::Place(place) => compile_place_read(ctx, place, builder, local_map),
        Value::Immediate(imm) => compile_immediate(ctx, imm, builder),
    }
}

/// Compile an immediate value.
fn compile_immediate(
    ctx: &CodegenContext<'_>,
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
        }

        ImmediateKind::FloatLiteral { bits, value } => {
            match bits {
                FloatBits::F32 => Ok(builder.ins().f32const(*value as f32)),
                FloatBits::F64 => Ok(builder.ins().f64const(*value)),
                FloatBits::F16 => {
                    // F16 needs special handling
                    Err(CodegenError::Unsupported("f16 literals".to_string()))
                }
            }
        }

        ImmediateKind::BoolLiteral(b) => {
            Ok(builder.ins().iconst(cl_types::I8, if *b { 1 } else { 0 }))
        }

        ImmediateKind::Unit => {
            // Unit is zero-sized, return dummy value
            Ok(builder.ins().iconst(cl_types::I8, 0))
        }

        ImmediateKind::StringLiteral(_) => {
            // TODO: String literals need data section handling
            Err(CodegenError::Unsupported("string literals".to_string()))
        }

        ImmediateKind::FunctionRef { .. } => {
            // TODO: Function references
            Err(CodegenError::Unsupported("function references".to_string()))
        }

        ImmediateKind::WitnessMethod { .. } => {
            // TODO: Witness method references
            Err(CodegenError::Unsupported("witness methods".to_string()))
        }

        ImmediateKind::NullPtr(_) => Ok(builder.ins().iconst(cl_types::I64, 0)),

        ImmediateKind::Error => Err(CodegenError::Unsupported("error immediate".to_string())),
    }
}

/// Compile a binary operation.
fn compile_binop(
    ctx: &CodegenContext<'_>,
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
        }
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
        }
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
    };

    Ok(result)
}

/// Compile a unary operation.
fn compile_unop(
    ctx: &CodegenContext<'_>,
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
        }
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
    callee: &Callee,
    args: &[CallArg],
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<CraneliftValue, CodegenError> {
    match callee {
        Callee::Direct { name, type_args } => {
            // Look up the Cranelift FuncId for this function.
            // We need to find the function by matching its qualified name.
            let mangled_name = mangle_name(ctx.mir, *name, type_args);

            let cl_func_id = ctx.func_ids_by_name.get(&mangled_name).ok_or_else(|| {
                CodegenError::Unsupported(format!(
                    "function not found: {} (mangled: {})",
                    ctx.mir.name(*name),
                    mangled_name
                ))
            })?;

            // Get the function reference for use in this function
            let func_ref = ctx.module.declare_func_in_func(*cl_func_id, builder.func);

            // Compile arguments
            let mut arg_values = Vec::with_capacity(args.len());
            for arg in args {
                let val = compile_value(ctx, func_def, &arg.value, builder, local_map)?;
                arg_values.push(val);
            }

            // Emit the call instruction
            let call_inst = builder.ins().call(func_ref, &arg_values);

            // Get the return value (if any)
            let results = builder.inst_results(call_inst);
            if results.is_empty() {
                // Unit return - return a dummy value
                Ok(builder.ins().iconst(cl_types::I8, 0))
            } else {
                Ok(results[0])
            }
        }

        Callee::Thin(_place) => Err(CodegenError::Unsupported(
            "thin function pointer calls".to_string(),
        )),

        Callee::Thick(_place) => Err(CodegenError::Unsupported(
            "thick function pointer calls".to_string(),
        )),

        Callee::Witness {
            protocol,
            method,
            for_type,
        } => Err(CodegenError::Unsupported(format!(
            "witness method call: {}.{} for {:?}",
            ctx.mir.name(*protocol),
            method,
            for_type
        ))),
    }
}
