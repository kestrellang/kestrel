// test: diagnostics
// stdlib: false

module Test
func empty() -> lang.i32 { '' } // ERROR: empty character literal
