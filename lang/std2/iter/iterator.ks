// Core iterator protocol

module std.iter

import std.result.(Optional)

// Iterator - produces a sequence of values
public protocol Iterator {
    type Item
    mutating func next() -> Optional[Item]
}

// Iterable - type that can produce an iterator
public protocol Iterable {
    type Item
    type Iter: Iterator where Iter.Item = Item
    func iter() -> Iter
}

// Collectable - type that can be built from an iterator
// Note: Generic inits with where clauses may not be fully supported yet
// public protocol Collectable {
//     type Item
//     init[I](from iter: I) where I: Iterator, I.Item = Item
// }

// Functor - for map operations (can transform inner type)
// Note: Self[U] syntax requires HKT support
// public protocol Functor {
//     type Inner
//     func map[U](transform: (Inner) -> U) -> Self[U]
// }
