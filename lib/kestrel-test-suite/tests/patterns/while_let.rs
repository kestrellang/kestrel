//! Tests for while-let expressions.
//!
//! These tests verify that:
//! - While-let syntax parses correctly
//! - Bindings are scoped to loop body
//! - Loop exits when pattern fails to match

use kestrel_test_suite::*;

// ============================================================================
// BASIC WHILE-LET
// ============================================================================

mod basic {
    use super::*;

    #[test]
    fn while_let_simple() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

struct Iterator {
    var current: lang.i64
    var max: lang.i64
}

func next(iter: Iterator) -> Option[lang.i64] {
    if lang.i64_signed_lt(iter.current, iter.max) {
        Option[lang.i64].Some(value: iter.current)
    } else {
        Option[lang.i64].None
    }
}

func test() {
    var iter = Iterator(current: 0, max: 10);
    while let .Some(item) = next(iter) {
        iter.current = lang.i64_add(iter.current, 1);
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn while_let_accumulate() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() -> lang.i64 {
    var sum: lang.i64 = 0;
    var current: Option[lang.i64] = .Some(value: 5);
    while let .Some(n) = current {
        sum = lang.i64_add(sum, n);
        if lang.i64_signed_gt(n, 0) {
            current = .Some(value: lang.i64_sub(n, 1));
        } else {
            current = .None;
        }
    }
    sum
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// TYPE OPERATOR (T?)
// ============================================================================

mod type_operator {
    use super::*;

    #[test]
    fn while_let_optional_type_operator() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    var opt: std.num.Int64? = .Some(1);
    var seen: lang.i64 = 0;
    while let .Some(_v) = opt {
        seen = 1;
        opt = .None;
    }
    seen
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

enum Option[T] {
    case Some(value: T)
    case None
}

func test() -> lang.i64 {
    var result = 0;
    var opt: Option[lang.i64] = .Some(value: 42);
    while let .Some(value) = opt {
        result = value;
        opt = .None;
    }
    result
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn binding_not_visible_after_loop() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() -> lang.i64 {
    var opt: Option[lang.i64] = .None;
    while let .Some(value) = opt {
        opt = .None;
    }
    value
}
"#,
        )
        .expect(Fails)
        .expect(HasError("undefined"));
    }

    #[test]
    fn outer_variables_accessible() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() -> lang.i64 {
    var sum: lang.i64 = 0;
    let multiplier: lang.i64 = 2;
    var opt: Option[lang.i64] = .Some(value: 5);
    while let .Some(value) = opt {
        sum = lang.i64_add(sum, lang.i64_mul(value, multiplier));
        if lang.i64_signed_gt(value, 0) {
            opt = .Some(value: lang.i64_sub(value, 1));
        } else {
            opt = .None;
        }
    }
    sum
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// CONTROL FLOW
// ============================================================================

mod control_flow {
    use super::*;

    #[test]
    fn while_let_with_break() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() -> lang.i64 {
    var count: lang.i64 = 0;
    var opt: Option[lang.i64] = .Some(value: 100);
    while let .Some(value) = opt {
        count = lang.i64_add(count, 1);
        if lang.i64_signed_gt(count, 5) {
            break
        }
        opt = .Some(value: lang.i64_sub(value, 1));
    }
    count
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn while_let_with_continue() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func getOption(n: lang.i64) -> Option[lang.i64] {
    if lang.i64_signed_gt(n, 0) {
        Option[lang.i64].Some(value: n)
    } else {
        Option[lang.i64].None
    }
}

func test() -> lang.i64 {
    var sum: lang.i64 = 0;
    var n: lang.i64 = 10;
    while let .Some(value) = getOption(n) {
        n = lang.i64_sub(n, 1);
        if lang.i64_eq(value, 5) {
            continue
        }
        sum = lang.i64_add(sum, value);
    }
    sum
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn while_let_with_return() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() -> lang.i64 {
    var opt: Option[lang.i64] = .Some(value: 42);
    while let .Some(value) = opt {
        if lang.i64_signed_gt(value, 40) {
            return value
        }
        opt = .Some(value: lang.i64_sub(value, 1));
    }
    0
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn nested_while_let() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() -> lang.i64 {
    var sum: lang.i64 = 0;
    var outer: Option[lang.i64] = .Some(value: 3);
    while let .Some(i) = outer {
        var inner: Option[lang.i64] = .Some(value: i);
        while let .Some(j) = inner {
            sum = lang.i64_add(sum, j);
            if lang.i64_signed_gt(j, 0) {
                inner = .Some(value: lang.i64_sub(j, 1));
            } else {
                inner = .None;
            }
        }
        if lang.i64_signed_gt(i, 0) {
            outer = .Some(value: lang.i64_sub(i, 1));
        } else {
            outer = .None;
        }
    }
    sum
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// WHILE-LET CHAINS
// ============================================================================

mod chains {
    use super::*;

    #[test]
    fn while_let_chain_two_patterns() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() {
    var a: Option[lang.i64] = Option.Some(value: 1);
    var b: Option[lang.i64] = Option.Some(value: 2);
    while let .Some(x) = a, let .Some(y) = b {
        let _ = lang.i64_add(x, y);
        a = Option[lang.i64].None;
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn while_let_chain_with_bool_condition() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() {
    var opt: Option[lang.i64] = Option.Some(value: 5);
    while let .Some(x) = opt, lang.i64_signed_gt(x, 0) {
        opt = Option.Some(value: lang.i64_sub(x, 1));
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn while_let_chain_binding_visible_in_later_conditions() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() {
    var a: Option[lang.i64] = Option.Some(value: 10);
    var b: Option[lang.i64] = Option.Some(value: 5);
    while let .Some(x) = a, let .Some(y) = b, lang.i64_signed_gt(x, y) {
        let _ = lang.i64_sub(x, y);
        a = Option.Some(value: lang.i64_sub(x, 1));
    }
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// TYPE INFERENCE
// ============================================================================

mod type_inference {
    use super::*;

    #[test]
    fn while_let_infers_binding_type() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() -> lang.i64 {
    var sum: lang.i64 = 0;
    var opt: Option[lang.i64] = .Some(value: 10);
    while let .Some(n) = opt {
        sum = lang.i64_add(sum, n);
        if lang.i64_signed_gt(n, 0) {
            opt = .Some(value: lang.i64_sub(n, 1));
        } else {
            opt = .None;
        }
    }
    sum
}
"#,
        )
        .expect(Compiles);
    }
}
