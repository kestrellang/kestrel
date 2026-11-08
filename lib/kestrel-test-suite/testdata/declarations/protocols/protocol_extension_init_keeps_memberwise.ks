// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

// A convenience init in a protocol extension is found on a concrete
// conformer (`W(tripled:)`), but it is only a fallback: it must not take
// away a struct's memberwise init (`W(base:)`), and a type's own init with
// the same labels wins over it.

protocol P {
    var base: Int64 { get }
    static func make(n: Int64) -> Self
}

extend P {
    init(tripled n: Int64) { self = Self.make(n * 3) }
}

struct W: P {
    var base: Int64
    static func make(n: Int64) -> W { W(base: n) }
}

struct Own: P {
    var base: Int64
    init(base b: Int64) { self.base = b; }
    init(tripled n: Int64) { self.base = n * 100; }
    static func make(n: Int64) -> Own { Own(base: n) }
}

@main
func main() -> lang.i64 {
    if W(base: 4).base != 4 { return 1 }
    if W(tripled: 2).base != 6 { return 2 }
    if Own(tripled: 2).base != 200 { return 3 }
    0
}
