// test: diagnostics
// stdlib: true

module Test

@extern(.C)
func genericExtern[T](x: T) -> T // ERROR: cannot be generic
