// test: execution
// stdlib: true
// expect-stdout: [fig, pear, kiwi, date, plum, apple, banana]\n[9, 7, 5, 3, 3, 2, 1] [5, 3, 9, 1, 7, 3, 2]\ntrue false\nabcd\n

module Test

// sort(byKey:) keys each element once and keeps ties in their original
// order; isSorted(byKey:) and Heap.pop give the same results as before.
@main
func main() {
    var w = ["pear", "fig", "banana", "kiwi", "apple", "date", "plum"];
    w.sort(byKey: { (s) in s.bytes.count });
    println(w);
    let v = [5, 3, 9, 1, 7, 3, 2];
    println("\(v.sorted(byKey: { (x) in 0 - x })) \(v)");
    println("\(w.iter().isSorted(byKey: { (s) in s.bytes.count })) \(v.iter().isSorted(byKey: { (x) in x }))");
    var h = Heap(from: ["d", "a", "c", "b"]);
    while let .Some(x) = h.pop() { print(x) }
    println("");
}
