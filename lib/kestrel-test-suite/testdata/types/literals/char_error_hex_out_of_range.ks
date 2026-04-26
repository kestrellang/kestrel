// test: diagnostics
// stdlib: false

module Test
func out_of_range() -> lang.i32 { '\xFF' } // ERROR: ASCII escape
