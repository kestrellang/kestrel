// test: diagnostics
// stdlib: true

module Test
import Prelude

struct IntA: FFISafe {}
struct IntB: FFISafe {}
struct FloatA: FFISafe {}
struct FloatB: FFISafe {}

@extern(.C)
func doStuff(a: IntA, b: IntB, c: FloatA, d: FloatB) -> IntA
