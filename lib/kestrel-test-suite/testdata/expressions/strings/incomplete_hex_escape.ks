// test: diagnostics
// stdlib: false

module Main

func testIncompleteHex() -> lang.str {
    "\xG" // ERROR: invalid escape sequence
}
