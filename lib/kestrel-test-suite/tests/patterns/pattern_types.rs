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

func test(x: lang.i64) -> lang.i64 {
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

func test(c: Color) -> lang.i64 {
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

func test(t: (lang.i64, lang.i64)) -> lang.i64 {
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

func test(t: (lang.i64, lang.i64, lang.i64)) -> lang.i64 {
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

func test(x: lang.i64) -> lang.i64 {
    match x {
        n => lang.i64_add(n, 1)
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

func test(x: lang.i64) -> lang.i64 {
    match x {
        var n => {
            n = lang.i64_add(n, 1);
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

func test(x: lang.i64) -> lang.i64 {
    match x {
        n => lang.i64_mul(n, 2)
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

func test(x: lang.i64) -> lang.i64 {
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

func test(x: lang.i64) -> lang.str {
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

func test(b: lang.i1) -> lang.i64 {
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

func test(s: lang.str) -> lang.i64 {
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
    fn char_literal_pattern() {
        // Tests char literal patterns with Char struct
        Test::new(
            r#"
module Test

import std.text.Char
import std.num.Int64

func classify(c: Char) -> Int64 {
    match c {
        'a' => 1,
        'b' => 2,
        'c' => 3,
        _ => 0
    }
}

func main() -> lang.i64 {
    let a: Char = 'a';
    let b: Char = 'b';
    let c: Char = 'c';
    let d: Char = 'd';

    if classify(a) != 1 { return 1 }
    if classify(b) != 2 { return 2 }
    if classify(c) != 3 { return 3 }
    if classify(d) != 0 { return 4 }
    0
}
"#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }

    #[test]
    fn char_literal_pattern_escape_sequences() {
        // Tests escape sequences in char literal patterns
        Test::new(
            r#"
module Test

import std.text.Char
import std.num.Int64

func classify(c: Char) -> Int64 {
    match c {
        '\n' => 1,
        '\t' => 2,
        '\\' => 3,
        '\'' => 4,
        '\0' => 5,
        _ => 0
    }
}

func main() -> lang.i64 {
    let newline: Char = '\n';
    let tab: Char = '\t';
    let backslash: Char = '\\';
    let quote: Char = '\'';
    let nul: Char = '\0';
    let other: Char = 'x';

    if classify(newline) != 1 { return 1 }
    if classify(tab) != 2 { return 2 }
    if classify(backslash) != 3 { return 3 }
    if classify(quote) != 4 { return 4 }
    if classify(nul) != 5 { return 5 }
    if classify(other) != 0 { return 6 }
    0
}
"#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }

    #[test]
    fn char_literal_pattern_unicode() {
        // Tests unicode char literal patterns
        Test::new(
            r#"
module Test

import std.text.Char
import std.num.Int64

func classify(c: Char) -> Int64 {
    match c {
        'Ω' => 1,
        '日' => 2,
        '本' => 3,
        _ => 0
    }
}

func main() -> lang.i64 {
    let omega: Char = 'Ω';
    let sun: Char = '日';
    let book: Char = '本';
    let ascii: Char = 'a';

    if classify(omega) != 1 { return 1 }
    if classify(sun) != 2 { return 2 }
    if classify(book) != 3 { return 3 }
    if classify(ascii) != 0 { return 4 }
    0
}
"#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }

    #[test]
    fn char_literal_pattern_unicode_escape() {
        // Tests unicode escape sequences in char literal patterns
        Test::new(
            r#"
module Test

import std.text.Char
import std.num.Int64

func isEmoji(c: Char) -> Int64 {
    match c {
        '\u{1F600}' => 1,
        '\u{1F601}' => 2,
        '\u{1F602}' => 3,
        _ => 0
    }
}

func main() -> lang.i64 {
    let grinning: Char = '\u{1F600}';
    let beaming: Char = '\u{1F601}';
    let joy: Char = '\u{1F602}';
    let letter: Char = 'a';

    if isEmoji(grinning) != 1 { return 1 }
    if isEmoji(beaming) != 2 { return 2 }
    if isEmoji(joy) != 3 { return 3 }
    if isEmoji(letter) != 0 { return 4 }
    0
}
"#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }

    #[test]
    #[ignore = "Or-patterns with Char literals need investigation"]
    fn char_literal_pattern_or() {
        // Tests or-patterns with char literals
        Test::new(
            r#"
module Test

import std.text.Char
import std.num.Int64

func classifyCaseInsensitive(c: Char) -> Int64 {
    match c {
        'a' or 'A' => 1,
        'b' or 'B' => 2,
        'c' or 'C' => 3,
        _ => 0
    }
}

func main() -> lang.i64 {
    let lowerA: Char = 'a';
    let upperA: Char = 'A';
    let lowerB: Char = 'b';
    let upperB: Char = 'B';
    let other: Char = 'x';

    if classifyCaseInsensitive(lowerA) != 1 { return 1 }
    if classifyCaseInsensitive(upperA) != 1 { return 2 }
    if classifyCaseInsensitive(lowerB) != 2 { return 3 }
    if classifyCaseInsensitive(upperB) != 2 { return 4 }
    if classifyCaseInsensitive(other) != 0 { return 5 }
    0
}
"#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }

    #[test]
    fn negative_integer_literal() {
        // Note: This may need special handling since -42 is unary minus + literal
        Test::new(
            r#"
module Main

func test(x: lang.i64) -> lang.str {
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

func test(t: (lang.i64, lang.i64)) -> lang.i64 {
    match t {
        (a, b) => lang.i64_add(a, b)
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

func test(t: ((lang.i64, lang.i64), lang.i64)) -> lang.i64 {
    match t {
        ((a, b), c) => lang.i64_add(lang.i64_add(a, b), c)
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

func test(t: (lang.i64, lang.i64, lang.i64)) -> lang.i64 {
    match t {
        (first, _, last) => lang.i64_add(first, last)
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

func test(t: (lang.i64, lang.i64)) -> lang.str {
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

func test(t: (lang.i64, lang.i64, lang.i64, lang.i64)) -> lang.i64 {
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

func test(t: (lang.i64, lang.i64, lang.i64, lang.i64)) -> lang.i64 {
    match t {
        (first, second, ..) => lang.i64_add(first, second)
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

func test(t: (lang.i64, lang.i64, lang.i64, lang.i64)) -> lang.i64 {
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

func test(t: (lang.i64, lang.i64, lang.i64, lang.i64)) -> lang.i64 {
    match t {
        (first, .., last) => lang.i64_add(first, last)
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

func test(t: (lang.i64, lang.i64, lang.i64, lang.i64)) -> lang.i64 {
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

func test(x: lang.i64) -> lang.str {
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

    #[test]
    #[ignore = "Char range patterns with Matchable not yet implemented"]
    fn char_range_pattern() {
        // Tests char range patterns with Char struct
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
    let upperZ: Char = 'Z';
    let digit0: Char = '0';
    let digit9: Char = '9';
    let space: Char = ' ';

    if classify(lowerA) != 1 { return 1 }
    if classify(lowerZ) != 1 { return 2 }
    if classify(lowerM) != 1 { return 3 }
    if classify(upperA) != 2 { return 4 }
    if classify(upperZ) != 2 { return 5 }
    if classify(digit0) != 3 { return 6 }
    if classify(digit9) != 3 { return 7 }
    if classify(space) != 0 { return 8 }
    0
}
"#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }

    #[test]
    #[ignore = "Char range patterns with Matchable not yet implemented"]
    fn char_range_pattern_exclusive() {
        // Tests exclusive char range patterns
        Test::new(
            r#"
module Test

import std.text.Char
import std.num.Int64

func classify(c: Char) -> Int64 {
    match c {
        'a'..<'z' => 1,
        'z' => 2,
        _ => 0
    }
}

func main() -> lang.i64 {
    let lowerA: Char = 'a';
    let lowerY: Char = 'y';
    let lowerZ: Char = 'z';
    let other: Char = '!';

    if classify(lowerA) != 1 { return 1 }
    if classify(lowerY) != 1 { return 2 }
    if classify(lowerZ) != 2 { return 3 }
    if classify(other) != 0 { return 4 }
    0
}
"#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }

    #[test]
    fn multiple_range_patterns() {
        Test::new(
            r#"
module Main

func test(score: lang.i64) -> lang.str {
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

func test(x: lang.i64) -> lang.str {
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
    #[ignore = "Char range patterns with Matchable not yet implemented"]
    fn char_range_with_integer_scrutinee() {
        // Char literals can match against integer types since they're both numeric
        // The char pattern infers to i32 but can unify with i64
        Test::new(
            r#"
module Test

import std.num.Int64

func classify(x: Int64) -> Int64 {
    match x {
        'a'..='z' => 1,
        _ => 0
    }
}

func main() -> lang.i64 {
    // 'a' = 97, 'z' = 122
    if classify(97) != 1 { return 1 }
    if classify(122) != 1 { return 2 }
    if classify(110) != 1 { return 3 }
    if classify(96) != 0 { return 4 }
    if classify(123) != 0 { return 5 }
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
    var x: lang.i64
    var y: lang.i64
}

func test(p: Point) -> lang.i64 {
    match p {
        Point { x, y } => lang.i64_add(x, y)
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
    var x: lang.i64
    var y: lang.i64
}

func test(p: Point) -> lang.i64 {
    match p {
        Point { x: a, y: b } => lang.i64_add(a, b)
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
    var x: lang.i64
    var y: lang.i64
    var z: lang.i64
}

func test(p: Point3D) -> lang.i64 {
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
    var x: lang.i64
    var y: lang.i64
}

func test(p: Point) -> lang.str {
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
    var x: lang.i64
    var y: lang.i64
}

struct Line {
    var start: Point
    var end: Point
}

func test(line: Line) -> lang.i64 {
    match line {
        Line { start: Point { x: x1, y: y1 }, end: Point { x: x2, y: y2 } } => lang.i64_add(lang.i64_add(lang.i64_add(x1, y1), x2), y2)
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
    var x: lang.i64
    var y: lang.i64
}

func test(p: Point) -> lang.i64 {
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
    var x: lang.i64
    var y: lang.i64
}

func test(p: Point) -> lang.i64 {
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

func test(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [a, b] => lang.i64_add(a, b),
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

func test(arr: [lang.i64]) -> lang.str {
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

func test(arr: [lang.i64]) -> lang.i64 {
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

func test(arr: [lang.i64]) -> [lang.i64] {
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

func test(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [first, second, ..] => lang.i64_add(first, second),
        [only] => only,
        [] => 0
    }
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn array_pattern_rest_at_beginning_not_supported() {
        // Array suffix patterns (elements after ..) are not yet supported
        Test::new(
            r#"
module Main

func test(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [.., last] => last,
        [] => 0
    }
}
"#,
        )
        .expect(HasError(
            "array patterns with suffix elements are not yet supported",
        ));
    }

    #[test]
    fn array_pattern_rest_in_middle_not_supported() {
        // Array suffix patterns (elements after ..) are not yet supported
        Test::new(
            r#"
module Main

func test(arr: [lang.i64]) -> lang.i64 {
    match arr {
        [first, .., last] => lang.i64_add(first, last),
        [only] => only,
        [] => 0
    }
}
"#,
        )
        .expect(HasError(
            "array patterns with suffix elements are not yet supported",
        ));
    }

    #[test]
    #[ignore]
    fn array_pattern_with_literals() {
        Test::new(
            r#"
module Main

func test(arr: [lang.i64]) -> lang.str {
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

indirect enum List[T] {
    case Cons(head: T, tail: List[T])
    case Nil
}

func test(list: List[lang.i64]) -> lang.i64 {
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

func test(opt: Option[lang.i64]) -> Option[lang.i64] {
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

func test(opt: Option[lang.i64]) -> Option[lang.i64] {
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

func test(x: lang.i64) -> lang.i64 {
    match x {
        1 @ n => n,
        _ => 0
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("At")); // Parser error reports token name 'At' for the @ symbol
    }

    #[test]
    fn nested_at_patterns_error() {
        Test::new(
            r#"
module Main

func test(x: lang.i64) -> lang.i64 {
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
