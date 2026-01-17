//! Integration tests for mutability validation.
//!
//! Tests that assignment to immutable variables and fields is properly rejected.

use kestrel_test_suite::*;

mod local_variables {
    use super::*;

    #[test]
    fn assign_to_var_succeeds() {
        Test::new(
            r#"module Test
            func test() -> lang.i64 {
                var x: lang.i64 = 5;
                x = 10;
                x
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("test")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        );
    }

    #[test]
    fn assign_to_let_fails() {
        Test::new(
            r#"module Test
            func test() -> lang.i64 {
                let x: lang.i64 = 5;
                x = 10;
                x
            }
        "#,
        )
        .expect(HasError("cannot assign to immutable variable"));
    }

    #[test]
    fn assign_to_parameter_fails() {
        // Parameters are immutable by default
        Test::new(
            r#"module Test
            func test(x: lang.i64) -> lang.i64 {
                x = 10;
                x
            }
        "#,
        )
        .expect(HasError("cannot assign to immutable"));
    }

    #[test]
    fn multiple_assignments_to_var() {
        Test::new(
            r#"module Test
            func test() -> lang.i64 {
                var x: lang.i64 = 1;
                x = 2;
                x = 3;
                x = 4;
                x
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("test")
                .is(SymbolKind::Function)
                .has(Behavior::HasBody(true)),
        );
    }
}

mod field_access {
    use super::*;

    #[test]
    fn assign_to_var_field_on_var_receiver() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }
            func test() -> lang.i64 {
                var p = Point(x: 1, y: 2);
                p.x = 10;
                p.x
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(2)),
        );
    }

    #[test]
    fn assign_to_immutable_field_fails() {
        // Tests that assigning to a let field fails regardless of receiver mutability
        Test::new(
            r#"module Test
            struct Point {
                let x: lang.i64
                var y: lang.i64
            }
            func test() -> lang.i64 {
                var p = Point(x: 1, y: 2);
                p.x = 10;
                p.x
            }
        "#,
        )
        .expect(HasError("cannot assign to immutable field"))
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(2)),
        );
    }

    #[test]
    fn assign_to_mutable_field_on_immutable_receiver_fails() {
        // Even though x is mutable (var), receiver p is immutable (let)
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }
            func test() -> lang.i64 {
                let p = Point(x: 1, y: 2);
                p.x = 10;
                p.x
            }
        "#,
        )
        .expect(HasError("cannot assign to immutable field"))
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(2)),
        );
    }

    #[test]
    fn nested_field_assignment_all_mutable_succeeds() {
        // All components in the chain are mutable
        Test::new(
            r#"module Test
            struct Inner {
                var value: lang.i64
            }
            struct Outer {
                var inner: Inner
            }
            func test() -> lang.i64 {
                var o = Outer(inner: Inner(value: 1));
                o.inner.value = 10;
                o.inner.value
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Outer")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(1)),
        )
        .expect(
            Symbol::new("Inner")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(1)),
        );
    }

    #[test]
    fn nested_field_assignment_inner_immutable_fails() {
        // Inner field is immutable (let)
        Test::new(
            r#"module Test
            struct Inner {
                let value: lang.i64
            }
            struct Outer {
                var inner: Inner
            }
            func test() -> lang.i64 {
                var o = Outer(inner: Inner(value: 1));
                o.inner.value = 10;
                o.inner.value
            }
        "#,
        )
        .expect(HasError("cannot assign to immutable field"))
        .expect(
            Symbol::new("Outer")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(1)),
        )
        .expect(
            Symbol::new("Inner")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(1)),
        );
    }

    #[test]
    fn nested_field_assignment_outer_immutable_fails() {
        // Outer field is immutable (let), blocking inner access
        Test::new(
            r#"module Test
            struct Inner {
                var value: lang.i64
            }
            struct Outer {
                let inner: Inner
            }
            func test() -> lang.i64 {
                var o = Outer(inner: Inner(value: 1));
                o.inner.value = 10;
                o.inner.value
            }
        "#,
        )
        .expect(HasError("cannot assign to immutable field"))
        .expect(
            Symbol::new("Outer")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(1)),
        )
        .expect(
            Symbol::new("Inner")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(1)),
        );
    }

    #[test]
    fn nested_field_assignment_receiver_immutable_fails() {
        // Receiver variable is immutable (let)
        Test::new(
            r#"module Test
            struct Inner {
                var value: lang.i64
            }
            struct Outer {
                var inner: Inner
            }
            func test() -> lang.i64 {
                let o = Outer(inner: Inner(value: 1));
                o.inner.value = 10;
                o.inner.value
            }
        "#,
        )
        .expect(HasError("cannot assign to immutable field"))
        .expect(
            Symbol::new("Outer")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(1)),
        )
        .expect(
            Symbol::new("Inner")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(1)),
        );
    }
}

mod initializers {
    use super::*;

    #[test]
    fn init_can_assign_to_all_field_types() {
        // In initializers, self.field = value is allowed for both let and var fields
        Test::new(
            r#"module Test
            struct Mixed {
                let id: lang.i64
                let name: lang.str                var value: lang.i64
                var count: lang.i64

                init(id: lang.i64, name: lang.str, value: lang.i64, count: lang.i64) {
                    self.id = id;
                    self.name = name;
                    self.value = value;
                    self.count = count;
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn init_with_only_let_fields() {
        // Initializer for struct with only immutable fields
        Test::new(
            r#"module Test
            struct Immutable {
                let x: lang.i64
                let y: lang.i64

                init(x: lang.i64, y: lang.i64) {
                    self.x = x;
                    self.y = y;
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn init_with_only_var_fields() {
        // Initializer for struct with only mutable fields
        Test::new(
            r#"module Test
            struct Mutable {
                var x: lang.i64
                var y: lang.i64

                init(x: lang.i64, y: lang.i64) {
                    self.x = x;
                    self.y = y;
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod invalid_targets {
    use super::*;

    #[test]
    fn assign_to_literal_fails() {
        Test::new(
            r#"module Test
            func test() -> lang.i64 {
                5 = 10;
                0
            }
        "#,
        )
        .expect(HasError("cannot assign to this expression"))
        .expect(
            Symbol::new("test")
                .is(SymbolKind::Function)
                .has(Behavior::HasBody(true)),
        );
    }

    #[test]
    fn assign_to_function_call_result_fails() {
        Test::new(
            r#"module Test
            func getValue() -> lang.i64 { 5 }
            func test() -> lang.i64 {
                getValue() = 10;
                0
            }
        "#,
        )
        .expect(HasError("cannot assign to this expression"))
        .expect(
            Symbol::new("getValue")
                .is(SymbolKind::Function)
                .has(Behavior::HasBody(true)),
        )
        .expect(
            Symbol::new("test")
                .is(SymbolKind::Function)
                .has(Behavior::HasBody(true)),
        );
    }

    #[test]
    fn assign_to_struct_initializer_fails() {
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }
            func test() -> lang.i64 {
                Point(x: 1, y: 2) = Point(x: 3, y: 4);
                0
            }
        "#,
        )
        .expect(HasError("cannot assign to this expression"))
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(2)),
        );
    }

    #[test]
    fn assign_to_binary_expression_fails() {
        Test::new(
            r#"module Test
            func test() -> lang.i64 {
                (5 + 10) = 20;
                0
            }
        "#,
        )
        .expect(HasError("cannot assign to this expression"));
    }
}

mod assignment_validation {
    use super::*;

    #[test]
    fn assign_to_immutable_field() {
        // TODO: Validate that target is assignable (var, not let)
        Test::new(
            r#"
module Main
struct S {
    let x: lang.i64
}
func test() {
    var s = S(x: 1);
    s.x = 2
}
"#,
        )
        .expect(HasError("cannot assign to immutable field 'x'"));
    }

    #[test]
    fn assign_to_field_on_immutable_receiver() {
        // TODO: Validate that target is assignable (field on mutable receiver)
        Test::new(
            r#"
module Main
struct S {
    var x: lang.i64
}
func test() {
    let s = S(x: 1);
    s.x = 2
}
"#,
        )
        .expect(HasError("cannot assign to immutable field 'x'"));
    }
}
