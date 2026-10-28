// test: execution
// stdlib: true

// `swap(a, b)` exchanges two locations with three bitwise moves (via
// `Pointer.take` + `write`) — no clone, no drop, for any non-Copyable type.
// Covers: locals and struct fields, value correctness, the no-clone /
// no-double-free / no-leak invariant (a `deinit`+`clone` counter), the
// self-swap guard, and a Copyable type.

module Test

import std.memory.(swap)
import std.numeric.(Int64)

var clones: Int64 = 0;
var drops: Int64 = 0;

struct R: Cloneable {
    var id: Int64;
    func clone() -> R { clones = clones + 1; R(id: self.id) }
    deinit { drops = drops + 1; }
}

struct Pair { var a: R; var b: R; }

// Swapping two locals: values exchange, nothing is cloned or dropped here.
func swapLocals() -> Int64 {
    var a = R(id: 1);
    var b = R(id: 2);
    swap(a, b);
    if a.id != 2 { return 11 }
    if b.id != 1 { return 12 }
    if clones != 0 { return 13 }
    if drops != 0 { return 14 }
    return 0
}  // a, b drop here

// Swapping two struct fields must also avoid cloning (no copy-in/out of the
// `mutating` args).
func swapFields() -> Int64 {
    var p = Pair(a: R(id: 1), b: R(id: 2));
    swap(p.a, p.b);
    if p.a.id != 2 { return 21 }
    if p.b.id != 1 { return 22 }
    if clones != 0 { return 23 }
    return 0
}  // p (a:2, b:1) drops here

@main
func main() -> Int64 {
    let r1 = swapLocals();
    if r1 != 0 { return r1 }
    if drops != 2 { return 30 + drops }   // exactly two locals dropped once each
    clones = 0; drops = 0;

    let r2 = swapFields();
    if r2 != 0 { return r2 }
    if drops != 2 { return 40 + drops }
    clones = 0; drops = 0;

    // Self-swap must be a no-op (not a double-take that double-owns/leaks).
    var s = R(id: 7);
    swap(s, s);
    if s.id != 7 { return 51 }
    if clones != 0 { return 52 }
    if drops != 0 { return 53 }

    // Copyable element type works too.
    var x: Int64 = 10;
    var y: Int64 = 20;
    swap(x, y);
    if x != 20 { return 61 }
    if y != 10 { return 62 }

    return 0
}
