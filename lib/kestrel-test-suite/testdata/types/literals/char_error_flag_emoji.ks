// test: diagnostics
// stdlib: false

module Test
func flag() -> lang.i32 { '🇺🇸' } // ERROR: character literal may only contain one codepoint
