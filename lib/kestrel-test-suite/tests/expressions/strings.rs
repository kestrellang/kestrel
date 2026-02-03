//! Tests for string literals, escape sequences, and raw strings.
//!
//! This module tests:
//! - Basic escape sequences: \n, \r, \t, \\, \", \', \0
//! - Hex ASCII escapes: \xNN (0x00-0x7F)
//! - Unicode escapes: \u{NNNN} (1-6 hex digits)
//! - Line continuation: \ + newline
//! - Raw strings: """..."""
//! - Error diagnostics for invalid escape sequences

use kestrel_test_suite::*;

mod escape_sequences {
    use super::*;

    #[test]
    fn basic_escape_sequences() {
        Test::new(
            r#"
module Main

func testNewline() -> lang.str {
    "hello\nworld"
}

func testCarriageReturn() -> lang.str {
    "hello\rworld"
}

func testTab() -> lang.str {
    "hello\tworld"
}

func testBackslash() -> lang.str {
    "hello\\world"
}

func testDoubleQuote() -> lang.str {
    "hello\"world"
}

func testSingleQuote() -> lang.str {
    "hello\'world"
}

func testNullChar() -> lang.str {
    "hello\0world"
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn hex_escape_sequences() {
        Test::new(
            r#"
module Main

func testHexEscape() -> lang.str {
    "\x00\x41\x7F"
}

func testHexMixedWithText() -> lang.str {
    "A\x42C"
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn unicode_escape_sequences() {
        Test::new(
            r#"
module Main

func testUnicodeBasic() -> lang.str {
    "\u{0041}"
}

func testUnicodeEmoji() -> lang.str {
    "\u{1F600}"
}

func testUnicodeMax() -> lang.str {
    "\u{10FFFF}"
}

func testUnicodeShort() -> lang.str {
    "\u{A}"
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn line_continuation() {
        Test::new(
            r#"
module Main

func testLineContinuation() -> lang.str {
    "hello \
    world"
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn mixed_escapes() {
        Test::new(
            r#"
module Main

func testMixedEscapes() -> lang.str {
    "Tab:\t Newline:\n Quote:\" Unicode:\u{2603}"
}
"#,
        )
        .expect(Compiles);
    }
}

mod raw_strings {
    use super::*;

    #[test]
    fn basic_raw_string() {
        Test::new(
            r#"
module Main

func testRawString() -> lang.str {
    """hello world"""
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn raw_string_with_newlines() {
        Test::new(
            r#"
module Main

func testMultilineRawString() -> lang.str {
    """hello
world"""
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn raw_string_no_escape_processing() {
        // Backslashes should be literal in raw strings
        Test::new(
            r#"
module Main

func testRawStringBackslash() -> lang.str {
    """hello\nworld"""
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn raw_string_with_quotes_inside() {
        Test::new(
            r#"
module Main

func testRawStringWithQuotes() -> lang.str {
    """he said "hi" to me"""
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn raw_string_four_quotes() {
        Test::new(
            r#"
module Main

func testFourQuoteRawString() -> lang.str {
    """"three quotes """ inside""""
}
"#,
        )
        .expect(Compiles);
    }

    #[test]
    fn empty_raw_string() {
        Test::new(
            r#"
module Main

func testEmptyRawString() -> lang.str {
    """"""
}
"#,
        )
        .expect(Compiles);
    }
}

mod escape_errors {
    use super::*;

    #[test]
    fn invalid_escape_sequence() {
        Test::new(
            r#"
module Main

func testInvalidEscape() -> lang.str {
    "hello\qworld"
}
"#,
        )
        .expect(HasError("invalid escape sequence"));
    }

    #[test]
    fn ascii_escape_out_of_range() {
        Test::new(
            r#"
module Main

func testAsciiOutOfRange() -> lang.str {
    "\x80"
}
"#,
        )
        .expect(HasError("out of range"));
    }

    #[test]
    fn unicode_escape_missing_brace() {
        Test::new(
            r#"
module Main

func testUnicodeMissingBrace() -> lang.str {
    "\u0041"
}
"#,
        )
        .expect(HasError("invalid Unicode escape"));
    }

    #[test]
    fn unicode_escape_empty_braces() {
        Test::new(
            r#"
module Main

func testUnicodeEmptyBraces() -> lang.str {
    "\u{}"
}
"#,
        )
        .expect(HasError("invalid Unicode escape"));
    }

    #[test]
    fn unicode_escape_out_of_range() {
        Test::new(
            r#"
module Main

func testUnicodeOutOfRange() -> lang.str {
    "\u{FFFFFF}"
}
"#,
        )
        .expect(HasError("invalid Unicode escape"));
    }

    #[test]
    fn unicode_escape_too_many_digits() {
        Test::new(
            r#"
module Main

func testUnicodeTooManyDigits() -> lang.str {
    "\u{1234567}"
}
"#,
        )
        .expect(HasError("invalid Unicode escape"));
    }

    #[test]
    fn incomplete_hex_escape() {
        Test::new(
            r#"
module Main

func testIncompleteHex() -> lang.str {
    "\xG"
}
"#,
        )
        .expect(HasError("invalid escape sequence"));
    }
}

mod string_interpolation {
    use super::*;

    /// Test basic string interpolation with multiple string values.
    /// Verifies: basic \(expr) syntax, multiple interpolations in one string.
    #[test]
    fn basic_multiple_string_interpolations() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> std.num.Int64 {
    let first = "Hello";
    let second = "World";
    let third = "Kestrel";
    let _ = println("\(first), \(second)! Welcome to \(third).");
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("Hello, World! Welcome to Kestrel.\n"));
    }

    /// Test integer interpolation without format options.
    /// Verifies: integer formatting with default options, multiple integer interpolations.
    #[test]
    fn integer_interpolation_no_format() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> std.num.Int64 {
    let a = 42;
    let b = 100;
    let sum = a + b;
    let _ = println("a=\(a), b=\(b), sum=\(sum)");
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("a=42, b=100, sum=142\n"));
    }

    /// Test interpolation with width and alignment format options.
    /// Verifies: right-align (>), left-align (<), center (^), and width specifiers.
    #[test]
    fn interpolation_with_alignment_format() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> std.num.Int64 {
    let val = 42;
    let name = "test";
    // Right-align val in width 8, left-align name in width 10
    let _ = println("[\(val:>8)] [\(name:<10)] [\(val:^6)]");
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("[      42] [test      ] [  42  ]\n"));
    }

    /// Test interpolation with radix format options (hex, binary).
    /// Verifies: hex lower (x), hex upper (X), binary (b), alternate form (#).
    #[test]
    fn interpolation_with_radix_format() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> std.num.Int64 {
    let val = 255;
    // hex lower, hex upper with prefix, binary with prefix, zero-padded hex
    let _ = println("hex:\(val:x) HEX:\(val:#X) bin:\(val:#b) pad:\(val:08x)");
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("hex:ff HEX:0xFF bin:0b11111111 pad:000000ff\n"));
    }

    /// Test mixed interpolation with strings, integers, and various format options.
    /// Verifies: combining different types and format specs in one string.
    #[test]
    fn mixed_types_and_formats() {
        Test::new(
            r#"
module Main
import std.io.stdio.println

func main() -> std.num.Int64 {
    let name = "Result";
    let value = 42;
    let hex_val = 0xAB;
    // Mix strings (no format), integers with padding, and hex with prefix
    let _ = println("\(name): \(value:05) (hex: \(hex_val:#x), bin: \(value:b))");
    0
}
"#,
        )
        .with_stdlib()
        .expect(StdoutEquals("Result: 00042 (hex: 0xab, bin: 101010)\n"));
    }
}
