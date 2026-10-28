// test: execution
// stdlib: true
// expect-exit: 0

// Pins the `escaping` / `mutating` / `consuming` kind prefix parsing and
// binding in a local annotation position, plus the semantics each kind
// promises there: escaping snapshots (owning capture), mutating writes back
// through frame views and lives in a `var`, consuming runs exactly once.
// See docs/design/closures.md — "The Four Kinds", "Capture Rules".
module Test

import std.numeric.(Int64)

@main
func main() -> lang.i64 {
    // escaping: owning capture — snapshots `base` at creation time.
    var base = 10;
    let snapshot: escaping () -> Int64 = { base };
    base = 20;
    if snapshot() != 10 { return 1 }

    // mutating: `&mutating` frame views — assignments hit `total`. Calls are
    // exclusive, so the closure is held in a `var`.
    var total = 0;
    var bump: mutating () -> () = { total = total + 5; };
    bump();
    bump();
    if total != 10 { return 2 }

    // consuming: owns its captures; the single call consumes the closure.
    let seed = 7;
    let once: consuming () -> Int64 = { seed };
    if once() != 7 { return 3 }

    0
}
