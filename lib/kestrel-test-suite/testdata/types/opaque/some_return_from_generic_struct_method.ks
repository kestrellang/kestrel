// test: execution
// stdlib: true
// expect-exit: 0

// #183: a method on a GENERIC struct that returns `some P` with an underlying
// type mentioning the STRUCT's type param. The opaque is resolved at mono by
// substituting the origin's type params into its concrete underlying type —
// but only the method's OWN params were collected, so the enclosing struct's
// `T` (the underlier here) leaked as a `TypeParam` past monomorphization (ICE).
// The equivalent free-function form already worked; this aligns the method case.

module Test

import std.numeric.Int64

protocol Counter { func count() -> Int64 }

struct Fixed { var n: Int64; }
extend Fixed: Counter { public func count() -> Int64 { self.n } }

struct Factory[T] where T: Counter {
    var proto: T;
    func makeOne() -> some Counter { self.proto }   // underlier = T (the struct param)
}

@main
func main() -> lang.i32 {
    let c = Factory(proto: Fixed(n: 5)).makeOne();
    if c.count() != 5 { return 10 }

    // A second instantiation with a different concrete T, to confirm the
    // container param is substituted per-instantiation, not fixed.
    let d = Factory(proto: Fixed(n: 8)).makeOne();
    if d.count() != 8 { return 11 }
    0
}
