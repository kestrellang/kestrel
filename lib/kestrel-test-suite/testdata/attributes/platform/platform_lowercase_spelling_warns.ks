// test: diagnostics
// stdlib: false

module Test

@platform(.linux, .Darwin) // WARN: '.linux' is the old spelling; write '.Linux'
func f() {}
