// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#206): a shift by >= bit width was bare native `shl`/`ashr`/`lshr`.
// Cranelift masks the amount mod width (defined); LLVM made it poison (garbage/0),
// so the backends diverged. Spec decision: mask the amount mod bit width
// (`amt & (width-1)`) — both backends now do this. Shift amounts come from an
// opaque function so the optimizer can't see through to a constant.

module Test

import std.numeric.Int64
import std.numeric.UInt64

func opaque(n: Int64) -> Int64 { var arr: [Int64] = [n]; arr(0) }

@main
func main() -> lang.i64 {
    let one: Int64 = 1;
    if one << opaque(64) != 1 { return 1 }    // 64 & 63 = 0
    if one << opaque(65) != 2 { return 2 }    // 65 & 63 = 1

    let max: Int64 = Int64.maxValue;
    if max >> opaque(64) != max { return 3 }  // 64 & 63 = 0

    let uone: UInt64 = 1;
    if uone << opaque(70) != 64 { return 4 }  // 70 & 63 = 6
    0
}
