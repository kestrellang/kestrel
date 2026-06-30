// test: execution
// stdlib: true
// expect-stdout: -128|-32768|-2147483648|-9223372036854775808

// Regression (#156): formatting a signed integer's `minValue` produced
// mirrored garbage bytes (below '0') in every radix. The negative branch
// used `n.negate()`, which on a two's-complement minimum overflows back to
// the same negative value, so the digit-extraction `% radix` yielded
// negative remainders that mapped to bytes below '0'. The fix converts to
// the unsigned magnitude (`UIntN.zero - UIntN(from: n)`) before extracting
// digits. Covers all four signed widths.

module Test

@main
func main() {
    print("\(Int8.minValue)|\(Int16.minValue)|\(Int32.minValue)|\(Int64.minValue)");
}
