// Rosetta Code: Set
// https://rosettacode.org/wiki/Set
module Main

import std.collections.Set

@main
func main() {
    let a: Set[Int64] = [1, 2, 3, 4, 5];
    let b: Set[Int64] = [4, 5, 6, 7];
    let c: Set[Int64] = [1, 2];
    println("A = \(a.sorted())");
    println("B = \(b.sorted())");
    println("3 in A: \(a.contains(3))");
    println("A union B: \(a.union(b).sorted())");
    println("A intersection B: \(a.intersection(b).sorted())");
    println("A minus B: \(a.difference(b).sorted())");
    println("C subset of A: \(c.isSubset(of: a))");
    println("A == B: \(a == b)");
    var m = a;
    m.insert(42);
    m.remove(1);
    println("A after insert 42, remove 1: \(m.sorted())");
}
