// test: execution
// stdlib: true
// expect-exit: 0

// "A `mutating` parameter inside the callback type, such as `(mutating T) -> R`,
// controls access to the callback's ARGUMENT and does not by itself make the
// closure kind `mutating`" (docs/design/closures-stdlib-audit.md — "Normal").
// The closure below is normal-kind: bound with `let`, freely copyable, callable
// through a plain (non-`mutating`) parameter — while its own parameter is
// passed mutably and written in place.
module Test

import std.numeric.(Int64)

struct Counter { var n: Int64 }

func apply(mutating c: Counter, with f: (mutating Counter) -> Int64) -> Int64 {
    f(c)
}

@main
func main() -> lang.i64 {
    var c = Counter(n: 10);

    let f: (mutating Counter) -> Int64 = { (x) in x.n = x.n + 3; x.n };
    // Normal kind is Copyable — copying the value is not a move.
    let g = f;

    if apply(c, with: f) != 13 { return 1 }
    if apply(c, with: g) != 16 { return 2 }
    if c.n != 16 { return 3 }

    0
}
