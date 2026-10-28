// test: diagnostics
// stdlib: true

module Test

struct MyInt: FFISafe {}

@extern(.C)
func getPoint(coords: (MyInt, MyInt)) -> MyInt
