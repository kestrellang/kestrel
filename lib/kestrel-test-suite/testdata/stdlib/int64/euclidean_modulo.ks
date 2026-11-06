// test: execution
// stdlib: true

module Test

// `%` is a truncated remainder (sign of the dividend); `modulo(by:)` is
// Euclidean (never negative), including at the `minValue` edge.
@main
func main() -> lang.i64 {
    let a: Int64 = -7;
    if a % 3 != -1 { return 1 }
    if a.modulo(by: 3) != 2 { return 2 }
    if (7).modulo(by: -3) != 1 { return 3 }
    if a.modulo(by: -3) != 2 { return 4 }
    if (-1).modulo(by: Int64.minValue) != Int64.maxValue { return 5 }
    if Int64.minValue.modulo(by: -1) != 0 { return 6 }
    let b: Int8 = -128;
    if b.modulo(by: 3) != 1 { return 7 }
    let u: UInt8 = 10;
    if u.modulo(by: 3) != 1 { return 8 }
    var m: Int64 = -17;
    m %= 5;
    if m != -2 { return 9 }
    0
}
