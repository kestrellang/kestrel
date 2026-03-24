// test: diagnostics
// stdlib: true

module Test
import Prelude

@extern(.C)
func genericExtern[T](x: T) -> T // ERROR: cannot be generic
