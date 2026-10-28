// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#159): signed `minValue / -1` overflows. The raw op was lowered to
// bare native `sdiv`/`srem` — Cranelift TRAPPED on it, LLVM made it UB (folded).
// The spec (int64.ks docs) is that it WRAPS: div → minValue, rem → 0. Both
// backends now swap the divisor -1→1 exactly on this overflow case so the native
// op yields min/1=min and min%1=0. Operands are routed through opaque functions
// so the optimizer can't constant-fold past the runtime op.

module Test

import std.numeric.Int64
import std.numeric.Int8

func li64(v: Int64) -> Int64 { v }
func li8(v: Int8) -> Int8 { v }

@main
func main() -> lang.i64 {
    let m = Int64.minValue;
    let negOne: Int64 = -1;
    if li64(m) / li64(negOne) != Int64.minValue { return 1 }
    if li64(m) % li64(negOne) != 0 { return 2 }

    let m8 = Int8.minValue;
    let n8: Int8 = -1;
    if li8(m8) / li8(n8) != Int8.minValue { return 3 }
    if li8(m8) % li8(n8) != 0 { return 4 }

    // Non-overflow signed division still works (no spurious swap).
    if li64(-84) / li64(negOne) != 84 { return 5 }
    0
}
