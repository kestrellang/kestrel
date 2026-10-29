// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

// Calling a value of generic type is a subscript, not an init: `t(2)` with
// `t: T` used to dispatch as `T.init(2)` (E102 "no method 'init'"), because
// `T` and `t: T` have the same type. Covers a subscript requirement, an
// extension subscript, `some P`, `self(...)` inside a protocol extension,
// and that `T(...)` still constructs.

protocol P {
    var base: Int64 { get }
    init(base b: Int64)
    subscript(n: Int64) -> Int64 { get }
}

extend P {
    subscript(times n: Int64) -> Int64 { self.base * n }
    func viaSelf() -> Int64 { self(times: 2) + self(1) }
}

struct W: P {
    var base: Int64
    init(base b: Int64) { self.base = b; }
    subscript(n: Int64) -> Int64 { self.base + n }
}

func requirement[T](t: T) -> Int64 where T: P { t(2) }
func extensionMember[T](t: T) -> Int64 where T: P { t(times: 2) }
func opaque(t: some P) -> Int64 { t(times: 3) }
func construct[T]() -> T where T: P { T(base: 9) }

@main
func main() -> lang.i64 {
    let w = W(base: 3);
    if requirement(w) != 5 { return 1 }
    if extensionMember(w) != 6 { return 2 }
    if opaque(w) != 9 { return 3 }
    if w.viaSelf() != 10 { return 4 }
    let made: W = construct[W]();
    if made.base != 9 { return 5 }
    0
}
