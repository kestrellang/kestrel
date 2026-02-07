//! Tests for short-circuit evaluation of `and` and `or` operators.
//!
//! These tests verify that:
//! - `and` and `or` operators return correct values
//! - `and` and `or` operators short-circuit correctly (RHS not evaluated when unnecessary)
//! - Chained operators work correctly
//! - Mixed operators respect precedence
//!
//! These tests use the stdlib and verify actual runtime behavior via stdout.

use kestrel_test_suite::*;

// ============================================================================
// BASIC AND - VALUE VERIFICATION
// ============================================================================

mod basic_and {
    use super::*;

    #[test]
    fn true_and_true_returns_true() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(true and true);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn true_and_false_returns_false() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(true and false);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("false\n"));
    }

    #[test]
    fn false_and_true_returns_false() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(false and true);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("false\n"));
    }

    #[test]
    fn false_and_false_returns_false() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(false and false);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("false\n"));
    }
}

// ============================================================================
// BASIC OR - VALUE VERIFICATION
// ============================================================================

mod basic_or {
    use super::*;

    #[test]
    fn true_or_true_returns_true() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(true or true);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn true_or_false_returns_true() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(true or false);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn false_or_true_returns_true() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(false or true);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn false_or_false_returns_false() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(false or false);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("false\n"));
    }
}

// ============================================================================
// SHORT-CIRCUIT BEHAVIOR VERIFICATION
// These tests print from the RHS to prove short-circuiting.
// If the RHS is NOT evaluated, its print won't appear in output.
// ============================================================================

mod short_circuit_verification {
    use super::*;

    #[test]
    fn and_with_false_lhs_does_not_evaluate_rhs() {
        // When LHS is false, RHS should NOT be evaluated
        // If short-circuit works, "RHS" will NOT be printed
        Test::new(
            r#"
module Main
import std.io.stdio.println

func sideEffect() -> Bool {
    let _ = println("RHS");
    true
}

func main() -> lang.i64 {
    let result = false and sideEffect();
    let _ = println(result);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("false\n"));
    }

    #[test]
    fn and_with_true_lhs_evaluates_rhs() {
        // When LHS is true, RHS should be evaluated
        // "RHS" WILL be printed
        Test::new(
            r#"
module Main
import std.io.stdio.println

func sideEffect() -> Bool {
    let _ = println("RHS");
    true
}

func main() -> lang.i64 {
    let result = true and sideEffect();
    let _ = println(result);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("RHS\ntrue\n"));
    }

    #[test]
    fn or_with_true_lhs_does_not_evaluate_rhs() {
        // When LHS is true, RHS should NOT be evaluated
        // If short-circuit works, "RHS" will NOT be printed
        Test::new(
            r#"
module Main
import std.io.stdio.println

func sideEffect() -> Bool {
    let _ = println("RHS");
    false
}

func main() -> lang.i64 {
    let result = true or sideEffect();
    let _ = println(result);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn or_with_false_lhs_evaluates_rhs() {
        // When LHS is false, RHS should be evaluated
        // "RHS" WILL be printed
        Test::new(
            r#"
module Main
import std.io.stdio.println

func sideEffect() -> Bool {
    let _ = println("RHS");
    true
}

func main() -> lang.i64 {
    let result = false or sideEffect();
    let _ = println(result);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("RHS\ntrue\n"));
    }
}

// ============================================================================
// CHAINED AND
// ============================================================================

mod chained_and {
    use super::*;

    #[test]
    fn all_true() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(true and true and true);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn first_false_short_circuits() {
        // false and b and c - should short-circuit immediately
        // Neither "B" nor "C" should be printed
        Test::new(
            r#"
module Main
import std.io.stdio.println

func effectB() -> Bool {
    let _ = println("B");
    true
}

func effectC() -> Bool {
    let _ = println("C");
    true
}

func main() -> lang.i64 {
    let result = false and effectB() and effectC();
    let _ = println(result);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("false\n"));
    }

    #[test]
    fn middle_false_short_circuits() {
        // true and false and c - should short-circuit after second
        // "C" should NOT be printed
        Test::new(
            r#"
module Main
import std.io.stdio.println

func effectC() -> Bool {
    let _ = println("C");
    true
}

func main() -> lang.i64 {
    let result = true and false and effectC();
    let _ = println(result);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("false\n"));
    }

    #[test]
    fn last_false() {
        // true and true and false - evaluates all
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(true and true and false);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("false\n"));
    }

    #[test]
    fn all_evaluated_when_all_true_until_last() {
        // Each step should print when evaluated
        Test::new(
            r#"
module Main
import std.io.stdio.println

func effectA() -> Bool {
    let _ = println("A");
    true
}

func effectB() -> Bool {
    let _ = println("B");
    true
}

func effectC() -> Bool {
    let _ = println("C");
    true
}

func main() -> lang.i64 {
    let result = effectA() and effectB() and effectC();
    let _ = println(result);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("A\nB\nC\ntrue\n"));
    }
}

// ============================================================================
// CHAINED OR
// ============================================================================

mod chained_or {
    use super::*;

    #[test]
    fn all_false() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(false or false or false);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("false\n"));
    }

    #[test]
    fn first_true_short_circuits() {
        // true or b or c - should short-circuit immediately
        // Neither "B" nor "C" should be printed
        Test::new(
            r#"
module Main
import std.io.stdio.println

func effectB() -> Bool {
    let _ = println("B");
    false
}

func effectC() -> Bool {
    let _ = println("C");
    false
}

func main() -> lang.i64 {
    let result = true or effectB() or effectC();
    let _ = println(result);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn middle_true_short_circuits() {
        // false or true or c - should short-circuit after second
        // "C" should NOT be printed
        Test::new(
            r#"
module Main
import std.io.stdio.println

func effectC() -> Bool {
    let _ = println("C");
    false
}

func main() -> lang.i64 {
    let result = false or true or effectC();
    let _ = println(result);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn last_true() {
        // false or false or true - evaluates all
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(false or false or true);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn all_evaluated_when_all_false_until_last() {
        // Each step should print when evaluated
        Test::new(
            r#"
module Main
import std.io.stdio.println

func effectA() -> Bool {
    let _ = println("A");
    false
}

func effectB() -> Bool {
    let _ = println("B");
    false
}

func effectC() -> Bool {
    let _ = println("C");
    true
}

func main() -> lang.i64 {
    let result = effectA() or effectB() or effectC();
    let _ = println(result);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("A\nB\nC\ntrue\n"));
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
        // true or (false and false) = true or false = true
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(true or false and false);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn and_binds_tighter_complex() {
        // a and b or c and d parses as (a and b) or (c and d)
        // (true and false) or (false and true) = false or false = false
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(true and false or false and true);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("false\n"));
    }

    #[test]
    fn and_binds_tighter_true_result() {
        // (true and true) or (false and false) = true or false = true
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(true and true or false and false);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn parenthesized_or_in_and() {
        // true and (false or true) = true and true = true
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(true and (false or true));
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn parenthesized_and_in_or() {
        // (true and false) or true = false or true = true
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println((true and false) or true);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn mixed_short_circuit_or_first() {
        // true or (effect and effect) should short-circuit
        // Neither effect should print
        Test::new(
            r#"
module Main
import std.io.stdio.println

func effect() -> Bool {
    let _ = println("EFFECT");
    true
}

func main() -> lang.i64 {
    let result = true or effect() and effect();
    let _ = println(result);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }
}

// ============================================================================
// WITH NOT OPERATOR
// ============================================================================

mod with_not {
    use super::*;

    #[test]
    fn not_true_and_true() {
        // not true and true = false and true = false
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(not true and true);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("false\n"));
    }

    #[test]
    fn not_false_or_false() {
        // not false or false = true or false = true
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(not false or false);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn true_and_not_false() {
        // true and not false = true and true = true
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(true and not false);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn false_or_not_false() {
        // false or not false = false or true = true
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(false or not false);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn not_with_short_circuit() {
        // (not false) = true, which should short-circuit the or
        // "EFFECT" should NOT be printed
        Test::new(
            r#"
module Main
import std.io.stdio.println

func effect() -> Bool {
    let _ = println("EFFECT");
    true
}

func main() -> lang.i64 {
    let result = not false or effect();
    let _ = println(result);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn not_precedence_higher_than_or() {
        // Verify that `not a or b` parses as `(not a) or b` (like Rust/Swift)
        // (not false) or true = true or true = true
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(not false or true);  // (not false) or true = true or true = true
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }
}

// ============================================================================
// WITH COMPARISON OPERATORS
// ============================================================================

mod with_comparisons {
    use super::*;

    #[test]
    fn comparison_and_comparison() {
        // 1 < 2 and 3 > 2 = true and true = true
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(1 < 2 and 3 > 2);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn comparison_or_comparison() {
        // 1 > 2 or 3 > 2 = false or true = true
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(1 > 2 or 3 > 2);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn comparison_short_circuits() {
        // 5 > 10 is false, should short-circuit the and
        // "EXPENSIVE" should NOT be printed
        Test::new(
            r#"
module Main
import std.io.stdio.println

func expensiveCheck() -> Bool {
    let _ = println("EXPENSIVE");
    true
}

func main() -> lang.i64 {
    let result = 5 > 10 and expensiveCheck();
    let _ = println(result);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("false\n"));
    }

    #[test]
    fn equality_and_inequality() {
        // 1 == 1 and 2 != 3 = true and true = true
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println(1 == 1 and 2 != 3);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }
}

// ============================================================================
// WITH FUNCTION CALLS
// ============================================================================

mod with_function_calls {
    use super::*;

    #[test]
    fn function_and_function() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func isPositive(n: Int) -> Bool {
    n > 0
}

func isEven(n: Int) -> Bool {
    n % 2 == 0
}

func main() -> lang.i64 {
    let _ = println(isPositive(4) and isEven(4));  // true and true
    let _ = println(isPositive(-1) and isEven(4)); // false and true (short-circuits)
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\nfalse\n"));
    }

    #[test]
    fn function_or_function() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func isZero(n: Int) -> Bool {
    n == 0
}

func isNegative(n: Int) -> Bool {
    n < 0
}

func main() -> lang.i64 {
    let _ = println(isZero(0) or isNegative(0));   // true or false (short-circuits)
    let _ = println(isZero(5) or isNegative(-3));  // false or true
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\ntrue\n"));
    }

    #[test]
    fn function_call_short_circuit_and() {
        // false from alwaysFalse should short-circuit, "TRUE" should NOT print
        Test::new(
            r#"
module Main
import std.io.stdio.println

func alwaysFalse() -> Bool {
    let _ = println("FALSE");
    false
}

func alwaysTrue() -> Bool {
    let _ = println("TRUE");
    true
}

func main() -> lang.i64 {
    let r = alwaysFalse() and alwaysTrue();
    let _ = println(r);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("FALSE\nfalse\n"));
    }

    #[test]
    fn function_call_short_circuit_or() {
        // true from alwaysTrue should short-circuit, "FALSE" should NOT print
        Test::new(
            r#"
module Main
import std.io.stdio.println

func alwaysFalse() -> Bool {
    let _ = println("FALSE");
    false
}

func alwaysTrue() -> Bool {
    let _ = println("TRUE");
    true
}

func main() -> lang.i64 {
    let r = alwaysTrue() or alwaysFalse();
    let _ = println(r);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("TRUE\ntrue\n"));
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
import std.io.stdio.println

func main() -> lang.i64 {
    if true and true {
        let _ = println("both true");
    } else {
        let _ = println("not both");
    }

    if true and false {
        let _ = println("both true");
    } else {
        let _ = println("not both");
    }
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("both true\nnot both\n"));
    }

    #[test]
    fn or_in_if_condition() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    if true or false {
        let _ = println("at least one");
    } else {
        let _ = println("neither");
    }

    if false or false {
        let _ = println("at least one");
    } else {
        let _ = println("neither");
    }
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("at least one\nneither\n"));
    }

    #[test]
    fn short_circuit_in_if_condition() {
        // "EFFECT" should NOT be printed in either case
        Test::new(
            r#"
module Main
import std.io.stdio.println

func effect() -> Bool {
    let _ = println("EFFECT");
    true
}

func main() -> lang.i64 {
    // false and should not call effect
    if false and effect() {
        let _ = println("yes");
    } else {
        let _ = println("no");
    }

    // true or should not call effect
    if true or effect() {
        let _ = println("yes");
    } else {
        let _ = println("no");
    }
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("no\nyes\n"));
    }

    #[test]
    fn and_in_while_condition() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    var i: Int = 0;
    var sum: Int = 0;
    while i < 5 and sum < 6 {
        sum = sum + i;
        i = i + 1;
    }
    let _ = println(i);
    let _ = println(sum);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("4\n6\n"));
    }

    #[test]
    fn or_in_while_condition() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    var a: Int = 0;
    var b: Int = 10;
    var count: Int = 0;
    // Loop while a < 3 OR b > 8 (exits when both are false)
    while a < 3 or b > 8 {
        a = a + 1;
        b = b - 1;
        count = count + 1;
    }
    let _ = println(count);  // 3: after 3 iterations, a=3 (not <3) and b=7 (not >8)
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("3\n"));
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
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println((true and (true and (true and true))));
    let _ = println((true and (true and (true and false))));
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\nfalse\n"));
    }

    #[test]
    fn deeply_nested_or() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println((false or (false or (false or true))));
    let _ = println((false or (false or (false or false))));
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\nfalse\n"));
    }

    #[test]
    fn complex_nested_mixed() {
        // (true and false) or (false and true) or (true and true)
        // = false or false or true = true
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let _ = println((true and false) or (false and true) or (true and true));
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn nested_short_circuit() {
        // (false and (effect() and effect())) short-circuits, then or true
        // No "EFFECT" should print
        Test::new(
            r#"
module Main
import std.io.stdio.println

func effect() -> Bool {
    let _ = println("EFFECT");
    true
}

func main() -> lang.i64 {
    let result = (false and (effect() and effect())) or true;
    let _ = println(result);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }
}

// ============================================================================
// WITH VARIABLES AND STRUCT FIELDS
// ============================================================================

mod with_variables {
    use super::*;

    #[test]
    fn and_with_variables() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let a = true;
    let b = false;
    let _ = println(a and b);
    let _ = println(a and a);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("false\ntrue\n"));
    }

    #[test]
    fn or_with_variables() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let a = true;
    let b = false;
    let _ = println(a or b);
    let _ = println(b or b);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\nfalse\n"));
    }

    #[test]
    fn and_with_struct_fields() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

struct Flags {
    let enabled: Bool
    let visible: Bool
}

func main() -> lang.i64 {
    let f1 = Flags(enabled: true, visible: true);
    let f2 = Flags(enabled: true, visible: false);
    let _ = println(f1.enabled and f1.visible);
    let _ = println(f2.enabled and f2.visible);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\nfalse\n"));
    }

    #[test]
    fn or_with_struct_fields() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

struct State {
    let loading: Bool
    let error: Bool
}

func main() -> lang.i64 {
    let s1 = State(loading: false, error: false);
    let s2 = State(loading: true, error: false);
    let _ = println(s1.loading or s1.error);
    let _ = println(s2.loading or s2.error);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("false\ntrue\n"));
    }
}
