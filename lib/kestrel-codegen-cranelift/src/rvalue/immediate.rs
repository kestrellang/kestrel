//! Immediate value compilation.

use crate::common;
use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::function::FunctionState;
use crate::types;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{
    self, InstBuilder, MemFlags, StackSlotData, StackSlotKind, Value as CrValue,
};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use kestrel_codegen2::mangle_function_with_self;
use kestrel_mir::{FloatBits, FunctionKind, Immediate, ImmediateKind, IntBits, MirTy};

/// Compile an immediate value to a Cranelift value.
pub fn compile_immediate(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    imm: &Immediate,
) -> Result<CrValue, CodegenError> {
    let ptr_ty = common::ptr_type(ctx.target);

    match &imm.kind {
        ImmediateKind::IntLiteral { bits, value } => {
            let cl_ty = types::int_bits_to_type(*bits);
            Ok(builder.ins().iconst(cl_ty, *value as i64))
        },

        ImmediateKind::FloatLiteral { bits, value } => match bits {
            FloatBits::F16 => Err(CodegenError::Unsupported(
                "f16 literals not yet supported".into(),
            )),
            FloatBits::F32 => Ok(builder.ins().f32const(*value as f32)),
            FloatBits::F64 => Ok(builder.ins().f64const(*value)),
        },

        ImmediateKind::BoolLiteral(val) => Ok(builder.ins().iconst(ir::types::I8, *val as i64)),

        ImmediateKind::StringLiteral(s) => {
            // Create a fat pointer (ptr, len) on the stack
            let data_id = ctx.get_or_create_string_data(s)?;
            let gv = ctx.cl_module.declare_data_in_func(data_id, builder.func);
            let str_ptr = builder.ins().global_value(ptr_ty, gv);
            let str_len = builder.ins().iconst(ir::types::I64, s.len() as i64);

            // Allocate stack slot for the fat pointer
            let ptr_size = ctx.target.pointer_size();
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                (ptr_size * 2) as u32,
                common::align_to_shift(ptr_size),
            ));
            let addr = builder.ins().stack_addr(ptr_ty, slot, Offset32::new(0));
            builder
                .ins()
                .store(MemFlags::new(), str_ptr, addr, Offset32::new(0));
            builder.ins().store(
                MemFlags::new(),
                str_len,
                addr,
                Offset32::new(ptr_size as i32),
            );

            Ok(addr)
        },

        ImmediateKind::StringPointer(s) => {
            let data_id = ctx.get_or_create_string_data(s)?;
            let gv = ctx.cl_module.declare_data_in_func(data_id, builder.func);
            Ok(builder.ins().global_value(ptr_ty, gv))
        },

        ImmediateKind::Unit => {
            // Unit is zero-sized — use pointer-type zero for phi node compatibility
            let ptr_ty = common::ptr_type(ctx.target);
            Ok(builder.ins().iconst(ptr_ty, 0))
        },

        ImmediateKind::FunctionRef { func, type_args } => {
            // Look up the function's mangled name and get its address
            let func_def = ctx
                .entity_to_func
                .get(func)
                .map(|id| &ctx.module.functions[id.index()]);
            let mangled = if let Some(fd) = func_def {
                // For init/deinit/method of non-generic types, compute self_type
                // from the parent so mangling matches the declared signature
                let self_type = match &fd.kind {
                    FunctionKind::Initializer { parent }
                    | FunctionKind::Deinit { parent }
                    | FunctionKind::Method { parent, .. } => Some(MirTy::Named {
                        entity: *parent,
                        type_args: type_args.to_vec(),
                    }),
                    _ => None,
                };
                mangle_function_with_self(ctx.module, fd, type_args, self_type.as_ref())
            } else {
                return Err(CodegenError::Unsupported(format!(
                    "FunctionRef: unknown entity {:?}",
                    func
                )));
            };

            if let Some(&func_id) = ctx.func_ids_by_name.get(&mangled) {
                let func_ref = ctx.cl_module.declare_func_in_func(func_id, builder.func);
                Ok(builder.ins().func_addr(ptr_ty, func_ref))
            } else {
                Err(CodegenError::Unsupported(format!(
                    "FunctionRef: undeclared function '{mangled}'"
                )))
            }
        },

        ImmediateKind::WitnessMethod {
            protocol,
            method,
            for_type,
        } => {
            // Resolve through witness table to get concrete function
            Err(CodegenError::Unsupported(
                "WitnessMethod immediate not yet implemented".into(),
            ))
        },

        ImmediateKind::NullPtr(_) => Ok(builder.ins().iconst(ptr_ty, 0)),

        ImmediateKind::Error => Ok(builder.ins().iconst(ir::types::I8, 0)),
    }
}
