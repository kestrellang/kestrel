// test: execution
// stdlib: true
// expect-stdout: wrap -9223372036854775808 0 144\nchecked true true true\nsaturate 9223372036854775807 0 -128\noverflowing -9223372036854775808 true 4 false\npow 1024 true 0 9223372036854775807\nrotate 128 128\nsigned -9223372036854775808 -9223372036854775808 -9223372036854775808\ndistance -5 5\nsum 45\n

module Test

// Each overflow policy is spelled out at the call site: Wrapping keeps
// the low bits, Checked returns None, Saturating clamps, and
// Overflowing returns the wrapped value plus whether it overflowed.
@main
func main() {
    let m = Int64.maxValue;
    let zero: UInt8 = 0;
    let big: UInt8 = 200;
    print("wrap \(m.addWrapping(1)) \(zero.subtractWrapping(0)) \(big.multiplyWrapping(2))\n");
    print("checked \(m.addChecked(1).isNone) \(zero.subtractChecked(1).isNone) \(big.multiplyChecked(2).isNone)\n");
    let lo: Int8 = -100;
    print("saturate \(m.addSaturating(1)) \(zero.subtractSaturating(1)) \(lo.subtractSaturating(100))\n");
    let (w, o) = m.addOverflowing(1);
    let (w2, o2) = (2).addOverflowing(2);
    print("overflowing \(w) \(o) \(w2) \(o2)\n");
    let two: Int64 = 2;
    print("pow \(two.pow(10)) \(two.powChecked(63).isNone) \(two.powWrapping(64)) \((-2).powSaturating(64))\n");
    let one: UInt8 = 1;
    print("rotate \(one.rotateLeft(by: -1)) \(one.rotateRight(by: 9))\n");
    print("signed \(Int64.minValue.absWrapping()) \(Int64.minValue.negateWrapping()) \(Int64.minValue.divideWrapping(-1))\n");
    let a: UInt64 = 0;
    let b: UInt64 = 5;
    print("distance \(b.distance(to: a)) \(a.distance(to: b))\n");
    var s: Int64 = 0;
    for i in 0..<10 { s += i }
    print("sum \(s)\n");
}
