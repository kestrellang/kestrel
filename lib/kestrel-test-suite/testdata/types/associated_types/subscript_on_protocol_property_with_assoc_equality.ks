// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: 2 9\n

// Subscripting a protocol property whose type is an associated type pinned
// by a where clause (`H.Item = Array[Int64]`): `h.item(1)` should dispatch
// Array's subscript. Today the subscript call keeps the unresolved
// projection's (empty) type args and fails post-mono verification.
module Test

import std.collections.(Array)

protocol Holder {
    type Item
    var item: Item { ref mutating ref }
}

struct Bag: Holder {
    type Item = Array[Int64]
    var item: Array[Int64]
}

func peek[H](h: &H) -> Int64 where H: Holder, H.Item = Array[Int64] { h.item(1) }

func poke[H](h: &mutating H) where H: Holder, H.Item = Array[Int64] { h.item(0) = 9 }

@main
func main() -> lang.i64 {
    var b = Bag(item: [1, 2]);
    poke(b);
    print("\(peek(b)) \(b.item(0))\n");
    0
}
