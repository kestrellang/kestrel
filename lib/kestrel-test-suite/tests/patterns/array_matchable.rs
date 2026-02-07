//! Tests for ArrayMatchable protocol and array patterns.
//!
//! These tests verify:
//! - Basic array patterns (prefix only)
//! - Rest patterns with and without binding
//! - Suffix patterns (via ArrayMatchable)
//! - Combined prefix + rest + suffix patterns
//! - Slice conformance for recursive destructuring
//!
//! NOTE: Tests use array parameters rather than array literals due to a
//! pre-existing monomorphization bug with array literal initialization.

use kestrel_test_suite::*;

// ============================================================================
// BASIC ARRAY PATTERNS (prefix only)
// ============================================================================

mod basic_patterns {
    use super::*;

    #[test]
    fn empty_array_pattern() {
        // Test empty array pattern matching (compilation only)
        Test::new(
            r#"
module Test

func matchEmpty(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [] => 1,
        _ => 0
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn single_element_pattern() {
        // Test single element pattern matching
        Test::new(
            r#"
module Test

func matchSingle(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [x] => x,
        _ => 0
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn multiple_element_pattern() {
        // Test multiple element pattern matching
        Test::new(
            r#"
module Test

func sum3(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [a, b, c] => lang.i64_add(a, lang.i64_add(b, c)),
        _ => 0
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// REST PATTERNS (prefix + rest)
// ============================================================================

mod rest_patterns {
    use super::*;

    #[test]
    fn rest_without_binding() {
        // [a, ..] - matches one or more elements
        Test::new(
            r#"
module Test

func getFirst(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [first, ..] => first,
        _ => 0
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn rest_with_binding() {
        // [a, ..rest] - captures rest as Slice[T]
        Test::new(
            r#"
module Test

import std.memory.Slice

func restLength(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [_, ..rest] => rest.count.raw,
        _ => 0
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn capture_all_as_slice() {
        // [..all] - captures entire array as Slice
        Test::new(
            r#"
module Test

import std.memory.Slice

func asSliceLength(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [..all] => all.count.raw
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn multiple_prefix_with_rest() {
        // [a, b, ..rest] - multiple prefix elements
        Test::new(
            r#"
module Test

func sumFirstTwo(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [a, b, ..] => lang.i64_add(a, b),
        _ => 0
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// SUFFIX PATTERNS (requires ArrayMatchable)
// ============================================================================

mod suffix_patterns {
    use super::*;

    #[test]
    fn suffix_only() {
        // [.., z] - get last element
        Test::new(
            r#"
module Test

func getLast(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [.., last] => last,
        _ => 0
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn prefix_and_suffix() {
        // [a, .., z] - first and last without capturing middle
        Test::new(
            r#"
module Test

func endpoints(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [first, .., last] => lang.i64_add(first, last),
        _ => 0
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn multiple_suffix() {
        // [.., y, z] - last two elements
        Test::new(
            r#"
module Test

func lastTwo(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [.., y, z] => lang.i64_add(y, z),
        _ => 0
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// COMBINED PATTERNS (prefix + rest + suffix)
// ============================================================================

mod combined_patterns {
    use super::*;

    #[test]
    fn prefix_rest_suffix() {
        // [a, ..rest, z] - capture middle as slice
        Test::new(
            r#"
module Test

import std.memory.Slice

func middleLength(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [_, ..middle, _] => middle.count.raw,
        _ => 0
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn multiple_prefix_rest_multiple_suffix() {
        // [a, b, ..rest, y, z]
        Test::new(
            r#"
module Test

import std.memory.Slice

func complexPattern(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [a, b, ..middle, y, z] => lang.i64_add(lang.i64_add(a, b), lang.i64_add(y, z)),
        _ => 0
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn rest_suffix_without_prefix() {
        // [..rest, y, z] - rest at start with suffix
        Test::new(
            r#"
module Test

import std.memory.Slice

func restAndLastTwo(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [..rest, y, z] => lang.i64_add(rest.count.raw, lang.i64_add(y, z)),
        _ => 0
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// NESTED PATTERNS
// ============================================================================

mod nested_patterns {
    use super::*;

    #[test]
    fn nested_enum_in_array() {
        // Nested enum patterns in array elements
        Test::new(
            r#"
module Test

import std.result.Optional

func firstSome(arr: [Optional[lang.i64]]) -> lang.i64 {
    match arr {
        [.Some(x), ..] => x,
        _ => 0
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn nested_tuple_in_array() {
        // Nested tuple patterns in array elements
        Test::new(
            r#"
module Test

func sumFirstPair(arr: [(lang.i64, lang.i64)]) -> lang.i64 {
    match arr {
        [(a, b), ..] => lang.i64_add(a, b),
        _ => 0
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// SLICE CONFORMANCE
// ============================================================================

mod slice_conformance {
    use super::*;

    #[test]
    fn slice_array_pattern() {
        // Slice[T] also conforms to ArrayMatchable
        Test::new(
            r#"
module Test

import std.memory.Slice

func sliceFirst(s: Slice[lang.i64]) -> lang.i64 {
    match s {
        [first, ..] => first,
        _ => 0
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn recursive_slice_destructuring() {
        // Destructure captured rest slice further
        Test::new(
            r#"
module Test

import std.memory.Slice

func nestedMatch(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [first, ..rest] => {
            match rest {
                [second, ..] => lang.i64_add(first, second),
                _ => first
            }
        },
        _ => 0
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}

// ============================================================================
// LET DESTRUCTURING WITH ARRAY PATTERNS
// ============================================================================

mod let_destructuring {
    use super::*;

    #[test]
    fn let_array_destructure() {
        // Let destructuring with array pattern - must use irrefutable pattern [..all]
        // Note: [a, b, c] is refutable because it only matches arrays of exactly 3 elements
        Test::new(
            r#"
module Test

import std.memory.Slice

func destructure(arr: [lang.i64]) -> lang.i64 {
    let [..all] = arr;
    all.count.raw
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }

    #[test]
    fn let_with_rest() {
        // Let destructuring with rest pattern - [first, ..rest] is refutable
        // because it requires at least 1 element. Use match instead.
        Test::new(
            r#"
module Test

import std.memory.Slice

func destructure(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [first, ..rest] => lang.i64_add(first, rest.count.raw),
        _ => 0
    }
}
"#,
        )
        .with_stdlib()
        .expect(Compiles);
    }
}
