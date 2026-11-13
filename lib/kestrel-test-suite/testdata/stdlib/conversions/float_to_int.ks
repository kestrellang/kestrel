// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

module Test

// Float to integer: `exactly:` is nil for a fraction, NaN, an infinity or
// out of range; `truncating:` and `clamping:` drop the fraction toward zero,
// saturate, and send NaN to 0. Both backends must agree (the LLVM cast is
// `fptosi.sat`), so the backends line is load-bearing.
@main
func main() -> lang.i64 {
    let a: Float64 = 3.7;
    let n: Float64 = -3.7;
    if Int64(exactly: a) != null { return 1 }
    if Int64(exactly: a.trunc())! != 3 { return 2 }
    if Int64(truncating: a) != 3 { return 3 }
    if Int64(truncating: n) != -3 { return 4 }
    if Int8(clamping: 300.0) != 127 { return 5 }
    if Int8(clamping: -300.0) != -128 { return 6 }
    if UInt8(clamping: -5.0) != 0 { return 7 }
    if UInt8(clamping: 300.0) != 255 { return 8 }
    if UInt8(truncating: Float64.nan) != 0 { return 9 }
    if UInt8(exactly: -0.0)! != 0 { return 10 }
    // UInt64's upper half doesn't fit an Int64 cast.
    if UInt64(clamping: 1.0e30) != UInt64.maxValue { return 11 }
    if UInt64(clamping: 1.0e19) != 10_000_000_000_000_000_000 { return 12 }
    if UInt64(clamping: -1.0) != 0 { return 13 }
    if UInt64(exactly: 18446744073709551616.0) != null { return 14 }
    // Bounds: 2^63 is just out of range, -2^63 is just in.
    if Int64(exactly: 9223372036854775808.0) != null { return 15 }
    if Int64(exactly: -9223372036854775808.0)! != Int64.minValue { return 16 }
    if Int64(clamping: Float64.infinity) != Int64.maxValue { return 17 }
    if Int64(truncating: -Float64.infinity) != Int64.minValue { return 18 }
    if Int64(exactly: Float64.nan) != null { return 19 }
    let f: Float32 = 2.5;
    if Int32(truncating: f) != 2 { return 20 }
    if UInt16(exactly: f) != null { return 21 }
    if UInt32(clamping: Float32.nan) != 0 { return 22 }
    0
}
