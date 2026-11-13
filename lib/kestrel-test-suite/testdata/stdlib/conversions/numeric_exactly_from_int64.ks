// test: execution
// stdlib: true
// expect-stdout: true -5 true 1.6777216e7 true 7\n

module Test

import std.numeric.(Numeric)

// `Numeric.init(exactly: Int64)?` gives generic code one way to turn a count
// into `T`: `nil` when the value doesn't fit, or (for a float) would round.
func conv[T](n: Int64) -> T? where T: Numeric { T(exactly: n) }

@main
func main() {
    let a: Int8? = conv(300);
    let b: Int8? = conv(-5);
    let c: Float64? = conv(9_007_199_254_740_993);   // 2^53 + 1 rounds
    let d: Float32? = conv(16_777_216);              // 2^24 is exact
    let e: UInt64? = conv(-1);
    let f: Int64? = conv(7);
    println("\(a == null) \(b!) \(c == null) \(d!) \(e == null) \(f!)");
}
