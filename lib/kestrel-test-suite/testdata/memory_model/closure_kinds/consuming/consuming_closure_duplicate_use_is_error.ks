// test: diagnostics
// stdlib: true

// docs/design/closures.md, kind table: `consuming` closures are `not Copyable`.
// Binding one twice is a duplication of a unique owner, so the second read of
// the source binding is an ordinary use-after-move (E500) — the same rule that
// governs any other non-Copyable value.
module Test

import std.numeric.Int64

func callIt(consuming f: consuming () -> Int64) -> Int64 { f() }

func main() -> lang.i64 {
    let n: Int64 = 1;
    let g: consuming () -> Int64 = { n };
    let first = g;
    let second = g; // ERROR(E500)
    let _ = callIt(first);
    0
}
