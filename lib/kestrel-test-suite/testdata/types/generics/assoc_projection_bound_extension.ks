// test: execution
// stdlib: true
// expect-exit: 0

// #185: an extension `where` clause with an associated-type-projection bound
// (`extend Box[T]: Show where T: Producer, T.Item: Show`). Collapsing `T.Item`
// to the bare `Item` entity made the frontend reject the conformance with
// "no associated type 'Item'"; the projection bound must keep its base so the
// clause resolves and the extension's method is usable.

module Test

import std.text.String
import std.numeric.Int64

protocol Show { func show() -> String }
extend Int64: Show { public func show() -> String { "i" } }

protocol Producer {
    type Item
    func produce() -> Item
}

struct IntSrc { var v: Int64; }
extend IntSrc: Producer {
    public type Item = Int64;
    public func produce() -> Int64 { self.v }
}

struct Box[T] { var inner: T; }

extend Box[T]: Show where T: Producer, T.Item: Show {
    public func show() -> String { "x" }
}

@main
func main() -> lang.i32 {
    if Box(inner: IntSrc(v: 9)).show() != "x" { return 10 }
    0
}
