// test: diagnostics
// stdlib: false

module Test
func three_chars() -> lang.i32 { 'abc' } // ERROR: character literal may only contain one codepoint
