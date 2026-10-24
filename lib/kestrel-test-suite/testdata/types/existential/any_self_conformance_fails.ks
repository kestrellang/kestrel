// test: diagnostics
// stdlib: true
//
// E146: `any P: P` only holds when every requirement is dispatchable. `==`
// takes `Self`, so `any Equatable` can't be passed where `T: Equatable` is
// required (design §6).

module Test

import std.numeric.(Int64)
import std.core.(Bool, Equatable)

struct A { var v: Int64; }
extend A: Equatable {
    public func isEqual(to other: A) -> Bool { self.v == other.v }
}

func needsEq[T](x: T) -> Int64 where T: Equatable { 0 }

func test() {
    let e: any Equatable = A(v: 1);
    let r = needsEq(e); // ERROR: 'any Equatable' does not conform to 'Equatable': requirement 'isEqual' cannot be called through an existential
}
