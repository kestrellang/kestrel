// test: diagnostics
// stdlib: true

module Test

struct MyInt: FFISafe {}

@extern(.C)
func explicit_consuming(consuming x: MyInt) -> MyInt
