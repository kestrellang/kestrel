use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, StackSlotData, StackSlotKind, Value};
use cranelift_frontend::FunctionBuilder;

use crate::ty::TypeRepr;

pub fn copy_aggregate(builder: &mut FunctionBuilder, size: u64, dest: Value, src: Value) {
    if size == 0 {
        return;
    }
    let mut offset = 0u64;

    while offset + 8 <= size {
        let val = builder
            .ins()
            .load(ir::types::I64, MemFlags::new(), src, Offset32::new(offset as i32));
        builder
            .ins()
            .store(MemFlags::new(), val, dest, Offset32::new(offset as i32));
        offset += 8;
    }
    if offset + 4 <= size {
        let val = builder
            .ins()
            .load(ir::types::I32, MemFlags::new(), src, Offset32::new(offset as i32));
        builder
            .ins()
            .store(MemFlags::new(), val, dest, Offset32::new(offset as i32));
        offset += 4;
    }
    if offset + 2 <= size {
        let val = builder
            .ins()
            .load(ir::types::I16, MemFlags::new(), src, Offset32::new(offset as i32));
        builder
            .ins()
            .store(MemFlags::new(), val, dest, Offset32::new(offset as i32));
        offset += 2;
    }
    if offset < size {
        let val = builder
            .ins()
            .load(ir::types::I8, MemFlags::new(), src, Offset32::new(offset as i32));
        builder
            .ins()
            .store(MemFlags::new(), val, dest, Offset32::new(offset as i32));
    }
}

pub fn zero_memory(builder: &mut FunctionBuilder, ptr: Value, size: u64) {
    if size == 0 {
        return;
    }
    let mut offset = 0u64;

    while offset + 8 <= size {
        let zero = builder.ins().iconst(ir::types::I64, 0);
        builder
            .ins()
            .store(MemFlags::new(), zero, ptr, Offset32::new(offset as i32));
        offset += 8;
    }
    if offset + 4 <= size {
        let zero = builder.ins().iconst(ir::types::I32, 0);
        builder
            .ins()
            .store(MemFlags::new(), zero, ptr, Offset32::new(offset as i32));
        offset += 4;
    }
    if offset + 2 <= size {
        let zero = builder.ins().iconst(ir::types::I16, 0);
        builder
            .ins()
            .store(MemFlags::new(), zero, ptr, Offset32::new(offset as i32));
        offset += 2;
    }
    if offset < size {
        let zero = builder.ins().iconst(ir::types::I8, 0);
        builder
            .ins()
            .store(MemFlags::new(), zero, ptr, Offset32::new(offset as i32));
    }
}

pub fn alloc_stack_slot(
    builder: &mut FunctionBuilder,
    size: u64,
    align: u64,
    ptr_ty: ir::Type,
) -> Value {
    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        size.max(1) as u32,
        align.trailing_zeros() as u8,
    ));
    builder.ins().stack_addr(ptr_ty, slot, Offset32::new(0))
}

pub fn store_to_repr(
    builder: &mut FunctionBuilder,
    repr: TypeRepr,
    dest: Value,
    value: Value,
) {
    match repr {
        TypeRepr::Scalar(_) => {
            builder
                .ins()
                .store(MemFlags::new(), value, dest, Offset32::new(0));
        }
        TypeRepr::Aggregate { size, .. } => {
            copy_aggregate(builder, size, dest, value);
        }
        TypeRepr::Zst => {}
    }
}

pub fn load_from_repr(
    builder: &mut FunctionBuilder,
    repr: TypeRepr,
    addr: Value,
    ptr_ty: ir::Type,
) -> Value {
    match repr {
        TypeRepr::Scalar(t) => builder
            .ins()
            .load(t, MemFlags::new(), addr, Offset32::new(0)),
        TypeRepr::Aggregate { .. } => addr,
        TypeRepr::Zst => builder.ins().iconst(ptr_ty, 0),
    }
}
