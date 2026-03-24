// test: diagnostics
// stdlib: false

module Main

func testInvalidEscape() -> lang.str {
    "hello\qworld" // ERROR: invalid escape sequence
}
