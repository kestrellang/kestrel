// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

module Test

// `Iterable.iter()` borrows the collection, and several iterators point into
// their collection's storage (Dictionary/Set buckets, a split's source). A
// `for` over a TEMPORARY collection must keep it alive for the whole loop —
// the desugaring binds it to a hidden local scoped to the loop. Heap-owning
// String elements make a premature free visible.

import std.collections.(Dictionary, Set)

func makeDict() -> Dictionary[String, String] {
    var d = Dictionary[String, String]();
    d.insert("alpha-\(1)", "one-\(1)");
    d.insert("beta-\(2)", "two-\(2)");
    d
}

func makeSet() -> Set[String] {
    var s = Set[String]();
    s.insert("gamma-\(3)");
    s.insert("delta-\(4)");
    s
}

@main
func main() -> Int64 {
    var dictBytes = 0;
    for (k, v) in makeDict() { dictBytes += k.bytes.count + v.bytes.count; }
    if dictBytes != 7 + 5 + 6 + 5 { return 1 }

    var setBytes = 0;
    for x in makeSet() { setBytes += x.bytes.count; }
    if setBytes != 14 { return 2 }

    var parts = 0;
    for p in "a&bb&ccc".split("&") { parts += p.bytes.count; }
    if parts != 6 { return 3 }
    0
}
