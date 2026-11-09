// test: diagnostics
// stdlib: true

// `d(key)` declares a setter taking `Optional[V]`, so plain assignment
// type-checks: a value inserts or replaces and `null` removes. The other
// write forms stay valid alongside it.
module Test

import std.collections.(Dictionary)

@main
func main() {
    var d: Dictionary[String, Int64] = [:];
    d("a") = 3;
    d("a") = null;
    d.insert("a", 3);
    d("b", default: 0) = 4;
}
