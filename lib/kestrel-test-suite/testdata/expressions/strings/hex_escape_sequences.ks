// test: diagnostics
// stdlib: false

module Main

func testHexEscape() -> lang.str { "\x00\x41\x7F" }
func testHexMixedWithText() -> lang.str { "A\x42C" }
