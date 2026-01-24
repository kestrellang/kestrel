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

func test(opt: Option[lang.i64]) -> lang.i64 {
    guard let .Some(value) = opt else {
        return 0
    }
    lang.i64_mul(value, 2)
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

func test(a: Option[lang.i64], b: Option[lang.i64]) -> lang.i64 {
    guard let .Some(x) = a else {
        return 0
    }
    guard let .Some(y) = b else {
        return 0
    }
    lang.i64_add(x, y)
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

func test(opt: Option[(lang.i64, lang.i64)]) -> lang.i64 {
    guard let .Some((a, b)) = opt else {
        return 0
    }
    lang.i64_add(a, b)
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
    fn guard_let_optional_type_operator() {
        Test::new(
            r#"
module Main

func test(opt: std.num.Int64?) -> lang.i64 {
    guard let .Some(_v) = opt else {
        return 0
    }
    1
}
"#,
        )
        .with_stdlib()
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

func test(opt: Option[lang.i64]) -> lang.i64 {
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
    fn guard_let_with_break_in_loop() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opts: [Option[lang.i64]]) -> lang.i64 {
    var sum: lang.i64 = 0;
    var i: lang.i64 = 0;
    while lang.i64_signed_lt(i, 10) {
        guard let .Some(value) = Option.Some(value: i) else {
            break
        }
        sum = lang.i64_add(sum, value);
        i = lang.i64_add(i, 1);
    }
    sum
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn guard_let_with_continue_in_loop() {
        // Test that continue in guard-let else block works in a loop
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() -> lang.i64 {
    var sum: lang.i64 = 0;
    var i: lang.i64 = 0;
    while lang.i64_signed_lt(i, 10) {
        i = lang.i64_add(i, 1);
        // Skip odd numbers using guard-let with continue
        guard let .Some(value) = if lang.i64_eq(lang.i64_signed_rem(i, 2), 0) { Option[lang.i64].Some(value: i) } else { Option[lang.i64].None } else {
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

func test(opt: Option[lang.i64]) -> lang.i64 {
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

func test(a: Option[lang.i64], b: Option[lang.i64]) -> lang.i64 {
    guard let .Some(x) = a else {
        return 0
    }
    guard let .Some(y) = b else {
        return x
    }
    lang.i64_add(x, y)
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

func test(opt: Option[lang.i64]) -> lang.i64 {
    guard let .Some(value) = opt else {
        return 0
    }
    let doubled = lang.i64_mul(value, 2);
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

func test(opt: Option[lang.i64]) -> lang.i64 {
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
// GUARD-LET CHAINS
// ============================================================================

mod chains {
    use super::*;

    #[test]
    fn guard_let_chain_two_patterns() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(a: Option[lang.i64], b: Option[lang.i64]) -> lang.i64 {
    guard let .Some(x) = a, let .Some(y) = b else {
        return 0
    }
    lang.i64_add(x, y)
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn guard_let_chain_with_bool_condition() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) -> lang.i64 {
    guard let .Some(x) = opt, lang.i64_signed_gt(x, 0) else {
        return 0
    }
    lang.i64_mul(x, 2)
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn guard_let_chain_binding_visible_in_later_conditions() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(a: Option[lang.i64], b: Option[lang.i64]) -> lang.i64 {
    guard let .Some(x) = a, let .Some(y) = b, lang.i64_signed_lt(x, y) else {
        return 0
    }
    lang.i64_sub(y, x)
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
    fn guard_let_infers_binding_type() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) -> lang.i64 {
    guard let .Some(x) = opt else {
        return 0
    }
    lang.i64_add(x, 1)
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
