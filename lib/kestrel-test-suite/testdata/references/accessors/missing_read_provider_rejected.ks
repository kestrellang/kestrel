// test: diagnostics
// stdlib: true

// A member with only a WRITE provider has no way to read: set-only and
// mutating-ref-only accessor blocks are rejected (E622). Reads always go
// through `get` or `ref`.
module Test

import std.memory.(Pointer)
import std.numeric.(Int64)

struct SetOnly {
    var stored: Int64
    subscript(at index: Int64) -> Int64 { // ERROR(E622)
        set { self.stored = newValue; }
    }
}

struct MutRefOnly {
    var p: Pointer[Int64]
    subscript(at index: Int64) -> Int64 { // ERROR(E622)
        mutating ref { self.p.offset(by: index).mutatingValue }
    }
}

struct SetOnlyProperty {
    var stored: Int64
    var value: Int64 { // ERROR(E622)
        set { self.stored = newValue; }
    }
}
