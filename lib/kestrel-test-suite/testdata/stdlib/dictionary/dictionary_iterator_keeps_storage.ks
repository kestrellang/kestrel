// test: execution
// stdlib: true
// expect-stdout: 6 150 12\n1275\n6030 300 0\n5 10\n

module Test

import std.collections.(DictionaryIterator)

// Dictionary iterators and the keys/values views hold a share of the
// storage, like ArrayIterator. They once held a bare bucket pointer, so
// inserting while iterating (which resizes and frees the table), returning
// `d.iter()` from a function, or clearing the dictionary read freed memory.
func makeIter() -> DictionaryIterator[String, Int64] {
    var d = Dictionary[String, Int64]();
    for i in 0..<50 { d.insert("k\(i)", i) }
    d.iter()
}

@main
func main() {
    // Inserting while iterating: the loop sees the entries as they were.
    var d = Dictionary[Int64, Int64]();
    for i in 0..<6 { d.insert(i, i * 10) }
    var n = 0;
    var sum = 0;
    for (k, v) in d.iter() {
        d.insert(k + 1000, v);
        n = n + 1;
        sum = sum + v;
    }
    println("\(n) \(sum) \(d.count)");

    // An iterator that outlives its dictionary.
    var it = makeIter();
    var total = 0;
    while let .Some((k, v)) = it.next() { total = total + v + (if k.isEmpty { 0 } else { 1 }) }
    println(total);

    // Views made before a clear still see the old entries.
    let ks = d.keys;
    let vs = d.values;
    d.clear();
    var c = 0;
    for k in ks { c = c + k }
    var c2 = 0;
    for v in vs { c2 = c2 + v }
    println("\(c) \(c2) \(d.count)");

    // Set iterates through the same iterator.
    var s = Set[String]();
    for i in 0..<5 { s.insert("s\(i)") }
    var m = 0;
    for x in s.iter() { s.insert(x + "!"); m = m + 1 }
    println("\(m) \(s.count)");
}
