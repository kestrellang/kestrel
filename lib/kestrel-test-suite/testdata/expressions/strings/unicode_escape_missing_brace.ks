// test: diagnostics
// stdlib: false

module Main

func testUnicodeMissingBrace() -> lang.str {
    "\u0041" // ERROR: invalid Unicode escape
}
