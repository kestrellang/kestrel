// test: execution
// stdlib: true
// expect-exit: 0

module Test

// A literal passed to `T(from:)` could spell several of `T`'s Convertible
// sources; it takes `T` itself (the identity conversion) rather than the
// Int64 default, which `UInt64` does not accept losslessly.
@main
func main() -> lang.i64 {
    let n: Int64 = 3;
    let a = UInt64(from: 1) << n;
    if a != 8 { return 1 }
    let b = UInt8(from: 200);
    if b != 200 { return 2 }
    let c = Float32(from: 1.5);
    if c != 1.5 { return 3 }
    0
}
