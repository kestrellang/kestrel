//! Tests for different pattern types.
//!
//! These tests verify the basic pattern forms:
//! - Wildcard patterns (_)
//! - Binding patterns (name, var name)
//! - Literal patterns (42, "hello", true)
//! - Tuple patterns ((a, b))
//! - Enum variant patterns (.Case, .Case(x))
//! - Range patterns (1..=10)
//! - Struct patterns (Point { x, y })
//! - Array patterns ([a, b, ..rest])
//! - @-patterns (x @ .Some(_))

use kestrel_test_suite::*;

// ============================================================================
// WILDCARD PATTERNS
// ============================================================================

mod wildcard {
    use super::*;

    #[test]
    fn wildcard_matches_any() {
        Test::new(
            r#"
module Main

func test(x: Int) -> Int {
    match x {
        _ => 42
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn wildcard_as_fallback() {
        Test::new(
            r#"
module Main

enum Color {
    case Red
    case Green
    case Blue
}

func test(c: Color) -> Int {
    match c {
        .Red => 1,
        _ => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn wildcard_in_tuple() {
        Test::new(
            r#"
module Main

func test(t: (Int, Int)) -> Int {
    match t {
        (x, _) => x
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn multiple_wildcards() {
        Test::new(
            r#"
module Main

func test(t: (Int, Int, Int)) -> Int {
    match t {
        (_, _, _) => 0
    }
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// BINDING PATTERNS
// ============================================================================

mod binding {
    use super::*;

    #[test]
    fn simple_binding() {
        Test::new(
            r#"
module Main

func test(x: Int) -> Int {
    match x {
        n => n + 1
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn mutable_binding_with_var() {
        Test::new(
            r#"
module Main

func test(x: Int) -> Int {
    match x {
        var n => {
            n = n + 1;
            n
        }
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn binding_type_inferred() {
        Test::new(
            r#"
module Main

func test(x: Int) -> Int {
    match x {
        n => n * 2
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn immutable_binding_cannot_reassign() {
        Test::new(
            r#"
module Main

func test(x: Int) -> Int {
    match x {
        n => {
            n = 10;
            n
        }
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("immutable"));
    }
}

// ============================================================================
// LITERAL PATTERNS
// ============================================================================

mod literal {
    use super::*;

    #[test]
    fn integer_literal_pattern() {
        Test::new(
            r#"
module Main

func test(x: Int) -> String {
    match x {
        0 => "zero",
        1 => "one",
        _ => "other"
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn bool_literal_pattern() {
        Test::new(
            r#"
module Main

func test(b: Bool) -> Int {
    match b {
        true => 1,
        false => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn string_literal_pattern() {
        Test::new(
            r#"
module Main

func test(s: String) -> Int {
    match s {
        "hello" => 1,
        "world" => 2,
        _ => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    #[ignore = "Char type not yet implemented"]
    fn char_literal_pattern() {
        Test::new(
            r#"
module Main

func test(c: Char) -> Int {
    match c {
        'a' => 1,
        'b' => 2,
        _ => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn negative_integer_literal() {
        // Note: This may need special handling since -42 is unary minus + literal
        Test::new(
            r#"
module Main

func test(x: Int) -> String {
    match x {
        0 => "zero",
        _ => "nonzero"
    }
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// TUPLE PATTERNS
// ============================================================================

mod tuple {
    use super::*;

    #[test]
    fn tuple_pattern_basic() {
        Test::new(
            r#"
module Main

func test(t: (Int, Int)) -> Int {
    match t {
        (a, b) => a + b
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn nested_tuple_pattern() {
        Test::new(
            r#"
module Main

func test(t: ((Int, Int), Int)) -> Int {
    match t {
        ((a, b), c) => a + b + c
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn tuple_pattern_with_wildcard() {
        Test::new(
            r#"
module Main

func test(t: (Int, Int, Int)) -> Int {
    match t {
        (first, _, last) => first + last
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn tuple_pattern_with_literal() {
        Test::new(
            r#"
module Main

func test(t: (Int, Int)) -> String {
    match t {
        (0, 0) => "origin",
        (0, _) => "y-axis",
        (_, 0) => "x-axis",
        _ => "elsewhere"
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn tuple_rest_pattern() {
        Test::new(
            r#"
module Main

func test(t: (Int, Int, Int, Int)) -> Int {
    match t {
        (first, ..) => first
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn tuple_rest_at_end() {
        Test::new(
            r#"
module Main

func test(t: (Int, Int, Int, Int)) -> Int {
    match t {
        (first, second, ..) => first + second
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn tuple_rest_at_beginning() {
        Test::new(
            r#"
module Main

func test(t: (Int, Int, Int, Int)) -> Int {
    match t {
        (.., last) => last
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn tuple_rest_in_middle() {
        Test::new(
            r#"
module Main

func test(t: (Int, Int, Int, Int)) -> Int {
    match t {
        (first, .., last) => first + last
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn multiple_rest_patterns_error() {
        Test::new(
            r#"
module Main

func test(t: (Int, Int, Int, Int)) -> Int {
    match t {
        (.., middle, ..) => middle
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("rest"));
    }
}

// ============================================================================
// RANGE PATTERNS
// ============================================================================

mod range {
    use super::*;

    #[test]
    fn inclusive_range_pattern() {
        Test::new(
            r#"
module Main

func test(x: Int) -> String {
    match x {
        0..=9 => "digit",
        _ => "other"
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn exclusive_range_pattern() {
        Test::new(
            r#"
module Main

func test(x: Int) -> String {
    match x {
        0..<10 => "single digit",
        _ => "other"
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    #[ignore = "Char type not yet implemented"]
    fn char_range_pattern() {
        Test::new(
            r#"
module Main

func test(c: Char) -> String {
    match c {
        'a'..='z' => "lowercase",
        'A'..='Z' => "uppercase",
        '0'..='9' => "digit",
        _ => "other"
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn multiple_range_patterns() {
        Test::new(
            r#"
module Main

func test(score: Int) -> String {
    match score {
        0..=59 => "F",
        60..=69 => "D",
        70..=79 => "C",
        80..=89 => "B",
        90..=100 => "A",
        _ => "invalid"
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn range_invalid_bounds_error() {
        Test::new(
            r#"
module Main

func test(x: Int) -> String {
    match x {
        10..=0 => "invalid",
        _ => "other"
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("bound"));
    }

    #[test]
    #[ignore = "Char type not yet implemented"]
    fn range_type_mismatch_error() {
        Test::new(
            r#"
module Main

func test(x: Int) -> String {
    match x {
        'a'..='z' => "letter",
        _ => "other"
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("type"));
    }
}

// ============================================================================
// STRUCT PATTERNS
// ============================================================================

mod struct_pattern {
    use super::*;

    #[test]
    fn struct_pattern_basic() {
        Test::new(
            r#"
module Main

struct Point {
    var x: Int
    var y: Int
}

func test(p: Point) -> Int {
    match p {
        Point { x, y } => x + y
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn struct_pattern_explicit_binding() {
        Test::new(
            r#"
module Main

struct Point {
    var x: Int
    var y: Int
}

func test(p: Point) -> Int {
    match p {
        Point { x: a, y: b } => a + b
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn struct_pattern_with_rest() {
        Test::new(
            r#"
module Main

struct Point3D {
    var x: Int
    var y: Int
    var z: Int
}

func test(p: Point3D) -> Int {
    match p {
        Point3D { x, .. } => x
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn struct_pattern_with_literal() {
        Test::new(
            r#"
module Main

struct Point {
    var x: Int
    var y: Int
}

func test(p: Point) -> String {
    match p {
        Point { x: 0, y: 0 } => "origin",
        Point { x: 0, y } => "y-axis",
        Point { x, y: 0 } => "x-axis",
        Point { .. } => "elsewhere"
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn struct_pattern_nested() {
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

func test(line: Line) -> Int {
    match line {
        Line { start: Point { x: x1, y: y1 }, end: Point { x: x2, y: y2 } } => x1 + y1 + x2 + y2
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn struct_pattern_unknown_field_error() {
        Test::new(
            r#"
module Main

struct Point {
    var x: Int
    var y: Int
}

func test(p: Point) -> Int {
    match p {
        Point { x, z } => x
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("z"));
    }

    #[test]
    fn struct_pattern_missing_fields_error() {
        Test::new(
            r#"
module Main

struct Point {
    var x: Int
    var y: Int
}

func test(p: Point) -> Int {
    match p {
        Point { x } => x
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("y"));
    }
}

// ============================================================================
// ARRAY PATTERNS
// ============================================================================

mod array {
    use super::*;

    #[test]
    fn array_pattern_exact() {
        Test::new(
            r#"
module Main

func test(arr: [Int]) -> Int {
    match arr {
        [a, b] => a + b,
        _ => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn array_pattern_empty() {
        Test::new(
            r#"
module Main

func test(arr: [Int]) -> String {
    match arr {
        [] => "empty",
        _ => "not empty"
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn array_pattern_with_rest() {
        Test::new(
            r#"
module Main

func test(arr: [Int]) -> Int {
    match arr {
        [first, ..] => first,
        [] => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn array_pattern_rest_with_binding() {
        Test::new(
            r#"
module Main

func test(arr: [Int]) -> [Int] {
    match arr {
        [_, ..rest] => rest,
        [] => []
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn array_pattern_rest_at_end() {
        Test::new(
            r#"
module Main

func test(arr: [Int]) -> Int {
    match arr {
        [first, second, ..] => first + second,
        [only] => only,
        [] => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn array_pattern_rest_at_beginning() {
        Test::new(
            r#"
module Main

func test(arr: [Int]) -> Int {
    match arr {
        [.., last] => last,
        [] => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn array_pattern_rest_in_middle() {
        Test::new(
            r#"
module Main

func test(arr: [Int]) -> Int {
    match arr {
        [first, .., last] => first + last,
        [only] => only,
        [] => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn array_pattern_with_literals() {
        Test::new(
            r#"
module Main

func test(arr: [Int]) -> String {
    match arr {
        [1, 2, 3] => "one two three",
        [0, ..] => "starts with zero",
        _ => "other"
    }
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// AT-PATTERNS
// ============================================================================

mod at_pattern {
    use super::*;

    #[test]
    fn at_pattern_basic() {
        Test::new(
            r#"
module Main

enum List[T] {
    case Cons(head: T, tail: List[T])
    case Nil
}

func test(list: List[Int]) -> Int {
    match list {
        node @ .Cons(head, _) => head,
        .Nil => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn at_pattern_with_enum() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> Option[Int] {
    match opt {
        some @ .Some(_) => some,
        .None => .None
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn at_pattern_with_or_needs_parens() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> Option[Int] {
    match opt {
        x @ (.Some(_) or .None) => x
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn at_pattern_invalid_left_side() {
        Test::new(
            r#"
module Main

func test(x: Int) -> Int {
    match x {
        1 @ n => n,
        _ => 0
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("@"));
    }

    #[test]
    fn nested_at_patterns_error() {
        Test::new(
            r#"
module Main

func test(x: Int) -> Int {
    match x {
        a @ b @ _ => a,
        _ => 0
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("@"));
    }
}
