// test: diagnostics
// stdlib: true

// Scope restriction (stage 1.5): ref accessors are legal only on
// concrete inherent declarations — rejected in protocols and protocol
// extensions (witness ref-returns are out of scope).
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
