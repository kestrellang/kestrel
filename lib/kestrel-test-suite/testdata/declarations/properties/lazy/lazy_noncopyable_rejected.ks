// test: diagnostics
// stdlib: true

// A `lazy` property returns a copy of its cached value on every read, so
// its type must be Copyable (E639).
module Test

import std.numeric.(Int64)

struct Handle: not Copyable {
    var id: Int64
}

struct Holder {
    lazy var handle: Handle = Handle(id: 1) // ERROR(E639): a `lazy` property's type must be Copyable
    lazy var count: Int64 = 2
}
