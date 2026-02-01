// Core iterator protocols for lazy sequence processing

module std.iter

import std.result.(Optional)

// ============================================================================
// CORE PROTOCOLS
// ============================================================================

/// Protocol for types that produce a sequence of values.
///
/// Iterators are the foundation of lazy sequence processing in Kestrel.
/// They produce elements one at a time via the `next()` method until exhausted.
///
/// Example:
///     struct CountDown: Iterator {
///         type Item = Int64
///         var current: Int64
///
///         mutating func next() -> Int64? {
///             if current <= 0 { return None }
///             let value = current
///             current -= 1
///             return Some(value)
///         }
///     }
@builtin(.IteratorProtocol)
public protocol Iterator {
    /// The type of elements yielded by this iterator.
    type Item

    /// Returns the next element, or None if the sequence is exhausted.
    ///
    /// Once None is returned, subsequent calls should continue to return None.
    @builtin(.IteratorNextMethod)
    mutating func next() -> Item?
}

/// Protocol for types that can produce an iterator.
///
/// Enables for-in loops and other iteration constructs.
/// Collections typically implement Iterable to allow iteration over their elements.
///
/// Example:
///     for item in myCollection {
///         // item is each element from myCollection.iter()
///     }
@builtin(.IterableProtocol)
public protocol Iterable {
    /// The type of elements produced by iteration.
    type Item

    /// The type of iterator that will be produced.
    type Iter: Iterator where Iter.Item = Item

    /// Creates an iterator over this collection's elements.
    @builtin(.IterableIterMethod)
    func iter() -> Iter
}

// ============================================================================
// ITERATOR IS ITERABLE
// ============================================================================

/// Extension making all Iterators also Iterable.
///
/// An iterator can serve as its own iterable, returning itself.
/// This allows iterators to be used directly in for-in loops.
extend Iterator: Iterable {
    type Iterable.Item = Self.Item
    type Iterable.Iter = Self

    /// Returns self, allowing an iterator to be used where an iterable is expected.
    func iter() -> Self { self }
}

// ============================================================================
// FUTURE PROTOCOLS (not yet implemented)
// ============================================================================

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
