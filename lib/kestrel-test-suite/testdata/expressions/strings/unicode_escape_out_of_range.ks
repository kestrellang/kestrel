// test: diagnostics
// stdlib: false

module Main

func testUnicodeOutOfRange() -> lang.str {
    "\u{FFFFFF}" // ERROR: invalid Unicode escape
}
