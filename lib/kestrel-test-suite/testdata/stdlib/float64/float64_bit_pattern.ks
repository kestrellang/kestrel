// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// `bitPattern` / `init(bitPattern:)` expose the exact IEEE-754 bits via the new
// f64_to_bits / f64_from_bits intrinsics (pure bitcast, no value conversion).

module Test

import std.numeric.Float64
import std.numeric.UInt64

func f(v: Float64) -> Float64 { v }

@main
func main() -> lang.i32 {
    // 1.0 = 0x3FF0000000000000
    if f(1.0).bitPattern != UInt64(from: 4607182418800017408) { return 1 };
    // 0.5 = 0x3FE0000000000000
    if f(0.5).bitPattern != UInt64(from: 4602678819172646912) { return 2 };
    // +0.0 has all-zero bits
    if f(0.0).bitPattern != UInt64.zero { return 3 };
    // round-trip bits -> float -> bits
    let back = Float64(bitPattern: UInt64(from: 4607182418800017408));
    if back != f(1.0) { return 4 };
    // value -> bits -> value round-trips for an arbitrary value
    let pi = f(3.14159);
    if Float64(bitPattern: pi.bitPattern) != pi { return 5 };
    0
}
