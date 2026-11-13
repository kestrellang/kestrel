// test: diagnostics
// stdlib: true
//
// Each body wraps the other's opaque return, so the concrete type behind
// both is infinite (`W[W[W[…]]]`). Unification never sees it; the expanded
// concrete-type query hits a cycle and both functions are reported.

module Test

import std.numeric.(Int64)

protocol P { func v() -> Int64 }

struct W[T]: P where T: P {
    var inner: T
    func v() -> Int64 { self.inner.v() + 1 }
}

func g(n: Int64) -> some P { W(inner: h(n)) } // ERROR: opaque return type of 'g' contains itself
func h(n: Int64) -> some P { W(inner: g(n)) } // ERROR: opaque return type of 'h' contains itself
