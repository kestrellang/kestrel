// test: diagnostics

// `.C` is the only calling convention a function type takes.

module Test

type Wrong = @extern(.Swift) (Int64) -> () // ERROR: expected `@extern(.C)` (the only convention a function type takes), found '@'

@main
func main() {}
