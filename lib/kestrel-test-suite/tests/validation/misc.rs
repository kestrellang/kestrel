use kestrel_test_suite::*;

mod function_body {
    use super::*;

    #[test]
    fn function_with_body_compiles_and_has_correct_properties() {
        Test::new("module Test\nfunc hasBody() { }")
            .expect(Compiles)
            .expect(Symbol::new("hasBody").is(SymbolKind::Function))
            .expect(Symbol::new("hasBody").has(Behavior::HasBody(true)))
            .expect(Symbol::new("hasBody").has(Behavior::ParameterCount(0)));
    }

    #[test]
    fn function_without_body_errors() {
        Test::new("module Test\nfunc missingBody() -> Int").expect(HasError("requires a body"));
    }

    #[test]
    fn function_with_return_type_requires_body() {
        // Verify that any function requiring a return type must have a body
        Test::new(
            r#"module Test
            func valid() { }
            func invalid() -> Int
        "#,
        )
        .expect(HasError("'invalid' requires a body"))
        .expect(Symbol::new("valid").is(SymbolKind::Function));
    }
}

mod protocol_methods {
    use super::*;

    #[test]
    fn protocol_method_without_body_compiles_and_verified() {
        Test::new(
            r#"module Test
            protocol Printable {
                func print() -> ()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Printable").is(SymbolKind::Protocol))
        .expect(Symbol::new("Printable.print").is(SymbolKind::Function))
        .expect(Symbol::new("Printable.print").has(Behavior::HasBody(false)));
    }

    #[test]
    fn protocol_method_with_body_errors() {
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw() -> () { }
            }
        "#,
        )
        .expect(HasError("cannot have a body"));
    }

    #[test]
    fn protocol_with_multiple_methods_verified() {
        Test::new(
            r#"module Test
            protocol Hashable {
                func hash() -> Int
                func equals(other to: Int) -> Bool
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Hashable").is(SymbolKind::Protocol))
        .expect(Symbol::new("Hashable.hash").is(SymbolKind::Function))
        .expect(Symbol::new("Hashable.hash").has(Behavior::ParameterCount(0)))
        .expect(Symbol::new("Hashable.equals").is(SymbolKind::Function))
        .expect(Symbol::new("Hashable.equals").has(Behavior::ParameterCount(1)));
    }
}

mod mixed {
    use super::*;

    #[test]
    fn protocol_and_regular_function_coexist() {
        Test::new(
            r#"module Test
            protocol Runnable {
                func run() -> ()
            }

            func execute() { }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Runnable").is(SymbolKind::Protocol))
        .expect(Symbol::new("Runnable.run").is(SymbolKind::Function))
        .expect(Symbol::new("Runnable.run").has(Behavior::HasBody(false)))
        .expect(Symbol::new("execute").is(SymbolKind::Function))
        .expect(Symbol::new("execute").has(Behavior::HasBody(true)));
    }
}

mod static_context {
    use super::*;

    #[test]
    fn static_function_in_struct_is_static_and_has_body() {
        Test::new(
            r#"module Test
            struct Counter {
                static func create() { }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Counter.create").is(SymbolKind::Function))
        .expect(Symbol::new("Counter.create").has(Behavior::IsStatic(true)))
        .expect(Symbol::new("Counter.create").has(Behavior::HasBody(true)));
    }

    #[test]
    fn static_function_in_protocol_is_static() {
        Test::new(
            r#"module Test
            protocol Factory {
                static func create() -> ()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Factory.create").is(SymbolKind::Function))
        .expect(Symbol::new("Factory.create").has(Behavior::IsStatic(true)))
        .expect(Symbol::new("Factory.create").has(Behavior::HasBody(false)));
    }

    #[test]
    fn static_function_at_module_level_errors() {
        Test::new("module Test\nstatic func topLevel() { }")
            .expect(HasError("cannot be static in this context"));
    }
}

mod duplicate_symbol {
    use super::*;

    #[test]
    fn duplicate_struct_errors() {
        Test::new(
            r#"module Test
            struct Foo { }
            struct Foo { }
        "#,
        )
        .expect(HasError("duplicate definition of struct 'Foo'"));
    }

    #[test]
    fn duplicate_protocol_errors() {
        Test::new(
            r#"module Test
            protocol Bar { }
            protocol Bar { }
        "#,
        )
        .expect(HasError("duplicate definition of protocol 'Bar'"));
    }

    #[test]
    fn struct_and_protocol_same_name_errors() {
        Test::new(
            r#"module Test
            struct Thing { }
            protocol Thing { }
        "#,
        )
        .expect(HasError("'Thing' is already defined as a struct"));
    }

    #[test]
    fn different_types_different_names_compiles_and_verified() {
        Test::new(
            r#"module Test
            struct Foo { }
            protocol Bar { }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Foo").is(SymbolKind::Struct))
        .expect(Symbol::new("Bar").is(SymbolKind::Protocol));
    }

    #[test]
    fn function_overloading_allowed_with_different_signatures() {
        Test::new(
            r#"module Test
            func process() { }
            func process(x: Int) { }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("process").is(SymbolKind::Function));
    }

    #[test]
    fn duplicate_type_alias_errors() {
        Test::new(
            r#"module Test
            type Alias = Int;
            type Alias = String;
        "#,
        )
        .expect(HasError("duplicate definition of type alias 'Alias'"));
    }

    #[test]
    fn duplicate_field_same_struct_errors() {
        Test::new(
            r#"module Test
            struct Record {
                let name: String
                let name: Int
            }
        "#,
        )
        .expect(HasError("duplicate definition of field 'name'"));
    }

    #[test]
    fn same_field_different_structs_compiles_and_verified() {
        Test::new(
            r#"module Test
            struct First {
                let value: Int
            }
            struct Second {
                let value: String
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("First").is(SymbolKind::Struct))
        .expect(Symbol::new("First").has(Behavior::FieldCount(1)))
        .expect(Symbol::new("Second").is(SymbolKind::Struct))
        .expect(Symbol::new("Second").has(Behavior::FieldCount(1)));
    }
}

mod visibility_consistency {
    use super::*;

    // These tests document expected behavior for visibility consistency validation.
    // Some are ignored because the validation is not yet fully implemented.

    #[test]
    fn public_field_with_private_type_errors() {
        Test::new(
            r#"module Test
            private struct PrivateType { }
            public struct Container {
                public let value: PrivateType
            }
        "#,
        )
        .expect(HasError("has type less visible than the field"));
    }

    #[test]
    fn public_function_with_private_return_type_errors() {
        Test::new(
            r#"module Test
            private struct Secret { }
            public func getSecret() -> Secret { }
        "#,
        )
        .expect(HasError("return type of 'getSecret' is less visible"));
    }

    #[test]
    fn public_function_with_private_parameter_type_errors() {
        Test::new(
            r#"module Test
            private struct Secret { }
            public func process(s: Secret) { }
        "#,
        )
        .expect(HasError("parameter type in 'process' is less visible"));
    }

    #[test]
    fn internal_function_with_private_return_type_compiles_and_verified() {
        // Internal function can use private types within same scope
        Test::new(
            r#"module Test
            private struct Internal { }
            func helper() -> Internal { Internal() }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("helper").is(SymbolKind::Function))
        .expect(Symbol::new("helper").has(Behavior::Visibility(Visibility::Internal)));
    }

    #[test]
    fn public_type_alias_with_private_underlying_errors() {
        Test::new(
            r#"module Test
            private struct Hidden { }
            public type Exposed = Hidden;
        "#,
        )
        .expect(HasError("aliased type in 'Exposed' is less visible"));
    }

    #[test]
    fn protocol_method_with_private_param_in_public_protocol_errors() {
        Test::new(
            r#"module Test
            private struct Secret { }
            public protocol Handler {
                func handle(s: Secret) -> ()
            }
        "#,
        )
        .expect(HasError("parameter type in 'handle' is less visible"));
    }
}

mod module_resolution {
    use super::*;

    #[test]
    fn nested_module_struct_compiles_and_verified() {
        Test::new(
            r#"module Outer.Inner
            struct NestedType { }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("NestedType").is(SymbolKind::Struct))
        .expect(Symbol::new("NestedType").has(Behavior::FieldCount(0)));
    }

    #[test]
    fn deeply_nested_module_struct_compiles_and_verified() {
        Test::new(
            r#"module A.B.C.D
            struct DeepType { }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("DeepType").is(SymbolKind::Struct))
        .expect(Symbol::new("DeepType").has(Behavior::FieldCount(0)));
    }
}
