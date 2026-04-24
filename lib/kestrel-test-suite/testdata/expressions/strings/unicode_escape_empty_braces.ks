// test: diagnostics
// stdlib: false

module Main

func testUnicodeEmptyBraces() -> lang.str {
    "\u{}" // ERROR: invalid Unicode escape
}
