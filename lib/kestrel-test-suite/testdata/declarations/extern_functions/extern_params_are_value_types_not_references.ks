// test: diagnostics
// stdlib: true

module Test
import Prelude

struct MyInt: FFISafe {}

@extern(.C)
func c_add(a: MyInt, b: MyInt) -> MyInt
