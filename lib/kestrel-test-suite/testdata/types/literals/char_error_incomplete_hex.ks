// test: diagnostics
// stdlib: false

module Test
func incomplete() -> lang.i32 { '\x4' } // ERROR: invalid escape sequence
