// test: execution
// stdlib: true
// expect-exit: 0

module Test

// Integer to float: `from:` when every source value fits the mantissa
// exactly (up to 24 bits for Float32, 53 for Float64), `rounding:` otherwise.
// Every type also converts from itself.
@main
func main() -> lang.i64 {
    let i: Int32 = -7;
    if Float64(from: i) != -7.0 { return 1 }
    let s: Int16 = 300;
    if Float32(from: s) != 300.0 { return 2 }
    // 2^53 + 1 rounds to 2^53.
    let big: Int64 = 9_007_199_254_740_993;
    if Float64(rounding: big) != 9_007_199_254_740_992.0 { return 3 }
    // 2^24 + 1 rounds to 2^24 in Float32.
    let mid: Int32 = 16_777_217;
    if Float32(rounding: mid) != 16_777_216.0 { return 4 }
    let h: Float32 = 1.5;
    if Float64(from: h) != 1.5 { return 5 }
    let d: Float64 = 0.1;
    if Float32(rounding: d) != 0.1 { return 6 }
    if Float64(from: d) != d { return 7 }
    let n: Int64 = 5;
    if Int64(from: n) != 5 { return 8 }
    // UInt64 has no unsigned cast underneath: the upper half is halved with
    // a sticky bit, converted, and doubled, so it still rounds correctly.
    let top: UInt64 = 18_446_744_073_709_551_615;
    if Float64(rounding: top) != 18_446_744_073_709_551_616.0 { return 9 }
    let up: UInt64 = 9_223_372_036_854_777_857;     // 2^63 + 2049 rounds up
    if Float64(rounding: up) != 9_223_372_036_854_777_856.0 { return 10 }
    let tie: UInt64 = 9_223_372_036_854_776_832;    // 2^63 + 1024: tie, to even
    if Float64(rounding: tie) != 9_223_372_036_854_775_808.0 { return 11 }
    let small: UInt64 = 12_345;
    if Float32(rounding: small) != 12_345.0 { return 12 }
    0
}
