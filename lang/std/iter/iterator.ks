// Core iterator protocol

module std.iter

import std.result.(Optional)

// Iterator - produces a sequence of values
@builtin(.IteratorProtocol)
public protocol Iterator {
    type Item
    @builtin(.IteratorNextMethod)
    mutating func next() -> Optional[Item]
}

// Iterable - type that can produce an iterator
@builtin(.IterableProtocol)
public protocol Iterable {
    type Item
    type Iter: Iterator where Iter.Item = Item
    @builtin(.IterableIterMethod)
    func iter() -> Iter
}

// TODO: Make Iterator conform to Iterable (an iterator is itself iterable)
// This extension requires better Self.Item support in protocol extensions
// extend Iterator: Iterable {
//     type Iterable.Item = Self.Item
//     type Iterable.Iter = Self
//     func iter() -> Self { self }
// }

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
