// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: -1

// Regression (#158): division by zero must trap deterministically (int64.ks docs:
// "the process aborts before producing a result"). Cranelift trapped natively but
// LLVM `sdiv` by zero was UB (silently folded to 0) — the backends diverged. Both
// now trap on a zero divisor. A signal-killed process reports no exit code, which
// the harness records as -1, so this is the portable "it trapped" assertion. The
// quotient is used (compared) so the divide can't be eliminated as dead.

module Test

import std.numeric.Int64

func zero() -> Int64 { 0 }

@main
func main() -> lang.i64 {
    let a: Int64 = 5;
    let r = a / zero();      // traps here — control never reaches below
    if r != 0 { return 1 } else { return 2 }
}
