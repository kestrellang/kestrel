use kestrel_test_suite::*;

// ============================================================================
// Value Promotion Tests
// ============================================================================
// These test the FromValue protocol-based implicit promotion from T to
// Optional[T] or Result[T, E].

#[test]
fn value_promotion_optional_in_binding() {
    // Value is implicitly promoted to Optional via FromValue.from()
    Test::new(
        r#"
        module Main
        import std.num.Int64
        func test() {
            let x: Int64? = 5;
        }
        "#,
    )
    .with_stdlib()
    .expect(Compiles);
}

#[test]
fn value_promotion_optional_in_return() {
    // Return value is implicitly promoted to Optional
    Test::new(
        r#"
        module Main
        import std.num.Int64
        func getValue() -> Int64? {
            return 5
        }
        "#,
    )
    .with_stdlib()
    .expect(Compiles);
}

#[test]
fn value_promotion_optional_in_yield() {
    // Yield expression is implicitly promoted to Optional
    Test::new(
        r#"
        module Main
        import std.num.Int64
        func getValue() -> Int64? {
            5
        }
        "#,
    )
    .with_stdlib()
    .expect(Compiles);
}

#[test]
fn value_promotion_result_in_binding() {
    // Value is implicitly promoted to Result via FromValue.from()
    Test::new(
        r#"
        module Main
        import std.num.Int64
        import std.text.String
        func test() {
            let r: Int64 throws String = 42;
        }
        "#,
    )
    .with_stdlib()
    .expect(Compiles);
}

#[test]
fn value_promotion_result_in_return() {
    // Return value is implicitly promoted to Result
    Test::new(
        r#"
        module Main
        import std.num.Int64
        import std.text.String
        func compute() -> Int64 throws String {
            return 42
        }
        "#,
    )
    .with_stdlib()
    .expect(Compiles);
}

#[test]
fn value_promotion_in_assignment() {
    // Assignment target is Optional, value is promoted
    Test::new(
        r#"
        module Main
        import std.num.Int64
        func test() {
            var x: Int64? = null;
            x = 10;
        }
        "#,
    )
    .with_stdlib()
    .expect(Compiles);
}

#[test]
fn value_promotion_in_function_arg() {
    // Function argument is promoted when parameter is Optional
    Test::new(
        r#"
        module Main
        import std.num.Int64
        func takesOptional(x: Int64?) {}
        func test() {
            takesOptional(5);
        }
        "#,
    )
    .with_stdlib()
    .expect(Compiles);
}

#[test]
fn value_promotion_in_if_branches() {
    // If branches can return promoted values when both branches are the same type
    // Note: Mixed literal cases (e.g., `5` and `null`) require additional work
    // to handle constraint ordering between different literal types.
    Test::new(
        r#"
        module Main
        import std.num.Int64
        import std.core.Bool
        func test(cond: Bool) -> Int64? {
            let x = if cond { 5 } else { 10 };
            x
        }
        "#,
    )
    .with_stdlib()
    .expect(Compiles);
}

#[test]
fn no_promotion_without_type_annotation() {
    // Without explicit type annotation, no promotion should happen
    // x should be inferred as Int64, not Int64?
    Test::new(
        r#"
        module Main
        import std.num.Int64
        func test() {
            let x = 5;
            let y: Int64 = x;  // Should work: x is Int64
        }
        "#,
    )
    .with_stdlib()
    .expect(Compiles);
}

#[test]
fn nested_optional_no_promotion() {
    // Int cannot be promoted to Optional[Optional[Int]] (double optional)
    // Optional[Optional[Int]] conforms to FromValue[Optional[Int]], not FromValue[Int]
    Test::new(
        r#"
        module Main
        import std.num.Int64
        import std.result.Optional
        func test() {
            let x: Optional[Optional[Int64]] = 5;
        }
        "#,
    )
    .with_stdlib()
    .expect(HasError("type mismatch"));
}

#[test]
fn incompatible_type_no_promotion() {
    // String? does not conform to FromValue[Int]
    Test::new(
        r#"
        module Main
        import std.num.Int64
        import std.text.String
        func test() {
            let x: String? = 5;
        }
        "#,
    )
    .with_stdlib()
    .expect(HasError("type mismatch"));
}

#[test]
fn generic_function_promotion() {
    // Generic functions can use promotion when return type is Optional[T]
    Test::new(
        r#"
        module Main
        func wrap[T](value: T) -> T? {
            return value
        }
        "#,
    )
    .with_stdlib()
    .expect(Compiles);
}

// ============================================================================
// Original Optional Tests
// ============================================================================

#[test]
fn null_assignable_to_optional_type() {
    // null uses ExpressibleByNullLiteral protocol, stdlib provides Optional implementation
    Test::new(
        r#"
        module Main
        import std.num.Int64
        func test() {
            let x: Int64? = null;
        }
        "#,
    )
    .with_stdlib()
    .expect(Compiles);
}

#[test]
fn non_optional_type_cannot_be_null() {
    // lang.i64 does not conform to ExpressibleByNullLiteral
    Test::new(
        r#"
        module Main
        func test() {
            let x: lang.i64 = null;
        }
        "#,
    )
    .with_stdlib()
    .expect(HasError("does not conform"));
}
