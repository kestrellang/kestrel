// test: execution
// stdlib: true
// expect-exit: 0
//
// #148: a generic function's default-parameter expression that calls a static
// requirement on its own type param (`x: T = T.makeDefault()`) is inline-lowered
// into the caller. The default body is inferred against the generic `T`, so its
// types must be substituted to the call site's concrete type arg — otherwise the
// callee's `TypeParam` leaks into the (non-generic) caller and survives to the
// mangler ("TypeParam(...) in value").

module Test

import std.numeric.Int64

protocol Defaultable {
    static func makeDefault() -> Self
}

extend Int64: Defaultable {
    static func makeDefault() -> Int64 { 42 }
}

struct Tag: Defaultable {
    var v: Int64;
    static func makeDefault() -> Tag { Tag(v: 7) }
}

func pick[T](x x: T = T.makeDefault()) -> T where T: Defaultable { x }

@main
func main() -> lang.i32 {
    let a: Int64 = pick();      // default fires: T = Int64 -> 42
    if a != 42 { return 1 }
    let b: Int64 = pick(x: 5);  // explicit arg still works
    if b != 5 { return 2 }
    let c: Tag = pick();        // distinct conformer: T = Tag -> Tag(v: 7)
    if c.v != 7 { return 3 }
    0
}
