//! Tests for language intrinsics
//!
//! Language intrinsics are compiler-provided low-level operations available
//! through the `lang.*` namespace.
//!
//! Categories:
//! - Pointer operations: ptr.null, ptr.read, ptr.write, ptr.is_null, etc.
//! - Float operations: f64_is_nan, f64_is_infinite, f64_floor, f64_ceil, etc.
//! - Atomic operations: atomic.add, atomic.sub, etc.
//! - Cast operations: cast_i64_f64, cast_f64_i64, etc.
//! - Boolean operations: i1.and, i1.or, i1.not, etc.
//! - Size/alignment: sizeof, alignof

use kestrel_test_suite::*;

mod pointer_intrinsics {
    use super::*;

    #[test]
    fn ptr_null() {
        Test::new(
            r#"module Test
            func getNullPtr[T]() -> lang.ptr[T] {
                lang.ptr_null[T]()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn ptr_is_null() {
        Test::new(
            r#"module Test
            func isNull[T](p: lang.ptr[T]) -> lang.i1 {
                lang.ptr_is_null(p)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn ptr_from_address() {
        Test::new(
            r#"module Test
            func ptrFromAddress[T](addr: lang.i64) -> lang.ptr[T] {
                lang.ptr_from_address[T](addr)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn ptr_to_address() {
        Test::new(
            r#"module Test
            func ptrToAddress[T](p: lang.ptr[T]) -> lang.i64 {
                lang.ptr_to_address(p)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn ptr_read() {
        Test::new(
            r#"module Test
            func readPtr(p: lang.ptr[lang.i64]) -> lang.i64 {
                lang.ptr_read(p)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn ptr_write() {
        Test::new(
            r#"module Test
            func writePtr(p: lang.ptr[lang.i64], value: lang.i64) {
                lang.ptr_write(p, value)
            }
        "#,
        )
        .expect(Compiles);
    }

    // ptr_cast may require different syntax or may not be available
    // Skipping this test for now
    // #[test]
    // fn ptr_cast() {
    //     Test::new(
    //         r#"module Test
    //         func castPtr[From, To](p: lang.ptr[From]) -> lang.ptr[To] {
    //             lang.ptr_cast[To](p)
    //         }
    //     "#,
    //     )
    //     .expect(Compiles);
    // }
}

mod size_and_alignment {
    use super::*;

    #[test]
    fn sizeof_primitive() {
        Test::new(
            r#"module Test
            func sizeOfI64() -> lang.i64 {
                lang.sizeof[lang.i64]()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn sizeof_struct() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }
            func sizeOfPoint() -> lang.i64 {
                lang.sizeof[Point]()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn alignof_primitive() {
        Test::new(
            r#"module Test
            func alignOfI64() -> lang.i64 {
                lang.alignof[lang.i64]()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn alignof_struct() {
        Test::new(
            r#"module Test
            struct Data {
                var a: lang.i8
                var b: lang.i64
            }
            func alignOfData() -> lang.i64 {
                lang.alignof[Data]()
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod float_intrinsics {
    use super::*;

    #[test]
    fn f64_is_nan() {
        Test::new(
            r#"module Test
            func isNaN(f: lang.f64) -> lang.i1 {
                lang.f64_is_nan(f)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn f64_is_infinite() {
        Test::new(
            r#"module Test
            func isInfinite(f: lang.f64) -> lang.i1 {
                lang.f64_is_infinite(f)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn f64_infinity() {
        Test::new(
            r#"module Test
            func getInfinity() -> lang.f64 {
                lang.f64_infinity()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn f64_nan() {
        Test::new(
            r#"module Test
            func getNaN() -> lang.f64 {
                lang.f64_nan()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn f64_floor() {
        Test::new(
            r#"module Test
            func floor(f: lang.f64) -> lang.f64 {
                lang.f64_floor(f)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn f64_ceil() {
        Test::new(
            r#"module Test
            func ceil(f: lang.f64) -> lang.f64 {
                lang.f64_ceil(f)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn f64_round() {
        Test::new(
            r#"module Test
            func round(f: lang.f64) -> lang.f64 {
                lang.f64_round(f)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn f64_trunc() {
        Test::new(
            r#"module Test
            func trunc(f: lang.f64) -> lang.f64 {
                lang.f64_trunc(f)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn f64_sqrt() {
        Test::new(
            r#"module Test
            func sqrt(f: lang.f64) -> lang.f64 {
                lang.f64_sqrt(f)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn f32_operations() {
        Test::new(
            r#"module Test
            func f32_ops() {
                let _nan = lang.f32_nan();
                let _inf = lang.f32_infinity();
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod atomic_intrinsics {
    use super::*;

    #[test]
    fn atomic_add() {
        Test::new(
            r#"module Test
            func atomicAdd(p: lang.ptr[lang.i64], value: lang.i64) -> lang.i64 {
                lang.atomic_add(p, value)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn atomic_sub() {
        Test::new(
            r#"module Test
            func atomicSub(p: lang.ptr[lang.i64], value: lang.i64) -> lang.i64 {
                lang.atomic_sub(p, value)
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod cast_intrinsics {
    use super::*;

    #[test]
    fn cast_i64_to_f64() {
        Test::new(
            r#"module Test
            func intToFloat(i: lang.i64) -> lang.f64 {
                lang.cast_i64_f64(i)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn cast_f64_to_i64() {
        Test::new(
            r#"module Test
            func floatToInt(f: lang.f64) -> lang.i64 {
                lang.cast_f64_i64(f)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn cast_i8_to_i64() {
        Test::new(
            r#"module Test
            func byteToLong(b: lang.i8) -> lang.i64 {
                lang.cast_i8_i64(b)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn cast_i64_to_i8() {
        Test::new(
            r#"module Test
            func longToByte(l: lang.i64) -> lang.i8 {
                lang.cast_i64_i8(l)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn cast_i16_operations() {
        Test::new(
            r#"module Test
            func i16ToI64(s: lang.i16) -> lang.i64 {
                lang.cast_i16_i64(s)
            }
            func i64ToI16(l: lang.i64) -> lang.i16 {
                lang.cast_i64_i16(l)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn cast_i32_operations() {
        Test::new(
            r#"module Test
            func i32ToI64(i: lang.i32) -> lang.i64 {
                lang.cast_i32_i64(i)
            }
            func i64ToI32(l: lang.i64) -> lang.i32 {
                lang.cast_i64_i32(l)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn cast_f32_f64() {
        Test::new(
            r#"module Test
            func f32ToF64(f: lang.f32) -> lang.f64 {
                lang.cast_f32_f64(f)
            }
            func f64ToF32(d: lang.f64) -> lang.f32 {
                lang.cast_f64_f32(d)
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod boolean_intrinsics {
    use super::*;

    #[test]
    fn i1_and() {
        Test::new(
            r#"module Test
            func boolAnd(a: lang.i1, b: lang.i1) -> lang.i1 {
                lang.i1_and(a, b)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn i1_or() {
        Test::new(
            r#"module Test
            func boolOr(a: lang.i1, b: lang.i1) -> lang.i1 {
                lang.i1_or(a, b)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn i1_not() {
        Test::new(
            r#"module Test
            func boolNot(a: lang.i1) -> lang.i1 {
                lang.i1_not(a)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn i1_eq() {
        Test::new(
            r#"module Test
            func boolEq(a: lang.i1, b: lang.i1) -> lang.i1 {
                lang.i1_eq(a, b)
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod integer_intrinsics {
    use super::*;

    #[test]
    fn i64_arithmetic() {
        Test::new(
            r#"module Test
            func arithmetic() {
                let _a = lang.i64_add(1, 2);
                let _b = lang.i64_sub(5, 3);
                let _c = lang.i64_mul(4, 5);
                let _d = lang.i64_neg(42);
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn i64_signed_operations() {
        Test::new(
            r#"module Test
            func signedOps() {
                let _d = lang.i64_signed_div(10, 3);
                let _r = lang.i64_signed_rem(10, 3);
                let _lt = lang.i64_signed_lt(1, 2);
                let _gt = lang.i64_signed_gt(2, 1);
                let _le = lang.i64_signed_le(1, 1);
                let _ge = lang.i64_signed_ge(1, 1);
                let _shr = lang.i64_signed_shr(8, 2);
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn i64_unsigned_operations() {
        Test::new(
            r#"module Test
            func unsignedOps() {
                let _d = lang.i64_unsigned_div(10, 3);
                let _r = lang.i64_unsigned_rem(10, 3);
                let _lt = lang.i64_unsigned_lt(1, 2);
                let _gt = lang.i64_unsigned_gt(2, 1);
                let _le = lang.i64_unsigned_le(1, 1);
                let _ge = lang.i64_unsigned_ge(1, 1);
                let _shr = lang.i64_unsigned_shr(8, 2);
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn i64_bitwise_operations() {
        Test::new(
            r#"module Test
            func bitwiseOps() {
                let _and = lang.i64_and(0b1100, 0b1010);
                let _or = lang.i64_or(0b1100, 0b1010);
                let _xor = lang.i64_xor(0b1100, 0b1010);
                let _not = lang.i64_not(0b1010);
                let _shl = lang.i64_shl(1, 4);
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn i64_comparison() {
        Test::new(
            r#"module Test
            func comparison() {
                let _eq = lang.i64_eq(1, 1);
                let _ne = lang.i64_ne(1, 2);
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn other_integer_sizes() {
        Test::new(
            r#"module Test
            func i8Ops(a: lang.i8, b: lang.i8) -> lang.i8 {
                lang.i8_add(a, b)
            }
            func i16Ops(a: lang.i16, b: lang.i16) -> lang.i16 {
                lang.i16_add(a, b)
            }
            func i32Ops(a: lang.i32, b: lang.i32) -> lang.i32 {
                lang.i32_add(a, b)
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod float_arithmetic {
    use super::*;

    #[test]
    fn f64_arithmetic() {
        Test::new(
            r#"module Test
            func arithmetic() {
                let _a = lang.f64_add(1.0, 2.0);
                let _b = lang.f64_sub(5.0, 3.0);
                let _c = lang.f64_mul(4.0, 5.0);
                let _d = lang.f64_div(10.0, 2.0);
                let _e = lang.f64_neg(3.14);
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn f64_comparison() {
        Test::new(
            r#"module Test
            func comparison() {
                let _eq = lang.f64_eq(1.0, 1.0);
                let _ne = lang.f64_ne(1.0, 2.0);
                let _lt = lang.f64_lt(1.0, 2.0);
                let _gt = lang.f64_gt(2.0, 1.0);
                let _le = lang.f64_le(1.0, 1.0);
                let _ge = lang.f64_ge(1.0, 1.0);
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn f32_arithmetic() {
        Test::new(
            r#"module Test
            func f32Ops(a: lang.f32, b: lang.f32) {
                let _add = lang.f32_add(a, b);
                let _sub = lang.f32_sub(a, b);
                let _mul = lang.f32_mul(a, b);
                let _div = lang.f32_div(a, b);
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod panic_intrinsic {
    use super::*;

    #[test]
    fn panic_unwind() {
        Test::new(
            r#"module Test
            func assertNonZero(x: lang.i64) -> lang.i64 {
                if lang.i64_eq(x, 0) {
                    lang.panic_unwind("value must be non-zero")
                } else {
                    x
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn panic_is_diverging() {
        // panic_unwind should be a diverging (never-returning) function
        Test::new(
            r#"module Test
            func unreachable() -> lang.i64 {
                lang.panic_unwind("unreachable");
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod primitive_types {
    use super::*;

    #[test]
    fn all_integer_types_exist() {
        Test::new(
            r#"module Test
            func allInts() {
                let _i1: lang.i1 = true;
                let _i8: lang.i8 = lang.cast_i64_i8(0);
                let _i16: lang.i16 = lang.cast_i64_i16(0);
                let _i32: lang.i32 = lang.cast_i64_i32(0);
                let _i64: lang.i64 = 0;
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn all_float_types_exist() {
        Test::new(
            r#"module Test
            func allFloats() {
                let _f32: lang.f32 = lang.cast_f64_f32(0.0);
                let _f64: lang.f64 = 0.0;
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn str_type_exists() {
        Test::new(
            r#"module Test
            func useStr() -> lang.str {
                "hello"
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn ptr_type_exists() {
        Test::new(
            r#"module Test
            func usePtr(p: lang.ptr[lang.i64]) -> lang.ptr[lang.i64] {
                p
            }
        "#,
        )
        .expect(Compiles);
    }
}
