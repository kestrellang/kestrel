// test: diagnostics
// stdlib: true

module Test
import Prelude

struct MyInt: FFISafe {}

@extern(.C)
func getPoint(coords: (MyInt, MyInt)) -> MyInt
