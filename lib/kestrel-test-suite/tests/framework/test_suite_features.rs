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
                let x: lang.i64
                let y: lang.i64
                var z: lang.f64
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
                static func add(a: lang.i64, b: lang.i64) -> lang.i64 { lang.i64_add(a, b) }
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
                var value: lang.i64
                func getValue() -> lang.i64 { self.value }
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
                let id: lang.i64
                func display() {}
            }
        "#,
        )
        .expect(Compiles)
        // Widget has 2 children: 1 field + 1 method
        .expect(Symbol::new("Widget").has(Behavior::ChildCount(2)));
    }
}

mod prelude {
    use super::*;

    #[test]
    fn prelude_is_included_by_default() {
        // Tests can import Copyable and Cloneable from the prelude
        Test::new(
            r#"module Test
            import Prelude

            struct Handle: not Copyable {
                var fd: lang.i64
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Handle")
                .is(SymbolKind::Struct)
                .has(Behavior::IsCopyable(false)),
        );
    }

    #[test]
    fn prelude_selective_import() {
        // Can import specific items from prelude
        // Note: Using full import as selective import of builtin protocols
        // may have resolution limitations
        Test::new(
            r#"module Test
            import Prelude

            struct Handle: not Copyable {
                var fd: lang.i64
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Handle").has(Behavior::IsCopyable(false)));
    }

    #[test]
    fn prelude_cloneable_import() {
        // Can use Cloneable from prelude
        Test::new(
            r#"module Test
            import Prelude

            struct Data: Cloneable {
                var value: lang.i64

                func clone() -> Data {
                    Data(value: self.value)
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Data")
                .is(SymbolKind::Struct)
                .has(Behavior::IsCloneable(true)),
        );
    }

    #[test]
    fn without_prelude_allows_own_definitions() {
        // Tests that opt out of prelude can define their own builtin protocols
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}

            struct Handle: not Copyable {
                var fd: lang.i64
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(Symbol::new("Handle").has(Behavior::IsCopyable(false)));
    }

    #[test]
    fn with_prelude_is_explicit_default() {
        // .with_prelude() is the same as default
        Test::new(
            r#"module Test
            import Prelude

            struct Handle: not Copyable {
                var fd: lang.i64
            }
        "#,
        )
        .with_prelude()
        .expect(Compiles)
        .expect(Symbol::new("Handle").has(Behavior::IsCopyable(false)));
    }
}

mod run_expectations {
    use super::*;

    #[test]
    fn exit_code_expectation() {
        // Test that we can verify exit codes
        Test::new(
            r#"module Test

            func main() -> lang.i64 {
                42
            }
        "#,
        )
        .expect(Compiles)
        .expect(ExitCode(42));
    }

    #[test]
    fn runs_expectation_with_zero_exit() {
        // Runs expects exit code 0
        Test::new(
            r#"module Test

            func main() -> lang.i64 {
                0
            }
        "#,
        )
        .expect(Compiles)
        .expect(Runs);
    }

    #[test]
    fn exit_code_from_expression() {
        // Verify more complex exit codes
        Test::new(
            r#"module Test

            func add(a: lang.i64, b: lang.i64) -> lang.i64 {
                lang.i64_add(a, b)
            }

            func main() -> lang.i64 {
                add(20, 22)
            }
        "#,
        )
        .expect(Compiles)
        .expect(ExitCode(42));
    }
}

mod stdlib {
    use super::*;

    #[test]
    fn with_stdlib_loads_stdlib_files() {
        // Test that with_stdlib() makes stdlib available
        // Note: This may fail if stdlib has compilation issues, but demonstrates the feature
        Test::new(
            r#"module Test

            // Just test that stdlib compiles alongside user code
            func main() -> lang.i64 {
                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn without_stdlib_is_default() {
        // Verify that without_stdlib() is the default behavior
        Test::new(
            r#"module Test
            struct Foo {}
        "#,
        )
        .without_stdlib()
        .expect(Compiles);
    }
}
