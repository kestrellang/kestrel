// test: diagnostics
// stdlib: false

module Main

func testUnicodeTooManyDigits() -> lang.str {
    "\u{1234567}" // ERROR: invalid Unicode escape
}
