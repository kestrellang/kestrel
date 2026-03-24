// test: diagnostics
// stdlib: false

module Main

func testAsciiOutOfRange() -> lang.str {
    "\x80" // ERROR: out of range
}
