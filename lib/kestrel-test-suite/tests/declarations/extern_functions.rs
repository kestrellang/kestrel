use kestrel_test_suite::*;

mod positive {
    use super::*;

    #[test]
    fn extern_basic_c_convention() {
        Test::new(
            r#"
            module Test
            import Prelude

            // Empty structs are trivially FFISafe (no fields to check)
            struct MyInt: FFISafe {}
            struct Ptr: FFISafe {}

            @extern(.C)
            func malloc(size: MyInt) -> Ptr
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn extern_with_mangle_name() {
        Test::new(
            r#"
            module Test
            import Prelude

            struct MyInt: FFISafe {}
            struct Ptr: FFISafe {}

            @extern(.C, mangleName: "read")
            func readSocket(fd: MyInt, buf: Ptr, count: MyInt) -> MyInt
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn extern_void_return() {
        Test::new(
            r#"
            module Test
            import Prelude

            struct Ptr: FFISafe {}

            @extern(.C)
            func free(ptr: Ptr)
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn extern_multiple_params() {
        Test::new(
            r#"
            module Test
            import Prelude

            struct IntA: FFISafe {}
            struct IntB: FFISafe {}
            struct FloatA: FFISafe {}
            struct FloatB: FFISafe {}

            @extern(.C)
            func doStuff(a: IntA, b: IntB, c: FloatA, d: FloatB) -> IntA
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn extern_tuple_of_ffi_safe_types() {
        Test::new(
            r#"
            module Test
            import Prelude

            struct MyInt: FFISafe {}

            @extern(.C)
            func getPoint(coords: (MyInt, MyInt)) -> MyInt
        "#,
        )
        .expect(Compiles);
    }
}

mod negative {
    use super::*;

    #[test]
    fn extern_cannot_be_generic() {
        Test::new(
            r#"
            module Test
            import Prelude

            @extern(.C)
            func genericExtern[T](x: T) -> T
        "#,
        )
        .expect(HasError("cannot be generic"));
    }

    #[test]
    fn extern_cannot_have_body() {
        Test::new(
            r#"
            module Test
            import Prelude

            struct MyInt: FFISafe {}

            @extern(.C)
            func hasBody(x: MyInt) -> MyInt { x }
        "#,
        )
        .expect(HasError("cannot have a body"));
    }

    #[test]
    fn extern_param_cannot_be_mutating() {
        Test::new(
            r#"
            module Test
            import Prelude

            struct MyInt: FFISafe {}

            @extern(.C)
            func mutatingParam(mutating x: MyInt) -> MyInt
        "#,
        )
        .expect(HasError("consuming"));
    }

    #[test]
    fn extern_param_type_must_be_ffi_safe() {
        Test::new(
            r#"
            module Test
            import Prelude

            struct MyInt: FFISafe {}

            struct NotFFISafe {
                let value: Int
            }

            @extern(.C)
            func badParam(s: NotFFISafe) -> MyInt
        "#,
        )
        .expect(HasError("FFISafe"));
    }

    #[test]
    fn extern_return_type_must_be_ffi_safe() {
        Test::new(
            r#"
            module Test
            import Prelude

            struct MyInt: FFISafe {}

            struct NotFFISafe {
                let value: Int
            }

            @extern(.C)
            func badReturn(x: MyInt) -> NotFFISafe
        "#,
        )
        .expect(HasError("FFISafe"));
    }

    #[test]
    fn extern_requires_calling_convention() {
        Test::new(
            r#"
            module Test
            import Prelude

            struct MyInt: FFISafe {}

            @extern
            func noConvention(x: MyInt) -> MyInt
        "#,
        )
        .expect(HasError("calling convention"));
    }

    #[test]
    fn enum_cannot_conform_to_ffi_safe() {
        Test::new(
            r#"
            module Test
            import Prelude

            enum MyEnum: FFISafe {
                case A
                case B
            }
        "#,
        )
        .expect(HasError("cannot conform"));
    }

    #[test]
    fn struct_fields_must_be_ffi_safe() {
        Test::new(
            r#"
            module Test
            import Prelude

            struct NotFFISafe {
                let value: Int
            }

            struct BadStruct: FFISafe {
                let name: NotFFISafe
            }
        "#,
        )
        .expect(HasError("do not"));
    }
}
