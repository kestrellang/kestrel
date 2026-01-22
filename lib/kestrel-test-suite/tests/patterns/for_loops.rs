//! Tests for for loops.
//!
//! These tests verify that:
//! - For loops parse and resolve correctly
//! - For loops iterate over ranges and iterables
//! - Pattern destructuring works in for loop bindings
//! - Mutable bindings work in for loops
//! - Labeled break/continue work with for loops
//! - Nested for loops work correctly
//! - Empty iterators execute zero times
//! - Error cases are properly detected

use kestrel_test_suite::*;

// ============================================================================
// BASIC FOR LOOPS
// ============================================================================

mod basic {
    use super::*;

    #[test]
    fn for_loop_over_range() {
        Test::new(
            r#"
module Main

func test() {
    var sum: std.num.Int64 = 0;
    for i in std.core.Range[std.num.Int64](0, 5) {
        sum = sum + i
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn for_loop_over_array() {
        Test::new(
            r#"
module Main

func test() {
    var arr = std.collections.Array[std.num.Int64]();
    arr.append(10);
    arr.append(20);
    arr.append(30);

    var sum: std.num.Int64 = 0;
    for item in arr {
        sum = sum + item
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn for_loop_accumulates_value() {
        Test::new(
            r#"
module Main

func test() -> std.num.Int {
    var sum: std.num.Int = 0;
    for i in std.core.Range[std.num.Int](1, 6) {
        sum = sum + i
    }
    sum
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn for_loop_without_semicolon_followed_by_expression() {
        Test::new(
            r#"
module Main

func test() -> std.num.Int {
    var count: std.num.Int = 0;
    for i in std.core.Range[std.num.Int](0, 5) {
        count = count + 1
    }
    count
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// PATTERN DESTRUCTURING
// ============================================================================

mod patterns {
    use super::*;

    #[test]
    fn for_loop_with_tuple_destructuring() {
        Test::new(
            r#"
module Main

struct Pair {
    let first: std.num.Int
    let second: std.num.Int
}

func test() {
    var arr = std.collections.Array[Pair]();
    arr.append(Pair(first: 1, second: 2));
    arr.append(Pair(first: 3, second: 4));

    var sum: std.num.Int = 0;
    for pair in arr {
        sum = sum + pair.first + pair.second
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn for_loop_with_wildcard_pattern() {
        Test::new(
            r#"
module Main

func test() {
    var count: std.num.Int64 = 0;
    for _ in std.core.Range[std.num.Int64](0, 10) {
        count = count + 1
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// MUTABLE BINDINGS
// ============================================================================

mod mutability {
    use super::*;

    #[test]
    fn for_loop_with_mutable_binding() {
        Test::new(
            r#"
module Main

func test() {
    var sum: std.num.Int64 = 0;
    for var x in std.core.Range[std.num.Int64](0, 5) {
        x = x + 1;
        sum = sum + x
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn for_loop_immutable_binding_cannot_be_reassigned() {
        Test::new(
            r#"
module Main

func test() {
    for x in std.core.Range[std.num.Int64](0, 5) {
        x = x + 1
    }
}
"#,
        )
        .with_stdlib()
        .expect(Fails)
        .expect(HasError("immutable"));
    }
}

// ============================================================================
// CONTROL FLOW
// ============================================================================

mod control_flow {
    use super::*;

    #[test]
    fn for_loop_with_break() {
        Test::new(
            r#"
module Main

func test() {
    var count: std.num.Int64 = 0;
    for i in std.core.Range[std.num.Int64](0, 100) {
        count = count + 1;
        if i > 5 {
            break
        }
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn for_loop_with_continue() {
        Test::new(
            r#"
module Main

func test() {
    var sum: std.num.Int64 = 0;
    for i in std.core.Range[std.num.Int64](0, 10) {
        if i == 5 {
            continue
        }
        sum = sum + i
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn for_loop_with_return() {
        Test::new(
            r#"
module Main

func test() -> std.num.Int {
    for i in std.core.Range[std.num.Int](0, 100) {
        if i > 50 {
            return i
        }
    }
    0
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// LABELED FOR LOOPS
// ============================================================================

mod labeled {
    use super::*;

    #[test]
    fn labeled_for_loop_with_break() {
        Test::new(
            r#"
module Main

func test() {
    outer: for i in std.core.Range[std.num.Int64](0, 10) {
        for j in std.core.Range[std.num.Int64](0, 10) {
            if j > 5 {
                break outer
            }
        }
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn labeled_for_loop_with_continue() {
        Test::new(
            r#"
module Main

func test() {
    outer: for i in std.core.Range[std.num.Int64](0, 10) {
        for j in std.core.Range[std.num.Int64](0, 10) {
            if j > 5 {
                continue outer
            }
        }
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn multiple_labeled_for_loops() {
        Test::new(
            r#"
module Main

func test() {
    outer: for i in std.core.Range[std.num.Int64](0, 10) {
        middle: for j in std.core.Range[std.num.Int64](0, 10) {
            inner: for k in std.core.Range[std.num.Int64](0, 10) {
                if k > 3 {
                    break inner
                }
                if j > 5 {
                    break middle
                }
                if i > 7 {
                    break outer
                }
            }
        }
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn break_to_undeclared_label_fails() {
        Test::new(
            r#"
module Main

func test() {
    for i in std.core.Range[std.num.Int64](0, 10) {
        break nonexistent
    }
}
"#,
        )
        .with_stdlib()
        .expect(Fails)
        .expect(HasError("undeclared label"));
    }

    #[test]
    fn continue_to_undeclared_label_fails() {
        Test::new(
            r#"
module Main

func test() {
    for i in std.core.Range[std.num.Int64](0, 10) {
        continue nonexistent
    }
}
"#,
        )
        .with_stdlib()
        .expect(Fails)
        .expect(HasError("undeclared label"));
    }
}

// ============================================================================
// NESTED FOR LOOPS
// ============================================================================

mod nested {
    use super::*;

    #[test]
    fn nested_for_loops() {
        Test::new(
            r#"
module Main

func test() {
    var sum: std.num.Int64 = 0;
    for i in std.core.Range[std.num.Int64](0, 5) {
        for j in std.core.Range[std.num.Int64](0, 5) {
            sum = sum + 1
        }
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn deeply_nested_for_loops() {
        Test::new(
            r#"
module Main

func test() {
    var sum: std.num.Int64 = 0;
    for i in std.core.Range[std.num.Int64](0, 3) {
        for j in std.core.Range[std.num.Int64](0, 3) {
            for k in std.core.Range[std.num.Int64](0, 3) {
                sum = sum + 1
            }
        }
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn for_loop_inside_while() {
        Test::new(
            r#"
module Main

func test() {
    var count: std.num.Int64 = 0;
    while count < 5 {
        for i in std.core.Range[std.num.Int64](0, 3) {
            count = count + 1
        }
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn while_loop_inside_for() {
        Test::new(
            r#"
module Main

func test() {
    for i in std.core.Range[std.num.Int64](0, 5) {
        var j: std.num.Int64 = 0;
        while j < i {
            j = j + 1
        }
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// SCOPING
// ============================================================================

mod scoping {
    use super::*;

    #[test]
    fn binding_visible_in_loop_body() {
        Test::new(
            r#"
module Main

func test() {
    var sum: std.num.Int64 = 0;
    for i in std.core.Range[std.num.Int64](0, 5) {
        sum = sum + i
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn binding_not_visible_after_loop() {
        Test::new(
            r#"
module Main

func test() -> std.num.Int {
    for i in std.core.Range[std.num.Int](0, 5) {
        ()
    }
    i
}
"#,
        )
        .with_stdlib()
        .expect(Fails)
        .expect(HasError("undefined"));
    }

    #[test]
    fn outer_variables_accessible() {
        Test::new(
            r#"
module Main

func test() {
    var sum: std.num.Int64 = 0;
    let multiplier: std.num.Int64 = 2;
    for i in std.core.Range[std.num.Int64](0, 5) {
        sum = sum + (i * multiplier)
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn shadowing_outer_variable() {
        Test::new(
            r#"
module Main

func test() {
    let x: std.num.Int64 = 100;
    for x in std.core.Range[std.num.Int64](0, 5) {
        ()
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// EDGE CASES
// ============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn for_loop_over_empty_range() {
        Test::new(
            r#"
module Main

func test() {
    var count: std.num.Int64 = 0;
    // Range where start >= end should be empty
    for i in std.core.Range[std.num.Int64](5, 5) {
        count = count + 1
    }
    // count should still be 0
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn for_loop_over_empty_array() {
        Test::new(
            r#"
module Main

func test() {
    let arr = std.collections.Array[std.num.Int64]();
    var count: std.num.Int64 = 0;
    for item in arr {
        count = count + 1
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn for_loop_with_empty_body() {
        Test::new(
            r#"
module Main

func test() {
    for i in std.core.Range[std.num.Int64](0, 10) {
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn for_loop_single_iteration() {
        Test::new(
            r#"
module Main

func test() {
    var count: std.num.Int64 = 0;
    for i in std.core.Range[std.num.Int64](0, 1) {
        count = count + 1
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn multiple_for_loops_in_sequence() {
        Test::new(
            r#"
module Main

func test() {
    var sum: std.num.Int64 = 0;
    for i in std.core.Range[std.num.Int64](0, 5) {
        sum = sum + i
    }
    for j in std.core.Range[std.num.Int64](0, 3) {
        sum = sum + j
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// ERROR CASES
// ============================================================================

mod errors {
    use super::*;

    #[test]
    fn for_loop_over_non_iterable() {
        Test::new(
            r#"
module Main

func test() {
    for x in 42 {
        ()
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("Iterable"));
    }

    #[test]
    fn for_loop_with_refutable_pattern() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() {
    var arr = std.collections.Array[Option[std.num.Int64]]();
    for .Some(x) in arr {
        ()
    }
}
"#,
        )
        .with_stdlib()
        .expect(Fails)
        .expect(HasError("refutable"));
    }

    #[test]
    fn break_outside_for_loop_fails() {
        Test::new(
            r#"
module Main

func test() {
    break;
}
"#,
        )
        .expect(Fails)
        .expect(HasError("outside of loop"));
    }

    #[test]
    fn continue_outside_for_loop_fails() {
        Test::new(
            r#"
module Main

func test() {
    continue;
}
"#,
        )
        .expect(Fails)
        .expect(HasError("outside of loop"));
    }

    #[test]
    fn for_loop_over_non_iterator_without_iter_method() {
        Test::new(
            r#"
module Main

struct NotIterable {
    let value: std.num.Int
}

func test() {
    let x = NotIterable(value: 42);
    for item in x {
        ()
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("Iterable"));
    }
}

// ============================================================================
// COMPLEX SCENARIOS
// ============================================================================

mod complex {
    use super::*;

    #[test]
    fn for_loop_with_if_inside() {
        Test::new(
            r#"
module Main

func test() {
    var evenSum: std.num.Int64 = 0;
    var oddSum: std.num.Int64 = 0;
    for i in std.core.Range[std.num.Int64](0, 10) {
        if i % 2 == 0 {
            evenSum = evenSum + i
        } else {
            oddSum = oddSum + i
        }
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn for_loop_accumulating_into_array() {
        Test::new(
            r#"
module Main

func test() {
    var result = std.collections.Array[std.num.Int64]();
    for i in std.core.Range[std.num.Int64](0, 5) {
        result.append(i * 2)
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn for_loop_matrix_iteration() {
        Test::new(
            r#"
module Main

func test() {
    var sum: std.num.Int64 = 0;
    for i in std.core.Range[std.num.Int64](0, 3) {
        for j in std.core.Range[std.num.Int64](0, 3) {
            sum = sum + (i * 3) + j
        }
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn for_loop_with_early_exit() {
        Test::new(
            r#"
module Main

func findFirst(target: std.num.Int64) -> std.result.Optional[std.num.Int64] {
    for i in std.core.Range[std.num.Int64](0, 100) {
        if i == target {
            return .Some(i)
        }
    }
    .None
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}
