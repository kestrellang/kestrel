// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/closures.md — the `Button` example: an `escaping` closure field
// is storable long-term (no E494 on the struct) and makes the aggregate
// Cloneable, so copying the struct SHARES the environment handle instead of
// bit-copying it. The copy therefore observes the original's mutations.
module Test

import std.numeric.Int64

struct Button {
    let onClick: escaping () -> Int64   // storable long-term; Button is Cloneable
}

func makeButton() -> Button {
    var clicks = 0;
    Button(onClick: { clicks = clicks + 1; clicks })
}

@main
func main() -> lang.i64 {
    let b = makeButton();               // callable long after makeButton returned
    let b2 = b;                         // struct copy shares (retains) the environment
    if (b.onClick)() != 1 { return 1 }
    if (b2.onClick)() != 2 { return 2 } // alias observes b's mutation
    if (b.onClick)() != 3 { return 3 }  // one environment, two Buttons
    0
}
