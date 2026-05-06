// test: diagnostics
// stdlib: false

module Main

func testMissingLeadingNewline() -> lang.str {
    """no newline after opener""" // ERROR: must be followed by a newline
}
