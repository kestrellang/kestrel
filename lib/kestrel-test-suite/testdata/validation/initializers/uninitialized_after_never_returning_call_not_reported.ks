// test: diagnostics
// stdlib: true
//
// G12: definite-assignment divergence for a `-> !` call had no coverage at all.
// The first function pins that E004 still fires on a genuinely reachable read;
// the second pins that a read *after* a diverging call does not, because the
// statement never executes (dead code warns on it instead).

module Main

func boom() -> ! {
    fatalError("boom");
}

func reachableRead() {
    var x: Int64;
    let y = x; // ERROR: access to uninitialized variable 'x'
}

func readAfterDivergingCall() {
    var x: Int64;
    boom();
    let y = x; // WARN: unreachable
}
