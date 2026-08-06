// test: execution
//
// Bound-driven literal pinning: with one concrete conformance per element
// type, the bound `R: Tagged[Int8]` uniquely selects `Box[Int8]`, so the
// int literal flowing into `Box(item: 41)` must be pinned to Int8 by the
// bound instead of defaulting to Int64 (which would select Box[Int64] and
// fail witness lookup). See parameterized_bound_literal_pinning.ks for the
// range-literal variant, which is still blocked on operator dispatch.

module Test

import std.numeric.(Int8, Int64)

protocol Tagged[B] {
    func tag() -> B
}

struct Box[T] {
    var item: T;
}

extend Box[Int64]: Tagged[Int64] {
    public func tag() -> Int64 { self.item }
}

extend Box[Int8]: Tagged[Int8] {
    public func tag() -> Int8 { self.item }
}

func readTag8[R](source: R) -> Int8 where R: Tagged[Int8] {
    source.tag()
}

@main
func main() -> lang.i64 {
    if readTag8(Box(item: 41)) != 41 { return 1 }
    0
}
