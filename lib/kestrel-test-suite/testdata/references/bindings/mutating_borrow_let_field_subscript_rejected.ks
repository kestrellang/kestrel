// test: diagnostics
// stdlib: true

// The field-subscript place is only mutable through a `var` field of a
// mutable base.
module Test

import std.numeric.(Int64)
import std.collections.(Array)

struct Bag {
    let items: Array[Int64]
    var more: Array[Int64]

    mutating func touch() {
        let r = &mutating self.items(at: 0); // ERROR(E210): cannot take a `&mutating` borrow of immutable field 'items'
    }

    func look() {
        let r = &mutating self.more(at: 0); // ERROR(E210): cannot take a `&mutating` borrow of immutable variable 'self'
    }
}
