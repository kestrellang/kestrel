// test: execution
// stdlib: true
// expect-stdout: 1 18446744073709551614\n2851976156094479555 3079830396031989985\n15 0\n1 0\n0 -1\n-9223372036854775805 -2\n1 4294967294\n0 1073741824\n1 65534\n-9464 -2\n1 254\n-128 -64\n

module Test

// `multiplyFull` returns the double-width product as `(low, high)`: `low`
// is the wrapped product and `high` the bits above it, sign-extended for
// signed types. Expected values come from Python's big integers.

func show[T](pair: (T, T)) where T: Formattable {
    let (low, high) = pair;
    print("\(low) \(high)\n");
}

@main
func main() {
    let max64: UInt64 = 18446744073709551615;
    show(max64.multiplyFull(max64));
    let a: UInt64 = 11400714819323198485;
    show(a.multiplyFull(4983270260364809079));
    let three: UInt64 = 3;
    show(three.multiplyFull(5));

    let minusOne: Int64 = -1;
    show(minusOne.multiplyFull(-1));
    show(Int64.minValue.multiplyFull(2));
    show(Int64.maxValue.multiplyFull(-3));

    let max32: UInt32 = 4294967295;
    show(max32.multiplyFull(max32));
    show(Int32.minValue.multiplyFull(Int32.minValue));
    let max16: UInt16 = 65535;
    show(max16.multiplyFull(max16));
    let n: Int16 = -300;
    show(n.multiplyFull(250));
    let max8: UInt8 = 255;
    show(max8.multiplyFull(max8));
    show(Int8.minValue.multiplyFull(Int8.maxValue));
}
