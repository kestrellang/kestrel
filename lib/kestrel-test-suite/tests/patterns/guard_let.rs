//! Tests for guard-let statements.
//!
//! These tests verify that:
//! - Guard-let syntax parses correctly
//! - Else block must diverge (return, break, continue, panic)
//! - Bindings are in scope after guard
//! - Guard-let works in various contexts

use kestrel_test_suite::*;

// ============================================================================
// BASIC GUARD-LET
// ============================================================================

mod basic {
    use super::*;

    #[test]
    fn guard_let_with_return() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> Int {
    guard let .Some(value) = opt else {
        return 0
    }
    value * 2
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn guard_let_multiple() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(a: Option[Int], b: Option[Int]) -> Int {
    guard let .Some(x) = a else {
        return 0
    }
    guard let .Some(y) = b else {
        return 0
    }
    x + y
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn guard_let_with_tuple() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[(Int, Int)]) -> Int {
    guard let .Some((a, b)) = opt else {
        return 0
    }
    a + b
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// ELSE BLOCK MUST DIVERGE
// ============================================================================

mod divergence {
    use super::*;

    #[test]
    fn guard_let_else_no_return_error() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> Int {
    guard let .Some(value) = opt else {
        0
    }
    value
}
"#,
        )
        .expect(Fails)
        .expect(HasError("diverge"));
    }

    #[test]
    #[ignore = "Requires bidirectional type inference for enum shorthands"]
    fn guard_let_with_break_in_loop() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opts: [Option[Int]]) -> Int {
    var sum = 0;
    var i = 0;
    while i < 10 {
        guard let .Some(value) = .Some(i) else {
            break
        }
        sum = sum + value;
        i = i + 1;
    }
    sum
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    #[ignore = "Requires bidirectional type inference for enum shorthands"]
    fn guard_let_with_continue_in_loop() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() -> Int {
    var sum = 0;
    var i = 0;
    while i < 10 {
        i = i + 1;
        guard let .Some(value) = if i > 5 { .Some(i) } else { .None } else {
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
}

// ============================================================================
// SCOPING
// ============================================================================

mod scoping {
    use super::*;

    #[test]
    fn binding_visible_after_guard() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> Int {
    guard let .Some(value) = opt else {
        return 0
    }
    value
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn binding_visible_to_later_guards() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(a: Option[Int], b: Option[Int]) -> Int {
    guard let .Some(x) = a else {
        return 0
    }
    guard let .Some(y) = b else {
        return x
    }
    x + y
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn binding_visible_in_final_expression() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> Int {
    guard let .Some(value) = opt else {
        return 0
    }
    let doubled = value * 2;
    doubled
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn binding_not_visible_in_else_block() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> Int {
    guard let .Some(value) = opt else {
        return value
    }
    value
}
"#,
        )
        .expect(Fails)
        .expect(HasError("undefined"));
    }
}

// ============================================================================
// TYPE INFERENCE
// ============================================================================

mod type_inference {
    use super::*;

    #[test]
    fn guard_let_infers_binding_type() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> Int {
    guard let .Some(x) = opt else {
        return 0
    }
    x + 1
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn guard_let_with_generic() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func unwrap[T](opt: Option[T], default: T) -> T {
    guard let .Some(value) = opt else {
        return default
    }
    value
}
"#,
        )
        .expect(Compiles);
    }
}
