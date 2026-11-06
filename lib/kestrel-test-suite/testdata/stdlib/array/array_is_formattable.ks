// test: execution
// stdlib: true
// expect-stdout: [1, 2, 3] [] [[1], [2, 3]]\n[4, 5]\n

// Array conforms to Formattable when its elements do, so it works in string
// interpolation and `println`. It used to have only a `format` method (from
// `extend Sliceable[T] where T: Formattable`) without the conformance.
module Test

@main
func main() -> lang.i64 {
    let xs = [1, 2, 3];
    let empty: [Int64] = [];
    let nested = [[1], [2, 3]];
    println("\(xs) \(empty) \(nested)");
    println([4, 5]);
    0
}
