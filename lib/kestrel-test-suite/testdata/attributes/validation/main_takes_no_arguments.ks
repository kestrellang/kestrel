// test: diagnostics
// stdlib: false

module Test
@main(1) // ERROR: '@main' takes no positional arguments
func entry() {}
