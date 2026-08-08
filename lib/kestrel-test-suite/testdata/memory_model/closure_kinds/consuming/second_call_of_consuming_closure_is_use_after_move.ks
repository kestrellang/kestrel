// test: diagnostics
// stdlib: true

// docs/design/closures.md, Diagnostics table: "E500 ... also fires on ...
// calling a consumed `consuming` closure". A `consuming` closure is callable
// exactly once — the call consumes the value — so the second call is an
// ordinary use-after-move rather than a bespoke error.
module Test

import std.numeric.Int64

func runTwice(consuming f: consuming () -> Int64) -> Int64 {
    let a = f();
    let b = f(); // ERROR(E500)
    a + b
}

func main() -> lang.i64 { 0 }
