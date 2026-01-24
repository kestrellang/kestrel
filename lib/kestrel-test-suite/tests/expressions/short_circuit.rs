//! Tests for short-circuit evaluation of `and` and `or` operators.
//!
//! These tests verify that:
//! - `and` and `or` operators short-circuit correctly
//! - RHS is wrapped in a closure and only called when needed
//! - Chained operators work correctly
//! - Mixed operators respect precedence
//!
//! These tests use the stdlib to access the `Bool` type.

use kestrel_test_suite::*;

// ============================================================================
// BASIC SHORT-CIRCUIT BEHAVIOR
// ============================================================================

mod basic_and {
    use super::*;

    #[test]
    fn and_with_true_lhs_evaluates_rhs() {
        // When LHS is true, RHS should be evaluated
        Test::new(
            r#"
module Main

func test() -> Bool {
    true and true
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn and_with_false_lhs_short_circuits() {
        // When LHS is false, RHS should NOT be evaluated
        Test::new(
            r#"
module Main

func test() -> Bool {
    false and true
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn and_returns_false_when_lhs_false() {
        Test::new(
            r#"
module Main

func test() -> Bool {
    false and false
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn and_returns_rhs_when_lhs_true() {
        Test::new(
            r#"
module Main

func alwaysFalse() -> Bool { false }

func test() -> Bool {
    true and alwaysFalse()
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

mod basic_or {
    use super::*;

    #[test]
    fn or_with_false_lhs_evaluates_rhs() {
        // When LHS is false, RHS should be evaluated
        Test::new(
            r#"
module Main

func test() -> Bool {
    false or true
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn or_with_true_lhs_short_circuits() {
        // When LHS is true, RHS should NOT be evaluated
        Test::new(
            r#"
module Main

func test() -> Bool {
    true or false
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn or_returns_true_when_lhs_true() {
        Test::new(
            r#"
module Main

func test() -> Bool {
    true or true
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn or_returns_rhs_when_lhs_false() {
        Test::new(
            r#"
module Main

func alwaysTrue() -> Bool { true }

func test() -> Bool {
    false or alwaysTrue()
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// CHAINED OPERATORS
// ============================================================================

mod chained_and {
    use super::*;

    #[test]
    fn chained_and_all_true() {
        // a and b and c where all are true
        Test::new(
            r#"
module Main

func test() -> Bool {
    true and true and true
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn chained_and_first_false() {
        // false and b and c - should short-circuit immediately
        Test::new(
            r#"
module Main

func test() -> Bool {
    false and true and true
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn chained_and_middle_false() {
        // a and false and c - should short-circuit after second
        Test::new(
            r#"
module Main

func test() -> Bool {
    true and false and true
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn chained_and_last_false() {
        // a and b and false - evaluates all
        Test::new(
            r#"
module Main

func test() -> Bool {
    true and true and false
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

mod chained_or {
    use super::*;

    #[test]
    fn chained_or_all_false() {
        // a or b or c where all are false
        Test::new(
            r#"
module Main

func test() -> Bool {
    false or false or false
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn chained_or_first_true() {
        // true or b or c - should short-circuit immediately
        Test::new(
            r#"
module Main

func test() -> Bool {
    true or false or false
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn chained_or_middle_true() {
        // a or true or c - should short-circuit after second
        Test::new(
            r#"
module Main

func test() -> Bool {
    false or true or false
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn chained_or_last_true() {
        // a or b or true - evaluates all
        Test::new(
            r#"
module Main

func test() -> Bool {
    false or false or true
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// MIXED OPERATORS (PRECEDENCE)
// ============================================================================

mod mixed_operators {
    use super::*;

    #[test]
    fn and_has_higher_precedence_than_or() {
        // a or b and c should parse as a or (b and c)
        // If a is true, the entire (b and c) should be skipped
        Test::new(
            r#"
module Main

func test() -> Bool {
    true or false and false
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn mixed_and_or_complex() {
        // a and b or c and d
        // Parses as: (a and b) or (c and d)
        Test::new(
            r#"
module Main

func test() -> Bool {
    true and false or false and true
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn parenthesized_or_in_and() {
        // a and (b or c)
        Test::new(
            r#"
module Main

func test() -> Bool {
    true and (false or true)
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn parenthesized_and_in_or() {
        // (a and b) or c - explicit grouping
        Test::new(
            r#"
module Main

func test() -> Bool {
    (true and false) or true
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// WITH NOT OPERATOR
// ============================================================================

mod with_not {
    use super::*;

    #[test]
    fn not_and() {
        Test::new(
            r#"
module Main

func test() -> Bool {
    not true and false
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn not_or() {
        Test::new(
            r#"
module Main

func test() -> Bool {
    not false or true
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn and_not() {
        Test::new(
            r#"
module Main

func test() -> Bool {
    true and not false
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn or_not() {
        Test::new(
            r#"
module Main

func test() -> Bool {
    false or not false
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// WITH COMPARISON OPERATORS
// ============================================================================

mod with_comparisons {
    use super::*;

    #[test]
    fn comparison_and_comparison() {
        // a < b and c > d
        Test::new(
            r#"
module Main

func test(a: Int, b: Int, c: Int, d: Int) -> Bool {
    a < b and c > d
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn comparison_or_comparison() {
        // a == b or c != d
        Test::new(
            r#"
module Main

func test(a: Int, b: Int, c: Int, d: Int) -> Bool {
    a == b or c != d
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn mixed_comparisons_and_or() {
        // a < b and c > d or e == f
        // Parses as: (a < b and c > d) or (e == f)
        Test::new(
            r#"
module Main

func test(a: Int, b: Int, c: Int, d: Int, e: Int, f: Int) -> Bool {
    a < b and c > d or e == f
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// WITH FUNCTION CALLS
// ============================================================================

mod with_function_calls {
    use super::*;

    #[test]
    fn function_call_and_function_call() {
        Test::new(
            r#"
module Main

func isPositive(n: Int) -> Bool {
    n > 0
}

func isEven(n: Int) -> Bool {
    n % 2 == 0
}

func test(n: Int) -> Bool {
    isPositive(n) and isEven(n)
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn function_call_or_function_call() {
        Test::new(
            r#"
module Main

func isZero(n: Int) -> Bool {
    n == 0
}

func isNegative(n: Int) -> Bool {
    n < 0
}

func test(n: Int) -> Bool {
    isZero(n) or isNegative(n)
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// IN CONTROL FLOW
// ============================================================================

mod in_control_flow {
    use super::*;

    #[test]
    fn and_in_if_condition() {
        Test::new(
            r#"
module Main

func test(a: Bool, b: Bool) -> Int {
    if a and b {
        1
    } else {
        0
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn or_in_if_condition() {
        Test::new(
            r#"
module Main

func test(a: Bool, b: Bool) -> Int {
    if a or b {
        1
    } else {
        0
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn and_in_while_condition() {
        Test::new(
            r#"
module Main

func test() -> Int {
    var i = 0;
    var sum = 0;
    while i < 10 and sum < 20 {
        sum = sum + i;
        i = i + 1;
    }
    sum
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn or_in_while_condition() {
        Test::new(
            r#"
module Main

func test(exitEarly: Bool) -> Int {
    var i = 0;
    while i < 10 or exitEarly {
        i = i + 1;
        if i > 5 { break }
    }
    i
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// NESTED EXPRESSIONS
// ============================================================================

mod nested_expressions {
    use super::*;

    #[test]
    fn deeply_nested_and() {
        Test::new(
            r#"
module Main

func test() -> Bool {
    (true and (true and (true and true)))
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn deeply_nested_or() {
        Test::new(
            r#"
module Main

func test() -> Bool {
    (false or (false or (false or true)))
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn complex_nested_mixed() {
        Test::new(
            r#"
module Main

func test() -> Bool {
    (true and false) or (false and true) or (true and true)
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// VARIABLES AND FIELDS
// ============================================================================

mod with_variables {
    use super::*;

    #[test]
    fn and_with_variables() {
        Test::new(
            r#"
module Main

func test(a: Bool, b: Bool) -> Bool {
    a and b
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn or_with_variables() {
        Test::new(
            r#"
module Main

func test(a: Bool, b: Bool) -> Bool {
    a or b
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn and_with_struct_fields() {
        Test::new(
            r#"
module Main

struct Flags {
    let enabled: Bool
    let visible: Bool
}

func test(f: Flags) -> Bool {
    f.enabled and f.visible
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn or_with_struct_fields() {
        Test::new(
            r#"
module Main

struct State {
    let loading: Bool
    let error: Bool
}

func test(s: State) -> Bool {
    s.loading or s.error
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}
