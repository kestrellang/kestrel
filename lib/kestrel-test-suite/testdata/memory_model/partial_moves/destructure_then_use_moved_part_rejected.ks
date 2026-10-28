// test: diagnostics
// stdlib: true

// After `let (x, _) = t`, `t.0` is moved and `t` is partially moved (E500).
module Test

import std.numeric.(Int64)

struct R: not Copyable { var id: Int64 }

func take(consuming r: R) -> Int64 { r.id }

func usedAfter() -> Int64 {
    let t = (R(id: 1), R(id: 10));
    let (x, _) = t;
    take(t.0) + take(x) // ERROR(E500)
}

func wholeAfter() -> Int64 {
    let t = (R(id: 1), R(id: 10));
    let (.., y) = t;
    let u = t; // ERROR(E500)
    take(y)
}
