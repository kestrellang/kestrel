//! Tests for the test suite framework itself.
//!
//! These tests verify that the test framework's features work correctly:
//! - Path-based symbol lookup
//! - New behavior assertions
//! - Negated behaviors
//! - Error count expectations

use kestrel_test_suite::*;

mod path_based_lookup {
    use super::*;

    #[test]
    fn simple_name_lookup() {
        Test::new(
            r#"module Test
            struct Foo {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Foo").is(SymbolKind::Struct));
    }

    #[test]
    fn nested_path_lookup() {
        Test::new(
            r#"module Test
            struct Outer {
                func inner() {}
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Outer").is(SymbolKind::Struct))
        .expect(Symbol::new("Outer.inner").is(SymbolKind::Function));
    }

    #[test]
    fn deep_nested_path() {
        Test::new(
            r#"module Test
            struct Level1 {
                struct Level2 {
                    func level3() {}
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Level1.Level2.level3").is(SymbolKind::Function));
    }

    #[test]
    fn disambiguate_same_names() {
        // Two structs can have methods with the same name
        // Path lookup ensures we find the right one
        Test::new(
            r#"module Test
            struct First {
                func method() {}
            }
            struct Second {
                func method() {}
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("First.method").is(SymbolKind::Function))
        .expect(Symbol::new("Second.method").is(SymbolKind::Function));
    }
}

mod field_count_behavior {
    use super::*;

    #[test]
    fn struct_with_no_fields() {
        Test::new(
            r#"module Test
            struct Empty {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Empty").has(Behavior::FieldCount(0)));
    }

    #[test]
    fn struct_with_multiple_fields() {
        Test::new(
            r#"module Test
            struct Point {
                let x: Int
                let y: Int
                var z: Float
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Point").has(Behavior::FieldCount(3)));
    }
}

mod function_behaviors {
    use super::*;

    #[test]
    fn static_function() {
        Test::new(
            r#"module Test
            struct Math {
                static func add(a: Int, b: Int) -> Int { a + b }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Math.add")
                .is(SymbolKind::Function)
                .has(Behavior::IsStatic(true))
                .has(Behavior::HasBody(true))
                .has(Behavior::ParameterCount(2)),
        );
    }

    #[test]
    fn instance_method() {
        // Note: self.field access requires a mutating method for mutable fields
        Test::new(
            r#"module Test
            struct Counter {
                var value: Int
                func getValue() -> Int { self.value }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Counter.getValue")
                .is(SymbolKind::Function)
                .has(Behavior::IsStatic(false))
                .has(Behavior::HasBody(true)),
        );
    }

    #[test]
    fn protocol_method_no_body() {
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw()
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Drawable.draw")
                .is(SymbolKind::Function)
                .has(Behavior::HasBody(false)),
        );
    }
}

mod negated_behaviors {
    use super::*;

    #[test]
    fn not_generic() {
        Test::new(
            r#"module Test
            struct Simple {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Simple").not(Behavior::IsGeneric(true)));
    }

    #[test]
    fn not_public() {
        Test::new(
            r#"module Test
            struct Internal {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Internal").not(Behavior::Visibility(Visibility::Public)));
    }
}

mod error_expectations {
    use super::*;

    #[test]
    fn fails_expectation() {
        Test::new(
            r#"module Test
            func test() { unknownFunction() }
        "#,
        )
        .expect(Fails);
    }

    #[test]
    fn error_count() {
        // Note: error messages vary - using a case that reliably produces errors
        Test::new(
            r#"module Test
            func test() {
                unknownName
            }
        "#,
        )
        .expect(HasErrorCount(1));
    }

    #[test]
    fn specific_error_message() {
        Test::new(
            r#"module Test
            func test() {
                unknownSymbol
            }
        "#,
        )
        .expect(HasError("undefined")); // Actual error message uses "undefined"
    }
}

mod conformance_behavior {
    use super::*;

    #[test]
    fn struct_with_conformance() {
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw()
            }
            struct Circle: Drawable {
                func draw() {}
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Circle").has(Behavior::ConformanceCount(1)));
    }

    #[test]
    fn struct_without_conformance() {
        Test::new(
            r#"module Test
            struct Plain {}
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Plain").has(Behavior::ConformanceCount(0)));
    }
}

mod child_count_behavior {
    use super::*;

    #[test]
    fn struct_children() {
        // Struct has fields and methods as children
        Test::new(
            r#"module Test
            struct Widget {
                let id: Int
                func display() {}
            }
        "#,
        )
        .expect(Compiles)
        // Widget has 2 children: 1 field + 1 method
        .expect(Symbol::new("Widget").has(Behavior::ChildCount(2)));
    }
}
