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
    var current: Int
    var max: Int
}

func next(iter: Iterator) -> Option[Int] {
    if iter.current < iter.max {
        Option[Int].Some(value: iter.current)
    } else {
        Option[Int].None
    }
}

func test() {
    var iter = Iterator(current: 0, max: 10);
    while let .Some(item) = next(iter) {
        iter.current = iter.current + 1;
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

func test() -> Int {
    var sum = 0;
    var current: Option[Int] = .Some(value: 5);
    while let .Some(n) = current {
        sum = sum + n;
        if n > 0 {
            current = .Some(value: n - 1);
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

func test() -> Int {
    var result = 0;
    var opt: Option[Int] = .Some(value: 42);
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

func test() -> Int {
    var opt: Option[Int] = .None;
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

func test() -> Int {
    var sum = 0;
    let multiplier = 2;
    var opt: Option[Int] = .Some(value: 5);
    while let .Some(value) = opt {
        sum = sum + (value * multiplier);
        if value > 0 {
            opt = .Some(value: value - 1);
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

func test() -> Int {
    var count = 0;
    var opt: Option[Int] = .Some(value: 100);
    while let .Some(value) = opt {
        count = count + 1;
        if count > 5 {
            break
        }
        opt = .Some(value: value - 1);
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

func getOption(n: Int) -> Option[Int] {
    if n > 0 {
        Option[Int].Some(value: n)
    } else {
        Option[Int].None
    }
}

func test() -> Int {
    var sum = 0;
    var n = 10;
    while let .Some(value) = getOption(n) {
        n = n - 1;
        if value == 5 {
            continue
        }
        sum = sum + value;
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

func test() -> Int {
    var opt: Option[Int] = .Some(value: 42);
    while let .Some(value) = opt {
        if value > 40 {
            return value
        }
        opt = .Some(value: value - 1);
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

func test() -> Int {
    var sum = 0;
    var outer: Option[Int] = .Some(value: 3);
    while let .Some(i) = outer {
        var inner: Option[Int] = .Some(value: i);
        while let .Some(j) = inner {
            sum = sum + j;
            if j > 0 {
                inner = .Some(value: j - 1);
            } else {
                inner = .None;
            }
        }
        if i > 0 {
            outer = .Some(value: i - 1);
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

func test() -> Int {
    var sum = 0;
    var opt: Option[Int] = .Some(value: 10);
    while let .Some(n) = opt {
        sum = sum + n;
        if n > 0 {
            opt = .Some(value: n - 1);
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
