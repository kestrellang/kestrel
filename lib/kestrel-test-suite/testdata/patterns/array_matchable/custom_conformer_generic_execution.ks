// test: execution
// stdlib: true
// expect-exit: 0
//
// #188 follow-up: array patterns work on a GENERIC user `ArrayMatchable`
// conformer. Pins the type-arg substitution path end-to-end: the `Element = T`
// binding must substitute `T -> Int64` (a regression here leaked `Bag.T` to the
// monomorphizer's mangler). Exercises prefix+rest binding and the rest slice.

module Test

import std.core.ArrayMatchable
import std.memory.ArraySlice
import std.collections.Array

struct Bag[T] {
    var items: Array[T]
}

extend Bag[T]: ArrayMatchable {
    type Element = T
    public func matchLength() -> Int64 { self.items.count }
    public func matchGet(index: Int64) -> T { self.items(index) }
    public func matchSlice(from: Int64, to: Int64) -> ArraySlice[T] {
        self.items(from..<to)
    }
}

func describe(b: Bag[Int64]) -> Int64 {
    match b {
        [first, ..rest] => first * 10 + rest.count,
        [] => -1
    }
}

@main
func main() -> lang.i32 {
    if describe(Bag(items: [7, 8, 9])) != 72 { return 1 } // first=7, rest=[8,9] count 2
    if describe(Bag(items: [5])) != 50 { return 2 }       // first=5, rest=[] count 0
    if describe(Bag(items: [])) != -1 { return 3 }
    0
}
