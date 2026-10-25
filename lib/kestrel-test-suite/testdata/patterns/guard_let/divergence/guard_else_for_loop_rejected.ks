// test: diagnostics
// stdlib: true
//
// Pinning test for the Stage 4 `Sugar` arm (G11). A `for` in a guard-else does
// not diverge — the loop ends and control falls through the guard.
//
// It is rejected today for the *wrong* reason: `guard.rs`'s `expr_diverges`
// has no `Sugar` arm, so the whole desugared `for` subtree is invisible and
// falls to `_ => false`. Once the `Sugar` arm lands, the desugared `Loop`
// becomes visible and the break check (G8) must be what rejects it.
//
// If this test starts failing after the `Sugar` arm is added, the arm is
// wrong, not the test.

module Main

import std.numeric.Int64
import std.core.Range

func test(x: Int64, xs: Range[Int64]) -> Int64 {
    guard x > 0 else {
        for i in xs { } // ERROR: guard else block must diverge
    }
    99
}
