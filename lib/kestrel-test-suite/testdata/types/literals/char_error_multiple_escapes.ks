// test: diagnostics
// stdlib: false

module Test
func two_escapes() -> lang.i32 { '\n\t' } // ERROR: character literal may only contain one codepoint
