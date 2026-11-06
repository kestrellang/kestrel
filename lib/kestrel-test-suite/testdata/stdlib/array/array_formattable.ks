// test: execution
// stdlib: true
// expect-stdout: [1, 2, 3]\n[1, 2, 3] and [[1], [2, 3]]\n[]\n[x, y]\n[1, 2]\n

module Test

// Array and Slice conform to Formattable, so they print and interpolate
// (this was E107 `Array[Int64] !: Formattable`).
@main
func main() {
    let a = [1, 2, 3];
    println(a);
    println("\(a) and \([[1], [2, 3]])");
    let e: Array[Int64] = [];
    println(e);
    println("\(["x", "y"])");
    println(a(0..<2).formatted());
}
