// test: diagnostics
// stdlib: false

module Test

func process() { }
func process() { } // ERROR: duplicate function signature
