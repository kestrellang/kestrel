// test: execution
// stdlib: true
// expect-stdout: 2.5 3\n3.5 7\n13\n5.0 1.4142135\n7 false\n

module Test

import std.numeric.(Numeric, SignedNumeric, Integer, SignedInteger, UnsignedInteger, FloatingPoint)

// The numeric protocol family lets one generic body serve every number
// type: `T: Numeric` pins each operator's `Output` to `T`, so sums and
// quotients need no casts.
func mean[T](xs: &Slice[T]) -> T where T: Numeric, T: Convertible[Int64] {
    var total = T.zero;
    for x in xs { total = total + x; }
    total / T(from: xs.count)
}

func magnitude[T](x: T) -> T where T: SignedNumeric { x.abs() }

func lowNibble[T](x: T) -> T where T: Integer { (x & ((T.one << 4) - T.one)) % T.maxValue }

func hypot[T](a: T, b: T) -> T where T: FloatingPoint, T: Copyable { (a * a + b * b).sqrt() }

func negated[T](x: T) -> T where T: SignedInteger { -x }

func isUnsigned[T](x: T) -> Bool where T: UnsignedInteger { true }

@main
func main() {
    let floats: [Float64] = [1.0, 2.0, 4.5];
    let ints: [Int32] = [1, 2, 6];
    println("\(mean(floats.asSlice())) \(mean(ints.asSlice()))");
    let n: Int32 = -7;
    println("\(magnitude(-3.5)) \(magnitude(n))");
    let k: UInt16 = 0xABCD;
    println(lowNibble(k));
    let f: Float32 = 1.0;
    println("\(hypot(3.0, 4.0)) \(hypot(f, f))");
    println("\(negated(n)) \(Float64.infinity.isFinite)");
    let _ = isUnsigned(k);
}
