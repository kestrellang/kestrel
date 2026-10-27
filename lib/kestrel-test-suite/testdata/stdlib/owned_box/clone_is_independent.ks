// test: execution
// stdlib: true
// expect-exit: 0
//
// `OwnedBox` is Cloneable, never bitwise: copying one allocates a new block
// holding a copy of the payload. Its `Cloneable where …` extension used to
// read as a conditional *Copyable* grant, so `OwnedBox[Int64]` was bit-copied
// and both copies freed the same block.

module Test

import std.numeric.(Int64)
import std.text.(String)
import std.memory.(OwnedBox)

var drops: Int64 = 0;

struct Tracked: Cloneable {
    var n: Int64
    deinit { drops = drops + 1; }
    func clone() -> Tracked { Tracked(n: self.n + 100) }
}

// Implicit copies clone: each copy owns its own block.
func implicitCopies() -> Int64 {
    var a = OwnedBox(5);
    let b = a;
    let c = a;
    a.pointeeMutRef() = 6;
    if b.pointeeRef() != 5 { return 1 }
    if c.pointeeRef() != 5 { return 2 }
    if a.pointeeRef() != 6 { return 3 }
    0
}

// An explicit clone is independent of the original.
func explicitClone() -> Int64 {
    let a = OwnedBox(5);
    var b = a.clone();
    b.pointeeMutRef() = 7;
    if a.pointeeRef() != 5 { return 1 }
    if b.pointeeRef() != 7 { return 2 }
    0
}

// A Cloneable payload is copied with its own `clone()`, exactly once.
func cloneablePayload() -> Int64 {
    let x = OwnedBox(Tracked(n: 1));
    let y = x;
    if x.pointeeRef().n != 1 { return 1 }
    if y.pointeeRef().n != 101 { return 2 }
    let s = OwnedBox("hello");
    let t = s;
    if t.pointeeRef() != "hello" { return 3 }
    0
}

@main
func main() -> Int64 {
    let r1 = implicitCopies();
    if r1 != 0 { return 10 + r1 }
    let r2 = explicitClone();
    if r2 != 0 { return 20 + r2 }
    let r3 = cloneablePayload();
    if r3 != 0 { return 30 + r3 }
    // Both boxes' payloads dropped, once each.
    if drops != 2 { return 40 + drops }
    0
}
