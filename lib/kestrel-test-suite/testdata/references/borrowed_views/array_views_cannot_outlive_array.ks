// test: diagnostics
// stdlib: true

// Array views (chunks, windows, reversed, split, refs) borrow the array,
// so they can't be returned past the array's lifetime.

module Test

import std.collections.(Array, ChunksView, ReversedView, ArrayRefsIterator)

func chunksOfLocal() -> ChunksView[Int64] {
    let xs = [1, 2, 3];
    xs.chunks(of: 2) // ERROR(E494)
}

func reversedOfLocal() -> ReversedView[Int64] {
    let xs = [1, 2, 3];
    xs.reversed // ERROR(E494)
}

func refsOfLocal() -> ArrayRefsIterator[Int64] {
    let xs = [1, 2, 3];
    xs.refs() // ERROR(E494)
}

func chunksOfParam(xs: Array[Int64]) -> ChunksView[Int64] {
    xs.chunks(of: 2)
}
