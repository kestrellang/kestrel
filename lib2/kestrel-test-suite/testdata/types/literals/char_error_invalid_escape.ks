// test: diagnostics
// stdlib: false

module Test
func invalid() -> lang.i32 { '\q' } // ERROR: invalid escape sequence
