// test: execution
// stdlib: true
// expect-stdout: [3, 1, 2]\n[b, a, c]\n50000 0 49999\n[7]\n

module Test

// unique() keeps first occurrences in order. It tracks seen elements in a
// Set, so it's linear: 100k elements used to take seconds (O(n²)).
@main
func main() {
    println([3, 1, 3, 2, 1].unique());
    println(["b", "a", "b", "c", "a"].unique());
    var big = Array[Int64]();
    for i in 0..<100000 { big.append(i % 50000) }
    let u = big.unique();
    println("\(u.count) \(u(0)) \(u(49999))");
    let r = [7, 7, 7];
    println(r.unique());
}
