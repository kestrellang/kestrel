// test: diagnostics
// stdlib: false

module Test
func surrogate() -> lang.i32 { '\u{D800}' } // ERROR: invalid Unicode escape
