// test: diagnostics
// stdlib: false

// The trailing `let y` genuinely never executes: `break` is `!`-typed whether
// or not it stands in a loop, so dead-code analysis warns on it (G12 dropped
// the `in_loop` carve-out that used to hide this). The `break`-outside-a-loop
// error is a separate, earlier hir-lower diagnostic and is unaffected.

module Main

func test() {
    let x: lang.i64 = 1;
    break; // ERROR: outside of loop
    let y: lang.i64 = 2; // WARN: unreachable
}
