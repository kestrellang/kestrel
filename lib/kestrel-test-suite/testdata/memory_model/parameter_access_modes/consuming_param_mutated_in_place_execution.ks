// test: execution
// stdlib: true
// expect-exit: 0

// A `consuming` param is owned and mutable. A mutating method call, a
// compound assignment or a `mutating` argument on it must change the
// param itself. The writes used to land on a throwaway copy when the
// value was passed as a scalar (one-field struct), so later reads saw
// the old value.

module Test

import std.numeric.Int64

struct Counter { var n: Int64 }

extend Counter {
    mutating func bump() { self.n += 1; }
}

func bumpTwice(mutating c: Counter) { c.bump(); c.bump(); }

func viaMethod(consuming c: Counter) -> Int64 { c.bump(); c.n }
func viaCompound(consuming c: Counter) -> Int64 { c.n += 1; c.n }
func viaArg(consuming c: Counter) -> Int64 { bumpTwice(c); c.n }
func viaReturn(consuming c: Counter) -> Counter { c.bump(); c }

@main
func main() -> Int64 {
    if viaMethod(Counter(n: 1)) != 2 { return 1; }
    if viaCompound(Counter(n: 1)) != 2 { return 2; }
    if viaArg(Counter(n: 1)) != 3 { return 3; }
    if viaReturn(Counter(n: 1)).n != 2 { return 4; }
    0
}
