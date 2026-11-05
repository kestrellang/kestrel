// test: diagnostics
// stdlib: true

// A protocol may require `ref` / `mutating ref` with bodyless clauses
// (protocol_ref_requirement.ks), but ref accessor BODIES stay concrete-only:
// rejected in protocols and protocol extensions.
module Test

import std.memory.(Pointer)
import std.numeric.(Int64)

protocol Viewable {
    var item: Int64 { // ERROR(E621)
        ref { 0 }
    }
}

extend Viewable {
    subscript(at index: Int64) -> Int64 { // ERROR(E621)
        ref { self.item }
    }
}
