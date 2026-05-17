// test: diagnostics
// stdlib: false

module Test
func two_chars() -> lang.i32 { 'ab' } // ERROR: character literal may only contain one codepoint
