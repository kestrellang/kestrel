//! Tests for if-let expressions.
//!
//! These tests verify that:
//! - If-let syntax parses correctly
//! - Bindings are scoped to then-branch
//! - If-let chains work
//! - Else branches work

use kestrel_test_suite::*;

// ============================================================================
// BASIC IF-LET
// ============================================================================

mod basic {
    use super::*;

    #[test]
    fn if_let_simple() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) -> lang.i64 {
    if let .Some(value) = opt {
        value
    } else {
        0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn if_let_without_else() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) {
    if let .Some(value) = opt {
        let _ = value;
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn if_let_with_else_if() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(a: Option[lang.i64], b: Option[lang.i64]) -> lang.i64 {
    if let .Some(x) = a {
        x
    } else if let .Some(y) = b {
        y
    } else {
        0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn if_let_nested() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Option[lang.i64]]) -> lang.i64 {
    if let .Some(inner) = opt {
        if let .Some(value) = inner {
            value
        } else {
            0
        }
    } else {
        0
    }
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// IF-LET CHAINS
// ============================================================================

mod chains {
    use super::*;

    #[test]
    fn if_let_chain_two_patterns() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(a: Option[lang.i64], b: Option[lang.i64]) -> lang.i64 {
    if let .Some(x) = a, let .Some(y) = b {
        x + y
    } else {
        0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn if_let_chain_with_bool_condition() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) -> lang.i64 {
    if let .Some(x) = opt, x > 0 {
        x
    } else {
        0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn if_let_chain_binding_visible_in_later_conditions() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(a: Option[lang.i64], b: Option[lang.i64]) -> lang.i64 {
    if let .Some(x) = a, let .Some(y) = b, x < y {
        y - x
    } else {
        0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn if_let_chain_three_conditions() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(a: Option[lang.i64], b: Option[lang.i64], c: Option[lang.i64]) -> lang.i64 {
    if let .Some(x) = a, let .Some(y) = b, let .Some(z) = c {
        x + y + z
    } else {
        0
    }
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
    fn binding_not_visible_in_else() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) -> lang.i64 {
    if let .Some(value) = opt {
        value
    } else {
        value
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("undefined"));
    }

    #[test]
    fn binding_not_visible_after_if_let() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) -> lang.i64 {
    if let .Some(value) = opt {
        value
    } else {
        0
    }
    value
}
"#,
        )
        .expect(Fails)
        .expect(HasError("undefined"));
    }

    #[test]
    fn chain_bindings_visible_in_body() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(a: Option[lang.i64], b: Option[lang.i64]) -> lang.i64 {
    if let .Some(x) = a, let .Some(y) = b {
        x + y
    } else {
        0
    }
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// WARNINGS
// ============================================================================

mod warnings {
    use super::*;

    #[test]
    fn irrefutable_if_let_warning() {
        Test::new(
            r#"
module Main

func test(t: (lang.i64, lang.i64)) -> lang.i64 {
    if let (a, b) = t {
        a + b
    } else {
        0
    }
}
"#,
        )
        .expect(HasWarning("irrefutable"));
    }

    #[test]
    fn irrefutable_binding_pattern_warning() {
        Test::new(
            r#"
module Main

func test(x: lang.i64) -> lang.i64 {
    if let y = x {
        y
    } else {
        0
    }
}
"#,
        )
        .expect(HasWarning("irrefutable"));
    }
}

// ============================================================================
// TYPE INFERENCE
// ============================================================================

mod type_inference {
    use super::*;

    #[test]
    fn if_let_infers_binding_type() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) -> lang.i64 {
    if let .Some(x) = opt {
        x + 1
    } else {
        0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn if_let_branches_same_type() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) -> lang.i64 {
    if let .Some(x) = opt {
        x
    } else {
        "zero"
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("type"));
    }
}
