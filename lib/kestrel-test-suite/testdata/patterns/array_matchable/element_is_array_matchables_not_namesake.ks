// test: execution
// stdlib: true
// expect-exit: 0
//
// An array pattern binds elements at `ArrayMatchable.Element` — the tagged
// associated type — in inference AND in MIR's element-type lookup, never a
// same-named `Element` from another conformance. `Pair` also conforms to
// `Shelf`, whose own `Element` is `String`.

module Test

import std.core.ArrayMatchable
import std.memory.UnsafeSlice

protocol Shelf {
    type Element
    func label() -> Element
}

struct Pair {
    var a: Int64
    var b: Int64
}

extend Pair: Shelf {
    type Shelf.Element = String
    func label() -> String { "pair" }
}

extend Pair: unsafe ArrayMatchable {
    type ArrayMatchable.Element = Int64
    type ArrayMatchable.Rest = UnsafeSlice[Int64]
    public func matchLength() -> Int64 { 2 }
    public func matchGet(index: Int64) -> Int64 {
        if index == 0 { self.a } else { self.b }
    }
    public func matchSlice(from: Int64, to: Int64) -> UnsafeSlice[Int64] {
        unsafe UnsafeSlice(pointer: Pointer(to: self.a).offset(by: from), count: to - from)
    }
}

func sum(p: Pair) -> Int64 {
    match p {
        [x, y] => x * 10 + y,
        _ => -1
    }
}

@main
func main() -> lang.i32 {
    if sum(Pair(a: 4, b: 2)) != 42 { return 1 }
    0
}
