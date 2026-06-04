//! Memory helpers — the boundary where the integer address model (Option A,
//! pointer-width scalars are `i64`/`i32`) meets LLVM's typed memory ops. Every
//! load/store/memcpy/memset `inttoptr`s the integer address to an LLVM `ptr`
//! right at the access site; LLVM folds the redundant `inttoptr(ptrtoint x)`.
//!
//! Faithful port of the Cranelift backend's `mem.rs`: `copy_aggregate` and
//! `zero_memory` use `llvm.memcpy`/`llvm.memset` (vs the manual byte loop), and
//! `alloc_stack_slot` lives on `FuncCompiler` (it needs the entry block) — see
//! `func.rs`.

use inkwell::AddressSpace;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::types::IntType;
use inkwell::values::{BasicValueEnum, IntValue, PointerValue};

use crate::ty::TypeRepr;

/// The pointer-width integer type (`i64` on 64-bit, `i32` on 32-bit).
pub fn ptr_int_type(cx: &Context, ptr_size: u64) -> IntType<'_> {
    if ptr_size == 8 {
        cx.i64_type()
    } else {
        cx.i32_type()
    }
}

/// A pointer-width integer constant (used for addresses, offsets, sizes).
pub fn ptr_const<'ctx>(cx: &'ctx Context, ptr_size: u64, value: i64) -> IntValue<'ctx> {
    ptr_int_type(cx, ptr_size).const_int(value as u64, false)
}

/// Reinterpret an integer address as an LLVM `ptr` for a memory access.
pub fn int_to_ptr<'ctx>(
    cx: &'ctx Context,
    builder: &Builder<'ctx>,
    addr: IntValue<'ctx>,
) -> PointerValue<'ctx> {
    builder
        .build_int_to_ptr(addr, cx.ptr_type(AddressSpace::default()), "p")
        .unwrap()
}

/// Reinterpret an LLVM `ptr` as a pointer-width integer address.
pub fn ptr_to_int<'ctx>(
    cx: &'ctx Context,
    builder: &Builder<'ctx>,
    ptr: PointerValue<'ctx>,
    ptr_size: u64,
) -> IntValue<'ctx> {
    builder
        .build_ptr_to_int(ptr, ptr_int_type(cx, ptr_size), "a")
        .unwrap()
}

/// Copy `size` bytes from `src` to `dest` (both integer addresses).
pub fn copy_aggregate<'ctx>(
    cx: &'ctx Context,
    builder: &Builder<'ctx>,
    ptr_size: u64,
    size: u64,
    dest: IntValue<'ctx>,
    src: IntValue<'ctx>,
) {
    if size == 0 {
        return;
    }
    let d = int_to_ptr(cx, builder, dest);
    let s = int_to_ptr(cx, builder, src);
    let n = ptr_const(cx, ptr_size, size as i64);
    // Align 1: always valid; LLVM refines from the pointee where it can.
    builder.build_memcpy(d, 1, s, 1, n).unwrap();
}

/// Zero `size` bytes at `ptr` (an integer address).
pub fn zero_memory<'ctx>(
    cx: &'ctx Context,
    builder: &Builder<'ctx>,
    ptr_size: u64,
    ptr: IntValue<'ctx>,
    size: u64,
) {
    if size == 0 {
        return;
    }
    let d = int_to_ptr(cx, builder, ptr);
    let zero = cx.i8_type().const_zero();
    let n = ptr_const(cx, ptr_size, size as i64);
    builder.build_memset(d, 1, zero, n).unwrap();
}

/// Store a value (scalar or aggregate-by-address) to `dest` per its repr.
pub fn store_to_repr<'ctx>(
    cx: &'ctx Context,
    builder: &Builder<'ctx>,
    ptr_size: u64,
    repr: TypeRepr,
    dest: IntValue<'ctx>,
    value: BasicValueEnum<'ctx>,
) {
    match repr {
        TypeRepr::Scalar(_) => {
            let p = int_to_ptr(cx, builder, dest);
            builder.build_store(p, value).unwrap();
        },
        TypeRepr::Aggregate { size, .. } => {
            // `value` is the source address.
            copy_aggregate(cx, builder, ptr_size, size, dest, value.into_int_value());
        },
        TypeRepr::Zst => {},
    }
}

/// Load a value from `addr` per its repr. Aggregates return the address itself
/// (they are always carried by pointer); ZSTs return a placeholder `0`.
pub fn load_from_repr<'ctx>(
    cx: &'ctx Context,
    builder: &Builder<'ctx>,
    ptr_size: u64,
    repr: TypeRepr,
    addr: IntValue<'ctx>,
) -> BasicValueEnum<'ctx> {
    match repr {
        TypeRepr::Scalar(s) => {
            let p = int_to_ptr(cx, builder, addr);
            builder.build_load(s.llvm(cx), p, "ld").unwrap()
        },
        TypeRepr::Aggregate { .. } => addr.into(),
        TypeRepr::Zst => ptr_const(cx, ptr_size, 0).into(),
    }
}
