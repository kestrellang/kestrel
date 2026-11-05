// test: diagnostics
// stdlib: true

// A protocol may require `ref` / `mutating ref` with bodyless clauses
// (protocol_ref_requirement.ks), and a protocol extension may give a ref
// accessor body (a default), but a body in the protocol itself is rejected.
module Test

import std.memory.(Pointer)
import std.numeric.(Int64)

protocol Viewable {
    var item: Int64 { // ERROR(E621)
        ref { 0 }
    }
}

extend Viewable {
    subscript(at index: Int64) -> Int64 {
        ref { self.item }
    }
}
