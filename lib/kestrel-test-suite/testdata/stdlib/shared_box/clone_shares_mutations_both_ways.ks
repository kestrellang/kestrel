// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/shared-box.md: "Duplicating a handle (`clone`) shares the
// storage" — the handles are peers, not copies. A mutation performed through
// either handle's `sharedMutRef()` is visible through the other, in both
// directions. This is the reference-semantics guarantee escaping closures and
// (later) classes are built on.
module Test

import std.memory.(RcBox)
import std.numeric.(Int64)

struct Counter {
    var n: Int64
    mutating func bump(by delta: Int64) { self.n = self.n + delta; }
}

@main
func main() -> lang.i64 {
    let a = RcBox(Counter(n: 0));
    let b = a.clone();

    a.sharedMutRef().bump(by: 5);                 // mutating method through the projection
    let seenThroughB: Int64 = b.pointeeRef().n;
    if seenThroughB != 5 { return 1; }            // b observes a's mutation

    b.sharedMutRef().bump(by: 4);
    let seenThroughA: Int64 = a.pointeeRef().n;
    if seenThroughA != 9 { return 2; }            // a observes b's mutation

    if a.isIdentical(to: b) == false { return 3; }
    0
}
