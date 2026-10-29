// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

// Members declared in `extend P` are callable on `any P` when `any P: P`
// holds: the box stands in for `Self` (design §6), and the body's
// requirement uses — including a property requirement — forward to the
// payload. These used to type-check and then fail post-mono ("does not
// implement 'twice'", then "does not implement 'base'").

protocol P {
    var base: Int64 { get }
}

struct W: P {
    var base: Int64
}

extend P {
    func twice() -> Int64 { self.base * 2 }
    var doubled: Int64 { self.base * 2 }
    subscript(times n: Int64) -> Int64 { self.base * n }
}

@main
func main() -> lang.i64 {
    let x: any P = W(base: 3);
    if x.twice() != 6 { return 1 }
    if x.doubled != 6 { return 2 }
    if x(times: 4) != 12 { return 3 }
    0
}
