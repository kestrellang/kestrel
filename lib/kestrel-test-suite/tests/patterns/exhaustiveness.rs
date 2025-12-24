//! Tests for exhaustiveness checking.
//!
//! These tests verify that:
//! - Non-exhaustive matches produce errors
//! - Exhaustive matches compile
//! - Wildcards cover remaining cases
//! - Guards don't count for exhaustiveness
//! - Redundant patterns produce warnings

use kestrel_test_suite::*;

// ============================================================================
// BASIC EXHAUSTIVENESS
// ============================================================================

mod basic {
    use super::*;

    #[test]
    fn exhaustive_bool() {
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
    fn non_exhaustive_bool() {
        Test::new(
            r#"
module Main

func test(b: Bool) -> Int {
    match b {
        true => 1
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("exhaustive"));
    }

    #[test]
    fn exhaustive_enum() {
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
        .Green => 2,
        .Blue => 3
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn non_exhaustive_enum() {
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
        .Green => 2
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("exhaustive"));
    }

    #[test]
    fn exhaustive_with_wildcard() {
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
    fn exhaustive_enum_with_associated_values() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> Int {
    match opt {
        .Some(value) => value,
        .None => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn non_exhaustive_enum_with_associated_values() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> Int {
    match opt {
        .Some(value) => value
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("exhaustive"));
    }
}

// ============================================================================
// GUARDS AND EXHAUSTIVENESS
// ============================================================================

mod guards {
    use super::*;

    #[test]
    fn guard_does_not_count_for_exhaustiveness() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> String {
    match opt {
        .Some(n) if n > 0 => "positive",
        .None => "nothing"
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("exhaustive"));
    }

    #[test]
    fn guard_with_fallback() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> String {
    match opt {
        .Some(n) if n > 0 => "positive",
        .Some(_) => "non-positive",
        .None => "nothing"
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn guard_on_all_cases_needs_fallback() {
        Test::new(
            r#"
module Main

func test(x: Int) -> String {
    match x {
        n if n > 0 => "positive",
        n if n < 0 => "negative"
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("exhaustive"));
    }

    #[test]
    fn guard_on_all_cases_with_fallback() {
        Test::new(
            r#"
module Main

func test(x: Int) -> String {
    match x {
        n if n > 0 => "positive",
        n if n < 0 => "negative",
        _ => "zero"
    }
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// INFINITE TYPES (INT, STRING)
// ============================================================================

mod infinite_types {
    use super::*;

    #[test]
    fn int_requires_wildcard() {
        Test::new(
            r#"
module Main

func test(x: Int) -> String {
    match x {
        0 => "zero",
        1 => "one"
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("exhaustive"));
    }

    #[test]
    fn int_with_wildcard() {
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
    fn string_requires_wildcard() {
        Test::new(
            r#"
module Main

func test(s: String) -> Int {
    match s {
        "hello" => 1,
        "world" => 2
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("exhaustive"));
    }

    #[test]
    fn string_with_wildcard() {
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
}

// ============================================================================
// TUPLES
// ============================================================================

mod tuples {
    use super::*;

    #[test]
    fn exhaustive_bool_tuple() {
        Test::new(
            r#"
module Main

func test(t: (Bool, Bool)) -> Int {
    match t {
        (true, true) => 1,
        (true, false) => 2,
        (false, true) => 3,
        (false, false) => 4
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn non_exhaustive_bool_tuple() {
        Test::new(
            r#"
module Main

func test(t: (Bool, Bool)) -> Int {
    match t {
        (true, true) => 1,
        (true, false) => 2,
        (false, true) => 3
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("exhaustive"));
    }

    #[test]
    fn tuple_with_wildcard() {
        Test::new(
            r#"
module Main

func test(t: (Bool, Bool)) -> Int {
    match t {
        (true, true) => 1,
        _ => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn tuple_with_partial_wildcards() {
        Test::new(
            r#"
module Main

func test(t: (Bool, Bool)) -> Int {
    match t {
        (true, _) => 1,
        (false, _) => 0
    }
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// NESTED PATTERNS
// ============================================================================

mod nested {
    use super::*;

    #[test]
    fn nested_enum_exhaustive() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Bool]) -> Int {
    match opt {
        .Some(value: true) => 1,
        .Some(value: false) => 2,
        .None => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn nested_enum_non_exhaustive() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Bool]) -> Int {
    match opt {
        .Some(value: true) => 1,
        .None => 0
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("exhaustive"));
    }

    #[test]
    fn nested_with_inner_wildcard() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Bool]) -> Int {
    match opt {
        .Some(value: true) => 1,
        .Some(_) => 2,
        .None => 0
    }
}
"#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// REDUNDANT PATTERNS
// ============================================================================

mod redundancy {
    use super::*;

    #[test]
    fn unreachable_after_wildcard() {
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
        _ => 0,
        .Green => 2
    }
}
"#,
        )
        .expect(HasWarning("unreachable"));
    }

    #[test]
    fn duplicate_pattern() {
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
        .Red => 2,
        .Green => 3,
        .Blue => 4
    }
}
"#,
        )
        .expect(HasWarning("unreachable"));
    }

    #[test]
    fn subsumed_by_earlier_pattern() {
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Int]) -> Int {
    match opt {
        .Some(_) => 1,
        .Some(value: 42) => 2,
        .None => 0
    }
}
"#,
        )
        .expect(HasWarning("unreachable"));
    }

    #[test]
    fn overlapping_ranges() {
        Test::new(
            r#"
module Main

func test(x: Int) -> String {
    match x {
        0..=10 => "first",
        5..=15 => "second",
        _ => "other"
    }
}
"#,
        )
        .expect(HasWarning("overlap"));
    }

    #[test]
    fn unreachable_overlap_nested() {
        Test::new(
            r#"
module Main

enum E {
    case A(x: Int, y: Int)
}

func test(e: E) -> Int {
    match e {
        .A(x: 1, y: _) => 1,
        .A(x: _, y: 1) => 2,
        .A(x: 1, y: 1) => 3, // UNREACHABLE
        .A(x: _, y: _) => 4
    }
}
"#,
        )
        .expect(HasWarning("unreachable"));
    }

    #[test]
    fn unreachable_array_rest() {
        Test::new(
            r#"
module Main

func test(arr: [Int]) -> Int {
    match arr {
        [] => 0,
        [x] => x,
        [first, ..rest, last] => first + last,
        [..] => -1 // UNREACHABLE
    }
}
"#,
        )
        .expect(HasWarning("unreachable"));
    }
}

// ============================================================================
// OR-PATTERNS AND EXHAUSTIVENESS
// ============================================================================

mod or_patterns {
    use super::*;

    #[test]
    fn or_pattern_covers_multiple() {
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
        .Red or .Green => 1,
        .Blue => 2
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn or_pattern_still_non_exhaustive() {
        Test::new(
            r#"
module Main

enum Color {
    case Red
    case Green
    case Blue
    case Yellow
}

func test(c: Color) -> Int {
    match c {
        .Red or .Green => 1,
        .Blue => 2
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("exhaustive"));
    }
}

// ============================================================================
// EMPTY MATCH
// ============================================================================

mod empty_match {
    use super::*;

    #[test]
    fn empty_match_on_inhabited_type_error() {
        Test::new(
            r#"
module Main

func test(x: Int) -> Int {
    match x {
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("empty"));
    }

    // Note: Empty match on Never type would be valid, but requires Never type support
}
