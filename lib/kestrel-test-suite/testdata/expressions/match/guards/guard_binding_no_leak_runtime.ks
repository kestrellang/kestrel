// test: execution
// stdlib: true

// Regression for #121: a `match` arm guard that BINDS a value must drop the
// arm's bound payload and the guard's own @owned temporaries (boxed literals,
// the boolean result) on BOTH the guard-true and guard-false edges, and must
// not leak the match's threaded slots at the merge. Before the fix the guard
// branch and the post-match merge left @owned values live at block exit (OSSA
// "@owned value live at block exit but never consumed"). The arms exercise a
// guard-true match, guard-false fall-through to a later arm, a heap (String)
// binding dropped on the guard-false edge, and the no-match (null) arm.
//
// The existing `expressions/match/guards/*` tests are diagnostics-only, so they
// never lowered to MIR/OSSA and could not catch this leak; this one runs.
module Test

// Chained guards over an `Int64` binding: the boxed literal + comparison Bool
// are the temporaries that leaked. `200`/`50`/`5`/`null` promote to Optional.
func classify(opt: std.result.Optional[std.numeric.Int64]) -> std.numeric.Int64 {
    match opt {
        some x if x > 100 => 3,
        some x if x > 10 => 2,
        some x => x,
        null => 0,
    }
}

// The bound `s` is a heap String; on the guard-false edge it must be dropped,
// not leaked.
func describe(name: std.text.String) -> std.numeric.Int64 {
    match name {
        s if s == "bob" => 1,
        s => 0,
    }
}

@main
func main() -> lang.i64 {
    // First guard true.
    if classify(200) != 3 { return 1 }
    // First guard false → second guard true.
    if classify(50) != 2 { return 2 }
    // Both guards false → bind and return x.
    if classify(5) != 5 { return 3 }
    // No payload → null arm.
    if classify(null) != 0 { return 4 }
    // Guard true on a heap binding.
    if describe("bob") != 1 { return 5 }
    // Guard false → String binding dropped, fall through.
    if describe("alice") != 0 { return 6 }
    0
}
