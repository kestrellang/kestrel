// test: diagnostics
// stdlib: true

module Test

struct MyInt: FFISafe {}

@extern(.C)
func c_add(a: MyInt, b: MyInt) -> MyInt
