// test: diagnostics
// stdlib: false

module Test
func family() -> lang.i32 { '👨‍👩‍👧' } // ERROR: character literal may only contain one codepoint
