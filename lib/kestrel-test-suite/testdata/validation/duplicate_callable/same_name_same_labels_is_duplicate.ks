// test: diagnostics
// stdlib: false

module Test
func process(x: ()) { }
func process(x: ()) { } // ERROR: duplicate function signature
