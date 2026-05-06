// test: diagnostics
// stdlib: false

module Main

func testUnderIndented() -> lang.str {
    """
    hello
  short // ERROR: less indented than closing delimiter
    """
}
