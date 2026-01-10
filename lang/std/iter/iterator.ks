// Core iterator protocol

module std.iter

import std.result.(Optional)

public protocol Iterator {
    type Item
    func next() -> Optional[Item]
}

// Type that can produce an iterator
public protocol Iterable {
    type Item
    type Iter: Iterator where Iter.Item = Item
    func iter() -> Iter
}

// Type that can be built from an iterator
public protocol Collectable {
    type Item
    init[I](from iter: I) where I: Iterator, I.Item = Item
}

// Functor protocol - for map operations
public protocol Functor {
    type Inner
    func map[U](transform: (Inner) -> U) -> Self[U]
}

// TODO: Protocol extensions not yet supported
// Iterable automatically gets all Iterator extension methods
// extend Iterable {
//     // Transform
//     public func map[U](transform: (Item) -> U) -> MapIterator[Iter, U] {
//         self.iter().map(transform)
//     }
//
//     public func filter(predicate: (Item) -> Bool) -> FilterIterator[Iter] {
//         self.iter().filter(predicate)
//     }
//
//     public func filterMap[U](transform: (Item) -> Optional[U]) -> FilterMapIterator[Iter, U] {
//         self.iter().filterMap(transform)
//     }
//
//     public func flatMap[U](transform: (Item) -> I) -> FlatMapIterator[Iter, I]
//         where I: Iterable, I.Item = U
//     {
//         self.iter().flatMap(transform)
//     }
//
//     // Take and skip
//     public func take(count: Int) -> TakeIterator[Iter] {
//         self.iter().take(count)
//     }
//
//     public func takeWhile(predicate: (Item) -> Bool) -> TakeWhileIterator[Iter] {
//         self.iter().takeWhile(predicate)
//     }
//
//     public func skip(count: Int) -> SkipIterator[Iter] {
//         self.iter().skip(count)
//     }
//
//     public func skipWhile(predicate: (Item) -> Bool) -> SkipWhileIterator[Iter] {
//         self.iter().skipWhile(predicate)
//     }
//
//     // Combine
//     public func enumerate() -> EnumerateIterator[Iter] {
//         self.iter().enumerate()
//     }
//
//     public func zip[Other](with other: Other) -> ZipIterator[Iter, Other] where Other: Iterator {
//         self.iter().zip(with: other)
//     }
//
//     public func chain[Other](other: Other) -> ChainIterator[Iter, Other]
//         where Other: Iterator, Other.Item = Item
//     {
//         self.iter().chain(other)
//     }
//
//     // Peek
//     public func peekable() -> PeekableIterator[Iter] {
//         self.iter().peekable()
//     }
//
//     // Consuming operations
//     public func fold[Acc](initial: Acc, combine: (Acc, Item) -> Acc) -> Acc {
//         self.iter().fold(initial: initial, combine: combine)
//     }
//
//     public func reduce(combine: (Item, Item) -> Item) -> Optional[Item] {
//         self.iter().reduce(combine)
//     }
//
//     public func collect[C]() -> C where C: Collectable, C.Item = Item {
//         self.iter().collect()
//     }
//
//     public func count() -> Int {
//         self.iter().count()
//     }
//
//     public func forEach(action: (Item) -> ()) {
//         self.iter().forEach(action)
//     }
//
//     public func any(predicate: (Item) -> Bool) -> Bool {
//         self.iter().any(predicate)
//     }
//
//     public func all(predicate: (Item) -> Bool) -> Bool {
//         self.iter().all(predicate)
//     }
//
//     public func find(predicate: (Item) -> Bool) -> Optional[Item] {
//         self.iter().find(predicate)
//     }
//
//     public func position(predicate: (Item) -> Bool) -> Optional[Int] {
//         self.iter().position(predicate)
//     }
//
//     public func first() -> Optional[Item] {
//         self.iter().next()
//     }
//
//     public func last() -> Optional[Item] {
//         self.iter().last()
//     }
//
//     public func nth(n: Int) -> Optional[Item] {
//         self.iter().nth(n)
//     }
//
//     public func min() -> Optional[Item] where Item: Comparable {
//         self.iter().min()
//     }
//
//     public func max() -> Optional[Item] where Item: Comparable {
//         self.iter().max()
//     }
//
//     public func sum() -> Item where Item: Addable[Item], Item: Numeric {
//         self.iter().sum()
//     }
//
//     public func product() -> Item where Item: Multipliable[Item], Item: Numeric {
//         self.iter().product()
//     }
// }
