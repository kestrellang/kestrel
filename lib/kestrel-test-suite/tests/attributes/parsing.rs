//! Attribute parsing tests
//!
//! Tests that verify the parser correctly handles attribute syntax.
//! These tests focus on syntax acceptance, not semantic validation.

use kestrel_test_suite::*;

// =============================================================================
// SIMPLE ATTRIBUTE SYNTAX
// =============================================================================

mod simple_attributes {
    use super::*;

    #[test]
    fn simple_attribute_on_struct() {
        Test::new(
            r#"module Test
            @dummy
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Foo").is(SymbolKind::Struct));
    }

    #[test]
    fn simple_attribute_on_protocol() {
        Test::new(
            r#"module Test
            @dummy
            protocol Bar {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Bar").is(SymbolKind::Protocol));
    }

    #[test]
    fn simple_attribute_on_enum() {
        Test::new(
            r#"module Test
            @dummy
            enum Color { case Red case Blue }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Color").is(SymbolKind::Enum));
    }

    #[test]
    fn simple_attribute_on_function() {
        Test::new(
            r#"module Test
            @dummy
            func foo() {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("foo").is(SymbolKind::Function));
    }

    #[test]
    fn simple_attribute_on_field() {
        Test::new(
            r#"module Test
            struct Foo {
                @dummy
                var x: lang.i64
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("x").is(SymbolKind::Field));
    }

    #[test]
    fn simple_attribute_on_initializer() {
        Test::new(
            r#"module Test
            struct Foo {
                @dummy
                init() {}
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Foo").is(SymbolKind::Struct));
    }

    #[test]
    fn simple_attribute_on_enum_case() {
        Test::new(
            r#"module Test
            enum Status {
                @dummy
                case Active
                case Inactive
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Status").is(SymbolKind::Enum));
    }
}

// =============================================================================
// ATTRIBUTE WITH EMPTY PARENTHESES
// =============================================================================

mod empty_parens {
    use super::*;

    #[test]
    fn attribute_with_empty_parens_on_struct() {
        Test::new(
            r#"module Test
            @dummy()
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Foo").is(SymbolKind::Struct));
    }

    #[test]
    fn attribute_with_empty_parens_on_function() {
        Test::new(
            r#"module Test
            @dummy()
            func bar() {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("bar").is(SymbolKind::Function));
    }
}

// =============================================================================
// ATTRIBUTE WITH SINGLE ARGUMENT
// =============================================================================

mod single_argument {
    use super::*;

    #[test]
    fn attribute_with_string_arg() {
        Test::new(
            r#"module Test
            @dummy("some message")
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Foo").is(SymbolKind::Struct));
    }

    #[test]
    fn attribute_with_integer_arg() {
        Test::new(
            r#"module Test
            @dummy(42)
            func bar() {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("bar").is(SymbolKind::Function));
    }

    #[test]
    fn attribute_with_float_arg() {
        Test::new(
            r#"module Test
            @dummy(3.14)
            struct Foo {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn attribute_with_boolean_arg() {
        Test::new(
            r#"module Test
            @dummy(true)
            func bar() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn attribute_with_implicit_member_arg() {
        // This is the syntax for enum-like options: .always, .never, .Copyable
        Test::new(
            r#"module Test
            @dummy(.always)
            func bar() {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("bar").is(SymbolKind::Function));
    }

    #[test]
    fn attribute_with_path_arg() {
        // Path argument like a type reference
        Test::new(
            r#"module Test
            struct Target {}
            @dummy(Target)
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Foo").is(SymbolKind::Struct));
    }
}

// =============================================================================
// ATTRIBUTE WITH MULTIPLE ARGUMENTS
// =============================================================================

mod multiple_arguments {
    use super::*;

    #[test]
    fn attribute_with_two_args() {
        Test::new(
            r#"module Test
            @dummy("message", 42)
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Foo").is(SymbolKind::Struct));
    }

    #[test]
    fn attribute_with_three_args() {
        Test::new(
            r#"module Test
            @dummy(1, 2, 3)
            func bar() {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn attribute_with_mixed_type_args() {
        Test::new(
            r#"module Test
            @dummy("text", 42, true, .option)
            struct Foo {}
        "#,
        )
        .expect(Compiles);
    }
}

// =============================================================================
// ATTRIBUTE WITH LABELED ARGUMENTS
// =============================================================================

mod labeled_arguments {
    use super::*;

    #[test]
    fn attribute_with_single_labeled_arg() {
        Test::new(
            r#"module Test
            @dummy(message: "hello")
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Foo").is(SymbolKind::Struct));
    }

    #[test]
    fn attribute_with_multiple_labeled_args() {
        Test::new(
            r#"module Test
            @dummy(iOS: 15, macOS: 12)
            func bar() {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("bar").is(SymbolKind::Function));
    }

    #[test]
    fn attribute_with_mixed_labeled_and_unlabeled() {
        Test::new(
            r#"module Test
            @dummy("first", key: "value", 42)
            struct Foo {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn attribute_with_float_labeled_arg() {
        // Common pattern for version requirements
        Test::new(
            r#"module Test
            @dummy(version: 1.5)
            struct Foo {}
        "#,
        )
        .expect(Compiles);
    }
}

// =============================================================================
// MULTIPLE ATTRIBUTES
// =============================================================================

mod multiple_attributes {
    use super::*;

    #[test]
    fn two_attributes_on_struct() {
        Test::new(
            r#"module Test
            @dummy
            @dummy
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Foo").is(SymbolKind::Struct));
    }

    #[test]
    fn two_different_attributes_on_function() {
        // Using dummy for both since we only have one known attribute
        Test::new(
            r#"module Test
            @dummy
            @dummy(42)
            func bar() {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("bar").is(SymbolKind::Function));
    }

    #[test]
    fn three_attributes_on_protocol() {
        Test::new(
            r#"module Test
            @dummy
            @dummy("note")
            @dummy(enabled: true)
            protocol Baz {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Baz").is(SymbolKind::Protocol));
    }

    #[test]
    fn attributes_with_visibility() {
        // Attributes come before visibility
        Test::new(
            r#"module Test
            @dummy
            public struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Foo").has(Behavior::Visibility(Visibility::Public)));
    }

    #[test]
    fn multiple_attributes_with_visibility() {
        Test::new(
            r#"module Test
            @dummy
            @dummy("info")
            public func bar() {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("bar").has(Behavior::Visibility(Visibility::Public)));
    }
}

// =============================================================================
// ATTRIBUTES ON NESTED DECLARATIONS
// =============================================================================

mod nested_declarations {
    use super::*;

    #[test]
    fn attribute_on_method_in_struct() {
        Test::new(
            r#"module Test
            struct Foo {
                @dummy
                func bar() {}
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Foo.bar").is(SymbolKind::Function));
    }

    #[test]
    fn attribute_on_struct_and_its_method() {
        Test::new(
            r#"module Test
            @dummy
            struct Foo {
                @dummy
                func bar() {}
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Foo").is(SymbolKind::Struct))
        .expect(Symbol::new("Foo.bar").is(SymbolKind::Function));
    }

    #[test]
    fn attribute_on_protocol_method() {
        Test::new(
            r#"module Test
            protocol Drawable {
                @dummy
                func draw()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Drawable.draw").is(SymbolKind::Function));
    }

    #[test]
    fn attribute_on_nested_struct() {
        Test::new(
            r#"module Test
            struct Outer {
                @dummy
                struct Inner {}
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Outer.Inner").is(SymbolKind::Struct));
    }

    #[test]
    fn attribute_on_nested_enum() {
        Test::new(
            r#"module Test
            struct Outer {
                @dummy
                enum Inner { case A }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Outer.Inner").is(SymbolKind::Enum));
    }
}

// =============================================================================
// UNKNOWN ATTRIBUTES (should parse but may warn)
// =============================================================================

mod unknown_attributes {
    use super::*;

    #[test]
    fn unknown_attribute_parses_successfully() {
        // Unknown attributes should parse without error
        // (warning is checked in semantic tests)
        Test::new(
            r#"module Test
            @unknownAttribute
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Foo").is(SymbolKind::Struct));
    }

    #[test]
    fn unknown_attribute_with_args_parses() {
        Test::new(
            r#"module Test
            @myCustomAttr(key: "value", 42)
            func bar() {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("bar").is(SymbolKind::Function));
    }

    #[test]
    fn multiple_unknown_attributes_parse() {
        Test::new(
            r#"module Test
            @first
            @second("arg")
            @third(a: 1, b: 2)
            struct Foo {}
        "#,
        )
        .expect(Compiles);
    }
}

// =============================================================================
// ATTRIBUTE EXPRESSION SUBSET
// =============================================================================

mod expression_subset {
    use super::*;

    // These tests verify that only the allowed expression types work in attributes

    #[test]
    fn string_literal_allowed() {
        Test::new(
            r#"module Test
            @dummy("hello world")
            struct Foo {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn integer_literal_allowed() {
        Test::new(
            r#"module Test
            @dummy(123)
            struct Foo {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn hex_integer_allowed() {
        Test::new(
            r#"module Test
            @dummy(0xFF)
            struct Foo {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn float_literal_allowed() {
        Test::new(
            r#"module Test
            @dummy(1.5)
            struct Foo {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn boolean_true_allowed() {
        Test::new(
            r#"module Test
            @dummy(true)
            struct Foo {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn boolean_false_allowed() {
        Test::new(
            r#"module Test
            @dummy(false)
            struct Foo {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn implicit_member_access_allowed() {
        Test::new(
            r#"module Test
            @dummy(.someOption)
            struct Foo {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn simple_path_allowed() {
        Test::new(
            r#"module Test
            struct Target {}
            @dummy(Target)
            struct Foo {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn qualified_path_allowed() {
        Test::new(
            r#"module Test
            struct Outer {
                struct Inner {}
            }
            @dummy(Outer.Inner)
            struct Foo {}
        "#,
        )
        .expect(Compiles);
    }
}
