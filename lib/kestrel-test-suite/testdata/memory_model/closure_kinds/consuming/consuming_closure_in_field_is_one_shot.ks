// test: diagnostics
// stdlib: true

// ============================================================================
// A `consuming` closure is ONE-SHOT: "the call consumes the value, and a
// second call is an ordinary use-after-move" (docs/design/closures.md, and
// plan D8's E500 extension site 1). Projected callees route through the same
// place-based move funnel as locals. MIR currently takes the whole holder when
// moving out its non-Copyable closure field, so the diagnostic remains rooted
// at `h` until MIR supports partial initialization and drop tracking.
// ============================================================================
module Test

import std.numeric.Int64

struct Holder: not Copyable {
    var f: consuming () -> Int64
}

@main
func main() -> lang.i64 {
    var h = Holder(f: { () in 5 });
    let a = (h.f)();
    let b = (h.f)(); // ERROR: use of moved value 'h'
    if a != 5 { return 1 }
    if b != 5 { return 2 }
    0
}
