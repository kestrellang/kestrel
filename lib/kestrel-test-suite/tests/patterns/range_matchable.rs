//! Tests for RangeMatchable protocol and range patterns.
//!
//! These tests verify:
//! - Range patterns on Char type (via Comparable -> RangeMatchable)
//! - Open-ended range patterns (..=end, ..<end, start..)
//! - Error cases

use kestrel_test_suite::*;

// ============================================================================
// CHAR RANGE PATTERNS (via Comparable -> RangeMatchable)
// ============================================================================

mod char_ranges {
    use super::*;

    #[test]
    fn char_range_inclusive() {
        // Char conforms to Comparable, which provides RangeMatchable[Char]
        Test::new(
            r#"
module Test

import std.text.Char
import std.num.Int64

func classify(c: Char) -> Int64 {
    match c {
        'a'..='z' => 1,
        'A'..='Z' => 2,
        '0'..='9' => 3,
        _ => 0
    }
}

func main() -> lang.i64 {
    let lowerA: Char = 'a';
    let lowerZ: Char = 'z';
    let lowerM: Char = 'm';
    let upperA: Char = 'A';
    let digit5: Char = '5';
    let space: Char = ' ';

    if classify(lowerA) != 1 { return 1 }
    if classify(lowerZ) != 1 { return 2 }
    if classify(lowerM) != 1 { return 3 }
    if classify(upperA) != 2 { return 4 }
    if classify(digit5) != 3 { return 5 }
    if classify(space) != 0 { return 6 }
    0
}
"#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }

    #[test]
    fn char_range_exclusive() {
        Test::new(
            r#"
module Test

import std.text.Char
import std.num.Int64

func classify(c: Char) -> Int64 {
    match c {
        'a'..<'z' => 1,  // a through y
        'z' => 2,        // exactly z
        _ => 0
    }
}

func main() -> lang.i64 {
    let lowerA: Char = 'a';
    let lowerY: Char = 'y';
    let lowerZ: Char = 'z';

    if classify(lowerA) != 1 { return 1 }
    if classify(lowerY) != 1 { return 2 }
    if classify(lowerZ) != 2 { return 3 }
    0
}
"#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }
}

// ============================================================================
// OPEN-ENDED RANGE PATTERNS
// ============================================================================

mod open_ended {
    use super::*;

    #[test]
    #[ignore = "Open-ended ranges need exhaustiveness checking updates"]
    fn range_to_inclusive() {
        // ..=end pattern
        Test::new(
            r#"
module Test

import std.num.Int64

func classify(x: Int64) -> Int64 {
    match x {
        ..=0 => 1,      // negative or zero
        1..=100 => 2,   // 1 to 100
        _ => 3          // over 100
    }
}

func main() -> lang.i64 {
    if classify(-10) != 1 { return 1 }
    if classify(0) != 1 { return 2 }
    if classify(1) != 2 { return 3 }
    if classify(100) != 2 { return 4 }
    if classify(101) != 3 { return 5 }
    0
}
"#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }

    #[test]
    #[ignore = "Open-ended ranges need exhaustiveness checking updates"]
    fn range_to_exclusive() {
        // ..<end pattern
        Test::new(
            r#"
module Test

import std.num.Int64

func classify(x: Int64) -> Int64 {
    match x {
        ..<0 => 1,      // negative only
        0 => 2,         // zero
        _ => 3          // positive
    }
}

func main() -> lang.i64 {
    if classify(-5) != 1 { return 1 }
    if classify(0) != 2 { return 2 }
    if classify(1) != 3 { return 3 }
    0
}
"#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }

    #[test]
    #[ignore = "Open-ended ranges need exhaustiveness checking updates"]
    fn range_from() {
        // start.. pattern
        Test::new(
            r#"
module Test

import std.num.Int64

func classify(x: Int64) -> Int64 {
    match x {
        ..<0 => 1,      // negative
        0..=59 => 2,    // failing
        60.. => 3       // passing (60 and above)
    }
}

func main() -> lang.i64 {
    if classify(-1) != 1 { return 1 }
    if classify(0) != 2 { return 2 }
    if classify(59) != 2 { return 3 }
    if classify(60) != 3 { return 4 }
    if classify(100) != 3 { return 5 }
    0
}
"#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }

    #[test]
    #[ignore = "Open-ended ranges need exhaustiveness checking updates"]
    fn char_range_from() {
        // start.. pattern with Char
        Test::new(
            r#"
module Test

import std.text.Char
import std.num.Int64

func isUpperOrBeyond(c: Char) -> Int64 {
    match c {
        ..<'A' => 0,
        'A'.. => 1
    }
}

func main() -> lang.i64 {
    let at: Char = '@';   // before 'A'
    let upperA: Char = 'A';
    let upperZ: Char = 'Z';
    let lowerA: Char = 'a';  // after 'Z' in ASCII

    if isUpperOrBeyond(at) != 0 { return 1 }
    if isUpperOrBeyond(upperA) != 1 { return 2 }
    if isUpperOrBeyond(upperZ) != 1 { return 3 }
    if isUpperOrBeyond(lowerA) != 1 { return 4 }
    0
}
"#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }
}

// ============================================================================
// ERROR CASES
// ============================================================================

mod errors {
    use super::*;

    #[test]
    fn invalid_range_bounds() {
        // Start > end should error (for types where this is detectable)
        Test::new(
            r#"
module Main

func test(x: lang.i64) -> lang.i64 {
    match x {
        10..=0 => 1,
        _ => 0
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("bound"));
    }
}

// ============================================================================
// INTEGER RANGE PATTERNS (existing, should still work)
// ============================================================================

mod integer_ranges {
    use super::*;

    #[test]
    fn integer_range_inclusive() {
        // Existing integer range patterns should continue to work
        Test::new(
            r#"
module Main

func grade(score: lang.i64) -> lang.str {
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
    fn integer_range_exclusive() {
        Test::new(
            r#"
module Main

func test(x: lang.i64) -> lang.str {
    match x {
        0..<10 => "single digit",
        _ => "other"
    }
}
"#,
        )
        .expect(Compiles);
    }
}
