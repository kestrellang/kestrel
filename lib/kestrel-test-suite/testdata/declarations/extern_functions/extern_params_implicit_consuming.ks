// test: diagnostics
// stdlib: true

module Test

struct MyInt: FFISafe {}

// No explicit 'consuming' keyword, but params should still be value types
@extern(.C)
func implicit_consuming(x: MyInt) -> MyInt
