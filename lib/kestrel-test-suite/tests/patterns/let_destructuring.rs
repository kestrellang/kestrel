//! Tests for let/var destructuring patterns.
//!
//! These tests verify that:
//! - Let/var bindings accept patterns
//! - Patterns are irrefutable (always match)
//! - Refutable patterns cause errors
//! - Type annotations work with patterns

use kestrel_test_suite::*;

// ============================================================================
// TUPLE DESTRUCTURING
// ============================================================================

mod tuple_destructuring {
    use super::*;

    #[test]
    fn basic_tuple_destructure() {
        Test::new(
            r#"
module Main

func test() -> Int {
    let (a, b) = (1, 2);
    a + b
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn nested_tuple_destructure() {
        Test::new(
            r#"
module Main

func test() -> Int {
    let ((a, b), c) = ((1, 2), 3);
    a + b + c
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn tuple_with_wildcard() {
        Test::new(
            r#"
module Main

func test() -> Int {
    let (a, _) = (1, 2);
    a
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn tuple_with_type_annotation() {
        Test::new(
            r#"
module Main

func test() -> Int {
    let (a, b): (Int, Int) = (1, 2);
    a + b
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn tuple_destructure_from_function() {
        Test::new(
            r#"
module Main

func pair() -> (Int, Int) {
    (1, 2)
}

func test() -> Int {
    let (a, b) = pair();
    a + b
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn var_tuple_destructure_mutable() {
        Test::new(
            r#"
module Main

func test() -> Int {
    var (a, b) = (1, 2);
    a = 10;
    b = 20;
    a + b
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn mixed_mutability_in_tuple() {
        Test::new(
            r#"
module Main

func test() -> Int {
    let (var a, b) = (1, 2);
    a = 10;
    a + b
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn immutable_element_cannot_assign() {
        Test::new(
            r#"
module Main

func test() -> Int {
    let (var a, b) = (1, 2);
    b = 20;
    a + b
}
"#,
        )
        .expect(Fails)
        .expect(HasError("immutable"));
    }

    #[test]
    fn tuple_arity_mismatch() {
        Test::new(
            r#"
module Main

func test() -> Int {
    let (a, b, c) = (1, 2);
    a + b + c
}
"#,
        )
        .expect(Fails);
    }
}

// ============================================================================
// STRUCT DESTRUCTURING
// ============================================================================

mod struct_destructuring {
    use super::*;

    #[test]
    fn basic_struct_destructure() {
        Test::new(
            r#"
module Main

struct Point {
    var x: Int
    var y: Int
}

func test() -> Int {
    let p = Point(x: 1, y: 2);
    let Point { x, y } = p;
    x + y
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn struct_destructure_with_rename() {
        Test::new(
            r#"
module Main

struct Point {
    var x: Int
    var y: Int
}

func test() -> Int {
    let p = Point(x: 1, y: 2);
    let Point { x: a, y: b } = p;
    a + b
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn struct_destructure_with_rest() {
        Test::new(
            r#"
module Main

struct Point {
    var x: Int
    var y: Int
}

func test() -> Int {
    let p = Point(x: 1, y: 2);
    let Point { x, .. } = p;
    x
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn nested_struct_destructure() {
        Test::new(
            r#"
module Main

struct Point {
    var x: Int
    var y: Int
}

struct Line {
    var start: Point
    var end: Point
}

func test() -> Int {
    let line = Line(start: Point(x: 0, y: 0), end: Point(x: 10, y: 10));
    let Line { start: Point { x: x1, .. }, end: Point { x: x2, .. } } = line;
    x2 - x1
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// IRREFUTABILITY
// ============================================================================

mod irrefutability {
    use super::*;

    #[test]
    fn refutable_enum_pattern_error() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> Int {
    let .Some(value) = opt;
    value
}
"#,
        )
        .expect(Fails)
        .expect(HasError("refutable"));
    }

    #[test]
    fn refutable_literal_pattern_error() {
        Test::new(
            r#"
module Main

func test(x: Int) -> Int {
    let 42 = x;
    42
}
"#,
        )
        .expect(Fails)
        .expect(HasError("refutable"));
    }

    #[test]
    fn single_case_enum_is_irrefutable() {
        Test::new(
            r#"
module Main

enum Wrapper[T] {
    case Value(inner: T)
}

func test(w: Wrapper[Int]) -> Int {
    let .Value(inner) = w;
    inner
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn wildcard_is_irrefutable() {
        Test::new(
            r#"
module Main

func test(x: Int) -> Int {
    let _ = x;
    42
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn binding_is_irrefutable() {
        Test::new(
            r#"
module Main

func test(x: Int) -> Int {
    let y = x;
    y
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn tuple_of_irrefutables_is_irrefutable() {
        Test::new(
            r#"
module Main

func test(t: (Int, Int)) -> Int {
    let (a, b) = t;
    a + b
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn tuple_with_refutable_is_refutable() {
        Test::new(
            r#"
module Main

func test(t: (Int, Int)) -> Int {
    let (0, b) = t;
    b
}
"#,
        )
        .expect(Fails)
        .expect(HasError("refutable"));
    }
}

// ============================================================================
// TYPE INFERENCE
// ============================================================================

mod type_inference {
    use super::*;

    #[test]
    fn infer_tuple_element_types() {
        Test::new(
            r#"
module Main

func test() -> Int {
    let (a, b) = (1, "hello");
    a
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn infer_nested_types() {
        Test::new(
            r#"
module Main

func test() -> Int {
    let ((a, b), c) = ((1, 2), 3);
    a + b + c
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn annotation_constrains_pattern() {
        Test::new(
            r#"
module Main

func test() -> Int {
    let (a, b): (Int, Int) = (1, 2);
    a + b
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn pattern_type_mismatch_with_annotation() {
        Test::new(
            r#"
module Main

func test() -> Int {
    let (a, b): (Int, Int) = (1, "hello");
    a
}
"#,
        )
        .expect(Fails)
        .expect(HasError("type"));
    }
}
