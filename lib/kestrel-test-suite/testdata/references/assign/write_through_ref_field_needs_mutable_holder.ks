// test: diagnostics
// stdlib: true

// A `&mutating` stored field gives no interior mutability: writing through
// it is a write to the holder, so a borrowing method or a `let` binding can
// only read through it. These writes used to be accepted.

module Test

import std.collections.(Array)

struct Point { var x: Int64 }

struct View: not Static {
    let base: &mutating Array[Int64]
    let p: &mutating Point
}

extend View {
    func subscriptWrite() { self.base(0) = 1; }     // ERROR(E201)
    func fieldWrite() { self.p.x = 2; }             // ERROR(E201)
    func methodCall() { self.base.append(3); }      // ERROR(E203)
    func wholeWrite() { self.p = Point(x: 4); }     // ERROR(E201)

    mutating func allowed() {
        self.base(0) = 1;
        self.p.x = 2;
        self.base.append(3);
        self.p = Point(x: 4);
    }
}

func viaParam(v: View) {
    v.base(0) = 5;                                  // ERROR(E201)
    v.p.x = 6;                                      // ERROR(E201)
    v.base.append(7);                               // ERROR(E203)
}

func viaConsuming(consuming v: View) {
    v.base(0) = 5;
    v.p.x = 6;
    v.base.append(7);
}
