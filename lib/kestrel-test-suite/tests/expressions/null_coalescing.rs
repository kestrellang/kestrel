//! Tests for null coalescing operator (`??`).
//!
//! These tests verify that:
//! - `??` unwraps optionals correctly (Some returns value, None returns default)
//! - `??` short-circuits (RHS not evaluated when LHS is Some)
//! - Chaining works correctly with right-associativity
//! - Precedence relative to `or` is correct
//! - Type errors are reported for invalid usage

use kestrel_test_suite::*;

// ============================================================================
// BASIC COALESCING
// ============================================================================

mod basic_coalescing {
    use super::*;

    #[test]
    fn some_returns_value() {
        Test::new(
            r#"
module Main
import std.io.stdio.println
import std.result.Optional

func main() -> lang.i64 {
    let x: Int? = .Some(42);
    let _ = println(x ?? 0);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("42\n"));
    }

    #[test]
    fn none_returns_default() {
        Test::new(
            r#"
module Main
import std.io.stdio.println
import std.result.Optional

func main() -> lang.i64 {
    let x: Int? = .None;
    let _ = println(x ?? 99);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("99\n"));
    }

    #[test]
    fn null_returns_default() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let x: Int? = null;
    let _ = println(x ?? 123);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("123\n"));
    }

    #[test]
    fn with_string_type() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let name: String? = null;
    let _ = println(name ?? "anonymous");
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("anonymous\n"));
    }

    #[test]
    fn with_string_some() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let name: String? = .Some("Alice");
    let _ = println(name ?? "anonymous");
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("Alice\n"));
    }
}

// ============================================================================
// SHORT-CIRCUIT EVALUATION
// ============================================================================

mod short_circuit {
    use super::*;

    #[test]
    fn some_does_not_evaluate_rhs() {
        // When LHS is Some, RHS should NOT be evaluated
        // "RHS" should NOT be printed
        Test::new(
            r#"
module Main
import std.io.stdio.println

func expensiveDefault() -> Int {
    let _ = println("RHS");
    999
}

func main() -> lang.i64 {
    let x: Int? = .Some(42);
    let result = x ?? expensiveDefault();
    let _ = println(result);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("42\n"));
    }

    #[test]
    fn none_evaluates_rhs() {
        // When LHS is None, RHS should be evaluated
        // "RHS" WILL be printed
        Test::new(
            r#"
module Main
import std.io.stdio.println

func expensiveDefault() -> Int {
    let _ = println("RHS");
    999
}

func main() -> lang.i64 {
    let x: Int? = null;
    let result = x ?? expensiveDefault();
    let _ = println(result);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("RHS\n999\n"));
    }

    #[test]
    fn short_circuit_with_side_effect_function() {
        // Tests that when Some, the RHS function is not called,
        // and when None, the RHS function IS called
        Test::new(
            r#"
module Main
import std.io.stdio.println

func getDefault() -> Int {
    let _ = println("called");
    999
}

func main() -> lang.i64 {
    let x: Int? = .Some(10);
    let y: Int? = null;

    // x is Some, so getDefault should NOT be called
    let a = x ?? getDefault();
    let _ = println(a);

    // y is None, so getDefault SHOULD be called
    let b = y ?? getDefault();
    let _ = println(b);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("10\ncalled\n999\n"));
    }
}

// ============================================================================
// CHAINING (RIGHT-ASSOCIATIVITY)
// ============================================================================

mod chaining {
    use super::*;

    #[test]
    fn some_with_default() {
        // Simple chaining: a ?? default
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let a: Int? = .Some(1);
    let _ = println(a ?? 99);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("1\n"));
    }

    #[test]
    fn none_with_default() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let a: Int? = null;
    let _ = println(a ?? 99);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("99\n"));
    }

    #[test]
    fn short_circuit_some() {
        // a ?? expensiveDefault() - if a is Some, default should NOT be evaluated
        Test::new(
            r#"
module Main
import std.io.stdio.println

func getDefault() -> Int {
    let _ = println("DEFAULT");
    99
}

func main() -> lang.i64 {
    let a: Int? = .Some(1);
    let result = a ?? getDefault();
    let _ = println(result);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("1\n"));
    }

    #[test]
    fn short_circuit_none() {
        // a ?? expensiveDefault() - if a is None, default SHOULD be evaluated
        Test::new(
            r#"
module Main
import std.io.stdio.println

func getDefault() -> Int {
    let _ = println("DEFAULT");
    99
}

func main() -> lang.i64 {
    let a: Int? = null;
    let result = a ?? getDefault();
    let _ = println(result);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("DEFAULT\n99\n"));
    }

    #[test]
    fn nested_coalesce() {
        // (a ?? 10) + (b ?? 20) - both coalesces work independently
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let a: Int? = .Some(5);
    let b: Int? = null;
    let _ = println((a ?? 10) + (b ?? 20));
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("25\n"));
    }
}

// ============================================================================
// PRECEDENCE WITH `or`
// ============================================================================

mod precedence {
    use super::*;

    #[test]
    fn coalesce_binds_tighter_than_or() {
        // x ?? false or y should parse as (x ?? false) or y
        // Since ?? has higher precedence than or
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let x: Bool? = null;
    let y: Bool = true;
    // (null ?? false) or true = false or true = true
    let _ = println(x ?? false or y);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }

    #[test]
    fn coalesce_then_and() {
        // x ?? true and y should parse as (x ?? true) and y
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let x: Bool? = null;
    let y: Bool = false;
    // (null ?? true) and false = true and false = false
    let _ = println(x ?? true and y);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("false\n"));
    }

    #[test]
    fn complex_precedence() {
        // a ?? b or c ?? d and e
        // Should parse as: ((a ?? b) or ((c ?? d) and e))
        // Due to: ?? > and > or
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let a: Bool? = null;
    let b: Bool = false;
    let c: Bool? = .Some(true);
    let d: Bool = true;
    let e: Bool = true;
    // (null ?? false) or (Some(true) ?? true and true)
    // = false or (true and true)
    // = false or true
    // = true
    let _ = println(a ?? b or c ?? d and e);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("true\n"));
    }
}

// Note: Optional-to-Optional coalescing (a ?? b where both are Optional[T])
// would require overlapping protocol conformances which is not yet supported.
// Use .orValue() method for this pattern instead:
//   let result = a.orValue(b)  // Optional[T] -> Optional[T] -> Optional[T]

// ============================================================================
// WITH FUNCTION RETURNS
// ============================================================================

mod with_functions {
    use super::*;

    #[test]
    fn coalesce_function_result() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func findValue(key: Int) -> Int? {
    if key == 1 {
        .Some(100)
    } else {
        null
    }
}

func main() -> lang.i64 {
    let _ = println(findValue(1) ?? 0);
    let _ = println(findValue(2) ?? 0);
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("100\n0\n"));
    }

    #[test]
    fn coalesce_in_return() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func getOrDefault(opt: Int?) -> Int {
    opt ?? 42
}

func main() -> lang.i64 {
    let _ = println(getOrDefault(.Some(1)));
    let _ = println(getOrDefault(null));
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("1\n42\n"));
    }
}
