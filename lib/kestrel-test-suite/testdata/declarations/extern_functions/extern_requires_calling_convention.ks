// test: diagnostics
// stdlib: true

module Test

struct MyInt: FFISafe {}

@extern
func noConvention(x: MyInt) -> MyInt // ERROR: calling convention
