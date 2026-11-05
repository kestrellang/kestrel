// test: diagnostics
// stdlib: true

// `d(key)` is a failable place: it lends or updates an existing value but
// never inserts or removes. Insert with `d.insert(key, v)` or
// `d(key, default: v)`; remove with `d.remove(key)`.
module Test

import std.collections.(Dictionary)

@main
func main() {
    var d: Dictionary[String, Int64] = [:];
    d("a") = 3; // ERROR(E214)
    d("a") = null; // ERROR(E214)
    d.insert("a", 3);
    d("b", default: 0) = 4;
}
