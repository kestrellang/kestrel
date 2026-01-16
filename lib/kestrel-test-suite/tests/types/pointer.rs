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
            type IntPtr = lang.ptr[Int];
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("IntPtr").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn lang_ptr_string_in_type_alias() {
        Test::new(
            r#"module Test
            type StringPtr = lang.ptr[String];
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("StringPtr").is(SymbolKind::TypeAlias));
    }

    #[test]
    fn lang_ptr_bool_in_type_alias() {
        Test::new(
            r#"module Test
            type BoolPtr = lang.ptr[Bool];
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
                let ptr: lang.ptr[Int]
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
            func usePtr(p: lang.ptr[Int]) {}
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
                let intPtr: lang.ptr[Int]
                let strPtr: lang.ptr[String]
                let boolPtr: lang.ptr[Bool]
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
            type Bad = lang.ptr[Int, String];
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
            type Bad = lang.ptr[Int, String, Bool];
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
            type PtrToPtr = lang.ptr[lang.ptr[Int]];
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn triple_nested_pointer() {
        Test::new(
            r#"module Test
            type PtrPtrPtr = lang.ptr[lang.ptr[lang.ptr[Int]]];
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn pointer_to_tuple() {
        Test::new(
            r#"module Test
            type TuplePtr = lang.ptr[(Int, String)];
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn pointer_to_array() {
        Test::new(
            r#"module Test
            type ArrayPtr = lang.ptr[[Int]];
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn pointer_to_user_struct() {
        Test::new(
            r#"module Test
            struct Point { let x: Int; let y: Int }
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
            type BoxPtr = lang.ptr[Box[Int]];
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn pointer_in_tuple() {
        Test::new(
            r#"module Test
            type PtrPair = (lang.ptr[Int], lang.ptr[String]);
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn pointer_in_array() {
        Test::new(
            r#"module Test
            type PtrArray = [lang.ptr[Int]];
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn struct_containing_pointer_to_self() {
        // This should compile - the pointer provides indirection
        Test::new(
            r#"module Test
            struct Node {
                let value: Int
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
                let ptr: lang.ptr[Int]
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
                let ptr: lang.ptr[Bool]
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
                let ptr: lang.ptr[String]
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
                let ptr: lang.ptr[lang.ptr[Int]]
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
            struct Point { let x: Int; let y: Int }
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
                let ptr: lang.ptr[(Int, Bool)]
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
    fn pointer_to_array_type_requires_resolution() {
        // NOTE: [Int] as a type annotation is not yet resolved to a concrete
        // struct type like Array[Int, GlobalAllocator]. This test verifies
        // that MIR lowering correctly fails for unresolved array types.
        // Once array type resolution is implemented, this test should be
        // updated to verify the correct MIR structure.
        Test::new(
            r#"module Test
            struct ArrayWrapper {
                let ptr: lang.ptr[[Int]]
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::fails());
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
            func usePtr(p: lang.ptr[Int]) {}
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        // Pointers are passed by reference like other types, so the parameter is &ptr[I64]
        .expect(
            Mir::mir_function("Test.usePtr").has_param("p", MirTy::ref_(MirTy::ptr(MirTy::I64))),
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
                let a: lang.ptr[Int]
                let b: lang.ptr[Bool]
                let c: lang.ptr[String]
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
            func getPtr(s: String) -> lang.ptr[I8] {
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
                let ptr: lang.ptr[I8]
            }
            func wrap(s: String) -> Holder {
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
            func len(s: String) -> Int {
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
                let ptr: lang.ptr[I8]
                let len: Int
            }
            func makeView(s: String) -> StringView {
                StringView(ptr: s.unsafePtr(), len: s.length())
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }
}
