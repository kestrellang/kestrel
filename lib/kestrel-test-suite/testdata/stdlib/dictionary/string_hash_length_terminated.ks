// test: execution
// stdlib: true
// expect-stdout: true true\n1999999000000\n

module Test

// A String hashes its bytes and then its length, so ["ab", "c"] and
// ["a", "bc"] (the same bytes split differently) hash apart. Integers
// hash as one word; a large Int64-keyed dictionary must still round-trip.
@main
func main() {
    var h1 = DefaultHasher();
    ["ab", "c"].hash(into: h1);
    var h2 = DefaultHasher();
    ["a", "bc"].hash(into: h2);
    var h5 = DefaultHasher();
    let one: Int64 = 1;
    one.hash(into: h5);
    var h6 = DefaultHasher();
    let two: Int64 = 2;
    two.hash(into: h6);
    println("\(h1.finish() != h2.finish()) \(h5.finish() != h6.finish())");
    var d = Dictionary[Int64, Int64]();
    for i in 0..<2000000 { d.insert(i * 7919, i) }
    var s: Int64 = 0;
    for i in 0..<2000000 { s = s + d(i * 7919, default: 0) }
    println(s);
}
