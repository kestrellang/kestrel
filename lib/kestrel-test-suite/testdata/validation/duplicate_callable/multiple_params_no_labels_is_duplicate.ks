// test: diagnostics
// stdlib: false

module Test
func add(a: (), b: ()) { }
func add(x: (), y: ()) { } // ERROR: duplicate function signature
