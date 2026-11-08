// test: diagnostics
// stdlib: true

// A `{ get }` requirement promises a read that leaves `self` alone, so a
// `modifying` read provider cannot satisfy it.
module Test

import std.numeric.(Int64)

protocol Sized2 {
    var size: Int64 { get }
}

struct Lazy: Sized2 { // ERROR: 'size' satisfies a `{ get }` requirement with a `modifying` accessor
    var n: Int64
    var size: Int64 {
        modifying get {
            self.n += 1;
            self.n
        }
    }
}
