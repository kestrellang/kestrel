// test: diagnostics
// stdlib: true

module Test
import Prelude

struct MyInt: FFISafe {}

@extern(.C)
func explicit_consuming(consuming x: MyInt) -> MyInt
