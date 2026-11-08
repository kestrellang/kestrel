// test: diagnostics
// stdlib: false

module Test
@extern(.C)
@main // ERROR: '@main' cannot be combined with '@extern': the entry point must be a Kestrel function
func entry()
