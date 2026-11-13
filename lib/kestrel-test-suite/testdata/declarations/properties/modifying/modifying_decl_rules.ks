// test: diagnostics
// stdlib: true

// A member has exactly one read provider, and `modifying` accessors need a
// `self` to modify.
module Test

import std.memory.(Pointer)
import std.numeric.(Int64)

struct TwoGets {
    var n: Int64
    var value: Int64 { // ERROR(E619): duplicate read provider
        get { self.n }
        modifying get { self.n }
    }
}

struct RefAndModifyingRef {
    var p: Pointer[Int64]
    subscript(i: Int64) -> Int64 { // ERROR(E619): duplicate read provider
        ref { unsafe self.p.value }
        modifying ref { unsafe self.p.value }
    }
}

struct Statics {
    static var count: Int64 { // ERROR(E636): `modifying` accessors need an instance receiver
        modifying get { 1 }
    }
}

var global: Int64 { // ERROR(E636): `modifying` accessors need an instance receiver
    modifying get { 1 }
}

protocol Named {
    var name: Int64 { get }
}

extend Named {
    var cached: Int64 { // ERROR(E621): a protocol requires `ref` / `mutating ref` with bodyless clauses
        modifying get { 1 }
    }
}
