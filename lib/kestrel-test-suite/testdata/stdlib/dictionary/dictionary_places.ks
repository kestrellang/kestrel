// test: execution
// stdlib: true
// expect-stdout: 1 102 0 5 -1 12 102 2 7 | b=5 | 1 9 -1 9 None a | 4 9 200 None\n

// Dictionary lookups are places that never insert or remove:
// - `d(key)` lends `Optional[&V]`; a `&mutating` binder updates in place.
// - `d(key, default: v)` reads without storing; a write inserts `v` first.
// - `d(unwrap: key)` lends `&V` and panics on a missing key.
// - `first(where:)` lends `(&K, &V)`.
// Set lookups and `xs(wrapped: i)` lend the same way.
module Test

import std.collections.(Array, Dictionary, Set)

@main
func main() -> lang.i64 {
    var d: Dictionary[String, Int64] = ["a": 1, "b": 2];
    let a = d("a");
    if let .Some(&mutating v) = d("b") { v += 10; }
    d("c", default: 0) += 5;
    d("a", default: 0) += 1;
    d(unwrap: "a") += 100;
    let copy = d;
    d(unwrap: "b") = 0;
    var g: Dictionary[String, Array[Int64]] = [:];
    g("x", default: []).append(1);
    g("x", default: []).append(2);
    print("\(a ?? 0) \(d("a") ?? 0) \(d("b") ?? 0) \(d("c") ?? 0) \(d("z") ?? -1) \(copy(unwrap: "b")) \(d(unwrap: "a")) \(g(unwrap: "x").count) \(d("q", default: 7))");

    let five: Dictionary[String, Int64] = ["a": 1, "b": 5];
    match five.find(where: { (k, v) in v > 2 }) {
        .Some((k, v)) => { print(" | \(k)=\(v)"); },
        .None => {},
    }

    let s: Set[Int64] = [3, 1, 4, 9, 5];
    let e = Set[Int64]();
    let names: Set[String] = ["b", "a"];
    print(" | \(s.min() ?? 0) \(s.max() ?? 0) \(e.min() ?? -1) \(s.find(where: { it > 8 }) ?? 0) \(s.find(where: { it > 80 })) \(names.min() ?? "")");

    var xs = [1, 2, 3, 4];
    let empty: [Int64] = [];
    let w = xs(wrapped: -1);
    if let .Some(&mutating v) = xs(wrapped: 5) { v = 200; }
    if let .Some(&mutating v) = xs(0..<2)(wrapped: -2) { v = 9; }
    print(" | \(w ?? 0) \(xs(0)) \(xs(1)) \(empty(wrapped: 3))\n");
    0
}
