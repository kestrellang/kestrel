// test: diagnostics
// stdlib: true

module Test
import Prelude

struct MyInt: FFISafe {}

@extern
func noConvention(x: MyInt) -> MyInt // ERROR: calling convention
