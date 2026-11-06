// test: diagnostics
// stdlib: true
//
// E146: an extension member called on `any P` runs with the box as `Self`,
// so `any P: P` must hold. A static requirement can't be answered by a box
// with no payload type, so the call is rejected at type-check time instead
// of failing after monomorphization (design §6).

module Test

import std.numeric.(Int64)

protocol P {
    var base: Int64 { get }
    static func seed() -> Int64
}

struct W: P {
    var base: Int64
    static func seed() -> Int64 { 1 }
}

extend P {
    func twice() -> Int64 { self.base * 2 }
}

func test() {
    let x: any P = W(base: 3);
    let r = x.twice(); // ERROR: 'any P' does not conform to 'P': requirement 'seed' cannot be called through an existential
}
