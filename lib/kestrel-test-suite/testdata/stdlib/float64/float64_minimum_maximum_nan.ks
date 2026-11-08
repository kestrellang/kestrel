// test: execution
// stdlib: true
// expect-stdout: 1 2 true true\ntrue false true\n-0 0 true true\n

module Test

// `minimum(other:)` / `maximum(other:)` are IEEE 754-2019 `minimum` and
// `maximum`: a NaN on either side gives NaN, and -0.0 is below +0.0.
@main
func main() {
    let one: Float64 = 1.0;
    let two: Float64 = 2.0;
    let nan = Float64.nan;
    print("\(one.minimum(two)) \(one.maximum(two)) \(nan.minimum(one).isNaN) \(one.maximum(nan).isNaN)\n");
    let neg: Float64 = -0.0;
    let pos: Float64 = 0.0;
    print("\(neg.isSignMinus) \(pos.isSignMinus) \((-one).isSignMinus)\n");
    print("\(pos.minimum(neg)) \(neg.maximum(pos)) \(pos.minimum(neg).isSignMinus) \(not neg.maximum(pos).isSignMinus)\n");
}
