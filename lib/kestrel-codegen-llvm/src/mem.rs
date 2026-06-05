//! Memory helpers — addresses are real LLVM `ptr` values (typed-`ptr` model).
//! Loads/stores/memcpy/memset operate on `ptr` directly, and byte-offset address
//! math is `getelementptr` (`field_gep` = `inbounds` for compiler-generated
//! within-object offsets; `raw_gep` = plain for user pointer arithmetic that may
//! reach one-past-the-end). The only `int<->ptr` conversions are `int_to_ptr`/
//! `ptr_to_int`, used solely for `Op::PtrFromAddress`/`Op::PtrToAddress`.
//!
//! `copy_aggregate`/`zero_memory` use `llvm.memcpy`/`llvm.memset`;
//! `alloc_stack_slot` lives on `FuncCompiler` (it needs the entry block) — see
//! `func.rs`.

use inkwell::AddressSpace;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::types::IntType;
use inkwell::values::{BasicValueEnum, IntValue, PointerValue};

use crate::ty::TypeRepr;

/// The pointer-width integer type (`i64` on 64-bit). Used for sizes/lengths and
/// the integer side of the `PtrToAddress`/`PtrFromAddress` boundary.
pub fn ptr_int_type(cx: &Context, ptr_size: u64) -> IntType<'_> {
    if ptr_size == 8 {
        cx.i64_type()
    } else {
        cx.i32_type()
    }
}

/// A pointer-width *integer* constant — sizes, lengths, GEP byte offsets,
/// `SizeOf`/`AlignOf`. (Addresses are `ptr` now; see `null_ptr`.)
pub fn usize_const<'ctx>(cx: &'ctx Context, ptr_size: u64, value: i64) -> IntValue<'ctx> {
    ptr_int_type(cx, ptr_size).const_int(value as u64, false)
}

/// A null LLVM `ptr` constant — the ZST/Unit placeholder and `PtrNull`.
pub fn null_ptr(cx: &Context) -> PointerValue<'_> {
    cx.ptr_type(AddressSpace::default()).const_null()
}

/// `base + offset` bytes as a within-object `getelementptr inbounds i8`.
/// Identity at offset 0 (don't perturb provenance). For field/element/payload
/// offsets, which provably stay within the allocated object.
pub fn field_gep<'ctx>(
    cx: &'ctx Context,
    builder: &Builder<'ctx>,
    base: PointerValue<'ctx>,
    offset: u64,
) -> PointerValue<'ctx> {
    if offset == 0 {
        return base;
    }
    let idx = cx.i64_type().const_int(offset, false);
    unsafe {
        builder
            .build_in_bounds_gep(cx.i8_type(), base, &[idx], "fa")
            .unwrap()
    }
}

/// `base + idx` bytes as a plain `getelementptr i8` (NOT inbounds). For raw user
/// pointer arithmetic (`Op::PtrOffset`), which may legitimately produce a
/// one-past-the-end pointer; `inbounds` there would introduce UB.
pub fn raw_gep<'ctx>(
    cx: &'ctx Context,
    builder: &Builder<'ctx>,
    base: PointerValue<'ctx>,
    idx: IntValue<'ctx>,
) -> PointerValue<'ctx> {
    unsafe {
        builder
            .build_gep(cx.i8_type(), base, &[idx], "ptroff")
            .unwrap()
    }
}

/// Reinterpret an integer address as an LLVM `ptr` (only `Op::PtrFromAddress`).
pub fn int_to_ptr<'ctx>(
    cx: &'ctx Context,
    builder: &Builder<'ctx>,
    addr: IntValue<'ctx>,
) -> PointerValue<'ctx> {
    builder
        .build_int_to_ptr(addr, cx.ptr_type(AddressSpace::default()), "p")
        .unwrap()
}

/// Reinterpret an LLVM `ptr` as a pointer-width integer (only `Op::PtrToAddress`
/// and the `main` aggregate-return marshalling).
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

/// Copy `size` bytes from `src` to `dest` (both `ptr` addresses).
pub fn copy_aggregate<'ctx>(
    cx: &'ctx Context,
    builder: &Builder<'ctx>,
    ptr_size: u64,
    size: u64,
    dest: PointerValue<'ctx>,
    src: PointerValue<'ctx>,
) {
    if size == 0 {
        return;
    }
    let n = usize_const(cx, ptr_size, size as i64);
    // Align 1: always valid; LLVM refines from the pointee where it can.
    builder.build_memcpy(dest, 1, src, 1, n).unwrap();
}

/// Zero `size` bytes at `ptr` (a `ptr` address).
pub fn zero_memory<'ctx>(
    cx: &'ctx Context,
    builder: &Builder<'ctx>,
    ptr_size: u64,
    ptr: PointerValue<'ctx>,
    size: u64,
) {
    if size == 0 {
        return;
    }
    let zero = cx.i8_type().const_zero();
    let n = usize_const(cx, ptr_size, size as i64);
    builder.build_memset(ptr, 1, zero, n).unwrap();
}

/// Store a value (scalar or aggregate-by-address) to `dest` per its repr.
pub fn store_to_repr<'ctx>(
    cx: &'ctx Context,
    builder: &Builder<'ctx>,
    ptr_size: u64,
    repr: TypeRepr,
    dest: PointerValue<'ctx>,
    value: BasicValueEnum<'ctx>,
) {
    match repr {
        TypeRepr::Scalar(_) => {
            builder.build_store(dest, value).unwrap();
        },
        TypeRepr::Aggregate { size, .. } => {
            // `value` is the source address.
            copy_aggregate(
                cx,
                builder,
                ptr_size,
                size,
                dest,
                value.into_pointer_value(),
            );
        },
        TypeRepr::Zst => {},
    }
}

/// Load a value from `addr` per its repr. Aggregates return the address itself
/// (they are always carried by pointer); ZSTs return a null-`ptr` placeholder.
pub fn load_from_repr<'ctx>(
    cx: &'ctx Context,
    builder: &Builder<'ctx>,
    _ptr_size: u64,
    repr: TypeRepr,
    addr: PointerValue<'ctx>,
) -> BasicValueEnum<'ctx> {
    match repr {
        TypeRepr::Scalar(s) => builder.build_load(s.llvm(cx), addr, "ld").unwrap(),
        TypeRepr::Aggregate { .. } => addr.into(),
        TypeRepr::Zst => null_ptr(cx).into(),
    }
}
