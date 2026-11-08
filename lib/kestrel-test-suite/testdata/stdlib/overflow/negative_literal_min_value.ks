// test: execution
// stdlib: true
// expect-stdout: -128 -32768 -9223372036854775808 -5\n

module Test

// `-128` is a negative literal, not `negate(128)`: 128 doesn't fit in
// `Int8`, and negating `Int8.minValue` traps when overflow checks are on.
@main
func main() {
    let a: Int8 = -128;
    let b: Int16 = -32768;
    let c: Int64 = -9223372036854775808;
    let d: Int32 = -5;
    print("\(a) \(b) \(c) \(d)\n");
}
