// test: diagnostics
// stdlib: true

module Test

struct MyInt: FFISafe {}

@extern(.C)
func hasBody(x: MyInt) -> MyInt { x } // ERROR: cannot have a body
