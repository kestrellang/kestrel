//! Tests for lang.ptr[T] intrinsic pointer type
//!
//! These tests verify that the lang.ptr[T] type:
//! - Resolves correctly in various contexts (type aliases, fields, parameters, returns)
//! - Validates type argument count (exactly 1 required)
//! - Works with nested and complex types
//! - Works in generic contexts with type parameters
//! - Lowers correctly to MIR Pointer type

use kestrel_test_suite::mir::*;
use kestrel_test_suite::*;

mod basic_resolution {
    use super::*;

    #[test]
    fn lang_ptr_int_in_type_alias() {
        Test::new(
            r#"module Test
            type IntPtr = lang.ptr[lang.i64];
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("IntPtr").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn lang_ptr_string_in_type_alias() {
        Test::new(
            r#"module Test
            type StringPtr = lang.ptr[lang.str];
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("StringPtr").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn lang_ptr_bool_in_type_alias() {
        Test::new(
            r#"module Test
            type BoolPtr = lang.ptr[lang.i1];
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("BoolPtr").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn lang_ptr_as_struct_field() {
        Test::new(
            r#"module Test
            struct Wrapper {
                let ptr: lang.ptr[lang.i64]
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Wrapper")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(1)),
        );
    }

    #[test]
    fn lang_ptr_as_function_parameter() {
        Test::new(
            r#"module Test
            func usePtr(p: lang.ptr[lang.i64]) {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("usePtr")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    // Note: Tests for function return type require lang.ptr_null() or similar intrinsics
    // which are not yet implemented. The type resolution is tested via struct fields and type aliases.

    #[test]
    fn lang_ptr_multiple_fields() {
        Test::new(
            r#"module Test
            struct MultiPtr {
                let intPtr: lang.ptr[lang.i64]
                let strPtr: lang.ptr[lang.str]
                let boolPtr: lang.ptr[lang.i1]
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("MultiPtr")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(3)),
        );
    }
}

mod type_argument_validation {
    use super::*;

    #[test]
    fn lang_ptr_without_type_args_error() {
        Test::new(
            r#"module Test
            type Bad = lang.ptr;
        "#,
        )
        .expect(HasError("type argument"));
    }

    #[test]
    fn lang_ptr_too_many_type_args_error() {
        Test::new(
            r#"module Test
            type Bad = lang.ptr[lang.i64, lang.str];
        "#,
        )
        .expect(HasError("too many type arguments"));
    }

    #[test]
    fn lang_ptr_empty_brackets_error() {
        Test::new(
            r#"module Test
            type Bad = lang.ptr[];
        "#,
        )
        .expect(HasError("type argument"));
    }

    #[test]
    fn lang_ptr_three_type_args_error() {
        Test::new(
            r#"module Test
            type Bad = lang.ptr[lang.i64, lang.str, lang.i1];
        "#,
        )
        .expect(HasError("too many type arguments"));
    }
}

mod nested_and_complex_types {
    use super::*;

    #[test]
    fn nested_pointer() {
        Test::new(
            r#"module Test
            type PtrToPtr = lang.ptr[lang.ptr[lang.i64]];
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn triple_nested_pointer() {
        Test::new(
            r#"module Test
            type PtrPtrPtr = lang.ptr[lang.ptr[lang.ptr[lang.i64]]];
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn pointer_to_tuple() {
        Test::new(
            r#"module Test
            type TuplePtr = lang.ptr[(lang.i64, lang.str)];
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn pointer_to_array() {
        Test::new(
            r#"module Test
            type ArrayPtr = lang.ptr[[lang.i64]];
        "#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn pointer_to_user_struct() {
        Test::new(
            r#"module Test
            struct Point { let x: lang.i64; let y: lang.i64 }
            type PointPtr = lang.ptr[Point];
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn pointer_to_generic_struct() {
        Test::new(
            r#"module Test
            struct Box[T] { let value: T }
            type BoxPtr = lang.ptr[Box[lang.i64]];
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn pointer_in_tuple() {
        Test::new(
            r#"module Test
            type PtrPair = (lang.ptr[lang.i64], lang.ptr[lang.str]);
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn pointer_in_array() {
        Test::new(
            r#"module Test
            type PtrArray = [lang.ptr[lang.i64]];
        "#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn struct_containing_pointer_to_self() {
        // This should compile - the pointer provides indirection
        Test::new(
            r#"module Test
            struct Node {
                let value: lang.i64
                let next: lang.ptr[Node]
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod generic_context {
    use super::*;

    #[test]
    fn pointer_with_type_parameter() {
        Test::new(
            r#"module Test
            struct Wrapper[T] {
                let ptr: lang.ptr[T]
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Wrapper")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1)),
        );
    }

    // Note: function_returning_pointer_to_type_param requires lang.ptr_null() intrinsic

    #[test]
    fn function_taking_pointer_to_type_param() {
        Test::new(
            r#"module Test
            func usePtr[T](p: lang.ptr[T]) {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn generic_struct_with_multiple_pointer_fields() {
        Test::new(
            r#"module Test
            struct Pair[A, B] {
                let first: lang.ptr[A]
                let second: lang.ptr[B]
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Pair")
                .is(SymbolKind::Struct)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(2))
                .has(Behavior::FieldCount(2)),
        );
    }

    #[test]
    fn type_alias_with_pointer_to_type_param() {
        Test::new(
            r#"module Test
            type Ptr[T] = lang.ptr[T];
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Ptr")
                .is(SymbolKind::TypeAlias)
                .has(Behavior::IsGeneric(true))
                .has(Behavior::TypeParamCount(1)),
        );
    }
}

mod mir_lowering {
    use super::*;

    #[test]
    fn pointer_field_lowers_to_mir_pointer() {
        Test::new(
            r#"module Test
            struct Wrapper {
                let ptr: lang.ptr[lang.i64]
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_struct("Test.Wrapper").has_field("ptr", MirTy::ptr(MirTy::I64)));
    }

    #[test]
    fn pointer_to_bool_lowers_correctly() {
        Test::new(
            r#"module Test
            struct BoolWrapper {
                let ptr: lang.ptr[lang.i1]
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_struct("Test.BoolWrapper").has_field("ptr", MirTy::ptr(MirTy::Bool)));
    }

    #[test]
    fn pointer_to_string_lowers_correctly() {
        Test::new(
            r#"module Test
            struct StrWrapper {
                let ptr: lang.ptr[lang.str]
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_struct("Test.StrWrapper").has_field("ptr", MirTy::ptr(MirTy::Str)));
    }

    #[test]
    fn nested_pointer_lowers_correctly() {
        Test::new(
            r#"module Test
            struct DoublePtr {
                let ptr: lang.ptr[lang.ptr[lang.i64]]
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_struct("Test.DoublePtr").has_field("ptr", MirTy::ptr(MirTy::ptr(MirTy::I64))),
        );
    }

    #[test]
    fn pointer_to_struct_lowers_correctly() {
        Test::new(
            r#"module Test
            struct Point { let x: lang.i64; let y: lang.i64 }
            struct Wrapper {
                let ptr: lang.ptr[Point]
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_struct("Test.Wrapper")
                .has_field("ptr", MirTy::ptr(MirTy::named("Test.Point"))),
        );
    }

    #[test]
    fn pointer_to_tuple_lowers_correctly() {
        Test::new(
            r#"module Test
            struct TupleWrapper {
                let ptr: lang.ptr[(lang.i64, lang.i1)]
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_struct("Test.TupleWrapper").has_field(
            "ptr",
            MirTy::ptr(MirTy::tuple(vec![MirTy::I64, MirTy::Bool])),
        ));
    }

    #[test]
    fn generic_pointer_field_lowers_correctly() {
        Test::new(
            r#"module Test
            struct Wrapper[T] {
                let ptr: lang.ptr[T]
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_struct("Test.Wrapper")
                .has_type_params(1)
                .has_field("ptr", MirTy::ptr(MirTy::type_param("T"))),
        );
    }

    #[test]
    fn function_with_pointer_param() {
        Test::new(
            r#"module Test
            func usePtr(p: lang.ptr[lang.i64]) {}
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        // Pointers are passed by reference like other types, so the parameter is &ptr[I64]
        .expect(
            Mir::mir_function("Test.usePtr$p").has_param("p", MirTy::ref_(MirTy::ptr(MirTy::I64))),
        );
    }

    // Note: function_returning_pointer and function_with_generic_pointer tests
    // require lang.ptr_null() or ability to construct pointer values, which are not yet implemented.
    // MIR lowering is tested via struct fields.

    #[test]
    fn multiple_pointer_fields_lower_correctly() {
        Test::new(
            r#"module Test
            struct MultiPtr {
                let a: lang.ptr[lang.i64]
                let b: lang.ptr[lang.i1]
                let c: lang.ptr[lang.str]
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_struct("Test.MultiPtr")
                .has_field("a", MirTy::ptr(MirTy::I64))
                .has_field("b", MirTy::ptr(MirTy::Bool))
                .has_field("c", MirTy::ptr(MirTy::Str)),
        );
    }
}

// Note: Type checking tests with actual pointer values will require
// lang.ptr_null(), lang.ptr_to(), etc. intrinsic functions to be implemented.
// For now, we test type resolution and MIR lowering only.

mod string_intrinsics {
    use super::*;

    #[test]
    fn string_unsafe_ptr_compiles() {
        Test::new(
            r#"module Test
            func getPtr(s: lang.str) -> lang.ptr[lang.i8] {
                s.unsafePtr()
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    #[test]
    fn string_unsafe_ptr_return_type() {
        Test::new(
            r#"module Test
            struct Holder {
                let ptr: lang.ptr[lang.i8]
            }
            func wrap(s: lang.str) -> Holder {
                Holder(ptr: s.unsafePtr())
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    #[test]
    fn string_length_still_works() {
        // Ensure we didn't break the existing length() method
        Test::new(
            r#"module Test
            func len(s: lang.str) -> lang.i64 {
                s.length()
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    #[test]
    fn string_unsafe_ptr_in_struct_field() {
        Test::new(
            r#"module Test
            struct StringView {
                let ptr: lang.ptr[lang.i8]
                let len: lang.i64
            }
            func makeView(s: lang.str) -> StringView {
                StringView(ptr: s.unsafePtr(), len: s.length())
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }
}

/// Regression tests for lang intrinsic type inference issues
mod regression {
    use super::*;

    /// Regression test for: Type inference fails with untyped lang intrinsics
    ///
    /// Before the fix, `lang.cast_ptr[T](lang.ptr_null())` would fail with
    /// "could not infer type for 1 placeholder(s)" because `ptr_null()` returned
    /// `Pointer[Ty::infer()]` and this wasn't constrained by `cast_ptr`.
    ///
    /// The fix adds a constraint in the type inference constraint generator:
    /// when `cast_ptr[T]` has a concrete target type T, the argument's pointee
    /// type is equated with T, allowing `ptr_null()` to infer its type.
    ///
    /// Issue documented in: docs/contributing/compiler-issues.md
    /// Fix in: lib/kestrel-semantic-type-inference/src/constraint_generator.rs
    #[test]
    fn cast_ptr_with_untyped_ptr_null() {
        Test::new(
            r#"module Test
            func test() -> lang.ptr[lang.i64] {
                lang.cast_ptr[lang.i64](lang.ptr_null())
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    /// Test that the workaround (explicit type argument) still works
    #[test]
    fn ptr_null_with_explicit_type_argument() {
        Test::new(
            r#"module Test
            func test() -> lang.ptr[lang.i64] {
                lang.ptr_null[lang.i64]()
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    /// Test with user-defined wrapper struct
    #[test]
    fn cast_ptr_in_generic_context() {
        Test::new(
            r#"module Test
            public struct Pointer[T] {
                var raw: lang.ptr[T]

                public init(raw: lang.ptr[T]) {
                    self.raw = raw;
                }
            }

            public func testCastPtr[T]() -> Pointer[T] {
                Pointer(lang.cast_ptr[T](lang.ptr_null()))
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    /// Test that cast_ptr works with different primitive types
    #[test]
    fn cast_ptr_with_various_primitives() {
        Test::new(
            r#"module Test
            func testI8() -> lang.ptr[lang.i8] {
                lang.cast_ptr[lang.i8](lang.ptr_null())
            }

            func testI16() -> lang.ptr[lang.i16] {
                lang.cast_ptr[lang.i16](lang.ptr_null())
            }

            func testI32() -> lang.ptr[lang.i32] {
                lang.cast_ptr[lang.i32](lang.ptr_null())
            }

            func testF32() -> lang.ptr[lang.f32] {
                lang.cast_ptr[lang.f32](lang.ptr_null())
            }

            func testF64() -> lang.ptr[lang.f64] {
                lang.cast_ptr[lang.f64](lang.ptr_null())
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    /// Regression test for: Pointer type casting between different concrete types fails
    ///
    /// Before the fix, `lang.cast_ptr[T](ptr)` where `ptr: lang.ptr[U]` and `T != U`
    /// would fail with a type error because the constraint generator incorrectly
    /// equated the source pointee type U with the target type T (creating `U == T`).
    ///
    /// This happened because the condition checked if target_ty was NOT infer:
    /// ```rust
    /// if !matches!(target_ty.kind(), TyKind::Infer) {
    ///     ctx.equate(arg_pointee.id(), target_ty.id(), ...);
    /// }
    /// ```
    ///
    /// The fix changes it to only equate when the SOURCE is infer (for ptr_null):
    /// ```rust
    /// if matches!(arg_pointee.kind(), TyKind::Infer) {
    ///     ctx.equate(arg_pointee.id(), target_ty.id(), ...);
    /// }
    /// ```
    ///
    /// This allows casting between different pointer types while still supporting
    /// type inference for `lang.cast_ptr[T](lang.ptr_null())`.
    #[test]
    fn cast_ptr_between_different_concrete_types() {
        Test::new(
            r#"module Test
            func castI32ToI8(p: lang.ptr[lang.i32]) -> lang.ptr[lang.i8] {
                lang.cast_ptr[lang.i8](p)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    /// Test casting from i8 pointer to other types (common for byte buffers)
    #[test]
    fn cast_ptr_from_i8_to_other_types() {
        Test::new(
            r#"module Test
            func castI8ToI32(p: lang.ptr[lang.i8]) -> lang.ptr[lang.i32] {
                lang.cast_ptr[lang.i32](p)
            }

            func castI8ToI64(p: lang.ptr[lang.i8]) -> lang.ptr[lang.i64] {
                lang.cast_ptr[lang.i64](p)
            }

            func castI8ToF32(p: lang.ptr[lang.i8]) -> lang.ptr[lang.f32] {
                lang.cast_ptr[lang.f32](p)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    /// Test casting to generic pointer type parameter
    #[test]
    fn cast_ptr_to_generic_type() {
        Test::new(
            r#"module Test
            func castToGeneric[T](p: lang.ptr[lang.i8]) -> lang.ptr[T] {
                lang.cast_ptr[T](p)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    /// Test casting from generic pointer type parameter
    #[test]
    fn cast_ptr_from_generic_type() {
        Test::new(
            r#"module Test
            func castFromGeneric[T](p: lang.ptr[T]) -> lang.ptr[lang.i8] {
                lang.cast_ptr[lang.i8](p)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    /// Test round-trip casting
    #[test]
    fn cast_ptr_round_trip() {
        Test::new(
            r#"module Test
            func roundTrip(p: lang.ptr[lang.i32]) -> lang.ptr[lang.i32] {
                var bytes = lang.cast_ptr[lang.i8](p);
                lang.cast_ptr[lang.i32](bytes)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }
}
