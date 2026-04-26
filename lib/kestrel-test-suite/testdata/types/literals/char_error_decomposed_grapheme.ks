// test: diagnostics
// stdlib: false

module Test
func decomposed_e() -> lang.i32 { 'e\u{0301}' } // ERROR: character literal may only contain one codepoint
