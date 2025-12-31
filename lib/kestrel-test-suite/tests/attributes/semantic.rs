//! Attribute semantic tests
//!
//! Tests that verify attributes are correctly resolved and attached to symbols
//! via `AttributesBehavior`, and that appropriate warnings are emitted.

use kestrel_test_suite::*;

// =============================================================================
// ATTRIBUTES BEHAVIOR ATTACHMENT
// =============================================================================

mod behavior_attachment {
    use super::*;

    #[test]
    fn struct_has_attribute_behavior() {
        Test::new(
            r#"module Test
            @dummy
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Foo")
                .is(SymbolKind::Struct)
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn function_has_attribute_behavior() {
        Test::new(
            r#"module Test
            @dummy
            func bar() {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("bar")
                .is(SymbolKind::Function)
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn protocol_has_attribute_behavior() {
        Test::new(
            r#"module Test
            @dummy
            protocol Baz {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Baz")
                .is(SymbolKind::Protocol)
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn enum_has_attribute_behavior() {
        Test::new(
            r#"module Test
            @dummy
            enum Color { case Red }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Color")
                .is(SymbolKind::Enum)
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn field_has_attribute_behavior() {
        Test::new(
            r#"module Test
            struct Foo {
                @dummy
                var x: Int
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("x")
                .is(SymbolKind::Field)
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn initializer_has_attribute_behavior() {
        Test::new(
            r#"module Test
            struct Foo {
                var x: Int
                @dummy
                init(x: Int) {
                    self.x = x;
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Foo.init")
                .is(SymbolKind::Initializer)
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn enum_case_has_attribute_behavior() {
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
        .expect(
            Symbol::new("Active")
                .is(SymbolKind::EnumCase)
                .has(Behavior::HasAttribute("dummy")),
        );
    }
}

// =============================================================================
// ATTRIBUTE COUNT
// =============================================================================

mod attribute_count {
    use super::*;

    #[test]
    fn symbol_with_no_attributes() {
        Test::new(
            r#"module Test
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Foo")
                .is(SymbolKind::Struct)
                .has(Behavior::AttributeCount(0)),
        );
    }

    #[test]
    fn symbol_with_one_attribute() {
        Test::new(
            r#"module Test
            @dummy
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Foo")
                .is(SymbolKind::Struct)
                .has(Behavior::AttributeCount(1)),
        );
    }

    #[test]
    fn symbol_with_two_attributes() {
        Test::new(
            r#"module Test
            @dummy
            @dummy
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Foo")
                .is(SymbolKind::Struct)
                .has(Behavior::AttributeCount(2)),
        );
    }

    #[test]
    fn symbol_with_three_attributes() {
        Test::new(
            r#"module Test
            @dummy
            @dummy
            @dummy
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Foo")
                .is(SymbolKind::Struct)
                .has(Behavior::AttributeCount(3)),
        );
    }
}

// =============================================================================
// ATTRIBUTE ARGUMENT COUNT
// =============================================================================

mod argument_count {
    use super::*;

    #[test]
    fn attribute_with_no_args() {
        Test::new(
            r#"module Test
            @dummy
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Foo")
                .has(Behavior::AttributeArgCount("dummy", 0)),
        );
    }

    #[test]
    fn attribute_with_empty_parens_has_zero_args() {
        Test::new(
            r#"module Test
            @dummy()
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Foo")
                .has(Behavior::AttributeArgCount("dummy", 0)),
        );
    }

    #[test]
    fn attribute_with_one_arg() {
        Test::new(
            r#"module Test
            @dummy("message")
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Foo")
                .has(Behavior::AttributeArgCount("dummy", 1)),
        );
    }

    #[test]
    fn attribute_with_two_args() {
        Test::new(
            r#"module Test
            @dummy(1, 2)
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Foo")
                .has(Behavior::AttributeArgCount("dummy", 2)),
        );
    }

    #[test]
    fn attribute_with_labeled_args() {
        Test::new(
            r#"module Test
            @dummy(a: 1, b: 2, c: 3)
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Foo")
                .has(Behavior::AttributeArgCount("dummy", 3)),
        );
    }
}

// =============================================================================
// UNKNOWN ATTRIBUTE WARNINGS
// =============================================================================

mod unknown_attribute_warnings {
    use super::*;

    #[test]
    fn unknown_attribute_emits_warning() {
        Test::new(
            r#"module Test
            @unknownAttr
            struct Foo {}
        "#,
        )
        .expect(Compiles)  // Should still compile
        .expect(HasWarning("unknown attribute"));
    }

    #[test]
    fn unknown_attribute_with_args_emits_warning() {
        Test::new(
            r#"module Test
            @customThing(key: "value")
            func bar() {}
        "#,
        )
        .expect(Compiles)
        .expect(HasWarning("unknown attribute"));
    }

    #[test]
    fn multiple_unknown_attributes_emit_multiple_warnings() {
        Test::new(
            r#"module Test
            @unknown1
            @unknown2
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(HasWarning("unknown attribute"));
    }

    #[test]
    fn known_attribute_no_warning() {
        Test::new(
            r#"module Test
            @dummy
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(NoWarnings);
    }

    #[test]
    fn mixed_known_and_unknown_attributes() {
        Test::new(
            r#"module Test
            @dummy
            @unknown
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(HasWarning("unknown attribute"))
        .expect(Symbol::new("Foo").has(Behavior::AttributeCount(2)));
    }
}

// =============================================================================
// ATTRIBUTE WITH DIFFERENT DECLARATION COMBINATIONS
// =============================================================================

mod declaration_combinations {
    use super::*;

    #[test]
    fn public_struct_with_attribute() {
        Test::new(
            r#"module Test
            @dummy
            public struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Foo")
                .is(SymbolKind::Struct)
                .has(Behavior::Visibility(Visibility::Public))
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn generic_struct_with_attribute() {
        Test::new(
            r#"module Test
            @dummy
            struct Box[T] {
                var value: T
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Box")
                .is(SymbolKind::Struct)
                .has(Behavior::TypeParamCount(1))
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn struct_with_conformance_and_attribute() {
        Test::new(
            r#"module Test
            protocol Printable {}
            
            @dummy
            struct Foo: Printable {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Foo")
                .is(SymbolKind::Struct)
                .has(Behavior::ConformanceCount(1))
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn static_function_with_attribute() {
        Test::new(
            r#"module Test
            struct Foo {
                @dummy
                static func create() -> Foo {
                    Foo()
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Foo.create")
                .is(SymbolKind::Function)
                .has(Behavior::IsStatic(true))
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn mutating_method_with_attribute() {
        Test::new(
            r#"module Test
            struct Counter {
                var count: Int
                
                @dummy
                mutating func increment() {
                    self.count = self.count + 1;
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Counter.increment")
                .is(SymbolKind::Function)
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn protocol_with_attributed_method() {
        Test::new(
            r#"module Test
            protocol Drawable {
                @dummy
                func draw()
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Drawable.draw")
                .is(SymbolKind::Function)
                .has(Behavior::HasAttribute("dummy")),
        );
    }

    #[test]
    fn indirect_enum_with_attribute() {
        Test::new(
            r#"module Test
            @dummy
            indirect enum Tree {
                case Leaf(value: Int)
                case Node(left: Tree, right: Tree)
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Tree")
                .is(SymbolKind::Enum)
                .has(Behavior::HasAttribute("dummy")),
        );
    }
}
