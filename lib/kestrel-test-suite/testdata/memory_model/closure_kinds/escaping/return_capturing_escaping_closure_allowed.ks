// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/closures.md — "Behavior Changes" #4: returning a CAPTURING
// closure is legal once the return type spells `escaping`; the owning
// environment is self-rooted, so the provenance escape check (E494) must NOT
// fire here. Both captures are read-only Copyable snapshots.
module Test

import std.numeric.Int64

func adder(base: Int64) -> escaping () -> Int64 {
    let bonus = 5;
    { base + bonus }         // owns copies of `base` and `bonus`; outlives this frame
}

@main
func main() -> lang.i64 {
    let g = adder(10);
    if g() != 15 { return 1 }
    if g() != 15 { return 2 }   // multi-call: escaping is not one-shot
    0
}
